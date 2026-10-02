/**
 * The records screen's client types and the reading of `GET /api/records` (SPEC-073 R13, R17,
 * R19).
 *
 * The server answers each personal record with its value, the study day it was set, the value it
 * beat, today's figure and the distance from today's figure to the record, and the record closest
 * to being beaten today as the one to chase. The page computes no distance of its own: the bar and
 * its text are the server's figures.
 */

/** One personal record and today's distance to it. */
export interface RecordLine {
  /** The record's kind, as the server names it. */
  readonly kind: string;
  readonly label: string;
  readonly value: number;
  /** The study day it was set, as `YYYY-MM-DD`. */
  readonly studyDay: string;
  /** The value it beat. */
  readonly previous: number;
  /** Today's figure of the same kind. */
  readonly today: number;
  /** How far today's figure is below the record; 0 once today reaches it. */
  readonly distance: number;
}

/** The record closest to being beaten today. */
export interface Chase {
  readonly kind: string;
  readonly label: string;
  readonly gap: number;
}

/** The owner's records as `GET /api/records` answers them. */
export interface RecordsView {
  readonly records: readonly RecordLine[];
  readonly chase: Chase | null;
}

/** The body of `GET /api/records`, or null when it is not one. */
export function parseRecords(body: unknown): RecordsView | null {
  void body;
  return null;
}
