/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import StreakScreen from './StreakScreen.svelte';
import type { StreakView } from './streak';

// SPEC-076 R23, A22. The streak screen shows both tracks side by side, law first when the law track
// has activity, the at-stake line as the rule with no loss wording, and a governor chip that says
// why it is disarmed.

/** A synthetic streak view: a law track with activity, and a governor in standby. */
function viewed(overrides: Partial<StreakView> = {}): StreakView {
  return {
    studyDay: '2001-02-03',
    language: { current: 12, longest: 20, heat: 3, freezes: 2, freezeCap: 3 },
    law: { current: 4, longest: 6, heat: 1 },
    atStake: { language: 'freeze', law: 'break' },
    governor: {
      verdict: 'standby',
      strength: 0.4,
      lapseSince: null,
      why: 'Your habit strength is below the arming level.',
      relightCards: null
    },
    ...overrides
  };
}

describe('StreakScreen', () => {
  it('shows law first and says why the governor is disarmed', () => {
    render(StreakScreen, { props: { view: viewed() } });

    const tracks = screen.queryAllByRole('article').map((track) => track.textContent ?? '');
    expect(tracks.length, 'the two tracks are shown').toBe(2);
    expect(tracks[0]).toContain('Law');
    expect(tracks[1]).toContain('Language');
    expect(screen.getByText('Standby: your habit strength is below the arming level.')).toBeTruthy();
    expect(screen.getByText('A freeze covers one missed day.')).toBeTruthy();
  });
});
