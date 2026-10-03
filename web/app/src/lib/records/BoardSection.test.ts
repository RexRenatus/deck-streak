/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import BoardSection from './BoardSection.svelte';
import type { BoardView } from './board';

// SPEC-075 R3, A9. The records screen's board section shows the owner's best day, today, the
// language streak and the level, in the server's order, each with the server's label, value and
// day, the streak with its longest run and the level with its title. The emoji is decoration, so
// assistive technology never reads it. The board ranks the owner against their own past only.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** A synthetic board: the four rows the server answers when a rollup exists. */
const BOARD: BoardView = {
  rows: [
    { kind: 'best_day', emoji: '🏅', label: 'Best day', value: 90, studyDay: '2025-01-11', longest: null, title: null },
    { kind: 'today', emoji: '📅', label: 'Today', value: 77, studyDay: '2025-01-14', longest: null, title: null },
    { kind: 'streak', emoji: '🔥', label: 'Streak', value: 3, studyDay: null, longest: 9, title: null },
    { kind: 'level', emoji: '⚡', label: 'Level', value: 9, studyDay: null, longest: null, title: 'Seedling' }
  ]
};

/** The board list's items, or none when there is no list. */
function boardItems(): HTMLElement[] {
  const list = screen.queryByRole('list', { name: 'Your personal board' });
  return list === null ? [] : Array.from(list.querySelectorAll<HTMLElement>(':scope > li'));
}

/** An element's text with its whitespace collapsed, or the empty string for no element. */
function textOf(element: Element | null | undefined): string {
  return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** An element's text without what assistive technology is told to skip. */
function spokenOf(element: HTMLElement): string {
  const copy = element.cloneNode(true) as HTMLElement;
  copy.querySelectorAll('[aria-hidden="true"]').forEach((hidden) => hidden.remove());
  return textOf(copy);
}

describe('BoardSection', () => {
  it('shows the best day, today, the streak and the level', () => {
    render(BoardSection, { props: { view: BOARD } });

    const items = examined('board row(s)', boardItems());
    expect(items.map((item) => item.dataset.kind)).toEqual(['best_day', 'today', 'streak', 'level']);
    expect(items.map(spokenOf)).toEqual([
      'Best day 90 2025-01-11',
      'Today 77 2025-01-14',
      'Streak 3 Longest run 9 days',
      'Level 9 Seedling'
    ]);
    expect(items.map(textOf)).toEqual([
      '🏅 Best day 90 2025-01-11',
      '📅 Today 77 2025-01-14',
      '🔥 Streak 3 Longest run 9 days',
      '⚡ Level 9 Seedling'
    ]);
    expect(
      items.map((item) => Array.from(item.querySelectorAll('[aria-hidden="true"]')).map(textOf))
    ).toEqual([['🏅'], ['📅'], ['🔥'], ['⚡']]);
  });

  it('shows the streak and the level alone when no day was stored yet', () => {
    render(BoardSection, { props: { view: { rows: BOARD.rows.slice(2) } } });

    expect(examined('board row(s)', boardItems()).map(spokenOf)).toEqual([
      'Streak 3 Longest run 9 days',
      'Level 9 Seedling'
    ]);
  });

  it('names its section by its heading', () => {
    render(BoardSection, { props: { view: BOARD } });

    const headings = screen.queryAllByRole('heading');
    expect(headings.map(textOf)).toEqual(['Your personal board']);
    expect(screen.queryByRole('region', { name: 'Your personal board' })?.tagName).toBe('SECTION');
  });
});
