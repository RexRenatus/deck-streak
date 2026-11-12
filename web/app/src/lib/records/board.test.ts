import { describe, expect, it } from 'vitest';
import { parseBoard } from './board';

// SPEC-075 R3, #79. The records screen reads `GET /api/board` as the server answers it: each row's
// kind, emoji, label and value, and its study day, longest run or title where the server sends one.
// A body that is not the board is refused whole rather than shown in part.
const BEST_DAY = { kind: 'best_day', emoji: '🏅', label: 'Best day', value: 90, study_day: '2025-01-11' };
const STREAK = { kind: 'streak', emoji: '🔥', label: 'Streak', value: 3, longest: 9 };
const LEVEL = { kind: 'level', emoji: '⚡', label: 'Level', value: 9, title: 'Seedling' };
const BODY = { rows: [BEST_DAY, STREAK, LEVEL] };

describe('parseBoard', () => {
  it('reads each row, with a null for each field the server left out', () => {
    expect(parseBoard(BODY)).toEqual({
      rows: [
        {
          kind: 'best_day',
          emoji: '🏅',
          label: 'Best day',
          value: 90,
          studyDay: '2025-01-11',
          longest: null,
          title: null
        },
        { kind: 'streak', emoji: '🔥', label: 'Streak', value: 3, studyDay: null, longest: 9, title: null },
        {
          kind: 'level',
          emoji: '⚡',
          label: 'Level',
          value: 9,
          studyDay: null,
          longest: null,
          title: 'Seedling'
        }
      ]
    });
  });

  it('reads a board with no rows', () => {
    expect(parseBoard({ rows: [] })).toEqual({ rows: [] });
  });

  it('refuses a body that is not the board answer', () => {
    // The positive control: the untouched rows parse, so each body below that differs from them in
    // one field is refused for that field.
    expect(parseBoard(BODY)?.rows.map((row) => row.kind)).toEqual(['best_day', 'streak', 'level']);
    const refused = [
      null,
      'rows',
      {},
      { rows: {} },
      { rows: [null] },
      [BEST_DAY],
      { rows: [BEST_DAY, 'streak'] }
    ];
    expect(refused.map(parseBoard), JSON.stringify(refused)).toEqual(refused.map(() => null));
  });

  it('refuses each field that is missing or of the wrong type, one row at a time', () => {
    const wrong: [Record<string, unknown>, string, unknown][] = [
      [BEST_DAY, 'kind', 7],
      [BEST_DAY, 'emoji', 7],
      [BEST_DAY, 'label', 7],
      [BEST_DAY, 'value', '90'],
      [BEST_DAY, 'study_day', 20250111],
      [BEST_DAY, 'study_day', null],
      [STREAK, 'longest', '9'],
      [STREAK, 'longest', null],
      [LEVEL, 'title', 7],
      [LEVEL, 'title', null]
    ];
    const bodies: unknown[] = [];
    for (const [row, key, value] of wrong) {
      bodies.push({ rows: [{ ...row, [key]: value }] });
    }
    for (const key of ['kind', 'emoji', 'label', 'value']) {
      const { [key]: _dropped, ...without } = BEST_DAY as Record<string, unknown>;
      bodies.push({ rows: [without] });
    }
    expect(bodies.map(parseBoard), JSON.stringify(bodies)).toEqual(bodies.map(() => null));
  });

  it('refuses a study day that is anything but four, two and two digits', () => {
    const days = [
      '2025-01-1',
      '2025-1-11',
      'x2025-01-11',
      '2025-01-11x',
      '2025-01-11\n',
      '25-01-11',
      '11 January 2025'
    ];
    expect(days.map((study_day) => parseBoard({ rows: [{ ...BEST_DAY, study_day }] }))).toEqual(
      days.map(() => null)
    );
    // An array of the one date prints as that date, so only the type check refuses it.
    expect(parseBoard({ rows: [{ ...BEST_DAY, study_day: ['2025-01-11'] }] })).toBeNull();
  });
});
