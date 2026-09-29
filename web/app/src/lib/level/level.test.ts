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
  it('refuses each field that is missing or of the wrong type, one at a time', () => {
    const wrong: Record<string, unknown> = {
      study_day: 20010203,
      level: '7',
      title: 7,
      emoji: 7,
      total_xp: '1234',
      xp_into_level: '40',
      xp_for_next: '200',
      today: 'none',
      run: '5',
      multiplier: '1.25',
      multiplier_after_a_miss: '1.1',
      ascendant: 1
    };
    const bodies: unknown[] = [];
    for (const key of Object.keys(wrong)) {
      bodies.push({ ...BODY, [key]: wrong[key] });
      const { [key]: _dropped, ...without } = BODY as Record<string, unknown>;
      bodies.push(without);
    }
    expect(bodies.map(parseLevel)).toEqual(bodies.map(() => null));
  });

  it('refuses a study day that is anything but four, two and two digits', () => {
    const days = ['2001-02-0', '2001-2-03', 'x2001-02-03', '2001-02-03x', '2001-02-03\n', '01-02-03'];
    expect(days.map((study_day) => parseLevel({ ...BODY, study_day }))).toEqual(days.map(() => null));
    // An array of the one date prints as that date, so only the type check refuses it.
    expect(parseLevel({ ...BODY, study_day: ['2001-02-03'] })).toBeNull();
  });

  it('refuses a today row with a missing or wrong field, and reads a settled one', () => {
    const row = { source: 'streak', track: 'language', amount: 10, state: 'settled' };
    expect(parseLevel({ ...BODY, today: [row] })?.today).toEqual([row]);
    const rows = [
      { ...row, source: 3 },
      { ...row, track: 3 },
      { ...row, amount: '10' },
      { ...row, state: 3 },
      { track: 'language', amount: 10, state: 'settled' },
      { source: 'streak', amount: 10, state: 'settled' },
      { source: 'streak', track: 'language', state: 'settled' },
      { source: 'streak', track: 'language', amount: 10 },
      null
    ];
    expect(rows.map((one) => parseLevel({ ...BODY, today: [one] }))).toEqual(rows.map(() => null));
    expect(parseLevel({ ...BODY, today: [row, { ...row, state: 'later' }] })).toBeNull();
  });
});
