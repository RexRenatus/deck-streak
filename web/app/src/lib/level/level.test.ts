import { describe, expect, it } from 'vitest';
import { parseLevel } from './level';

// SPEC-072 R23, R26. The level screen reads `GET /api/level` by exact key: a body that is not the
// level view is refused whole, so no screen shows a half-read one.
const BODY = {
  study_day: '2001-02-03',
  level: 7,
  title: 'Adept',
  emoji: '🌿',
  total_xp: 1234,
  xp_into_level: 40,
  xp_for_next: 200,
  today: [{ source: 'streak', track: 'language', amount: 10, state: 'provisional' }],
  run: 5,
  multiplier: 1.25,
  multiplier_after_a_miss: 1.1,
  ascendant: true
};

describe('the level view reading', () => {
  it('reads the server body into the view', () => {
    expect(parseLevel(BODY)).toEqual({
      studyDay: '2001-02-03',
      level: 7,
      title: 'Adept',
      emoji: '🌿',
      totalXp: 1234,
      xpIntoLevel: 40,
      xpForNext: 200,
      today: [{ source: 'streak', track: 'language', amount: 10, state: 'provisional' }],
      run: 5,
      multiplier: 1.25,
      multiplierAfterAMiss: 1.1,
      ascendant: true
    });
  });

  it('refuses a body that is not a level view', () => {
    const bodies = [
      null,
      {},
      { ...BODY, study_day: 'today' },
      { ...BODY, run: '5' },
      { ...BODY, ascendant: 'yes' },
      { ...BODY, today: [{ source: 'streak', track: 'language', amount: 10, state: 'later' }] }
    ];
    expect(bodies.map(parseLevel)).toEqual(bodies.map(() => null));
  });
});
