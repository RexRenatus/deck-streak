import { flushSync, mount, unmount } from 'svelte';
import { test } from 'vitest';
import type { CardView, Head, UndoOffer } from '#lib/engine/protocol.js';
import { m } from '#lib/paraglide/messages.js';
import type { StudyClient } from '../../src/lib/study/review';
import ReviewScreen from '../../src/lib/study/ReviewScreen.svelte';

// SPEC-618 R9, R13: the review screen over one card, as review-screen.test.ts builds it: the screen
// mounts, the card arrives and its question shows, Show answer turns it to the answer side, and the
// screen unmounts. The card and the engine's answers are fixed, so every run measures the same
// steps. The bench asserts nothing about what the screen shows.

const CARD: CardView = {
  id: BigInt(1),
  ordinal: 0,
  flag: 0,
  question: '<p>question 1</p>',
  answer: '<p>answer 1</p>',
  css: '.card { color: navy; }',
  labels: ['<1m', '<6m', '<10m', '4d'],
  undo: null,
  late: false
};

const HEAD: Head = { counts: { new: 1, learning: 0, review: 2 }, card: CARD };

/** The engine as the screen sees it: the one card, whatever is asked. */
const CLIENT: StudyClient = {
  card: async () => HEAD,
  rate: async () => null,
  bury: async () => null,
  flag: async () => 1,
  undo: async () => null,
  undoOffer: async (): Promise<UndoOffer> => ({ offer: null, why: 'none' })
};

/** The screen's Show answer button; a screen without one is no screen this bench measures. */
function showAnswer(target: HTMLElement): HTMLButtonElement {
  const label = m.study_show_answer();
  const found = Array.from(target.querySelectorAll('button')).find((button) => button.textContent?.trim() === label);
  if (found === undefined) throw new Error('the review screen shows no Show answer button');
  return found;
}

const BOUND_MS = 60_000; // the floor: 1.5 x its worst CI time, 23082 ms (job 127064902478), is less

test('ReviewScreen', { timeout: BOUND_MS }, async ({ bench }) => {
  await bench('ReviewScreen', async () => {
    const target = document.body.appendChild(document.createElement('div'));
    const screen = mount(ReviewScreen, { target, props: { client: async () => CLIENT } });
    // the card arrives once the answers in flight settle; then Svelte draws its question
    await new Promise((settled) => setTimeout(settled, 0));
    flushSync();
    showAnswer(target).click();
    flushSync();
    await unmount(screen);
    target.remove();
  }).run();
});
