// SPEC-507 R6 to R8; ADR-525. The card health panel's lines (stub: the shapes only).
import type { HealthReadout } from '$lib/engine/protocol';

/** The words that would judge a card rather than state a fact about it (SPEC-507 R8). */
export const VERDICT = /$^/;

/** The recall rows' lines for `readout`. */
export function recallLines(readout: HealthReadout): { label: string | null; lines: string[] } {
  void readout;
  return { label: null, lines: [] };
}

/** `ms` as seconds with one decimal. */
export function seconds(ms: number): string {
  void ms;
  return '';
}

/** The UTC date of the instant `ms`, as `yyyy-mm-dd`. */
export function createdDate(ms: number): string {
  void ms;
  return '';
}

/** The name of the button a review row records. */
export function answerName(grade: number): string {
  void grade;
  return '';
}
