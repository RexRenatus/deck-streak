/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Answer } from '$lib/api';
import type { Deck } from '$lib/engine/protocol';
import AiDecksRoute from '../../routes/study/ai-decks/+page.svelte';
import AiDecks from './AiDecks.svelte';
import type { MarksClient } from './ai-decks';
import DeckList from './DeckList.svelte';

// SPEC-381 R8, R9, A12-A14; ADR-392 D3. The AI-and-your-decks screen shows the engine's deck tree
// with one native switch per deck, off unless the server holds the deck's mark. A deck under a
// marked deck shows on, cannot be changed, and names the nearest marked deck above it. Turning a
// switch saves at once; a change the server did not save is turned back and announced. The route
// draws the screen over the app's one engine and its one API client, which are replaced before the
// route's imports run.
const mocks = vi.hoisted(() => ({ client: vi.fn(), sensitiveDecks: vi.fn(), setSensitive: vi.fn() }));
vi.mock('$lib/study/engine', () => ({ studyEngine: () => ({ client: mocks.client }) }));
vi.mock('$lib/api', () => ({
  api: { sensitiveDecks: mocks.sensitiveDecks, setSensitive: mocks.setSensitive }
}));

afterEach(() => {
  mocks.client.mockReset();
  mocks.sensitiveDecks.mockReset();
  mocks.setSensitive.mockReset();
});

function deck(id: number, name: string, children: Deck[] = []): Deck {
  return { id: BigInt(id), name, level: 1, new: 0, learning: 0, review: 0, children };
}

/** The engine as the screen sees it: the deck tree, or a refusal. */
function engine(tree: Deck[] | Error) {
  return async () => ({
    decks: async () => {
      if (tree instanceof Error) throw tree;
      return tree;
    }
  });
}

/**
 * The server as the screen sees it: it keeps the marked set, records each change it is asked for,
 * and answers the set after the change, or refuses to save while `refuseSave` is set. A load
 * answers the next of `loads` while any is queued, then the set it keeps.
 */
class FakeMarks implements MarksClient {
  calls: [string, boolean][] = [];
  marked: Set<string>;
  loads: Answer<string[]>[];
  refuseSave = false;

  constructor(marked: string[], loads: Answer<string[]>[] = []) {
    this.marked = new Set(marked);
    this.loads = loads;
  }

  private answer(): Answer<string[]> {
    return { kind: 'ok', value: [...this.marked].sort((a, b) => Number(a) - Number(b)) };
  }

  async sensitiveDecks(): Promise<Answer<string[]>> {
    return this.loads.shift() ?? this.answer();
  }

  async setSensitive(id: string, sensitive: boolean): Promise<Answer<string[]>> {
    this.calls.push([id, sensitive]);
    if (this.refuseSave) return { kind: 'unavailable' };
    if (sensitive) this.marked.add(id);
    else this.marked.delete(id);
    return this.answer();
  }
}

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

function status(): string {
  return screen.getByRole('status').textContent?.trim() ?? '';
}

/** Each switch in document order: its label, whether it is on, whether it is locked, and its note. */
function shown(): [string, boolean, boolean, string][] {
  return screen.queryAllByRole('switch').map((element) => {
    const box = element as HTMLInputElement;
    const described = box.getAttribute('aria-describedby');
    const note = described === null ? '' : (document.getElementById(described)?.textContent?.trim() ?? '');
    const label = box.closest('label')?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
    return [label, box.checked, box.disabled, note];
  });
}

function switchFor(name: string): HTMLInputElement {
  return screen.getByRole('switch', { name: `Keep ${name} away from AI` }) as HTMLInputElement;
}

const LEAD =
  "Turn on a deck's switch to keep its cards away from AI features. Studying works the same either way.";

