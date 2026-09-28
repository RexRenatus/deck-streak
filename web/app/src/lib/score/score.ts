/**
 * The score screen's client types and the reading of `GET /api/score` (SPEC-071 R20, R22).
 *
 * The server answers the current study day and its score, or `null` while no recompute has rolled
 * the day up. The score carries its total, its grade and its five pillars; the retention pillar is
 * `null` on a day with no answered review, and the screen shows that as a gap, never as 0. The
 * numbers are the ones the bot's `/score` reports for the same day: both surfaces read one server
 * use case, so the page computes nothing from them.
 */

/** The five pillars, in the order the breakdown shows them. */
export const PILLARS = ['consistency', 'retention', 'workload', 'volume', 'mastery'] as const;

/** One of the five pillars. */
export type Pillar = (typeof PILLARS)[number];

/** A study day's score as `GET /api/score` answers it. */
export interface DayScore {
  /** The weighted total, 0 to 100. */
  readonly total: number;
  /** The grade band: its label and its emoji. */
  readonly grade: { readonly label: string; readonly emoji: string };
  /** Each pillar's value, 0 to 100; an absent one is null. */
  readonly pillars: Readonly<Record<Pillar, number | null>>;
}

/** The current study day, as an ISO date, and its score, or null while it has none. */
export interface ScoreToday {
  readonly studyDay: string;
  readonly score: DayScore | null;
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A pillar's value: a number, or null for the retention of a day with no answered review. */
function pillarValue(pillar: Pillar, value: unknown): number | null | undefined {
  if (typeof value === 'number') return value;
  return value === null && pillar === 'retention' ? null : undefined;
}

/** The score `value` holds, or undefined when it is not one. */
function dayScore(value: unknown): DayScore | undefined {
  const { total, grade, pillars } = (value ?? {}) as Record<string, unknown>;
  if (typeof total !== 'number') return undefined;
  const { label, emoji } = (grade ?? {}) as Record<string, unknown>;
  if (typeof label !== 'string' || typeof emoji !== 'string') return undefined;
  const given = (pillars ?? {}) as Record<string, unknown>;
  const values = PILLARS.map((pillar) => pillarValue(pillar, given[pillar]));
  if (values.includes(undefined)) return undefined;
  return {
    total,
    grade: { label, emoji },
    pillars: Object.fromEntries(PILLARS.map((pillar, at) => [pillar, values[at]])) as Record<
      Pillar,
      number | null
    >
  };
}

/** The body of `GET /api/score`, or null when it is not one. */
export function parseScore(body: unknown): ScoreToday | null {
  const { study_day: day, score } = (body ?? {}) as Record<string, unknown>;
  if (typeof day !== 'string' || !ISO_DATE.test(day)) return null;
  if (score === null) return { studyDay: day, score: null };
  const parsed = dayScore(score);
  return parsed === undefined ? null : { studyDay: day, score: parsed };
}
