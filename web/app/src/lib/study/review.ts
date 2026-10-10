// SPEC-350 R2, R7, R10, A11, A12; ADR-361. The review's machine, a table as #663's wake lock is:
// every event passes through `step`, so the review's behaviour is the table's. The page keeps only
// which side is shown; the engine side keeps the card it showed, and every rating, bury and flag
// names that card. A gesture while a request is in flight finds no cell and fires nothing.
// SPEC-371 R9; ADR-382. An undo press asks the engine for an offer of the review's own last answer
// and writes nothing; the offer moves the review to `confirming`, where only the Undo action again
// writes, naming the offered card and step, and every other action keeps the answer and is not
// carried out.
// SPEC-383 R10, R13; ADR-397. The same press and confirmation reach the review's last bury or flag:
// the control names the change it would undo, a flag's own return offers the flag's undo, and a
// refused undo of a bury or a flag reads the change's notice, picked by the kind it pressed.
import { frameDocument } from '$lib/card/frame-document';
import { EngineError } from '$lib/engine/client';
import type { CardView, Clip, Counts, ErrorCode, Faces, Head, Rating, UndoOffer } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';
import { sideAfter, type Action, type Side } from '$lib/remote/actions';
import { statusText } from './refusal';

/** The schematic's states: a card loading, a side shown, a request in flight, an undo offered and
 * asking to be confirmed, a refusal, done. */
export type Phase = 'loading' | 'question' | 'answer' | 'busy' | 'confirming' | 'refused' | 'done';

/** What moves the machine: an action, keeping an offered answer, or the outcome of the request in
 * flight. */
export type ReviewEvent =
  | Action
  | 'keep'
  | 'view'
  | 'empty'
  | 'settled'
  | 'flagged'
  | 'not-shown'
  | 'refusal'
  | 'retry'
  | 'offered'
  | 'not-offered'
  | 'undo-refused';

/** The request a step asks for. */
export type Effect = 'none' | 'card' | 'rate' | 'bury' | 'flag' | 'offer' | 'undo' | 'replay';

/** The change an offer names: its card, the step the confirmation carries back and its text; an
 * answer's grade and the state it returns to (SPEC-371 R7), a bury's state, or what a flag's undo
 * does to the flag (SPEC-383 R9). */
export type Offer = Extract<UndoOffer, { offer: object }>['offer'];

/** The machine's state: its phase, and the side the review shows or returns to. */
export interface ReviewState {
  phase: Phase;
  side: Side;
}

/** One cell: the next phase (`side` is the side's own phase) and the request it asks for. A cell
 * that `keeps` carries out no action, so the side stays where it was. */
interface Cell {
  phase: Phase | 'side';
  effect: Effect;
  keeps?: true;
}

const RATE: Cell = { phase: 'busy', effect: 'rate' };
const BURY: Cell = { phase: 'busy', effect: 'bury' };
const FLAG: Cell = { phase: 'busy', effect: 'flag' };
/** An undo press asks for the offer, and writes nothing (SPEC-371 R9). */
const OFFER: Cell = { phase: 'busy', effect: 'offer' };
/** The confirmation of an offer, the one cell whose request is the undo (A21). */
const UNDO: Cell = { phase: 'busy', effect: 'undo' };
/** While an offer asks, keeping it and every other action return to the side, and that action is
 * not carried out. */
const KEEP: Cell = { phase: 'side', effect: 'none', keeps: true };
const NEXT: Cell = { phase: 'loading', effect: 'card' };
const REFUSED: Cell = { phase: 'refused', effect: 'none' };
/** Replay plays the side's replay clips and stays on the side (SPEC-350 R15). */
const REPLAY: Cell = { phase: 'side', effect: 'replay' };

/** Every cell the machine has. An event with no cell in its phase fires nothing. */
const TABLE: Record<Phase, Partial<Record<ReviewEvent, Cell>>> = {
  loading: {
    view: { phase: 'question', effect: 'none' },
    empty: { phase: 'done', effect: 'none' },
    refusal: REFUSED
  },
  question: { 'show-answer': { phase: 'answer', effect: 'none' }, undo: OFFER, bury: BURY, flag: FLAG, replay: REPLAY },
  answer: { again: RATE, good: RATE, undo: OFFER, bury: BURY, flag: FLAG, replay: REPLAY },
  busy: {
    settled: NEXT,
    'not-shown': NEXT,
    flagged: { phase: 'side', effect: 'none' },
    refusal: REFUSED,
    offered: { phase: 'confirming', effect: 'none' },
    'not-offered': { phase: 'side', effect: 'none' },
    'undo-refused': NEXT
  },
  confirming: {
    undo: UNDO,
    keep: KEEP,
    'show-answer': KEEP,
    again: KEEP,
    good: KEEP,
    bury: KEEP,
    flag: KEEP,
    replay: KEEP
  },
  refused: { retry: NEXT },
  done: {}
};

