// SPEC-350 R2, R7, R10, A11, A12; ADR-361. The review's machine, a table as #663's wake lock is:
// every event passes through `step`, so the review's behaviour is the table's. The page keeps only
// which side is shown; the engine side keeps the card it showed, and every rating, bury and flag
// names that card. A gesture while a request is in flight finds no cell and fires nothing.
import { frameDocument } from '$lib/card/frame-document';
import { EngineError } from '$lib/engine/client';
import type { CardView, Counts, ErrorCode, Head, Rating } from '$lib/engine/protocol';
import { sideAfter, type Action, type Side } from '$lib/remote/actions';

/** The schematic's states: a card loading, a side shown, a request in flight, a refusal, done. */
export type Phase = 'loading' | 'question' | 'answer' | 'busy' | 'refused' | 'done';

/** What moves the machine: an action, or the outcome of the request in flight. */
export type ReviewEvent =
  | Action
  | 'view'
  | 'empty'
  | 'settled'
  | 'flagged'
  | 'not-shown'
  | 'refusal'
  | 'retry';

/** The request a step asks for. */
export type Effect = 'none' | 'card' | 'rate' | 'bury' | 'flag' | 'undo';

/** The machine's state: its phase, and the side the review shows or returns to. */
export interface ReviewState {
  phase: Phase;
  side: Side;
}

/** One cell: the next phase (`side` is the side's own phase) and the request it asks for. */
interface Cell {
  phase: Phase | 'side';
  effect: Effect;
}

const RATE: Cell = { phase: 'busy', effect: 'rate' };
const BURY: Cell = { phase: 'busy', effect: 'bury' };
const FLAG: Cell = { phase: 'busy', effect: 'flag' };
const UNDO: Cell = { phase: 'busy', effect: 'undo' };
const NEXT: Cell = { phase: 'loading', effect: 'card' };
const REFUSED: Cell = { phase: 'refused', effect: 'none' };

/** Every cell the machine has. An event with no cell in its phase fires nothing. */
const TABLE: Record<Phase, Partial<Record<ReviewEvent, Cell>>> = {
  loading: {
    view: { phase: 'question', effect: 'none' },
    empty: { phase: 'done', effect: 'none' },
    refusal: REFUSED
  },
  question: { 'show-answer': { phase: 'answer', effect: 'none' }, undo: UNDO, bury: BURY, flag: FLAG },
  answer: { again: RATE, hard: RATE, good: RATE, easy: RATE, undo: UNDO, bury: BURY, flag: FLAG },
  busy: { settled: NEXT, 'not-shown': NEXT, flagged: { phase: 'side', effect: 'none' }, refusal: REFUSED },
  refused: { retry: NEXT },
  done: {}
};

/** The state `event` moves `state` to, and the request it asks for. */
export function step(state: ReviewState, event: ReviewEvent): { state: ReviewState; effect: Effect } {
  const cell = TABLE[state.phase][event];
  if (cell === undefined) return { state, effect: 'none' };
  // a new card shows its question; an action moves the side as the remote's reviewer does
  const side = event === 'view' ? 'question' : sideAfter(event as Action, state.side);
  return { state: { phase: cell.phase === 'side' ? side : cell.phase, side }, effect: cell.effect };
}

/** The four grades, as the wire numbers them. */
const RATING: Partial<Record<Action, Rating>> = { again: 1, hard: 2, good: 3, easy: 4 };

/** What the review asks of the engine: the study operations, and nothing else (R2, A17). */
export interface StudyClient {
  card(): Promise<Head>;
  rate(card: bigint, rating: Rating, ms: number): Promise<null>;
  bury(card: bigint): Promise<null>;
  flag(card: bigint): Promise<number>;
  undo(): Promise<null>;
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
  #state: ReviewState = { phase: 'loading', side: 'question' };
  #view: CardView | null = null;
  #face: Face | null = null;
  #counts: Counts | null = null;
  #refusal: ErrorCode | null = null;
  /** A refusal the review recovered from, announced until the next gesture. */
  #notice: Status | null = null;
  #shownAt = 0;
  #inFlight: Promise<void> = Promise.resolve();

  /** `client` is the engine's, once started; `now` is in milliseconds; `onChange` follows each step. */
  constructor(client: () => Promise<StudyClient>, now: () => number, onChange: () => void) {
    this.#client = client;
    this.#now = now;
    this.#onChange = onChange;
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

  /** The controls the face shows: a card the frame refused keeps them (A12); undo only while the
   * engine names an undoable action (R7). They stay shown while a request is in flight. */
  get controls(): Action[] {
    const face = this.#face;
    if (face === null) return [];
    const side: Action[] = face.side === 'question' ? ['show-answer'] : ['again', 'hard', 'good', 'easy'];
    const undo: Action[] = face.view.undo ? ['undo'] : [];
    return [...side, ...undo, 'bury', 'flag'];
  }

  /** Loads the first card. */
  start(): void {
    this.#run('card');
    this.#onChange();
  }

  /** The one handler every source reaches: a key, a gamepad, the stick and a click (R8). */
  act(action: Action): void {
    // undo acts only while the engine names an undoable action
    if (action === 'undo' && !this.#view?.undo) return;
    this.#notice = null;
    this.#fire(action);
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

  #fire(event: ReviewEvent): void {
    const next = step(this.#state, event);
    this.#state = next.state;
    this.#show(next.state.phase);
    this.#run(next.effect, event);
    this.#onChange();
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
    if (effect === 'none') return;
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
      this.#counts = head.counts;
      this.#view = head.card;
      this.#shownAt = this.#now();
      return head.card === null ? 'empty' : 'view';
    }
    const card = (this.#view as CardView).id;
    if (effect === 'rate') {
      await client.rate(card, RATING[event as Action] as Rating, Math.round(this.#now() - this.#shownAt));
    } else if (effect === 'bury') {
      await client.bury(card);
    } else if (effect === 'undo') {
      await client.undo();
    } else {
      const flag = await client.flag(card);
      this.#view = { ...(this.#view as CardView), flag };
      return 'flagged';
    }
    return 'settled';
  }

  /** A refusal: a stale card loads the queue's head again, and says so; any other is announced. */
  #refused(error: unknown): void {
    if (error instanceof EngineError && error.code === 'not-shown') {
      this.#notice = 'not-shown';
      this.#fire('not-shown');
      return;
    }
    this.#refusal = error instanceof EngineError ? error.code : 'engine-failed';
    this.#fire('refusal');
  }
}
