/**
 * The level screen's client types and the reading of `GET /api/level` (SPEC-072 R23, R26).
 *
 * The server answers the owner's level, its title, the XP toward the next level, today's XP by
 * source with each row provisional or settled, the consistency run with its multiplier and the
 * multiplier after one missed day, and whether today is an Ascendant day. The numbers are the ones
 * the bot's `/level` reports: both surfaces read one server use case, so the page computes none of
 * them.
 */

/** Whether a source's XP for the day is still moving or is the day's settled amount. */
export type XpState = 'provisional' | 'settled';

/** One source's XP for the current study day. */
export interface TodayXp {
  /** The derived source, as the server names it. */
  readonly source: string;
  /** The track it pays on. */
  readonly track: string;
  /** The XP the source has earned today. */
  readonly amount: number;
  /** Provisional until the day's close, settled after it. */
  readonly state: XpState;
}

/** The owner's level view as `GET /api/level` answers it. */
export interface LevelView {
  readonly studyDay: string;
  readonly level: number;
  readonly title: string;
  readonly emoji: string;
  readonly totalXp: number;
  readonly xpIntoLevel: number;
  readonly xpForNext: number;
  readonly today: readonly TodayXp[];
  /** The consistency run, in days on pace. */
  readonly run: number;
  readonly multiplier: number;
  /** The multiplier the run would fall to after one missed day. */
  readonly multiplierAfterAMiss: number;
  readonly ascendant: boolean;
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A row of today's XP, or undefined when `value` is not one. */
function todayXp(value: unknown): TodayXp | undefined {
  const { source, track, amount, state } = (value ?? {}) as Record<string, unknown>;
  if (typeof source !== 'string' || typeof track !== 'string' || typeof amount !== 'number') {
    return undefined;
  }
  if (state !== 'provisional' && state !== 'settled') return undefined;
  return { source, track, amount, state };
}

/** The body of `GET /api/level`, or null when it is not one. */
export function parseLevel(body: unknown): LevelView | null {
  const given = (body ?? {}) as Record<string, unknown>;
  const {
    study_day: studyDay,
    level,
    title,
    emoji,
    total_xp: totalXp,
    xp_into_level: xpIntoLevel,
    xp_for_next: xpForNext,
    today,
    run,
    multiplier,
    multiplier_after_a_miss: multiplierAfterAMiss,
    ascendant
  } = given;
  if (typeof studyDay !== 'string' || !ISO_DATE.test(studyDay)) return null;
  if (typeof title !== 'string' || typeof emoji !== 'string') return null;
  const numbers = [level, totalXp, xpIntoLevel, xpForNext, run, multiplier, multiplierAfterAMiss];
  if (!numbers.every((n) => typeof n === 'number')) return null;
  if (!Array.isArray(today) || typeof ascendant !== 'boolean') return null;
  const rows = today.map(todayXp);
  if (rows.includes(undefined)) return null;
  return {
    studyDay,
    level: level as number,
    title,
    emoji,
    totalXp: totalXp as number,
    xpIntoLevel: xpIntoLevel as number,
    xpForNext: xpForNext as number,
    today: rows as TodayXp[],
    run: run as number,
    multiplier: multiplier as number,
    multiplierAfterAMiss: multiplierAfterAMiss as number,
    ascendant
  };
}
