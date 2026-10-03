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
