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

/** The body of `GET /api/badges`, or null when it is not one. */
export function parseBadges(body: unknown): BadgesView | null {
  void body;
  return null;
}
