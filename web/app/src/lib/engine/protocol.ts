// The web engine's protocol between the page and its Worker (SPEC-338 R3, ADR-348). The page sends
// a request with an id and an operation; the Worker answers each with that id and a value, or with
// an error code and a message. Nothing else crosses: no SQL, no service index, no engine handle.

/** The operations the Worker serves, and nothing else. */
export const OPS = ['open', 'seed', 'next', 'answer', 'undo', 'snapshot', 'close'] as const;
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
  | { id: number; op: 'open' | 'next' | 'undo' | 'close' }
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

export function parseRequest(data: unknown): Parsed {
  return { request: data as Request };
}
