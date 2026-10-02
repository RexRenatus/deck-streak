/**
 * The badge gallery's client types and the reading of `GET /api/badges` (SPEC-073 R16, R19).
 *
 * The server answers the badges the owner has earned, newest first, each with its tier, name, emoji
 * and the study day it was earned, and the catalog's badges still locked, each with its criteria,
 * its family and its progress: a locked study badge carries its input's value against its
 * threshold, and a badge whose input the server does not keep carries none. The page computes no
 * progress of its own.
 */

/** The catalog family a badge belongs to. */
export type BadgeFamily = 'study' | 'habit' | 'focus';

/** A badge the owner has earned. */
export interface EarnedBadge {
  readonly key: string;
  readonly tier: number;
  readonly name: string;
  readonly emoji: string;
  /** The study day it was earned, as `YYYY-MM-DD`. */
  readonly studyDay: string;
}

/** A locked badge's input against the threshold that earns it. */
export interface BadgeProgress {
  readonly value: number;
  readonly threshold: number;
}

/** A catalog badge the owner has not earned yet. */
export interface LockedBadge {
  readonly key: string;
  readonly name: string;
  readonly emoji: string;
  /** What earns it, in the catalog's words. */
  readonly criteria: string;
  readonly family: BadgeFamily;
  /** Its input against its threshold, or null when the server keeps no such input. */
  readonly progress: BadgeProgress | null;
}

/** The owner's badges as `GET /api/badges` answers them. */
export interface BadgesView {
  readonly earned: readonly EarnedBadge[];
  readonly locked: readonly LockedBadge[];
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;
const FAMILIES: readonly unknown[] = ['study', 'habit', 'focus'];

/** An earned badge, or undefined when `value` is not one. */
function earnedBadge(value: unknown): EarnedBadge | undefined {
  const { key, tier, name, emoji, study_day: studyDay } = (value ?? {}) as Record<string, unknown>;
  if (typeof key !== 'string' || typeof name !== 'string' || typeof emoji !== 'string') {
    return undefined;
  }
  if (typeof tier !== 'number') return undefined;
  if (typeof studyDay !== 'string' || !ISO_DATE.test(studyDay)) return undefined;
  return { key, tier, name, emoji, studyDay };
}

/** A locked badge's progress, null for none, or undefined when `value` is neither. */
function badgeProgress(value: unknown): BadgeProgress | null | undefined {
  if (value === null) return null;
  const { value: input, threshold } = (value ?? {}) as Record<string, unknown>;
  if (typeof input !== 'number' || typeof threshold !== 'number') return undefined;
  return { value: input, threshold };
}

/** A locked badge, or undefined when `value` is not one. */
function lockedBadge(value: unknown): LockedBadge | undefined {
  const given = (value ?? {}) as Record<string, unknown>;
  const { key, name, emoji, criteria, family } = given;
  if (typeof key !== 'string' || typeof name !== 'string' || typeof emoji !== 'string') {
    return undefined;
  }
  if (typeof criteria !== 'string') return undefined;
  if (!FAMILIES.includes(family)) return undefined;
  const progress = badgeProgress(given.progress);
  if (progress === undefined) return undefined;
  return { key, name, emoji, criteria, family: family as BadgeFamily, progress };
}

/** The body of `GET /api/badges`, or null when it is not one. */
export function parseBadges(body: unknown): BadgesView | null {
  const { earned, locked } = (body ?? {}) as Record<string, unknown>;
  if (!Array.isArray(earned) || !Array.isArray(locked)) return null;
  const earnedRows = earned.map(earnedBadge);
  const lockedRows = locked.map(lockedBadge);
  if (earnedRows.includes(undefined) || lockedRows.includes(undefined)) return null;
  return { earned: earnedRows as EarnedBadge[], locked: lockedRows as LockedBadge[] };
}
