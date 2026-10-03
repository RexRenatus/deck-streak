/**
 * @vitest-environment jsdom
 */
import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import CourseLadder from './CourseLadder.svelte';
import type { CourseProgress } from './progress';

// SPEC-077 R17, A19. The progress screen draws each course's ladder as six band cells from the
// bands the route answers, each an HTML cell whose text names its band, its mastery and its mature
// cards; the current band's cell is marked, and the course states its current unit. The page
// computes nothing of its own: every figure is the server's, rounded to the whole percent.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

function textOf(element: Element | null | undefined): string {
  return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** A synthetic course: A1 and A2 achieved, B1 under way, B2 begun, C1 and C2 empty. */
function course(overrides: Partial<CourseProgress> = {}): CourseProgress {
  return {
    code: 'de',
    name: 'German',
    flag: '🇩🇪',
    masteryPct: 41.6,
    currentBand: 'A2',
    currentUnit: 14,
    bands: [
      { band: 'A1', total: 120, mature: 110, pct: 91.5, achieved: true },
      { band: 'A2', total: 140, mature: 118, pct: 82.25, achieved: true },
      { band: 'B1', total: 160, mature: 40, pct: 31.75, achieved: false },
      { band: 'B2', total: 90, mature: 2, pct: 4.4, achieved: false },
      { band: 'C1', total: 0, mature: 0, pct: 0, achieved: false },
      { band: 'C2', total: 0, mature: 0, pct: 0, achieved: false }
    ],
    ...overrides
  };
}

describe('the course ladder', () => {
  it('draws each band cell with its mastery and the current unit', () => {
    render(CourseLadder, { course: course() });

    const ladder = screen.queryByRole('list', { name: 'German, band by band' });
    const cells = examined('band cells', ladder ? within(ladder).queryAllByRole('listitem') : []);

    // six cells, A1 to C2, in the route's order, each stating its band and its mastery as text
    expect(cells.map((cell) => cell.dataset.band)).toEqual(['A1', 'A2', 'B1', 'B2', 'C1', 'C2']);
    expect(cells.map(textOf)).toEqual([
      'A1 92% mastery 110 of 120 mature Achieved',
      'A2 82% mastery 118 of 140 mature Achieved Current band',
      'B1 32% mastery 40 of 160 mature',
      'B2 4% mastery 2 of 90 mature',
      'C1 0% mastery 0 of 0 mature',
      'C2 0% mastery 0 of 0 mature'
    ]);
    // each cell's meter is the band's mastery
    expect(cells.map((cell) => cell.querySelector('meter')?.getAttribute('value'))).toEqual([
      '91.5',
      '82.25',
      '31.75',
      '4.4',
      '0',
      '0'
    ]);

    // the current band's cell is marked, and no other
    expect(cells.map((cell) => cell.getAttribute('aria-current'))).toEqual([null, 'step', null, null, null, null]);

    // the course states its band, its mastery and its current unit
    const heading = screen.queryByRole('heading', { name: /German/ });
    expect(textOf(heading)).toBe('🇩🇪 German');
    expect(textOf(heading?.parentElement?.querySelector('[data-course-summary]'))).toBe(
      'Band A2, 42% mastery. Current unit: 14'
    );
  });
});
