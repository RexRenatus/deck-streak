/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import { EngineError } from '$lib/engine/client';
import type { Deck } from '$lib/engine/protocol';
import DeckList from './DeckList.svelte';

// SPEC-350 R6, R10, R11; ADR-361. The deck list shows the engine's deck tree with each deck's new,
// learning and review counts, and choosing a deck makes it the one studied, then opens the review.
// A refusal is announced in the status region with a way to ask again; a collection with no deck
// says so (ruling 317 OQ4: a message only).
function deck(id: number, name: string, counts: [number, number, number], children: Deck[] = []): Deck {
  const [fresh, learning, review] = counts;
  return { id: BigInt(id), name, level: 1, new: fresh, learning, review, children };
}

/** The engine as the deck list sees it: each call recorded, each answer queued or refused. */
class FakeDecks {
  calls: string[] = [];
  answers: (Deck[] | Error)[];
  refuseStudy: Error | null = null;

  constructor(answers: (Deck[] | Error)[]) {
    this.answers = answers;
  }

  async decks(): Promise<Deck[]> {
    this.calls.push('decks');
    const answer = this.answers.shift() as Deck[] | Error;
    if (answer instanceof Error) throw answer;
    return answer;
  }

  async study(id: bigint): Promise<null> {
    this.calls.push(`study ${id}`);
    if (this.refuseStudy !== null) throw this.refuseStudy;
    return null;
  }
}

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

function status(): string {
  return screen.getByRole('status').textContent?.trim() ?? '';
}

/** Each deck button's accessible name, in document order. */
function deckNames(): string[] {
  const names: string[] = [];
  screen.queryAllByRole('button', {
    name: (name) => {
      names.push(name);
      return true;
    }
  });
  return names;
}

describe('the deck list', () => {
  it('the deck list shows each deck with its counts and opens the one chosen', async () => {
    const engine = new FakeDecks([
      [deck(1, 'Default', [1, 0, 2], [deck(3, 'Kana', [0, 1, 0])]), deck(2, 'Spanish', [0, 0, 0])]
    ]);
    let opened = 0;
    const shown = render(DeckList, { client: async () => engine, onopen: () => (opened += 1) });

    // before the tree arrives: loading, and a quiet status region
    expect([screen.queryByText('Loading…') !== null, status()]).toEqual([true, '']);
    await settle();

    // the tree, its children nested under their parent, each deck named with its counts
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Study');
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Decks');
    expect(deckNames()).toEqual([
      'Default 1 new, 0 learning, 2 to review',
      'Kana 0 new, 1 learning, 0 to review',
      'Spanish 0 new, 0 learning, 0 to review'
    ]);
    const lists = [...shown.container.querySelectorAll('ul')];
    expect(lists).toHaveLength(2);
    const kana = screen.getByRole('button', { name: /^Kana/ });
    expect(kana.closest('ul')?.parentElement?.querySelector('button')?.textContent).toContain('Default');
    expect([screen.queryByText('Loading…'), status(), screen.queryByRole('button', { name: 'Try again' })]).toEqual([
      null,
      '',
      null
    ]);

    // choosing a deck makes it the deck studied, then opens the review
    await fireEvent.click(kana);
    await settle();
    expect([engine.calls, opened]).toEqual([['decks', 'study 3'], 1]);
  });

  it('the deck list announces a refusal and asks again', async () => {
    const engine = new FakeDecks([
      new EngineError('collection-busy', 'another tab has the collection'),
      new Error('the Worker stopped'),
      [deck(1, 'Default', [1, 0, 2])],
      [deck(1, 'Default', [0, 0, 2])]
    ]);
    let opened = 0;
    render(DeckList, { client: async () => engine, onopen: () => (opened += 1) });
    await settle();

    // a refusal is announced, with a way to ask again, and no list
    expect(status()).toBe('Your collection is open in another tab. Close it there, then try again.');
    expect([screen.queryByText('Loading…'), screen.queryByRole('list')]).toEqual([null, null]);
    // asking again loads again, quietly: the click is dispatched and drawn before the answer arrives
    screen.getByRole('button', { name: 'Try again' }).click();
    flushSync();
    expect([screen.queryByText('Loading…') !== null, status()]).toEqual([true, '']);
    await settle();

    // a failure that is no refusal of the engine's reads as the engine stopping
    expect(status()).toBe('The study engine stopped. Try again.');
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    await settle();
    expect([status(), deckNames()]).toEqual(['', ['Default 1 new, 0 learning, 2 to review']]);

    // a deck the engine refuses to study is announced, the review is not opened, and asking again reloads
    engine.refuseStudy = new EngineError('not-open', 'the collection is closed');
    await fireEvent.click(screen.getByRole('button', { name: /^Default/ }));
    await settle();
    expect([status(), opened]).toEqual(['Your collection is not open yet. Try again.', 0]);
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    await settle();
    expect([status(), deckNames(), engine.calls]).toEqual([
      '',
      ['Default 0 new, 0 learning, 2 to review'],
      ['decks', 'decks', 'decks', 'study 1', 'decks']
    ]);
  });

  it('a collection with no deck says so', async () => {
    render(DeckList, { client: async () => new FakeDecks([[]]), onopen: () => undefined });
    await settle();
    expect([screen.queryByText('There is no deck to study yet.') !== null, screen.queryByRole('list'), status()]).toEqual([
      true,
      null,
      ''
    ]);
  });
});
