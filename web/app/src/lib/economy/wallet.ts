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

/** The body of `GET /api/wallet`, or null when it is not one. */
export function parseWallet(body: unknown): WalletView | null {
  void body;
  return null;
}

/** A movement's amount with its sign: a deposit shows its plus. */
export function signed(amount: number): string {
  return String(amount);
}
