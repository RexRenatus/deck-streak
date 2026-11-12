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
function rows(): { name: string; value: string | null | undefined; text: string; meter: HTMLMeterElement | null }[] {
  const list = screen.getByRole('list', { name: 'The five pillars of the score' });
  return examined('pillar row(s)', within(list).getAllByRole('listitem')).map((row) => ({
    name: row.querySelector('span')?.textContent ?? '',
    value: row.querySelectorAll('span')[1]?.textContent,
    text: (row.textContent ?? '').replace(/\s+/g, ' ').trim(),
    meter: row.querySelector('meter')
  }));
}

// SPEC-606 R1, R2, R5; ADR-627 D1, D2, D5. Each pillar's sentence, spelled literally here: a test
// that read it from the message files would take its expected value from the data under test.
const ABOUT = {
  Consistency:
    'Whether you studied today and how long your streak is: studying today gives 70, and your streak adds up to 30 more as it grows to 25 days.',
  Retention:
    'The share of review cards you recalled on their first answer today: 50% or less scores 0, 95% or more scores 100, and with fewer than 5 answers it moves halfway toward 70.',
  Workload:
    'The share of the review cards due today that you cleared, less 1 point for every 5 overdue cards, at most 40.',
  Volume:
    'How much you studied today, in reviews and minutes, against your usual day: a usual day scores 80, and a quarter more scores 100.',
  Mastery:
    'Not a mastery percentage: 3 cards reaching a review interval of 21 days or more today score 100, and each active leech, a card you keep forgetting, takes off 3 points, at most 30.'
} as const;

/** Each pillar's list item, by its `data-pillar`. */
function items(): HTMLElement[] {
  const list = screen.getByRole('list', { name: 'The five pillars of the score' });
  return examined('pillar row(s)', within(list).getAllByRole('listitem'));
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
    expect(retention.text).toBe(`Retention No review answered yet ${ABOUT.Retention}`);
    expect(retention.value).not.toMatch(/\d/);
    expect(retention.meter).toBeNull();

    // every other pillar shows its value as text and as a bar named for it
    for (const [name, text, value] of [
      ['Consistency', '76', 76],
      ['Workload', '70', 70.4],
      ['Volume', '61', 60.5],
      ['Mastery', '55', 55]
    ] as const) {
      const row = shown.find((candidate) => candidate.name === name);
      expect(row?.text).toBe(`${name} ${text} ${ABOUT[name]}`);
      const meter = screen.getByRole('meter', { name }) as HTMLMeterElement;
      expect([meter.min, meter.max, meter.value]).toEqual([0, 100, value]);
    }
  });

  it('shows a retention the day has as its value and its bar', () => {
    render(ScoreBreakdown, { props: { score: scored(85) } });

    const retention = rows()[1];
    expect(retention.text).toBe(`Retention 85 ${ABOUT.Retention}`);
    expect((screen.getByRole('meter', { name: 'Retention' }) as HTMLMeterElement).value).toBe(85);
    expect(screen.getAllByRole('meter')).toHaveLength(5);
  });

  it('each pillar says in plain words what it measures beside its figure', () => {
    render(ScoreBreakdown, { props: { score: scored(85) } });

    const shown = items();
    expect(shown.map((item) => item.dataset.pillar)).toEqual([
      'consistency',
      'retention',
      'workload',
      'volume',
      'mastery'
    ]);
    for (const item of shown) {
      const pillar = item.dataset.pillar ?? '';
      const name = (item.querySelector('span')?.textContent ?? '') as keyof typeof ABOUT;
      // the sentence is the row's last element, after the figure and its bar, as ordinary text
      const about = item.lastElementChild as HTMLElement | null;
      expect(about?.tagName, `${name}: the row's last element`).toBe('P');
      expect(about?.id).toBe(`pillar-${pillar}-about`);
      expect(about?.textContent?.trim()).toBe(ABOUT[name]);
      expect(about?.previousElementSibling?.tagName, `${name}: the sentence follows the bar`).toBe('METER');
      for (const attribute of ['title', 'hidden', 'aria-hidden', 'style']) {
        expect(about?.hasAttribute(attribute), `${name}'s sentence carries ${attribute}`).toBe(false);
      }
      expect(about?.closest('details')).toBeNull();
      // the bar names the sentence as its description, beside its name
      const meter = within(item).getByRole('meter', { name, description: ABOUT[name] });
      expect(meter.getAttribute('aria-labelledby')).toBe(`pillar-${pillar}`);
      expect(meter.getAttribute('aria-describedby')).toBe(`pillar-${pillar}-about`);
    }
    expect(shown).toHaveLength(5);
  });

  it('the retention gap still says what retention measures', () => {
    render(ScoreBreakdown, { props: { score: scored(null) } });

    const retention = items().find((item) => item.dataset.pillar === 'retention');
    // the gap line, no bar, and the sentence last
    expect(retention?.querySelectorAll('span')[1]?.textContent).toBe('No review answered yet');
    expect(retention?.querySelector('meter')).toBeNull();
    const about = retention?.lastElementChild;
    expect(about?.tagName, "the retention row's last element").toBe('P');
    expect(about?.id).toBe('pillar-retention-about');
    expect(about?.textContent?.trim()).toBe(ABOUT.Retention);
    expect(about?.previousElementSibling?.querySelectorAll('span')[1]?.textContent).toBe('No review answered yet');
  });
});
