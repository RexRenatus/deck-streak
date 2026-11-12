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
  T1: ['message'],
  T2: ['message'],
  T3: ['placeholder', 'message'],
  T4: ['dice', 'message'],
  T5: ['dice', 'message', 'card']
};

/** Every tier the router writes; an item of any other is not the feed's. */
const TIERS: readonly unknown[] = Object.keys(BEATS);

/**
 * Whether the device asks for reduced motion: a device with no media queries, as a test's document
 * may be, asks for nothing.
 */
export function reducedMotion(): boolean {
  return typeof matchMedia === 'function' && matchMedia(REDUCED_MOTION).matches;
}

/** Whether `candidate` is an item of the feed: a kind, a text and a tier the router writes. */
function isItem(candidate: unknown): candidate is FeedItem {
  const { kind, text, tier } = (candidate ?? {}) as Record<string, unknown>;
  return typeof kind === 'string' && typeof text === 'string' && TIERS.includes(tier);
}

/**
 * The body of the feed's route, or null when it is not one. An item that is not one is skipped,
 * and each item keeps only what the animation reads.
 */
export function parseFeed(body: unknown): FeedItem[] | null {
  const { items } = (body ?? {}) as Record<string, unknown>;
  if (!Array.isArray(items)) return null;
  return items.filter(isItem).map(({ kind, text, tier }) => ({ kind, text, tier }));
}

/** The items the feed serves, or none when it serves nothing: a feed it cannot read shows nothing. */
export async function celebrations(source: {
  feed(): Promise<Answer<FeedItem[]>>;
}): Promise<FeedItem[]> {
  const answer = await source.feed();
  return answer.kind === 'ok' ? answer.value : [];
}
