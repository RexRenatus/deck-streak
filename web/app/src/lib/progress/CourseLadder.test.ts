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

// SPEC-408 R2, R4, A2. Each course says what its mastery measures, in plain words, right after its
// summary.
const COURSE_ABOUT =
  'Mastery is an estimate from your reviews: the average, over the cards it counts, of how likely you are to recall each card now, with cards not yet firmly learned counted for less and new or suspended cards counted as 0.';
const LAW_ABOUT =
  'Law mastery starts at 100% and drops 3 points for each active law leech, a law card you keep forgetting, never below 70%; it does not measure how much law you know.';

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
  it('each course says what its mastery measures right after its summary', () => {
    const { container } = render(CourseLadder, { course: course() });

    const summary = container.querySelector('[data-course-summary]');
    const about = summary?.nextElementSibling;
    expect(about?.getAttribute('data-mastery-about')).toBe('course');
    expect(textOf(about)).toBe(COURSE_ABOUT);
    const shown = examined('shown mastery descriptions', [...container.querySelectorAll('[data-mastery-about]')]);
    expect(shown).toHaveLength(1);
    expect(textOf(container)).not.toContain(LAW_ABOUT);
  });

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
    // each cell's meter is the band's mastery, out of 100, named by its band
    const meters = cells.map((cell) => cell.querySelector('meter'));
    expect(meters.map((meter) => [meter?.min, meter?.max, meter?.value])).toEqual([
      [0, 100, 91.5],
      [0, 100, 82.25],
      [0, 100, 31.75],
      [0, 100, 4.4],
      [0, 100, 0],
      [0, 100, 0]
    ]);
    expect(meters.map((meter) => meter?.getAttribute('aria-label'))).toEqual([
      'A1 mastery',
      'A2 mastery',
      'B1 mastery',
      'B2 mastery',
      'C1 mastery',
      'C2 mastery'
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

  it('names the course as its region, and says so when no unit is current yet', () => {
    render(CourseLadder, { course: course({ code: 'fr', name: 'French', flag: '🇫🇷', currentUnit: null, currentBand: 'A1' }) });

    const region = screen.queryByRole('region', { name: 'French' });
    expect(textOf(region?.querySelector('[data-course-summary]'))).toBe('Band A1, 42% mastery. No unit yet');
    const cells = examined('band cells', region ? within(region).queryAllByRole('listitem') : []);
    expect(cells.map((cell) => cell.getAttribute('aria-current'))).toEqual(['step', null, null, null, null, null]);
    expect(cells.filter((cell) => textOf(cell).endsWith('Current band')).map((cell) => cell.dataset.band)).toEqual(['A1']);
  });
});
