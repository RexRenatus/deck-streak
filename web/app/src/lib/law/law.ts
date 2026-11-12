/**
 * The law tab's client types (SPEC-077 R10 to R15; SPEC-072 R24).
 *
 * `GET /api/law` answers the law block: whether it is shown, whether its lifetime XP line names the
 * level, the keys of the lines it shows in order, and each count. A count the recompute has not
 * stored, or one whose port is not wired, is null beside its pending flag, never 0, and the page
 * renders it as pending. `GET /api/level/law-tiers` answers the law cards and today's law XP by
 * tier. The page computes none of them.
 */

/** The key of one line the block shows, the name of the count it shows. */
export type LawLineKey = 'total_xp' | 'streak' | 'xp_today' | 'dues' | 'mastery' | 'leeches';

/** The law block as `GET /api/law` answers it; a null count is pending. */
export interface LawView {
  readonly shown: boolean;
  readonly levelShown: boolean;
  readonly lines: readonly LawLineKey[];
  readonly streak: number;
  readonly xpToday: number;
  readonly totalXp: number;
  readonly level: number;
  readonly dues: number | null;
  readonly leeches: number | null;
  readonly mastery: number | null;
}

/** The tiers of SPEC-072's law distribution, in the order the route answers them. */
export const TIERS = ['T1', 'T2', 'T3', 'T4', 'none'] as const;

/** One tier of the law distribution. */
export type Tier = (typeof TIERS)[number];

/** The law cards and today's law XP by tier, as `GET /api/level/law-tiers` answers them. */
export interface LawTiersView {
  readonly cards: Readonly<Record<Tier, number>>;
  readonly xpToday: Readonly<Record<Tier, number>>;
}

const LINE_KEYS: readonly LawLineKey[] = ['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches'];

/** Whether `value` is a count: a whole number, never negative. */
function count(value: unknown): value is number {
  return Number.isInteger(value) && (value as number) >= 0;
}

/** Whether `value` is a percent, 0 to 100. */
function percent(value: unknown): value is number {
  return typeof value === 'number' && value >= 0 && value <= 100;
}

/**
 * A count beside its pending flag: null when pending, the count otherwise, or undefined when the
 * two disagree or the count is not one.
 */
function pendingOr(
  value: unknown,
  pending: unknown,
  valid: (value: unknown) => value is number
): number | null | undefined {
  if (pending === true && value === null) return null;
  return pending === false && valid(value) ? value : undefined;
}

/** The body of `GET /api/law`, or null when it is not one. */
export function parseLaw(body: unknown): LawView | null {
  const given = (body ?? {}) as Record<string, unknown>;
  const { shown, level_shown: levelShown, lines, streak, xp_today: xpToday } = given;
  const { total_xp: totalXp, level } = given;
  if (typeof shown !== 'boolean' || typeof levelShown !== 'boolean') return null;
  if (![streak, xpToday, totalXp, level].every(count)) return null;
  const dues = pendingOr(given.dues, given.dues_pending, count);
  const leeches = pendingOr(given.leeches, given.leeches_pending, count);
  const mastery = pendingOr(given.mastery, given.mastery_pending, percent);
  if (dues === undefined || leeches === undefined || mastery === undefined) return null;
  if (!Array.isArray(lines) || !lines.every((key) => LINE_KEYS.includes(key))) return null;
  // a pending count is never a shown line: there is no figure to show
  const counts: Partial<Record<LawLineKey, number | null>> = { dues, leeches, mastery };
  if (lines.some((key) => counts[key as LawLineKey] === null)) return null;
  return {
    shown,
    levelShown,
    lines: lines as LawLineKey[],
    streak: streak as number,
    xpToday: xpToday as number,
    totalXp: totalXp as number,
    level: level as number,
    dues,
    leeches,
    mastery
  };
}

/** One tier table, or undefined when `value` is not one. */
function byTier(value: unknown): Record<Tier, number> | undefined {
  if (value === null || typeof value !== 'object') return undefined;
  // exactly the five tiers, each a count: an array or any other key refuses the table
  const given = value as Record<string, unknown>;
  if (Object.keys(given).length !== TIERS.length) return undefined;
  return TIERS.every((tier) => count(given[tier])) ? (given as Record<Tier, number>) : undefined;
}

/** The body of `GET /api/level/law-tiers`, or null when it is not one. */
export function parseLawTiers(body: unknown): LawTiersView | null {
  const { cards, xp_today: xp } = (body ?? {}) as Record<string, unknown>;
  const readCards = byTier(cards);
  const readXp = byTier(xp);
  if (readCards === undefined || readXp === undefined) return null;
  return { cards: readCards, xpToday: readXp };
}
