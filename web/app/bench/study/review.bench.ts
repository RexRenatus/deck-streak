import { test } from 'vitest';
import { step, type ReviewEvent, type ReviewState } from '../../src/lib/study/review';

// SPEC-618 R9, R13: the review state machine over one 200-press session: 100 cards, each shown,
// revealed and answered again or good (from a fixed-seed generator), then settled, so every run
// folds the same 400 events. The bench asserts nothing about the states it reaches.

/** A fixed-seed linear congruential generator, its values in [0, 1). */
function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 4294967296;
  };
}

/** The events of a session of `cards` cards, two presses each: Show answer, then a grade. */
function session(cards: number): ReviewEvent[] {
  const next = seeded(618);
  return Array.from({ length: cards }, (): ReviewEvent[] => [
    'view',
    'show-answer',
    next() < 0.2 ? 'again' : 'good',
    'settled'
  ]).flat();
}

const EVENTS = session(100);

test('step', async ({ bench }) => {
  await bench('step', () => {
    let state: ReviewState = { phase: 'loading', side: 'question' };
    for (const event of EVENTS) state = step(state, event).state;
  }).run();
});
