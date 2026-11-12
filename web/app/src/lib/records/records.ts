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

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A record line, or undefined when `value` is not one. */
function recordLine(value: unknown): RecordLine | undefined {
  const given = (value ?? {}) as Record<string, unknown>;
  const { kind, label, value: record, study_day: studyDay, previous, today, distance } = given;
  if (typeof kind !== 'string' || typeof label !== 'string') return undefined;
  if (typeof studyDay !== 'string' || !ISO_DATE.test(studyDay)) return undefined;
  const numbers = [record, previous, today, distance];
  if (!numbers.every((n) => typeof n === 'number')) return undefined;
  return {
    kind,
    label,
    value: record as number,
    studyDay,
    previous: previous as number,
    today: today as number,
    distance: distance as number
  };
}

/** The record to chase, null for none, or undefined when `value` is neither. */
function chase(value: unknown): Chase | null | undefined {
  if (value === null) return null;
  const { kind, label, gap } = (value ?? {}) as Record<string, unknown>;
  if (typeof kind !== 'string' || typeof label !== 'string' || typeof gap !== 'number') {
    return undefined;
  }
  return { kind, label, gap };
}

/** The body of `GET /api/records`, or null when it is not one. */
export function parseRecords(body: unknown): RecordsView | null {
  const given = (body ?? {}) as Record<string, unknown>;
  if (!Array.isArray(given.records)) return null;
  const records = given.records.map(recordLine);
  if (records.includes(undefined)) return null;
  const chased = chase(given.chase);
  if (chased === undefined) return null;
  return { records: records as RecordLine[], chase: chased };
}
