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
// reads one track's calendar for the other is red. A64: the window is laid out as whole weeks,
// Monday first, each day in its own weekday's column.
//
// The window is the predecessor's (SPEC-076 section 27): from the Monday on or before the served
// day less 181 days through the served day, 182 to 188 days by weekday. Member k is served on
// FIRST_SERVED + k, so the twelve members serve every weekday and every window length.
const FIRST_SERVED = 20_102; // 2025-01-14, a Tuesday (epoch day numbers)
const DAY_MS = 86_400_000;
const MARKERS: Marker[] = ['skip', 'freeze', 'break'];
const MARKER_WORD: Record<Marker, string> = { skip: 'Skip', freeze: 'Freeze', break: 'Break' };
const MARKER_GLYPH: Record<Marker, string> = { skip: 'S', freeze: 'F', break: 'B' };

function iso(epochDay: number): string {
  return new Date(epochDay * DAY_MS).toISOString().slice(0, 10);
}

/** The first served day of member `k`'s window: epoch day 0, 1970-01-01, was a Thursday. */
function windowFirst(k: number): number {
  const reach = FIRST_SERVED + k - 181;
  return reach - (((reach + 3) % 7) + 7) % 7;
}

function windowLength(k: number): number {
  return FIRST_SERVED + k - windowFirst(k) + 1;
}

/** Which markers sit on window index `index` (0 is the oldest) of member `k` and `track`. */
function markersAt(k: number, track: number, index: number): Marker[] {
  const found: Marker[] = [];
  MARKERS.forEach((marker, which) => {
    // Each marker has its own day in the window, shifted by the member and the track, so no two
    // markers share a day and no member repeats another's placement; the shifts reach the window's
    // first and last weeks.
    const day = (29 * (which + 1) + 13 * k + 61 * track + 3 * which * which) % windowLength(k);
    if (day === index) found.push(marker);
  });
  return found;
}

function days(k: number, track: number): CalendarDay[] {
  return Array.from({ length: windowLength(k) }, (_, index) => ({
    day: iso(windowFirst(k) + index),
    studied: (index + k + track) % 3 !== 0,
    markers: markersAt(k, track, index)
  }));
}

function member(k: number): StreakView {
  return {
    studyDay: iso(FIRST_SERVED + k),
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
    let derived = 0;
    for (let k = 0; k < 12; k += 1) {
      const { container, unmount } = render(StreakScreen, { view: member(k) });
      for (const [key, track] of [['language', 0], ['law', 1]] as const) {
        const drawn = cells(container, key);
        expect(drawn, `member ${k} ${key}`).toHaveLength(windowLength(k));
        derived += windowLength(k);
        drawn.forEach((cell, index) => {
          const want = markersAt(k, track, index);
          expect(cell.dataset.day, `member ${k} ${key} cell ${index}`).toBe(iso(windowFirst(k) + index));
          expect(cell.dataset.markers, `member ${k} ${key} ${cell.dataset.day}`).toBe(want.join(' '));
          const words = [...cell.querySelectorAll('[data-marker]')].map((el) => el.textContent);
          expect(words).toEqual(want.map((marker) => MARKER_WORD[marker]));
          const glyphs = [...cell.querySelectorAll('[data-glyph]')].map((el) => el.textContent);
          expect(glyphs).toEqual(want.map((marker) => MARKER_GLYPH[marker]));
          want.forEach((marker) => seen.add(marker));
          examined += 1;
        });
      }
      unmount();
    }
    console.log(`examined ${examined} calendar cell(s)`);
    expect(examined).toBe(derived);
    // The twelve members serve every window length, 182 to 188 days.
    const lengths = new Set(Array.from({ length: 12 }, (_, k) => windowLength(k)));
    expect([...lengths].sort()).toEqual([182, 183, 184, 185, 186, 187, 188]);
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
    expect(language).toHaveLength(windowLength(3));
    expect(law).toHaveLength(windowLength(3));
    expect(language).not.toEqual(law);
  });

  it('draws no calendar when the body carries none', () => {
    const view = member(0);
    delete view.calendar;
    const { container } = render(StreakScreen, { view });
    expect(container.querySelectorAll('li[data-day]')).toHaveLength(0);
    const drawn = render(StreakScreen, { view: member(0) });
    expect(drawn.container.querySelectorAll('li[data-day]')).toHaveLength(2 * windowLength(0));
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
    expect(bad({ language: [], law: 5 })).toBeNull();
    expect(bad({ language: [], law: {} })).toBeNull();
    expect(bad({ language: [{ day: ['2025-01-14'], studied: true, markers: [] }], law: [] })).toBeNull();
    expect(bad({ language: [{ day: '2025-01-14', studied: 'yes', markers: [] }], law: [] })).toBeNull();
    expect(bad({ language: [{ day: '2025-01-14', studied: true, markers: [] }], law: [] })).not.toBeNull();
  });

  it('draws a cell per day with its day number, every marker joined, and a named list', () => {
    const view = member(0);
    view.calendar = {
      language: [
        { day: '2025-01-13', studied: true, markers: ['skip', 'break'] },
        { day: '2025-01-14', studied: false, markers: [] }
      ],
      law: []
    };
    const { container } = render(StreakScreen, { view });
    const drawn = cells(container, 'language');
    expect(drawn.map((cell) => cell.dataset.markers)).toStrictEqual(['skip break', '']);
    expect(drawn.map((cell) => cell.querySelector('span')?.textContent)).toStrictEqual(['13', '14']);
    expect(container.querySelectorAll('ol[aria-label="The last 26 weeks"]')).toHaveLength(1);
    expect(cells(container, 'law')).toHaveLength(0);
    expect(container.querySelectorAll('ol')).toHaveLength(1);
  });

  it('lays the served window out as whole weeks, Monday first', () => {
    let examined = 0;
    let derived = 0;
    for (let k = 0; k < 7; k += 1) {
      const { container, unmount } = render(StreakScreen, { view: member(k) });
      for (const key of ['language', 'law'] as const) {
        const drawn = cells(container, key);
        derived += windowLength(k);
        // Every served day has one cell, and no day has two.
        expect(new Set(drawn.map((cell) => cell.dataset.day)).size, `member ${k} ${key}`).toBe(
          windowLength(k)
        );
        drawn.forEach((cell, index) => {
          // The window opens on a Monday and runs day by day, so cell `index` falls on weekday
          // index % 7 (Monday 0), and sits in that weekday's column and in no other.
          const columns = [...cell.classList].filter((name) => name.startsWith('col-start-'));
          expect(columns, `member ${k} ${key} ${cell.dataset.day}`).toEqual([
            `col-start-${(index % 7) + 1}`
          ]);
          examined += 1;
        });
        // The first week is whole: its first cell is a Monday's.
        expect(drawn[0]?.classList.contains('col-start-1')).toBe(true);
      }
      unmount();
    }
    console.log(`examined ${examined} calendar cell(s) for their weekday column`);
    expect(examined).toBe(derived);
    // 2025-01-13 is a Monday and 2025-01-19 a Sunday, read from the calendar, not from the code.
    const view = member(0);
    view.calendar = {
      language: [
        { day: '2025-01-13', studied: true, markers: [] },
        { day: '2025-01-19', studied: false, markers: [] }
      ],
      law: []
    };
    const { container } = render(StreakScreen, { view });
    const columns = cells(container, 'language').map((cell) =>
      [...cell.classList].filter((name) => name.startsWith('col-start-'))
    );
    expect(columns).toStrictEqual([['col-start-1'], ['col-start-7']]);
  });
});
