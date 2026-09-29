import type { Answer } from '../api';

/**
 * The owner's in-app feed as the ladder reads it (SPEC-084 R10; ADR-041's origin rule): each item
 * the router delivered to the Mini App, with the tier it rendered at, and what each tier shows.
 */

/** A celebration's tier, as the router writes it. */
export type Tier = 'T0' | 'T1' | 'T2' | 'T3' | 'T4' | 'T5';

/** One beat of a tier's animation: the reveal's placeholder, the dice, the message, the card. */
export type Beat = 'placeholder' | 'dice' | 'message' | 'card';

/** One item of the feed. */
export interface FeedItem {
  readonly kind: string;
  readonly text: string;
  readonly tier: Tier;
}

/** The feed's route. */
export const FEED_PATH = '/api/notifications/feed';

/** The dice a T4 or a T5 rolls, the bot's own. */
export const DICE = '\u{1f3b0}';

/** The media query of a device that asks for reduced motion. */
export const REDUCED_MOTION = '(prefers-reduced-motion: reduce)';

/** Each tier's beats, in the order the animation shows them. */
export const BEATS: Readonly<Record<Tier, readonly Beat[]>> = {
  T0: [],
  T1: [],
  T2: [],
  T3: [],
  T4: [],
  T5: []
};

/** Whether the device asks for reduced motion. */
export function reducedMotion(): boolean {
  return false;
}

/** The body of the feed's route, or null when it is not one. */
export function parseFeed(_body: unknown): FeedItem[] | null {
  return null;
}

/** The items the feed serves, or none when it serves nothing. */
export async function celebrations(source: {
  feed(): Promise<Answer<FeedItem[]>>;
}): Promise<FeedItem[]> {
  await source.feed();
  return [];
}
