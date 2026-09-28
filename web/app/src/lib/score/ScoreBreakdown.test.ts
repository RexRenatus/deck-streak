/**
 * @vitest-environment jsdom
 */
import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import ScoreBreakdown from './ScoreBreakdown.svelte';
import type { DayScore } from './score';

// SPEC-071 R22, A25. The score screen's breakdown shows the total and the grade beside the five
// pillars, each pillar's value as text and as a bar; a retention the day does not have, because no
// review was answered, is a gap: its name and a sentence, never a 0 and never a bar.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** A synthetic day's score: a day with no answered review has no retention pillar. */
function scored(retention: number | null): DayScore {
  return {
    total: 64,
    grade: { label: 'SOLID', emoji: '✅' },
    pillars: { consistency: 76, retention, workload: 70.4, volume: 60.5, mastery: 55 }
  };
}

/** Each pillar's row: its name, and what it shows, its whitespace collapsed. */
function rows(): { name: string; text: string; meter: HTMLMeterElement | null }[] {
  const list = screen.getByRole('list', { name: 'The five pillars of the score' });
  return examined('pillar row(s)', within(list).getAllByRole('listitem')).map((row) => ({
    name: row.querySelector('span')?.textContent ?? '',
    text: (row.textContent ?? '').replace(/\s+/g, ' ').trim(),
    meter: row.querySelector('meter')
  }));
}

describe('ScoreBreakdown', () => {
  it('shows five pillars and an absent retention as a gap', () => {
    render(ScoreBreakdown, { props: { score: scored(null) } });

    // the total and the grade, beside the pillars
    expect(screen.getByText('64')).toBeTruthy();
    expect(screen.getByText('✅ SOLID')).toBeTruthy();

    const shown = rows();
    expect(shown.map((row) => row.name)).toEqual([
      'Consistency',
      'Retention',
      'Workload',
      'Volume',
      'Mastery'
    ]);

    // the retention is a gap: a sentence, no number, and no bar
    const retention = shown[1];
    expect(retention.text).toBe('Retention No review answered yet');
    expect(retention.text).not.toMatch(/\d/);
    expect(retention.meter).toBeNull();

    // every other pillar shows its value as text and as a bar named for it
    for (const [name, text, value] of [
      ['Consistency', '76', 76],
      ['Workload', '70', 70.4],
      ['Volume', '61', 60.5],
      ['Mastery', '55', 55]
    ] as const) {
      const row = shown.find((candidate) => candidate.name === name);
      expect(row?.text).toBe(`${name} ${text}`);
      const meter = screen.getByRole('meter', { name });
      expect([meter.min, meter.max, meter.value]).toEqual([0, 100, value]);
    }
  });

  it('shows a retention the day has as its value and its bar', () => {
    render(ScoreBreakdown, { props: { score: scored(85) } });

    const retention = rows()[1];
    expect(retention.text).toBe('Retention 85');
    expect(screen.getByRole('meter', { name: 'Retention' }).value).toBe(85);
    expect(screen.getAllByRole('meter')).toHaveLength(5);
  });
});
