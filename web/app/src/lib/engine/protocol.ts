// The web engine's protocol between the page and its Worker (SPEC-338 R3, ADR-348). The page sends
// a request with an id and an operation; the Worker answers each with that id and a value, or with
// an error code and a message. Nothing else crosses: no SQL, no service index, no engine handle.

/** The operations the Worker serves, and nothing else. */
export const OPS = ['open', 'seed', 'next', 'answer', 'undo', 'snapshot', 'memory', 'close'] as const;
export type Op = (typeof OPS)[number];

/** Why the Worker refused a request. */
export type ErrorCode =
  | 'bad-request'
  | 'collection-busy'
  | 'storage-refused'
  | 'engine-failed'
  | 'not-open';

/** A wire rating, as Anki's buttons number the answers: again, hard, good, easy. */
export type Rating = 1 | 2 | 3 | 4;

export type Request =
  | { id: number; op: 'open' | 'next' | 'undo' | 'memory' | 'close' }
  | { id: number; op: 'seed'; count: number }
  | { id: number; op: 'answer'; rating: Rating; ms: number }
  | { id: number; op: 'snapshot'; card: bigint };

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

export type Reply =
  | { id: number; ok: true; value: unknown }
  | { id: number | null; ok: false; code: ErrorCode; message: string };

/** The request `data` holds, or the id it carried (when readable) and why it is refused. */
export type Parsed = { request: Request } | { id: number | null; message: string };

const U32 = 2 ** 32 - 1;
const I64 = 2n ** 63n - 1n;
const whole = (value: unknown, least: number, most: number) =>
  Number.isSafeInteger(value) && (value as number) >= least && (value as number) <= most;

/** Each operation's arguments, and the test each must pass: the engine's own types bound them. */
const ARGS: Record<Op, Record<string, (value: unknown) => boolean>> = {
  open: {},
  next: {},
  undo: {},
  memory: {},
  close: {},
  seed: { count: (value) => whole(value, 1, U32) },
  answer: { rating: (value) => whole(value, 1, 4), ms: (value) => whole(value, 0, U32) },
  snapshot: { card: (value) => typeof value === 'bigint' && value >= 1n && value <= I64 }
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
