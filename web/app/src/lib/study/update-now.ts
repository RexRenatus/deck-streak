// SPEC-409 R2, R4 to R7, R12; ADR-423 D1, D2, D7. The state of the review's one-tap sync offer:
// hidden, shown, working, an outcome and dismissed. The screen draws it and the machine in
// `review.ts` counts the late reviews that raise it. A tap runs the client's own normal sync and
// then reads the count of reviews still waiting, as the Sync page's "Sync now" does, and names the
// outcome in the Sync page's own words. It never offers the full-sync choice: that stays on the
// Sync page, where its warnings are.
import type { Synced } from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';
import { choosing, signsIn, statusText, type SyncClient } from '$lib/sync/status';

/** What the offer asks of the engine's client: the two calls of the Sync page's "Sync now". */
export type UpdateClient = Pick<SyncClient, 'sync' | 'unsynced'>;

/** What the offer shows: whether it is up, whether a sync runs, its line, whether a link to the
 * Sync page follows, and the count of reviews still waiting once one has been read. */
export interface OfferFace {
  shown: boolean;
  working: boolean;
  line: string;
  link: boolean;
  count: number | null;
}

/** The line a sync's answer earns, and whether it sends the learner to the Sync page. */
function outcomeOf(answer: Synced): { line: string; link: boolean } {
  if (choosing(answer.required)) return { line: m.sync_full_required(), link: true };
  if (answer.status === 'held') return { line: m.study_update_done(), link: false };
  return { line: statusText(answer.status), link: signsIn(answer.status) };
}

export class UpdateNow {
  readonly #client: () => (() => Promise<UpdateClient>) | undefined;
  readonly #onChange: () => void;
  #shown = false;
  #working = false;
  #dismissed = false;
  #outcome: { line: string; link: boolean; count: number | null } | null = null;
  /** Which tap the screen still listens to: dismissing ends the running one's say. */
  #tap = 0;

  /** `client` reads the screen's update property: the engine's once started, or `undefined` on a
   * screen given none, which then never shows the offer; `onChange` follows each step. */
  constructor(client: () => (() => Promise<UpdateClient>) | undefined, onChange: () => void) {
    this.#client = client;
    this.#onChange = onChange;
  }

  /** What the screen draws now. */
  get face(): OfferFace {
    const outcome = this.#outcome;
    return {
      shown: this.#shown,
      working: this.#working,
      line: this.#working ? m.sync_working() : (outcome?.line ?? m.study_update_prompt()),
      link: outcome?.link ?? false,
      count: outcome?.count ?? null
    };
  }

  /** A late review settled: the offer shows, unless the screen has no client or the learner said
   * not now. It makes no call. */
  raise(): void {
    if (this.#client() === undefined || this.#dismissed || this.#shown) return;
    this.#shown = true;
    this.#onChange();
  }

  /** "Not now": the offer is gone for the screen's life, and a running sync ends with no line. */
  dismiss(): void {
    this.#dismissed = true;
    this.#shown = false;
    this.#working = false;
    this.#outcome = null;
    this.#tap += 1;
    this.#onChange();
  }

  /** "Sync now": one sync, then one read of the count, and the outcome's line. */
  async tap(): Promise<void> {
    const client = this.#client();
    if (client === undefined || this.#working) return;
    const tap = (this.#tap += 1);
    this.#working = true;
    this.#onChange();
    let outcome: { line: string; link: boolean; count: number | null };
    try {
      const engine = await client();
      const answer = await engine.sync();
      const waiting = await engine.unsynced();
      outcome = { ...outcomeOf(answer), count: waiting.reviews };
    } catch {
      outcome = { line: m.study_refused_engine(), link: false, count: null };
    }
    if (tap !== this.#tap) return;
    this.#working = false;
    this.#outcome = outcome;
    this.#onChange();
  }
}
