/**
 * @vitest-environment jsdom
 */
import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import LevelScreen from './LevelScreen.svelte';
import type { LevelView } from './level';

// SPEC-072 R26, A28, A29. The level screen shows today's XP by source with each provisional row
// marked as settling at the day's close, and the consistency run as a flame meter with its
// multiplier and the one-miss preview beside it.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** A synthetic level view: a settled source, a provisional one, and a run of five days. */
function viewed(overrides: Partial<LevelView> = {}): LevelView {
  return {
    studyDay: '2001-02-03',
    level: 7,
    title: 'Adept',
    emoji: '🌿',
    totalXp: 1234,
    xpIntoLevel: 40,
    xpForNext: 200,
    today: [
      { source: 'reviews', track: 'language', amount: 24, state: 'settled' },
      { source: 'streak', track: 'language', amount: 10, state: 'provisional' }
    ],
    run: 5,
    multiplier: 1.25,
    multiplierAfterAMiss: 1.1,
    ascendant: false,
    ...overrides
  };
}

/** Each source row of today's XP, its whitespace collapsed. */
function rows(): string[] {
  const list = screen.getByRole('list', { name: "Today's XP by source" });
  return examined('source row(s)', within(list).getAllByRole('listitem')).map((row) =>
    (row.textContent ?? '').replace(/\s+/g, ' ').trim()
  );
}

describe('LevelScreen', () => {
  it("marks today's provisional XP as settling at the day's close", () => {
    render(LevelScreen, { props: { view: viewed() } });

    expect(rows()).toEqual(['Reviews 24', "Streak bonus 10 Settling at the day's close"]);
  });

  it('shows the one-miss preview beside the flame meter', () => {
    render(LevelScreen, { props: { view: viewed() } });

    const run = screen.getByRole('region', { name: 'Consistency run' });
    const flame = within(run).getByRole('meter', { name: 'Consistency run' }) as HTMLMeterElement;
    expect(flame.value).toBe(5);
    expect(within(run).getByText('5 days on pace')).toBeTruthy();
    expect(within(run).getByText('Multiplier x1.25')).toBeTruthy();
    expect(within(run).getByText('After one missed day: x1.10')).toBeTruthy();
  });

  it('names each section, the progress bar and the level bar text', () => {
    render(LevelScreen, { props: { view: viewed() } });

    expect(screen.getByRole('heading', { name: 'Your level' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: "Today's XP by source" })).toBeTruthy();
    expect(screen.getByText('40 of 200 XP to the next level')).toBeTruthy();
    const bar = screen.getByRole('meter', { name: 'Progress to the next level' }) as HTMLMeterElement;
    expect([bar.value, bar.max]).toEqual([40, 200]);
    expect(screen.getByText('🌿 7 Adept')).toBeTruthy();
    expect(screen.queryByText(/Ascendant day: today's XP earns a bonus/)).toBeNull();
  });

  it('says a day with no XP has none yet, and lists no rows', () => {
    render(LevelScreen, { props: { view: viewed({ today: [] }) } });

    expect(screen.getByText('No XP earned yet today.')).toBeTruthy();
    expect(screen.queryByRole('list', { name: "Today's XP by source" })).toBeNull();
  });

  it('shows a source it has no name for by its own token', () => {
    render(LevelScreen, {
      props: {
        view: viewed({ today: [{ source: 'tokenx', track: 'language', amount: 3, state: 'settled' }] })
      }
    });

    expect(rows()).toEqual(['tokenx 3']);
  });

  it('shows the Ascendant chip as text on an Ascendant day', () => {
    render(LevelScreen, { props: { view: viewed({ ascendant: true }) } });

    expect(screen.getByText(/Ascendant day: today's XP earns a bonus/)).toBeTruthy();
  });

  it('keeps the flame meter at a week until the run is longer', () => {
    const maxOf = (run: number): number => {
      const { unmount } = render(LevelScreen, { props: { view: viewed({ run }) } });
      const flame = screen.getByRole('meter', { name: 'Consistency run' }) as HTMLMeterElement;
      const max = flame.max;
      unmount();
      return max;
    };
    expect([maxOf(5), maxOf(7), maxOf(12)]).toEqual([7, 7, 12]);
  });
});
