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
