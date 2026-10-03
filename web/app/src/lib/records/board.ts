/**
 * The personal board's client types (SPEC-075 R1, R3, #79).
 *
 * The board ranks the owner against their own past, never against another person: the best day
 * among the stored rollups, today, the language streak and the level, in the order the server
 * answers them. The page computes nothing: every figure, label and day is the server's.
 */

/** One row of the board, as `GET /api/board` answers it. */
export interface BoardRow {
  /** The row's kind, as the server names it: `best_day`, `today`, `streak` or `level`. */
  readonly kind: string;
  /** The row's emoji, decoration only: the label names the row. */
  readonly emoji: string;
  readonly label: string;
  readonly value: number;
  /** The study day the row is for, as `YYYY-MM-DD`; null for a row with no day. */
  readonly studyDay: string | null;
  /** The longest run, in days; null for a row that is not the streak. */
  readonly longest: number | null;
  /** The level's title; null for a row that is not the level. */
  readonly title: string | null;
}

/** The owner's board as `GET /api/board` answers it. */
export interface BoardView {
  readonly rows: readonly BoardRow[];
}
