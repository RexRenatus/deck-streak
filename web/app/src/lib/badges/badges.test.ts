import { describe, expect, it } from 'vitest';
import { parseBadges } from './badges';

// SPEC-073 R16. The gallery reads `GET /api/badges` as the server answers it, and refuses a body
// that is not one rather than showing a part of it.
const BODY = {
  earned: [
    { key: 'centurion_day', tier: 0, name: 'Centurion Day', emoji: '💯', study_day: '2025-01-13' }
  ],
  locked: [
    {
      key: 'monthly_monk',
      name: 'Monthly Monk',
      emoji: '🧘',
      criteria: '30-day streak',
      family: 'study',
      progress: { value: 3, threshold: 30 }
    },
    {
      key: 'first_page',
      name: 'First Page',
      emoji: '📖',
      criteria: 'Logged your first reading session',
      family: 'habit',
      progress: null
    }
  ]
};

/** BODY with the first locked badge's fields replaced by `fields`. */
function withLocked(fields: Record<string, unknown>): unknown {
  return { ...BODY, locked: [{ ...BODY.locked[0], ...fields }, BODY.locked[1]] };
}

/** BODY with the earned badge's fields replaced by `fields`. */
function withEarned(fields: Record<string, unknown>): unknown {
  return { ...BODY, earned: [{ ...BODY.earned[0], ...fields }] };
}

describe('parseBadges', () => {
  it('reads the earned and the locked badges, a locked one with or without progress', () => {
    expect(parseBadges(BODY)).toEqual({
      earned: [
        { key: 'centurion_day', tier: 0, name: 'Centurion Day', emoji: '💯', studyDay: '2025-01-13' }
      ],
      locked: [
        {
          key: 'monthly_monk',
          name: 'Monthly Monk',
          emoji: '🧘',
          criteria: '30-day streak',
          family: 'study',
          progress: { value: 3, threshold: 30 }
        },
        {
          key: 'first_page',
          name: 'First Page',
          emoji: '📖',
          criteria: 'Logged your first reading session',
          family: 'habit',
          progress: null
        }
      ]
    });
  });

  it('reads an owner with no badges at all', () => {
    expect(parseBadges({ earned: [], locked: [] })).toEqual({ earned: [], locked: [] });
  });

  it('reads a locked focus badge', () => {
    const focus = withLocked({ key: 'deep_focus', family: 'focus', progress: null });
    expect(parseBadges(focus)?.locked[0]).toEqual({
      key: 'deep_focus',
      name: 'Monthly Monk',
      emoji: '🧘',
      criteria: '30-day streak',
      family: 'focus',
      progress: null
    });
  });

  it('refuses a body that is not the badges answer', () => {
    // The positive control: the untouched body parses, so each body below that differs from it in
    // one field is refused for that field.
    const read = parseBadges(BODY);
    expect(read?.earned.map((badge) => [badge.key, badge.studyDay])).toEqual([
      ['centurion_day', '2025-01-13']
    ]);
    expect(read?.locked.map((badge) => [badge.key, badge.family, badge.progress])).toEqual([
      ['monthly_monk', 'study', { value: 3, threshold: 30 }],
      ['first_page', 'habit', null]
    ]);
    const refused = [
      null,
      'badges',
      {},
      { earned: [] },
      { locked: [] },
      { earned: {}, locked: [] },
      withEarned({ study_day: '13 January 2025' }),
      withEarned({ tier: '0' }),
      withEarned({ name: undefined }),
      withEarned({ key: 7 }),
      withEarned({ emoji: null }),
      withEarned({ study_day: ['2025-01-13'] }),
      withEarned({ study_day: 'on 2025-01-13' }),
      withEarned({ study_day: '2025-01-13 at noon' }),
      withLocked({ key: 7 }),
      withLocked({ name: undefined }),
      withLocked({ emoji: null }),
      withLocked({ family: 'band' }),
      withLocked({ criteria: 30 }),
      withLocked({ progress: { value: 3 } }),
      withLocked({ progress: { value: '3', threshold: 30 } }),
      withLocked({ progress: undefined })
    ];
    for (const body of refused) {
      expect(parseBadges(body), JSON.stringify(body)).toBeNull();
    }
  });
});
