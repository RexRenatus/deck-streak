/**
 * The wallet's client types and the reading of `GET /api/wallet` (SPEC-082 R15, R17; ADR-315).
 *
 * The server answers the owner's balance, today's loss cap and what is left of it, and one page of
 * the coin movements, newest first, with the cursor of the next page. The page computes none of
 * these numbers and keeps the server's order.
 */

/** The wallet view. */
export const WALLET_PATH = '/api/wallet';

/** One coin movement, as the history lists it. */
export interface Movement {
  /** The movement's row: the next page's cursor when it is the last one shown. */
  readonly id: number;
  /** The study day it belongs to, as an ISO date. */
  readonly studyDay: string;
  /** What moved the coins, as the server names it. */
  readonly source: string;
  /** The signed whole coins it moved. */
  readonly amount: number;
}

/** The wallet as `GET /api/wallet` answers it. */
export interface WalletView {
  readonly studyDay: string;
  readonly balance: number;
  /** Today's loss cap, read from the wallet at the day's start. */
  readonly lossCap: number;
  /** What today's debits have left of the cap. */
  readonly lossCapLeft: number;
  /** One page of the movements, newest first. */
  readonly movements: readonly Movement[];
  /** The cursor of the next, older page, or null when there is none. */
  readonly next: number | null;
}

/** The path of the page after the movement `before`, or of the first page. */
export function walletPath(before?: number): string {
  return before === undefined ? WALLET_PATH : `${WALLET_PATH}?before=${before}`;
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** Whether `value` is an ISO calendar date, `YYYY-MM-DD`. */
function isIsoDate(value: unknown): value is string {
  return typeof value === 'string' && ISO_DATE.test(value);
}

/** One movement of the server's body, or null when `item` is not one. */
function parseMovement(item: unknown): Movement | null {
  if (item === null || typeof item !== 'object') return null;
  const { id, study_day: studyDay, source, amount } = item as Record<string, unknown>;
  if (!Number.isInteger(id) || !isIsoDate(studyDay) || typeof source !== 'string') return null;
  if (!Number.isInteger(amount)) return null;
  return { id: id as number, studyDay, source, amount: amount as number };
}

/** The body of `GET /api/wallet`, or null when it is not one. */
export function parseWallet(body: unknown): WalletView | null {
  if (body === null || typeof body !== 'object') return null;
  const given = body as Record<string, unknown>;
  const { study_day: studyDay, balance, loss_cap: lossCap, loss_cap_left: lossCapLeft } = given;
  if (!isIsoDate(studyDay)) return null;
  if (![balance, lossCap, lossCapLeft].every((number) => Number.isInteger(number))) return null;
  if (!Array.isArray(given.movements)) return null;
  const movements: Movement[] = [];
  for (const item of given.movements) {
    const movement = parseMovement(item);
    if (movement === null) return null;
    movements.push(movement);
  }
  const { next } = given;
  if (next !== null && !Number.isInteger(next)) return null;
  return {
    studyDay,
    balance: balance as number,
    lossCap: lossCap as number,
    lossCapLeft: lossCapLeft as number,
    movements,
    next: next as number | null
  };
}

/** A movement's amount with its sign: a deposit shows its plus. */
export function signed(amount: number): string {
  return amount > 0 ? `+${amount}` : String(amount);
}