/** The state `event` moves `state` to, and the request it asks for. */
export function step(state: ReviewState, event: ReviewEvent): { state: ReviewState; effect: Effect } {
  const cell = TABLE[state.phase][event];
  if (cell === undefined) return { state, effect: 'none' };
  // a new card shows its question; an action moves the side as the remote's reviewer does, and an
  // action that is kept rather than carried out moves nothing
  const side = event === 'view' ? 'question' : cell.keeps ? state.side : sideAfter(event as Action, state.side);
  return { state: { phase: cell.phase === 'side' ? side : cell.phase, side }, effect: cell.effect };
}

/** The events that show a side for the first time: a new card's question, and the revealed answer.
 * Only these play the side's autoplay clips (SPEC-350 R15). */
const AUTOPLAY: readonly ReviewEvent[] = ['view', 'show-answer'];

/** The effects the page carries out itself, with no request to the engine. */
const LOCAL: readonly Effect[] = ['none', 'replay'];

/** The two grades, as the wire numbers them. */
const RATING: Partial<Record<Action, Rating>> = { again: 1, good: 3 };

/** The view's undo values the control offers: the review's last answer, bury or flag, while it has
 * not synced (SPEC-371 R7, SPEC-383 R10). */
const OFFERED: readonly CardView['undo'][] = ['answer', 'bury', 'flag'];

/** The control's words, by what the view would undo: a synced answer keeps the answer's, and a
 * synced bury or flag reads the plain "Undo" (SPEC-383 R10). */
const LABELS: Record<NonNullable<CardView['undo']>, () => string> = {
  answer: () => m.study_undo_answer(),
  synced: () => m.study_undo_answer(),
  bury: () => m.study_undo_bury(),
  flag: () => m.study_undo_flag(),
  'change-synced': () => m.study_undo()
};

/** The view's undo values whose refusal reads the change's notice, not the answer's (SPEC-383 R13). */
const CHANGES: readonly CardView['undo'][] = ['bury', 'flag', 'change-synced'];

/** A refused undo of a bury or a flag, by its notice: it has synced, or something changed after it
 * (SPEC-383 R13). */
const CHANGE_NOTICES: Partial<Record<Status, () => string>> = {
  'undo-synced': () => m.undo_change_synced(),
  'not-undoable': () => m.undo_change_gone()
};

/** What the review asks of the engine: the study operations, and nothing else (R2, A17). */
export interface StudyClient {
  card(): Promise<Head>;
  rate(card: bigint, rating: Rating, ms: number): Promise<null>;
  bury(card: bigint): Promise<null>;
  flag(card: bigint): Promise<number>;
  /** Reverts the answer an offer named, by its card and its step (SPEC-371 R12). */
  undo(card: bigint, step: number): Promise<null>;
  /** The offer of the review's own last answer, or why there is none; it writes nothing. */
  undoOffer(): Promise<UndoOffer>;
  /** Both faces of the shown card, completed by the core with its media (SPEC-350 R14). A client
   * without it shows the card view's own sides, as part 1 does. */
  faces?(card: bigint): Promise<Faces>;
}

/** What plays a face's clips: the page's player (`audio.ts`). */
export interface ClipPlayer {
  play(clips: readonly Clip[]): Promise<boolean>;
}

/** What the status region announces: a refusal's code, a card the frame refused, a done deck. */
export type Status = ErrorCode | 'escaped' | 'done';

/** What the frame shows: the card last shown, and its side. */
export interface Face {
  view: CardView;
  side: Side;
}

export class Review {
  readonly #client: () => Promise<StudyClient>;
  readonly #now: () => number;
  readonly #onChange: () => void;
  readonly #player: ClipPlayer | undefined;
  #state: ReviewState = { phase: 'loading', side: 'question' };
  #view: CardView | null = null;
  #face: Face | null = null;
  #counts: Counts | null = null;
  /** The shown card's faces as the core completed them, or `null` when the client has none. */
  #faces: Faces | null = null;
  #refusal: ErrorCode | null = null;
  /** A refusal the review recovered from, announced until the next gesture. */
  #notice: Status | null = null;
  /** The change the engine offered to undo, kept until its confirmation sends it. */
  #offer: Offer | null = null;
  /** What the last undo press was for, so its refusal reads that change's notice (SPEC-383 R13). */
  #pressed: CardView['undo'] = null;
  #shownAt = 0;
  #inFlight: Promise<void> = Promise.resolve();

