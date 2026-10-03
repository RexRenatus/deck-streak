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
