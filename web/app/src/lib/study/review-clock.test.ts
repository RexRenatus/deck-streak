/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from 'vitest';
import type { CardView, Head, UndoOffer } from '$lib/engine/protocol';
import { Review, type StudyClient } from './review';

// SPEC-507 R9, A5. The answer clock excludes every overlay's time: over random sequences of a card
// shown, overlays opened and closed (one or two at once, as the panel and an editor may be), the
// question revealed and the card graded, with a fake clock, each grade sends the milliseconds the
// card was shown less every moment any overlay was open, and names the card shown.

/** A seeded generator (mulberry32), so a failing sequence repeats. */
function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function view(id: number): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false
  };
}

/** An engine that shows cards 1, 2, 3 and so on, and records each rating. */
class Cards implements StudyClient {
  next = 1;
  rated: string[] = [];

  async card(): Promise<Head> {
    const card = view(this.next);
    this.next += 1;
    return { counts: { new: 1, learning: 0, review: 0 }, card };
  }

  async rate(card: bigint, _rating: number, ms: number): Promise<null> {
    this.rated.push(`${card} ${ms}`);
    return null;
  }

  async bury(): Promise<null> {
    return null;
  }

  async flag(): Promise<number> {
    return 0;
  }

  async undo(): Promise<null> {
    return null;
  }

  async undoOffer(): Promise<UndoOffer> {
    return { offer: null, why: 'none' };
  }
}

describe('the answer clock', () => {
  it("the answer clock excludes every overlay's time", async () => {
    const SEQUENCES = 300;
    let grades = 0;
    let paused = 0;
    for (let seed = 1; seed <= SEQUENCES; seed += 1) {
      const random = generator(seed);
      let now = 0;
      const client = new Cards();
      const review = new Review(async () => client, () => now, () => undefined);
      review.start();
      await review.settled();

      // the model: the card shown, when it showed, and the overlays' time since
      let shown = review.face?.view.id ?? null;
      let shownAt = now;
      let overlays = 0;
      let openedAt = 0;
      let held = 0;
      const expected: string[] = [];

      for (let step = 0; step < 40; step += 1) {
        const roll = random();
        if (roll < 0.35) {
          // the clock moves, by a fraction of a millisecond up to a minute
          now += random() * 60_000;
        } else if (roll < 0.55 && overlays < 2 && review.face !== null) {
          if (overlays === 0) openedAt = now;
          overlays += 1;
          review.openOverlay();
        } else if (roll < 0.75 && overlays > 0) {
          overlays -= 1;
          if (overlays === 0) {
            held += now - openedAt;
            paused += 1;
          }
          review.closeOverlay();
        } else if (overlays === 0 && review.phase === 'question') {
          review.act('show-answer');
        } else if (overlays === 0 && review.phase === 'answer') {
          expected.push(`${shown} ${Math.round(now - shownAt - held)}`);
          review.act(random() < 0.5 ? 'again' : 'good');
          await review.settled();
          grades += 1;
          shown = review.face?.view.id ?? null;
          shownAt = now;
          held = 0;
          // an overlay still open when the next card shows holds its time from the show on
          if (overlays > 0) openedAt = now;
        }
      }
      expect(client.rated, `sequence ${seed}`).toEqual(expected);
    }
    console.log(`examined ${SEQUENCES} sequence(s), ${grades} grade(s), ${paused} pause(s)`);
    expect(grades).toBeGreaterThan(0);
    expect(paused).toBeGreaterThan(0);
  });
});
