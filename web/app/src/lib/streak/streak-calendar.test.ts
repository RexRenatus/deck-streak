/**
 * @vitest-environment jsdom
 */
import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import StreakScreen from './StreakScreen.svelte';
import { parseStreak, type CalendarDay, type Marker, type StreakView } from './streak';

// SPEC-076 A61, the class rule on the screen: every served day carries exactly the markers that sit
// on it and no other day's. The population places each marker on a different day of each track's
// window, in several members, so a screen that draws a marker on a neighbour's cell, drops one, or
// reads one track's calendar for the other is red.
const WINDOW = 35;
const LAST_DAY = Date.UTC(2025, 0, 14);
const DAY_MS = 86_400_000;
const MARKERS: Marker[] = ['skip', 'freeze', 'break'];
const MARKER_WORD: Record<Marker, string> = { skip: 'Skip', freeze: 'Freeze', break: 'Break' };

function iso(offsetFromLast: number): string {
  return new Date(LAST_DAY - offsetFromLast * DAY_MS).toISOString().slice(0, 10);
}

/** Which markers sit on window index `index` (0 is the oldest) of member `k` and `track`. */
function markersAt(k: number, track: number, index: number): Marker[] {
  const found: Marker[] = [];
  MARKERS.forEach((marker, which) => {
    // Each marker has its own day in the window, shifted by the member and the track, so no two
    // markers share a day and no member repeats another's placement.
    const day = (7 * (which + 1) + 5 * k + 11 * track + 3 * which * which) % WINDOW;
    if (day === index) found.push(marker);
  });
  return found;
}

function days(k: number, track: number): CalendarDay[] {
  return Array.from({ length: WINDOW }, (_, index) => ({
    day: iso(WINDOW - 1 - index),
    studied: (index + k + track) % 3 !== 0,
    markers: markersAt(k, track, index)
  }));
}

function member(k: number): StreakView {
  return {
    studyDay: iso(0),
    language: { current: 5, longest: 9, heat: 2, freezes: 1, freezeCap: 3 },
    law: { current: 0, longest: 4, heat: 0 },
    atStake: { language: 'none', law: 'none' },
    calendar: { language: days(k, 0), law: days(k, 1) },
    governor: { verdict: 'armed', strength: 0.9, lapseSince: null, why: null, relightCards: null }
  };
}

function cells(container: HTMLElement, key: 'language' | 'law'): HTMLElement[] {
  const article = container.querySelector<HTMLElement>(`article[data-track="${key}"]`);
  if (article === null) throw new Error(`no ${key} article`);
  return [...article.querySelectorAll<HTMLElement>('li[data-day]')];
}

describe('the streak calendar on the screen', () => {
  it("the screen reads each marker from its own day's cell", () => {
    let examined = 0;
    const seen = new Set<Marker>();
    for (let k = 0; k < 12; k += 1) {
      const { container, unmount } = render(StreakScreen, { view: member(k) });
      for (const [key, track] of [['language', 0], ['law', 1]] as const) {
        const drawn = cells(container, key);
        expect(drawn, `member ${k} ${key}`).toHaveLength(WINDOW);
        drawn.forEach((cell, index) => {
          const want = markersAt(k, track, index);
          expect(cell.dataset.day, `member ${k} ${key} cell ${index}`).toBe(iso(WINDOW - 1 - index));
          expect(cell.dataset.markers, `member ${k} ${key} ${cell.dataset.day}`).toBe(want.join(' '));
          const words = [...cell.querySelectorAll('[data-marker]')].map((el) => el.textContent);
          expect(words).toEqual(want.map((marker) => MARKER_WORD[marker]));
          want.forEach((marker) => seen.add(marker));
          examined += 1;
        });
      }
      unmount();
    }
    console.log(`examined ${examined} calendar cell(s)`);
    expect(examined).toBe(12 * 2 * WINDOW);
    // Non-vacuity: every marker was drawn somewhere, and each pair sits on a different day.
    expect([...seen].sort()).toEqual([...MARKERS].sort());
    for (let k = 0; k < 12; k += 1) {
      for (const track of [0, 1]) {
        const placed = MARKERS.map((marker) =>
          days(k, track).findIndex((day) => day.markers.includes(marker))
        );
        expect(new Set(placed).size, `member ${k} track ${track}`).toBe(MARKERS.length);
      }
    }
  });

  it('serves the two tracks their own calendars', () => {
    const { container } = render(StreakScreen, { view: member(3) });
    const language = cells(container, 'language').map((cell) => cell.dataset.markers);
    const law = cells(container, 'law').map((cell) => cell.dataset.markers);
    expect(language).not.toEqual(law);
  });

  it('draws no calendar when the body carries none', () => {
    const view = member(0);
    delete view.calendar;
    const { container } = render(StreakScreen, { view });
    expect(container.querySelectorAll('li[data-day]')).toHaveLength(0);
  });

  it('reads the served calendar into its own place and refuses a malformed one', () => {
    const view = member(5);
    const body = {
      study_day: view.studyDay,
      language: { current: 5, longest: 9, heat: 2, freezes: 1, freeze_cap: 3 },
      law: { current: 0, longest: 4, heat: 0 },
      at_stake: { language: 'none', law: 'none' },
      calendar: view.calendar
    };
    expect(parseStreak(body)?.calendar).toStrictEqual(view.calendar);
    const bad = (calendar: unknown): unknown => parseStreak({ ...body, calendar });
    expect(bad({ language: [{ day: '2025-01-14', studied: true, markers: ['sleep'] }], law: [] })).toBeNull();
    expect(bad({ language: [{ day: 'today', studied: true, markers: [] }], law: [] })).toBeNull();
    expect(bad({ language: [], law: 'none' })).toBeNull();
  });
});
