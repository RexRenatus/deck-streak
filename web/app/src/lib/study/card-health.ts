// SPEC-507 R6 to R8; ADR-525. The card health panel's lines: the recall rows, which show a number only
// from the scheduler's Ready readout and the state's line otherwise; the times in seconds; the created
// date; and the name of the button each review row records. The panel judges no card.
import type { HealthReadout } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';

/** The words that would judge a card rather than state a fact about it (SPEC-507 R8). */
export const VERDICT = /\b(hard|difficult|ambiguous|leech|risk|risky|problem|problems|struggl\w*|weak|trouble\w*)\b|at risk/i;

/** Each state in which the scheduler gives no readout, and the line its recall rows read. */
const STATE_LINES: Record<Exclude<HealthReadout['state'], 'ready'>, () => string> = {
  absent: () => m.card_health_absent(),
  fixture: () => m.card_health_fixture(),
  unready: () => m.card_health_unready(),
  stale: () => m.card_health_stale()
};

/** The recall rows for `readout`: under the scheduler's label, its two figures when it is Ready; the
 * state's line, with no label and no number, in every other state (SPEC-507 R6). */
export function recallLines(readout: HealthReadout): { label: string | null; lines: string[] } {
  if (readout.state === 'ready') {
    return {
      label: m.card_health_estimates(),
      lines: [
        m.card_health_recall_chance({ percent: (readout.recallPpm / 10_000).toFixed(1) }),
        m.card_health_days_to_90({ days: String(readout.daysTo90) })
      ]
    };
  }
  return { label: null, lines: [STATE_LINES[readout.state]()] };
}

/** `ms` as seconds with one decimal. */
export function seconds(ms: number): string {
  return m.card_health_seconds({ seconds: (ms / 1000).toFixed(1) });
}

/** The UTC date of the instant `ms`, as `yyyy-mm-dd`. */
export function createdDate(ms: number): string {
  return new Date(ms).toISOString().slice(0, 10);
}

/** The name of the button a review row records, by the review's own answer names; 0 is an entry set
 * by hand. */
export function answerName(grade: number): string {
  const names: Record<number, () => string> = {
    0: () => m.card_health_manual(),
    1: () => m.study_again(),
    2: () => m.answer_hard(),
    3: () => m.study_good(),
    4: () => m.answer_easy()
  };
  return names[grade]?.() ?? String(grade);
}
