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
});
