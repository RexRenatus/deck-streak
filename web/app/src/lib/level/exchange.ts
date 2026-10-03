/**
 * The XP exchange readout's client types (SPEC-075 R4, R7, R9, #80).
 *
 * The server answers, for each source bucket, the XP it paid, the cards that graduated on the days
 * it paid, and the XP it paid per graduated card. A bucket with no graduation has no rate: the
 * server sends `rate` null beside `rate_defined` false, and the card shows it as undefined, never
 * as 0. The page computes nothing: every figure is the server's.
 */

/** The study days a windowed readout covers, both ends included, as `YYYY-MM-DD`. */
export interface ExchangeWindow {
  readonly first: string;
  readonly last: string;
}

/** One source bucket's rate. */
export interface SourceRate {
  /** The bucket, as the server names it: a source's text up to and including its first colon. */
  readonly source: string;
  readonly totalXp: number;
  readonly graduatedCards: number;
  /** XP per graduated card, or null when no card graduated. */
  readonly rate: number | null;
  /** Whether a card graduated, so the rate is defined; false exactly when `rate` is null. */
  readonly rateDefined: boolean;
}

/** The readout as `GET /api/xp/exchange` answers it. */
export interface ExchangeView {
  /** The window read, or null when every day was read. */
  readonly window: ExchangeWindow | null;
  /** The buckets, ordered by name. */
  readonly rates: readonly SourceRate[];
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** Whether `value` is a study day as `YYYY-MM-DD`. */
function isDay(value: unknown): value is string {
  return typeof value === 'string' && ISO_DATE.test(value);
}

/** The window read, null for every day, or undefined when `value` is neither. */
function exchangeWindow(value: unknown): ExchangeWindow | null | undefined {
  if (value === null) return null;
  const { first, last } = (value ?? {}) as Record<string, unknown>;
  if (!isDay(first) || !isDay(last)) return undefined;
  return { first, last };
}

/** A bucket's rate, or undefined when `value` is not one. */
function sourceRate(value: unknown): SourceRate | undefined {
  const given = (value ?? {}) as Record<string, unknown>;
  const {
    source,
    total_xp: totalXp,
    graduated_cards: graduatedCards,
    rate,
    rate_defined: rateDefined
  } = given;
  if (typeof source !== 'string' || typeof totalXp !== 'number') return undefined;
  if (typeof graduatedCards !== 'number' || typeof rateDefined !== 'boolean') return undefined;
  // A rate is defined exactly when a card graduated: a null rate the server calls defined, or a
  // number it calls undefined, is a malformed body (SPEC-075 R7).
  if (rateDefined ? typeof rate !== 'number' : rate !== null) return undefined;
  return { source, totalXp, graduatedCards, rate: rate as number | null, rateDefined };
}

/** The body of `GET /api/xp/exchange`, or null when it is not one. */
export function parseExchange(body: unknown): ExchangeView | null {
  const given = (body ?? {}) as Record<string, unknown>;
  const window = exchangeWindow(given.window);
  if (window === undefined || !Array.isArray(given.rates)) return null;
  const rates = given.rates.map(sourceRate);
  if (rates.includes(undefined)) return null;
  return { window, rates: rates as SourceRate[] };
}
