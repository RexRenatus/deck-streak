// The web engine's protocol between the page and its Worker (SPEC-338 R3, ADR-348). The page sends
// a request with an id and an operation; the Worker answers each with that id and a value, or with
// an error code and a message. Nothing else crosses: no SQL, no service index, no engine handle.

/** The operations the Worker serves, and nothing else. Six are the review's (SPEC-350 R4): the deck
 * list, the current deck, the card view, and a rating, bury or flag of the shown card. `faces`, after
 * them, completes both faces of the shown card with its media (SPEC-350 R14). The last two read and
 * forget the sync credential, each answering a status word (SPEC-363 R15). A grade is recorded only
 * by `rate`, on the card shown: no operation answers the queue's head (SPEC-365 R9). `undo-offer`
 * reads the review's own last answer and writes nothing, and `undo` reverts only the answer an
 * offer named, by its card and its step (SPEC-371 R12). */
export const OPS = [
  'open',
  'seed',
  'next',
  'undo',
  'undo-offer',
  'snapshot',
  'memory',
  'close',
  'decks',
  'study',
  'card',
  'rate',
  'bury',
  'flag',
  'faces',
  'credential-status',
  'credential-forget'
] as const;
export type Op = (typeof OPS)[number];

/** Why the Worker refused a request. */
export type ErrorCode =
  | 'bad-request'
  | 'collection-busy'
  | 'storage-refused'
  | 'engine-failed'
  | 'not-open'
  | 'not-shown'
  | 'undo-synced'
  | 'not-undoable';

/** A wire rating, as Anki's buttons number the answers: again (1) and good (3). */
export type Rating = 1 | 3;

export type Request =
  | { id: number; op: 'open'; languages?: string[] }
  | { id: number; op: 'next' | 'undo-offer' | 'memory' | 'close' | 'decks' | 'card' }
  | { id: number; op: 'undo'; card: bigint; step: number }
  | { id: number; op: 'seed'; count: number }
  | { id: number; op: 'snapshot' | 'bury' | 'flag' | 'faces'; card: bigint }
  | { id: number; op: 'study'; deck: bigint }
  | { id: number; op: 'rate'; card: bigint; rating: Rating; ms: number }
  | { id: number; op: 'credential-status' | 'credential-forget' };

/** What the credential operations answer, and all they answer: whether this origin keeps a sealed
 * sync key, whether the Worker holds it open, and why it could not be opened (SPEC-363 R15). Never a
 * key, a user or a password. */
export const STATUS_WORDS = ['absent', 'sealed', 'held', 'needs-sign-in', 'offline'] as const;
export type StatusWord = (typeof STATUS_WORDS)[number];

/** A request as the page writes it; the client numbers it. */
export type Body = Request extends infer R ? (R extends Request ? Omit<R, 'id'> : never) : never;

/** What `open` answers: whether the collection was already in storage, and its note count. */
export interface Opened {
  existed: boolean;
  notes: number;
}

/** One card's scheduling fields, read by the engine's one fixed query. */
export interface Snapshot {
  id: bigint;
  queue: number;
  type: number;
  due: number;
  interval: number;
  reps: number;
  lapses: number;
}

/** One deck of the engine's deck tree, with today's counts. */
export interface Deck {
  id: bigint;
  name: string;
  level: number;
  new: number;
  learning: number;
  review: number;
  children: Deck[];
}

/** The queue's new, learning and review counts. */
export interface Counts {
  new: number;
  learning: number;
  review: number;
}

/** The card the review shows: both sides rendered by the engine with sound and speech tags
 * stripped, the note type's CSS, the four interval labels, and whether the review's own last answer
 * can be undone: `answer` when it can, `synced` when it has synced, `null` when there is none to
 * undo (SPEC-371 R7). */
export interface CardView {
  id: bigint;
  ordinal: number;
  flag: number;
  question: string;
  answer: string;
  css: string;
  labels: string[];
  undo: 'answer' | 'synced' | null;
}

/** The grade an offered answer gave, as the review names it. */
export type OfferGrade = 'again' | 'good';

/** The state an undone answer returns its card to. */
export type Returns = 'new' | 'learning' | 'review' | 'relearning' | 'preview';

/** What `undo-offer` answers: the review's own last answer, its card's text as one line, its grade
 * and the state the card goes back to, with the card and the step a confirmation carries back; or
 * no offer, and why: `synced` when it has synced, `none` for every other reason (SPEC-371 R7). */
export type UndoOffer =
  | { offer: { card: bigint; step: number; text: string; grade: OfferGrade; returns: Returns } }
  | { offer: null; why: 'synced' | 'none' };

/** What `card` answers: the queue's counts, and the card it shows, or `null` when the deck is done. */
export interface Head {
  counts: Counts;
  card: CardView | null;
}

