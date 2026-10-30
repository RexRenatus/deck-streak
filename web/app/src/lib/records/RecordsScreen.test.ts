/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import RecordsScreen from './RecordsScreen.svelte';
import type { RecordsView } from './records';

// SPEC-073 R13, R19, A24. The records screen shows each record with its value, the day it was set
// and the value it beat, and a bar of today's distance to it: the bar's gap is the server's
// distance, and a record today has reached reads as reached. Then the record to chase.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** A synthetic view: three records, the minutes one already reached today, and a chase. */
function viewed(overrides: Partial<RecordsView> = {}): RecordsView {
  return {
    records: [
      {
        kind: 'best_score',
        label: 'Best daily score',
        value: 120,
        studyDay: '2025-01-09',
        previous: 100,
        today: 77,
        distance: 43
      },
      {
        kind: 'most_reviews',
        label: 'Most reviews in a day',
        value: 300,
        studyDay: '2025-01-10',
        previous: 250,
        today: 42,
        distance: 258
      },
      {
        kind: 'most_minutes',
        label: 'Most minutes in a day',
        value: 8,
        studyDay: '2025-01-11',
        previous: 5,
        today: 10,
        distance: 0
      }
    ],
    chase: { kind: 'best_score', label: 'Best daily score', gap: 43 },
    ...overrides
  };
}

/** The records list's items, or none when there is no list. */
function recordItems(): HTMLElement[] {
  const list = screen.queryByRole('list', { name: 'Personal records' });
  return list === null ? [] : Array.from(list.querySelectorAll<HTMLElement>(':scope > li'));
}

/** An element's text with its whitespace collapsed, or the empty string for no element. */
function textOf(element: Element | null | undefined): string {
  return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

describe('RecordsScreen', () => {
  it("shows each record's distance from today", () => {
    render(RecordsScreen, { props: { view: viewed() } });

    const items = recordItems();
    expect(items.map(textOf)).toEqual([
      'Best daily score 120 Set 2025-01-09, beating 100 43 to go today',
      'Most reviews in a day 300 Set 2025-01-10, beating 250 258 to go today',
      'Most minutes in a day 8 Set 2025-01-11, beating 5 Reached today'
    ]);
    const bars = items.map((item) => {
      const bar = item.querySelector('meter');
      return bar === null ? 'no bar' : [bar.max - bar.value, bar.getAttribute('aria-label')];
    });
    expect(bars).toEqual([
      [43, 'Today toward Best daily score'],
      [258, 'Today toward Most reviews in a day'],
      [0, 'Today toward Most minutes in a day']
    ]);
    examined('record(s)', items);
  });

  it('names the record to chase after the records', () => {
    render(RecordsScreen, { props: { view: viewed() } });

    const chase = screen.queryByText(/^Chase it/);
    expect(textOf(chase)).toBe('Chase it: 43 from “Best daily score” today.');
  });

  it('names no record to chase when the server names none', () => {
    render(RecordsScreen, { props: { view: viewed({ chase: null }) } });

    expect(recordItems().map((item) => item.dataset.kind)).toEqual([
      'best_score',
      'most_reviews',
      'most_minutes'
    ]);
    expect(screen.queryByText(/^Chase it/)).toBeNull();
  });

  it('says there are no records yet, and lists none, when the owner has none', () => {
    render(RecordsScreen, { props: { view: viewed({ records: [], chase: null }) } });

    expect(textOf(screen.queryByText(/^No records yet/))).toBe(
      'No records yet — they mint themselves as you study.'
    );
    expect(screen.queryByRole('list', { name: 'Personal records' })).toBeNull();
  });

  it('names its section', () => {
    render(RecordsScreen, { props: { view: viewed() } });

    expect(screen.queryAllByRole('heading').map(textOf)).toEqual(['Personal records']);
  });
});
