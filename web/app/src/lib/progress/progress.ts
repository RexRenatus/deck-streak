/**
 * The progress screen's client types (SPEC-077 R15, R17).
 *
 * `GET /api/progress` answers each configured course, ordered by name, with its mastery, its
 * current band, its current unit and its six band cells, A1 to C2: each cell's cards, its mature
 * cards, its mastery percent and whether it is achieved. The page computes none of them.
 */

/** One band cell of a course's ladder. */
export interface BandCell {
  /** The band's name, A1 to C2. */
  readonly band: string;
  /** The course's counted cards in the band. */
  readonly total: number;
  /** Of those, the cards whose mastery is at least one half. */
  readonly mature: number;
  /** The band's mean mastery, as a percent. */
  readonly pct: number;
  readonly achieved: boolean;
}

/** One course's Road to C2. */
export interface CourseProgress {
  readonly code: string;
  readonly name: string;
  readonly flag: string;
  /** The course's mean mastery, as a percent. */
  readonly masteryPct: number;
  readonly currentBand: string;
  /** The highest unit holding a mature card, or null before one is mature. */
  readonly currentUnit: number | null;
  readonly bands: readonly BandCell[];
}

/** The owner's courses as `GET /api/progress` answers them. */
export interface ProgressView {
  readonly courses: readonly CourseProgress[];
}

/** A band cell, or undefined when `value` is not one. */
function bandCell(value: unknown): BandCell | undefined {
  const { band, total, mature, pct, achieved } = (value ?? {}) as Record<string, unknown>;
  if (typeof band !== 'string' || typeof achieved !== 'boolean') return undefined;
  if (!count(total) || !count(mature) || !percent(pct)) return undefined;
  return { band, total, mature, pct, achieved };
}

/** One course, or undefined when `value` is not one. */
function courseProgress(value: unknown): CourseProgress | undefined {
  const given = (value ?? {}) as Record<string, unknown>;
  const { code, name, flag, mastery_pct: masteryPct, current_band: currentBand } = given;
  const { current_unit: currentUnit, bands } = given;
  if (typeof code !== 'string' || typeof name !== 'string' || typeof flag !== 'string') {
    return undefined;
  }
  if (typeof currentBand !== 'string' || !percent(masteryPct)) return undefined;
  if (currentUnit !== null && !count(currentUnit)) return undefined;
  if (!Array.isArray(bands)) return undefined;
  const cells = bands.map(bandCell);
  if (cells.includes(undefined)) return undefined;
  return {
    code,
    name,
    flag,
    masteryPct,
    currentBand,
    currentUnit,
    bands: cells as BandCell[]
  };
}

/** Whether `value` is a count: a whole number, never negative. */
function count(value: unknown): value is number {
  return Number.isInteger(value) && (value as number) >= 0;
}

/** Whether `value` is a percent, 0 to 100. */
function percent(value: unknown): value is number {
  return typeof value === 'number' && value >= 0 && value <= 100;
}

/** The body of `GET /api/progress`, or null when it is not one: a malformed course refuses it whole. */
export function parseProgress(body: unknown): ProgressView | null {
  const { courses } = (body ?? {}) as Record<string, unknown>;
  if (!Array.isArray(courses)) return null;
  const read = courses.map(courseProgress);
  if (read.includes(undefined)) return null;
  return { courses: read as CourseProgress[] };
}