/** One clip of a face, in the core's order: a sound as its bytes and the media type the core's table
 * gives its name (null when the table holds none), or speech as its text, its BCP 47 language and
 * the native platform's rate (SPEC-350 R15, R16; SPEC-348 P3). */
export type Clip =
  | { kind: 'sound'; name: string; type: string | null; bytes: Uint8Array }
  | { kind: 'speech'; text: string; language: string; rate: number };

/** One face as the core completed it: its text, with each media file inline as the core's `data:`
 * URL, the note type's CSS, the clips that play on show and on replay, and each name it omitted. */
export interface FaceView {
  text: string;
  css: string;
  autoplay: Clip[];
  replay: Clip[];
  omitted: string[];
}

/** A media file the core asked for and was not given: its name, and the bytes it may read of it. */
export interface MediaAsk {
  name: string;
  limit: bigint;
}

/** What `faces` answers: both faces of the shown card, and the files the core still wants. */
export interface Faces {
  question: FaceView;
  answer: FaceView;
  wanted: MediaAsk[];
}

export type Reply =
  | { id: number; ok: true; value: unknown }
  | { id: number | null; ok: false; code: ErrorCode; message: string };

/** The request `data` holds, or the id it carried (when readable) and why it is refused. */
export type Parsed = { request: Request } | { id: number | null; message: string };

const U32 = 2 ** 32 - 1;
const I64 = 2n ** 63n - 1n;
const whole = (value: unknown, least: number, most: number) =>
  Number.isSafeInteger(value) && (value as number) >= least && (value as number) <= most;
/** A wire rating the page offers: Again is 1 and Good is 3, and no other number. */
const grade = (value: unknown) => value === 1 || value === 3;
/** An engine id: a positive i64, carried as a bigint. */
const engineId = (value: unknown) => typeof value === 'bigint' && value >= 1n && value <= I64;
/** A language tag as the engine names its languages: `ja`, `zh-CN`. */
const TAG = /^[a-z]{2,3}(-[A-Za-z0-9]{2,8})?$/;
/** At most this many languages, in the order the engine prefers them. */
const LANGUAGES = 8;
const languages = (value: unknown) =>
  value === undefined ||
  (Array.isArray(value) &&
    value.length >= 1 &&
    value.length <= LANGUAGES &&
    value.every((tag) => typeof tag === 'string' && TAG.test(tag)));

/** Each operation's arguments, and the test each must pass: the engine's own types bound them. */
const ARGS: Record<Op, Record<string, (value: unknown) => boolean>> = {
  open: { languages },
  next: {},
  undo: { card: engineId, step: (value) => whole(value, 0, U32) },
  'undo-offer': {},
  memory: {},
  close: {},
  decks: {},
  card: {},
  seed: { count: (value) => whole(value, 1, U32) },
  snapshot: { card: engineId },
  study: { deck: engineId },
  rate: { card: engineId, rating: (value) => grade(value), ms: (value) => whole(value, 0, U32) },
  bury: { card: engineId },
  flag: { card: engineId },
  faces: { card: engineId },
  'credential-status': {},
  'credential-forget': {}
};

/** Reads a request off the wire. Anything but an operation of `OPS` with exactly its arguments,
 * each within the engine's type, is refused here, before any engine call. */
export function parseRequest(data: unknown): Parsed {
  if (typeof data !== 'object' || data === null || Array.isArray(data)) {
    return { id: null, message: 'a request is an object with an id and an op' };
  }
  const fields = data as Record<string, unknown>;
  if (!whole(fields.id, 0, Number.MAX_SAFE_INTEGER)) {
    return { id: null, message: "a request's id is a whole number from 0" };
  }
  const id = fields.id as number;
  const op = fields.op as Op;
  if (!OPS.includes(op)) return { id, message: `unknown operation ${String(op)}` };
  const args = ARGS[op];
  const extra = Object.keys(fields).find((key) => key !== 'id' && key !== 'op' && !Object.hasOwn(args, key));
  if (extra !== undefined) return { id, message: `${op} takes no ${extra}` };
  const malformed = Object.keys(args).find((key) => !args[key](fields[key]));
  if (malformed !== undefined) return { id, message: `${op}'s ${malformed} is malformed` };
  return { request: fields as unknown as Request };
}

/** Whether a listener hears a message, by its sender's origin (SPEC-338 R13, ASVS 5.0.0 3.5.5). A
 * dedicated Worker's channel delivers each message with an empty origin, both ways; a message that
 * names an origin other than the listener's own came from somewhere else, and is not heard. */
export function admitsOrigin(sender: string, own: string): boolean {
  return sender === '' || sender === own;
}