  /** `client` is the engine's, once started; `now` is in milliseconds; `onChange` follows each step;
   * `player` plays the faces' clips, and a review with none plays nothing. */
  constructor(client: () => Promise<StudyClient>, now: () => number, onChange: () => void, player?: ClipPlayer) {
    this.#client = client;
    this.#now = now;
    this.#onChange = onChange;
    this.#player = player;
  }

  get phase(): Phase {
    return this.#state.phase;
  }

  get side(): Side {
    return this.#state.side;
  }

  /** The card shown, or `null` before the first and after the last. */
  get view(): CardView | null {
    return this.#view;
  }

  /** The card and side the frame shows, or `null` when it shows none. */
  get face(): Face | null {
    return this.#face;
  }

  /** The queue's counts, as the last `card` read them. */
  get counts(): Counts | null {
    return this.#counts;
  }

  /** The change the review asks to undo, while it asks, or `null` (SPEC-371 R10, SPEC-383 R11). */
  get offer(): Offer | null {
    return this.#state.phase === 'confirming' ? this.#offer : null;
  }

  /** Whether the frame refuses the face it shows, which then shows a message in its place. */
  get escaped(): boolean {
    const face = this.#face;
    if (face === null) return false;
    const html = face.side === 'answer' ? face.view.answer : face.view.question;
    return frameDocument(html, face.view.css).refused === 'escaped';
  }

