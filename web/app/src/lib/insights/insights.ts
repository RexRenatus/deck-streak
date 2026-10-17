/**
 * The instruments' shapes as `GET /api/insights/{id}` serves them (SPEC-094 R18, R19; ADR-085):
 * numbers and names only, so the client draws whatever it shows.
 */

/** One stored report: the run's study day, the reads that failed, and the report or null. */
export interface Envelope<R = unknown> {
  readonly instrument: string;
  readonly studyDay: number;
  readonly failedReads: readonly string[];
  /** Null when the run itself failed: nothing is claimed. */
  readonly report: R | null;
}

/** One entry of `GET /api/insights`. */
export interface Listing {
  readonly id: string;
  readonly cadence: string;
  readonly studyDay: number | null;
}

/** A field no template references. */
export interface DarkField {
  readonly noteType: string;
  readonly field: string;
  readonly reviewedNotes: number;
}

/** A note type whose templates could not be verified. */
export interface Unparseable {
  readonly noteType: string;
  readonly noteTypeId: number;
}

/** Dark Fields' report as it is served: capped, with the totals. */
export interface DarkFieldsReport {
  readonly darkFields: readonly DarkField[];
  readonly darkFieldsTotal: number;
  readonly unparseable: readonly Unparseable[];
  readonly unparseableTotal: number;
  readonly notetypesChecked: number;
  readonly reviewedNoteCount: number;
  readonly isCold: boolean;
}

/** The instrument id Dark Fields is stored under. */
export const DARK_FIELDS_ID = 'dark_fields';

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function count(value: unknown): number | null {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 ? value : null;
}

function names(value: unknown): string[] | null {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
    ? (value as string[])
    : null;
}

/** The body of `GET /api/insights`, or null when it is not one. */
export function parseListings(body: unknown): Listing[] | null {
  const list = record(body)?.instruments;
  if (!Array.isArray(list)) return null;
  const out: Listing[] = [];
  for (const item of list) {
    const row = record(item);
    const day = row?.study_day;
    if (row === null || typeof row.id !== 'string' || typeof row.cadence !== 'string') return null;
    if (day !== null && count(day) === null) return null;
    out.push({ id: row.id, cadence: row.cadence, studyDay: day as number | null });
  }
  return out;
}

/** The body of `GET /api/insights/{id}`: null for no report yet, undefined when it is not one. */
export function parseEnvelope(body: unknown): Envelope | null | undefined {
  const top = record(body);
  if (top === null || typeof top.instrument !== 'string') return undefined;
  if (top.report === null) return null;
  const stored = record(top.report);
  const failed = names(stored?.failed_reads);
  const day = count(stored?.study_day);
  if (stored === null || failed === null || day === null) return undefined;
  return {
    instrument: top.instrument,
    studyDay: day,
    failedReads: failed,
    report: stored.report === null ? null : stored.report
  };
}

/** Dark Fields' report as stored, or null when it is not one. */
export function parseDarkFields(report: unknown): DarkFieldsReport | null {
  const row = record(report);
  if (row === null) return null;
  const fields = row.dark_fields;
  const loose = row.unparseable;
  const total = count(row.dark_fields_total);
  const looseTotal = count(row.unparseable_total);
  const checked = count(row.notetypes_checked);
  const reviewed = count(row.reviewed_note_count);
  if (!Array.isArray(fields) || !Array.isArray(loose)) return null;
  if (total === null || looseTotal === null || checked === null || reviewed === null) return null;
  const darkFields: DarkField[] = [];
  for (const item of fields) {
    const f = record(item);
    const n = count(f?.reviewed_notes);
    if (f === null || typeof f.note_type !== 'string' || typeof f.field !== 'string' || n === null)
      return null;
    darkFields.push({ noteType: f.note_type, field: f.field, reviewedNotes: n });
  }
  const unparseable: Unparseable[] = [];
  for (const item of loose) {
    const u = record(item);
    const id = count(u?.note_type_id);
    if (u === null || typeof u.note_type !== 'string' || id === null) return null;
    unparseable.push({ noteType: u.note_type, noteTypeId: id });
  }
  return {
    darkFields,
    darkFieldsTotal: total,
    unparseable,
    unparseableTotal: looseTotal,
    notetypesChecked: checked,
    reviewedNoteCount: reviewed,
    isCold: row.is_cold === true
  };
}
