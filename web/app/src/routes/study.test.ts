/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { hosted } from '$lib/card/frame-host';
import type { CardView, Deck, Head } from '$lib/engine/protocol';
import Study from './study/+page.svelte';
import StudyReview from './study/review/+page.svelte';

// SPEC-350 R6, R7; ADR-361. The two study routes: /study draws the deck list over the app's one
// engine and opens /study/review once a deck is chosen; /study/review draws the review screen over
// the same engine. The engine and SvelteKit's navigation are replaced before the routes' imports
// run.
const mocks = vi.hoisted(() => ({ client: vi.fn(), goto: vi.fn() }));
vi.mock('$lib/study/engine', () => ({ studyEngine: () => ({ client: mocks.client }) }));
vi.mock('$app/navigation', () => ({ goto: mocks.goto }));

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

afterEach(() => {
  mocks.client.mockReset();
  mocks.goto.mockReset();
});

describe('the study routes', () => {
  it('the deck list route studies the deck chosen, then opens the review', async () => {
    const calls: string[] = [];
    const deck: Deck = { id: 4n, name: 'Kana', level: 1, new: 2, learning: 0, review: 1, children: [] };
    mocks.client.mockResolvedValue({
      decks: async () => [deck],
      study: async (id: bigint) => {
        calls.push(`study ${id}`);
        return null;
      }
    });
    mocks.goto.mockResolvedValue(undefined);
    render(Study);
    await settle();

    await fireEvent.click(screen.getByRole('button', { name: 'Kana 2 new, 0 learning, 1 to review' }));
    await settle();
    expect([calls, mocks.goto.mock.calls]).toEqual([['study 4'], [['/study/review']]]);
  });

  it('the review route reviews the deck studied through the same engine', async () => {
    const card: CardView = {
      id: 9n,
      ordinal: 1,
      flag: 0,
      question: '<p>question 9</p>',
      answer: '<p>answer 9</p>',
      css: '',
      labels: ['<1m', '<6m', '<10m', '4d'],
      undo: null,
      late: false
    };
    const head: Head = { counts: { new: 1, learning: 0, review: 0 }, card };
    mocks.client.mockResolvedValue({ card: async () => head });
    render(StudyReview);
    await settle();

    const frame = screen.getByTitle("The card's question");
    const body = new DOMParser().parseFromString(hosted(frame.getAttribute('srcdoc') ?? '') ?? '', 'text/html').body;
    expect([body.className, body.textContent?.trim(), mocks.client.mock.calls.length > 0]).toEqual([
      'card card2',
      'question 9',
      true
    ]);
  });
});