describe('the AI-and-your-decks screen', () => {
  it("a deck's switch is off until it is turned on, and turning it on saves the mark", async () => {
    const tree = [deck(1, 'Default', [deck(3, 'Kana')]), deck(2, 'Spanish')];
    const marks = new FakeMarks([]);
    const first = render(AiDecks, { client: engine(tree), marks });

    // before the tree arrives: loading, and a quiet status region
    expect([screen.queryByText('Loading…') !== null, status()]).toEqual([true, '']);
    await settle();

    // every switch off, none locked, each a native checkbox whose row is a target 44 px tall
    expect([screen.getByRole('heading', { level: 1 }).textContent, screen.queryByText(LEAD) !== null]).toEqual([
      'AI and your decks',
      true
    ]);
    expect(shown()).toEqual([
      ['Keep Default away from AI', false, false, ''],
      ['Keep Kana away from AI', false, false, ''],
      ['Keep Spanish away from AI', false, false, '']
    ]);
    expect(
      screen
        .getAllByRole('switch')
        .map((box) => [box.tagName, box.getAttribute('type'), box.closest('label')?.classList.contains('min-h-11')])
    ).toEqual([
      ['INPUT', 'checkbox', true],
      ['INPUT', 'checkbox', true],
      ['INPUT', 'checkbox', true]
    ]);

    // turning a switch on saves the mark at once, and the switch stays on
    await fireEvent.click(switchFor('Spanish'));
    await settle();
    expect([marks.calls, shown()[2], status(), [...marks.marked]]).toEqual([
      [['2', true]],
      ['Keep Spanish away from AI', true, false, ''],
      '',
      ['2']
    ]);

    // the mark is the server's: a fresh screen shows it on, and turning it off saves the unmark
    first.unmount();
    render(AiDecks, { client: engine(tree), marks });
    await settle();
    expect(shown().map(([label, on]) => [label, on])).toEqual([
      ['Keep Default away from AI', false],
      ['Keep Kana away from AI', false],
      ['Keep Spanish away from AI', true]
    ]);
    await fireEvent.click(switchFor('Spanish'));
    await settle();
    expect([marks.calls, shown()[2], [...marks.marked]]).toEqual([
      [
        ['2', true],
        ['2', false]
      ],
      ['Keep Spanish away from AI', false, false, ''],
      []
    ]);
  });

  it('a deck under a kept-away deck shows on and cannot be changed', async () => {
    const tree = [deck(1, 'Default', [deck(3, 'Kana', [deck(5, 'Hiragana')])]), deck(2, 'Spanish')];
    const marks = new FakeMarks(['1']);
    const first = render(AiDecks, { client: engine(tree), marks });
    await settle();

    // the marked deck is on and can be changed; each deck under it is on, locked, and names it
    expect(shown()).toEqual([
      ['Keep Default away from AI', true, false, ''],
      ['Keep Kana away from AI', true, true, 'Kept away because Default is.'],
      ['Keep Hiragana away from AI', true, true, 'Kept away because Default is.'],
      ['Keep Spanish away from AI', false, false, '']
    ]);

    // a locked switch cannot be changed: a press saves nothing and leaves it on
    switchFor('Kana').click();
    await settle();
    expect([marks.calls, shown()[1]]).toEqual([[], ['Keep Kana away from AI', true, true, 'Kept away because Default is.']]);

    // turning the marked deck off frees every deck under it
    await fireEvent.click(switchFor('Default'));
    await settle();
    expect([marks.calls, shown()]).toEqual([
      [['1', false]],
      [
        ['Keep Default away from AI', false, false, ''],
        ['Keep Kana away from AI', false, false, ''],
        ['Keep Hiragana away from AI', false, false, ''],
        ['Keep Spanish away from AI', false, false, '']
      ]
    ]);

    // with two marked decks on one branch, a deck names the nearer one
    first.unmount();
    render(AiDecks, { client: engine(tree), marks: new FakeMarks(['1', '3']) });
    await settle();
    expect(shown()).toEqual([
      ['Keep Default away from AI', true, false, ''],
      ['Keep Kana away from AI', true, true, 'Kept away because Default is.'],
      ['Keep Hiragana away from AI', true, true, 'Kept away because Kana is.'],
      ['Keep Spanish away from AI', false, false, '']
    ]);
  });

  it('a failed save turns the switch back and says so', async () => {
    const tree = [deck(1, 'Default'), deck(2, 'Spanish')];
    const marks = new FakeMarks([]);
    marks.refuseSave = true;
    render(AiDecks, { client: engine(tree), marks });
    await settle();

    // the server did not save: the switch is turned back, and the status says so
    await fireEvent.click(switchFor('Spanish'));
    await settle();
    expect([marks.calls, shown(), status()]).toEqual([
      [['2', true]],
      [
        ['Keep Default away from AI', false, false, ''],
        ['Keep Spanish away from AI', false, false, '']
      ],
      'That change was not saved. Try again.'
    ]);

    // the next change that saves clears the message
    marks.refuseSave = false;
    await fireEvent.click(switchFor('Spanish'));
    await settle();
    expect([marks.calls, shown()[1], status()]).toEqual([
      [
        ['2', true],
        ['2', true]
      ],
      ['Keep Spanish away from AI', true, false, ''],
      ''
    ]);

    // a failed unmark turns the switch back on
    marks.refuseSave = true;
    await fireEvent.click(switchFor('Spanish'));
    await settle();
    expect([marks.calls.at(-1), shown()[1], status()]).toEqual([
      ['2', false],
      ['Keep Spanish away from AI', true, false, ''],
      'That change was not saved. Try again.'
    ]);
  });

  // Mutation coverage: a failed load, from the server or from the engine, says so and asks again.
  it('a failed load says so and asks again', async () => {
    const tree = [deck(1, 'Default')];
    const marks = new FakeMarks([], [{ kind: 'unavailable' }, { kind: 'reopen' }]);
    render(AiDecks, { client: engine(tree), marks });
    await settle();

    expect([status(), shown(), screen.queryByText('Loading…')]).toEqual(['These settings could not be loaded.', [], null]);
    screen.getByRole('button', { name: 'Try again' }).click();
    flushSync();
    expect([screen.queryByText('Loading…') !== null, status()]).toEqual([true, '']);
    await settle();
    expect([status(), shown()]).toEqual(['These settings could not be loaded.', []]);

    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    await settle();
    expect([status(), shown(), screen.queryByRole('button', { name: 'Try again' })]).toEqual([
      '',
      [['Keep Default away from AI', false, false, '']],
      null
    ]);
    expect(screen.getByRole('link', { name: 'Back to decks' }).getAttribute('href')).toBe('/study');
  });

  // Mutation coverage: an engine that refuses the deck tree is a failed load too.
  it('an engine that refuses the deck tree is a failed load', async () => {
    render(AiDecks, { client: engine(new Error('the Worker stopped')), marks: new FakeMarks([]) });
    await settle();
    expect([status(), shown(), screen.queryByRole('button', { name: 'Try again' }) !== null]).toEqual([
      'These settings could not be loaded.',
      [],
      true
    ]);
  });

  // Mutation coverage: the route draws the screen over the app's one engine and its API client.
  it('the route reads the deck tree from the engine and the marks from the server', async () => {
    mocks.client.mockResolvedValue({ decks: async () => [deck(4, 'Kana')] });
    mocks.sensitiveDecks.mockResolvedValue({ kind: 'ok', value: [] });
    render(AiDecksRoute);
    await settle();
    expect([shown(), mocks.client.mock.calls.length > 0, mocks.sensitiveDecks.mock.calls.length]).toEqual([
      [['Keep Kana away from AI', false, false, '']],
      true,
      1
    ]);
  });

  // Mutation coverage: the deck list links to the screen.
  it('the deck list links to AI and your decks', async () => {
    render(DeckList, { client: async () => ({ decks: async () => [], study: async () => null }), onopen: () => undefined });
    await settle();
    expect(screen.getByRole('link', { name: 'AI and your decks' }).getAttribute('href')).toBe('/study/ai-decks');
  });
});