  /** What the status region announces now, or `null` when it is quiet. */
  get status(): Status | null {
    if (this.#state.phase === 'refused') return this.#refusal;
    if (this.#state.phase === 'done') return 'done';
    return this.#notice ?? (this.escaped ? 'escaped' : null);
  }

  /** The status region's words, or `null` when it is quiet: a refused undo of a bury or a flag
   * reads the change's notice, and every other status its own (SPEC-383 R13). */
  get statusLine(): string | null {
    const status = this.status;
    if (status === null) return null;
    const change = CHANGES.includes(this.#pressed) ? CHANGE_NOTICES[status] : undefined;
    return change?.() ?? statusText(status);
  }

  /** The undo control's words, by the change the shown card's view would undo (SPEC-383 R10). */
  get undoLabel(): string {
    return LABELS[this.#face?.view.undo ?? 'answer']();
  }

  /** The controls the face shows: a card the frame refused keeps them (A12); undo only while the
   * review's own last answer, bury or flag can be undone, which a synced one cannot (SPEC-371 R7,
   * SPEC-383 R10). They stay shown while a request is in flight. */
  get controls(): Action[] {
    const face = this.#face;
    if (face === null) return [];
    const side: Action[] = face.side === 'question' ? ['show-answer'] : ['again', 'good'];
    const undo: Action[] = OFFERED.includes(face.view.undo) ? ['undo'] : [];
    // Replay only on a side with replay clips; a blocked play leaves it there to ask again (R15)
    const replay: Action[] = this.#faces?.[face.side].replay.length ? ['replay'] : [];
    return [...side, ...undo, ...replay, 'bury', 'flag'];
  }

  /** The languages the shown card speaks in, each once, for the voice picker (SPEC-350 R16). */
  get languages(): string[] {
    const faces = this.#faces;
    if (faces === null) return [];
    const clips = [faces.question, faces.answer].flatMap((face) => [...face.autoplay, ...face.replay]);
    return [...new Set(clips.flatMap((clip) => (clip.kind === 'speech' ? [clip.language] : [])))];
  }

  /** Loads the first card. */
  start(): void {
    this.#run('card');
    this.#onChange();
  }

  /** The one handler every source reaches: a key, a gamepad, the stick and a click (R8). */
  act(action: Action): void {
    // undo asks only while the review's own last answer, bury or flag can be undone; a synced one
    // is announced and nothing is sent (SPEC-371 R9, SPEC-383 R10)
    if (action === 'undo') {
      const undo = this.#view?.undo ?? null;
      if (undo === null) return;
      this.#pressed = undo;
      if (undo === 'synced' || undo === 'change-synced') {
        this.#announce('undo-synced');
        return;
      }
    }
    this.#notice = null;
    this.#fire(action);
  }

  /** Keeps the offered answer: "Keep it" and Escape (SPEC-371 R10). */
  keep(): void {
    this.#notice = null;
    this.#fire('keep');
  }

  /** Asks again after a refusal. */
  retry(): void {
    this.#notice = null;
    this.#fire('retry');
  }

  /** Settles when no request is in flight, the requests that followed included. */
  async settled(): Promise<void> {
    let current: Promise<void>;
    do {
      current = this.#inFlight;
      await current;
    } while (current !== this.#inFlight);
  }

  /** Announces `notice` on the side the review shows, and moves nothing. */
  #announce(notice: Status): void {
    if (this.#state.phase !== 'question' && this.#state.phase !== 'answer') return;
    this.#notice = notice;
    this.#onChange();
  }

  #fire(event: ReviewEvent): void {
    const from = this.#state.phase;
    const next = step(this.#state, event);
    this.#state = next.state;
    this.#show(next.state.phase);
    this.#sound(event, from, next.effect);
    this.#run(next.effect, event);
    this.#onChange();
  }

  /** A side a new card or a reveal shows plays its autoplay clips, and a replay plays the side's
   * replay clips. A flag's return and a gesture that moved nothing play nothing (SPEC-350 R15). */
  #sound(event: ReviewEvent, from: Phase, effect: Effect): void {
    const faces = this.#faces;
    if (faces === null) return;
    const face = faces[this.#state.side];
    if (effect === 'replay') {
      void this.#player?.play(face.replay);
    } else if (AUTOPLAY.includes(event) && this.#state.phase !== from) {
      void this.#player?.play(face.autoplay);
    }
  }

  /** Entering a side shows its card on that side; a request in flight and the next card's load
   * keep the face, so a grade leaves the rated card's answer on screen until the next card shows;
   * a refusal and a done deck show none. */
  #show(phase: Phase): void {
    if (phase === 'question' || phase === 'answer') {
      this.#face = { view: this.#view as CardView, side: phase };
    } else if (phase === 'refused' || phase === 'done') {
      this.#face = null;
    }
  }

  #run(effect: Effect, event?: ReviewEvent): void {
    if (LOCAL.includes(effect)) return;
    this.#inFlight = this.#request(effect, event).then(
      (outcome) => this.#fire(outcome),
      (error: unknown) => this.#refused(error)
    );
  }

  /** Sends the request `effect` names, and answers the event its reply moves the machine by. */
  async #request(effect: Effect, event?: ReviewEvent): Promise<ReviewEvent> {
    const client = await this.#client();
    if (effect === 'card') {
      const head = await client.card();
      // the core completes both faces of the card it showed, its media inline (SPEC-350 R14)
      const faces = head.card === null ? undefined : await client.faces?.(head.card.id);
      this.#counts = head.counts;
      this.#faces = faces ?? null;
      this.#view =
        faces === undefined
          ? head.card
          : { ...(head.card as CardView), question: faces.question.text, answer: faces.answer.text };
      this.#shownAt = this.#now();
      return head.card === null ? 'empty' : 'view';
    }
    if (effect === 'offer') return this.#offered(await client.undoOffer());
    const card = (this.#view as CardView).id;
    if (effect === 'rate') {
      await client.rate(card, RATING[event as Action] as Rating, Math.round(this.#now() - this.#shownAt));
    } else if (effect === 'bury') {
      await client.bury(card);
    } else if (effect === 'undo') {
      const offer = this.#offer as Offer;
      this.#offer = null;
      await client.undo(offer.card, offer.step);
    } else {
      // the flag is now the review's last change, so the control offers its undo (SPEC-383 R10)
      const flag = await client.flag(card);
      this.#view = { ...(this.#view as CardView), flag, undo: 'flag' };
      return 'flagged';
    }
    return 'settled';
  }

  /** The offer kept for its confirmation, or why none was made, as the notice the side shows. */
  #offered(offered: UndoOffer): ReviewEvent {
    if (offered.offer === null) {
      this.#notice = offered.why === 'synced' ? 'undo-synced' : 'not-undoable';
      return 'not-offered';
    }
    this.#offer = offered.offer;
    return 'offered';
  }

  /** A refusal: a stale card loads the queue's head again, and says so; a refused undo loads the
   * next card, and says why (SPEC-371 R9); any other is announced. */
  #refused(error: unknown): void {
    if (error instanceof EngineError && error.code === 'not-shown') {
      this.#notice = 'not-shown';
      this.#fire('not-shown');
      return;
    }
    if (error instanceof EngineError && (error.code === 'undo-synced' || error.code === 'not-undoable')) {
      this.#notice = error.code;
      this.#fire('undo-refused');
      return;
    }
    this.#refusal = error instanceof EngineError ? error.code : 'engine-failed';
    this.#fire('refusal');
  }
}
