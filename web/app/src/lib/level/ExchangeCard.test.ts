/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import ExchangeCard from './ExchangeCard.svelte';
import type { ExchangeView, SourceRate } from './exchange';

// SPEC-075 R9, A10. The level screen's exchange card shows, for each source bucket, the XP it paid,
// the cards that graduated on the days it paid and the XP per graduated card, with one decimal. A
// bucket no card graduated for has no rate, and the card says so in words: an undefined rate is
// never shown as 0.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** One bucket of a synthetic readout. */
function rate(source: string, totalXp: number, graduatedCards: number, value: number | null): SourceRate {
  return { source, totalXp, graduatedCards, rate: value, rateDefined: value !== null };
}

/** A synthetic readout over every day: one bucket with no graduation, one with a rate. */
const READOUT: ExchangeView = {
  window: null,
  rates: [rate('focus', 50, 0, null), rate('reviews', 10, 4, 2.5)]
};

/** An element's text with its whitespace collapsed, or the empty string for no element. */
function textOf(element: Element | null | undefined): string {
  return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** The readout table's body rows, each as its cells' text. */
function bodyRows(): string[][] {
  const table = screen.queryByRole('table');
  if (table === null) return [];
  return Array.from(table.querySelectorAll('tbody > tr')).map((row) =>
    Array.from(row.querySelectorAll(':scope > th, :scope > td')).map(textOf)
  );
}

describe('ExchangeCard', () => {
  it('renders an undefined rate as undefined, never zero', () => {
    render(ExchangeCard, { props: { view: READOUT } });

    const rows = examined('readout row(s)', bodyRows());
    expect(rows).toEqual([
      ['focus', '50', '0', 'Undefined: no card graduated'],
      ['Reviews', '10', '4', '2.5']
    ]);
    const undefinedRate = rows[0]?.[3] ?? '';
    expect(undefinedRate).not.toMatch(/\d/);
  });

  it('shows each bucket with its XP, its graduated cards and its rate to one decimal', () => {
    const view: ExchangeView = {
      window: { first: '2025-01-13', last: '2025-01-14' },
      rates: [
        rate('quest:', 75, 4, 18.75),
        rate('streak', 1234, 1, 1234),
        rate('studied', 30, 3, 10)
      ]
    };
    render(ExchangeCard, { props: { view } });

    expect(examined('readout row(s)', bodyRows())).toEqual([
      ['quest:', '75', '4', '18.8'],
      ['Streak bonus', '1234', '1', '1,234.0'],
      ['Studied today', '30', '3', '10.0']
    ]);
  });

  it('says no source has paid XP yet, and shows no table, when the readout is empty', () => {
    render(ExchangeCard, { props: { view: { window: null, rates: [] } } });

    expect(textOf(screen.queryByText(/^No XP/))).toBe('No XP has been earned yet, so there is no rate to show.');
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('names its table by its caption and each column by its header', () => {
    render(ExchangeCard, { props: { view: READOUT } });

    const table = screen.queryByRole('table', { name: 'XP paid per graduated card, by source' });
    expect(table?.tagName).toBe('TABLE');
    const headers = Array.from(table?.querySelectorAll('thead th') ?? []);
    expect(headers.map((header) => [textOf(header), header.getAttribute('scope')])).toEqual([
      ['Source', 'col'],
      ['XP', 'col'],
      ['Cards graduated', 'col'],
      ['XP per graduated card', 'col']
    ]);
    const sources = Array.from(table?.querySelectorAll('tbody th') ?? []);
    expect(sources.map((source) => [textOf(source), source.getAttribute('scope')])).toEqual([
      ['focus', 'row'],
      ['Reviews', 'row']
    ]);
  });

  it('names its section by its heading', () => {
    render(ExchangeCard, { props: { view: READOUT } });

    expect(screen.queryAllByRole('heading').map(textOf)).toEqual(['XP exchange rates']);
    expect(screen.queryByRole('region', { name: 'XP exchange rates' })?.tagName).toBe('SECTION');
  });
});
