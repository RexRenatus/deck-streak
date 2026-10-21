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

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A board row, or undefined when `value` is not one. */
function boardRow(value: unknown): BoardRow | undefined {
  const given = (value ?? {}) as Record<string, unknown>;
  const { kind, emoji, label, value: figure, study_day: studyDay, longest, title } = given;
  if (typeof kind !== 'string' || typeof emoji !== 'string' || typeof label !== 'string') {
    return undefined;
  }
  if (typeof figure !== 'number') return undefined;
  // The server leaves out a field a row does not have; one it sends must be of its type.
  if (studyDay !== undefined && (typeof studyDay !== 'string' || !ISO_DATE.test(studyDay))) {
    return undefined;
  }
  if (longest !== undefined && typeof longest !== 'number') return undefined;
  if (title !== undefined && typeof title !== 'string') return undefined;
  return {
    kind,
    emoji,
    label,
    value: figure,
    studyDay: studyDay ?? null,
    longest: longest ?? null,
    title: title ?? null
  };
}

/** The body of `GET /api/board`, or null when it is not one. */
export function parseBoard(body: unknown): BoardView | null {
  const given = (body ?? {}) as Record<string, unknown>;
  if (!Array.isArray(given.rows)) return null;
  const rows = given.rows.map(boardRow);
  if (rows.includes(undefined)) return null;
  return { rows: rows as BoardRow[] };
}
