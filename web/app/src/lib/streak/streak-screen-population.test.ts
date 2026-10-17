/**
 * @vitest-environment jsdom
 */
import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import StreakScreen from './StreakScreen.svelte';
import type { AtStake, GovernorView, StreakTrack, StreakView } from './streak';

// SPEC-076 R23. One class rule per branch of the streak screen, each over a generated population
// whose size is asserted: the law-first order over the sign of the law run, the freezes line over
// the four presence combinations, the at-stake line over every stake value on each track, and the
// governor line over every verdict. Text and elements are asserted exactly, not only presence.
const STAKES: readonly AtStake[] = ['freeze', 'break', 'none'];
const STAKE_TEXT: Record<AtStake, string | null> = {
  freeze: 'A freeze covers one missed day.',
  break: 'Studying today keeps this streak going.',
  none: null
};

function governor(overrides: Partial<GovernorView> = {}): GovernorView {
  return { verdict: 'armed', strength: 0.9, lapseSince: null, why: null, relightCards: null, ...overrides };
}

function view(overrides: Partial<StreakView> = {}): StreakView {
  return {
    studyDay: '2001-02-03',
    language: { current: 12, longest: 20, heat: 3, freezes: 2, freezeCap: 3 },
    law: { current: 4, longest: 6, heat: 1 },
    atStake: { language: 'none', law: 'none' },
    governor: governor(),
    ...overrides
  };
}

/** The rendered article of `key`, with the paragraphs it holds. */
function article(container: HTMLElement, key: 'law' | 'language'): { el: HTMLElement; lines: string[] } {
  const el = container.querySelector<HTMLElement>(`article[data-track="${key}"]`);
  if (el === null) throw new Error(`no ${key} article`);
  return { el, lines: [...el.querySelectorAll('p')].map((p) => p.textContent ?? '') };
}

describe('the streak screen frame', () => {
  it('titles the section, states the study day for readers of a screen reader, and shows the two tracks', () => {
    const { container } = render(StreakScreen, { props: { view: view() } });
    const section = container.querySelector('section');
    expect(section?.getAttribute('aria-labelledby')).toBe('streak-title');
    const title = container.querySelector('h2#streak-title');
    expect(title?.textContent).toBe('Your streaks');
    expect(container.querySelector('p.sr-only')?.textContent).toBe('2001-02-03');
    expect(container.querySelectorAll('article').length).toBe(2);
    expect(article(container, 'language').el.querySelector('h3')?.textContent).toBe('Language');
    expect(article(container, 'law').el.querySelector('h3')?.textContent).toBe('Law');
  });

  it('writes each track its own days and best, from that track alone', () => {
    const { container } = render(StreakScreen, { props: { view: view() } });
    expect(article(container, 'language').lines.slice(0, 2)).toEqual(['12 days', 'Best 20']);
    expect(article(container, 'law').lines).toEqual(['4 days', 'Best 6']);
  });
});

describe('law first when the law run is positive', () => {
  it('orders the tracks by the sign of the law run', () => {
    const runs = [-5, -1, 0, 1, 2, 40];
    expect(runs.length, 'law runs generated').toBe(6);
    for (const current of runs) {
      const { container, unmount } = render(StreakScreen, {
        props: { view: view({ law: { current, longest: 50, heat: 0 } }) }
      });
      const order = [...container.querySelectorAll('article')].map((a) => a.getAttribute('data-track'));
      expect(order, `law run ${current}`).toEqual(current > 0 ? ['law', 'language'] : ['language', 'law']);
      unmount();
    }
  });
});

describe('the freezes line', () => {
  it('shows only when both the held count and the cap are present, with both numbers', () => {
    const combos: Array<[number | undefined, number | undefined]> = [
      [2, 3],
      [2, undefined],
      [undefined, 3],
      [undefined, undefined],
      [0, 0]
    ];
    expect(combos.length, 'freeze combinations').toBe(5);
    for (const [freezes, freezeCap] of combos) {
      const language: StreakTrack = { current: 1, longest: 2, heat: 0, freezes, freezeCap };
      const { container, unmount } = render(StreakScreen, { props: { view: view({ language }) } });
      const shown = freezes !== undefined && freezeCap !== undefined;
      const lines = article(container, 'language').lines;
      expect(lines, `${freezes}/${freezeCap}`).toEqual(
        shown ? ['1 days', 'Best 2', `${freezes} of ${freezeCap} freezes held`] : ['1 days', 'Best 2']
      );
      unmount();
    }
  });

  it('shows a freezes line on the law track when the view gives it both numbers, and none otherwise', () => {
    const law: StreakTrack = { current: 1, longest: 2, heat: 0, freezes: 1, freezeCap: 4 };
    const { container } = render(StreakScreen, { props: { view: view({ law }) } });
    expect(article(container, 'law').lines).toEqual(['1 days', 'Best 2', '1 of 4 freezes held']);
  });
});

describe('the at-stake line', () => {
  it('states the rule for each stake on each track, and nothing for none', () => {
    let cases = 0;
    for (const language of STAKES) {
      for (const law of STAKES) {
        cases += 1;
        const { container, unmount } = render(StreakScreen, {
          props: { view: view({ atStake: { language, law } }) }
        });
        const langLines = article(container, 'language').lines;
        const lawLines = article(container, 'law').lines;
        const langStake = STAKE_TEXT[language];
        const lawStake = STAKE_TEXT[law];
        expect(langLines.slice(3), `language ${language}`).toEqual(langStake === null ? [] : [langStake]);
        expect(lawLines.slice(2), `law ${law}`).toEqual(lawStake === null ? [] : [lawStake]);
        unmount();
      }
    }
    expect(cases, 'stake pairs').toBe(9);
  });
});

describe('the governor line', () => {
  it('says each verdict in its words, tagged with the verdict', () => {
    const cases: Array<[GovernorView, string]> = [
      [governor({ verdict: 'armed' }), 'Armed: your habit is strong.'],
      [
        governor({ verdict: 'standby', why: 'strength_below_arming_level' }),
        'Standby: your habit strength is below the arming level.'
      ],
      [
        governor({ verdict: 'lapse', why: 'no_study_days', relightCards: 12, lapseSince: '2001-02-01' }),
        'Lapse: 12 reviews on one day relight it.'
      ],
      [governor({ verdict: 'lapse', why: 'no_study_days', relightCards: 0 }), 'Lapse: 0 reviews on one day relight it.'],
      [governor({ verdict: 'lapse', why: 'no_study_days', relightCards: null }), 'Lapse: 0 reviews on one day relight it.']
    ];
    expect(cases.length, 'governor cases').toBe(5);
    for (const [gov, text] of cases) {
      const { container, unmount } = render(StreakScreen, { props: { view: view({ governor: gov }) } });
      const line = container.querySelector('p[data-verdict]');
      expect(line?.getAttribute('data-verdict'), text).toBe(gov.verdict);
      expect(line?.textContent, text).toBe(text);
      unmount();
    }
  });
});
