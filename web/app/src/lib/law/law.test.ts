import { describe, expect, it } from 'vitest';
import { parseLaw, parseLawTiers } from './law';

// SPEC-077 R10, R12, R15; SPEC-072 R24. The law tab reads `GET /api/law` and
// `GET /api/level/law-tiers` as the routes answer them, and refuses, whole, a body that is not that
// answer: a pending count is null beside its pending flag, never a figure, and a shown line always
// has its figure. Mutation coverage of the readers, not an acceptance criterion.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every line shown, every count stored, at the edges of every bound. */
const LAW = {
  shown: true,
  level_shown: false,
  lines: ['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches'],
  streak: 0,
  xp_today: 0,
  total_xp: 0,
  level: 1,
  dues: 0,
  dues_pending: false,
  leeches: 3,
  leeches_pending: false,
  mastery: 100,
  mastery_pending: false
};

/** The route's answer over an empty database: nothing shown, every count pending. */
const EMPTY = {
  shown: false,
  level_shown: false,
  lines: [],
  streak: 0,
  xp_today: 0,
  total_xp: 0,
  level: 1,
  dues: null,
  dues_pending: true,
  leeches: null,
  leeches_pending: true,
  mastery: null,
  mastery_pending: true
};

const TIERS = {
  cards: { T1: 0, T2: 1, T3: 2, T4: 3, none: 4 },
  xp_today: { T1: 5, T2: 6, T3: 7, T4: 8, none: 0 }
};

describe('the law reader', () => {
  it('reads the block with every count stored', () => {
    expect(parseLaw(LAW)).toEqual({
      shown: true,
      levelShown: false,
      lines: ['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches'],
      streak: 0,
      xpToday: 0,
      totalXp: 0,
      level: 1,
      dues: 0,
      leeches: 3,
      mastery: 100
    });
    expect(parseLaw({ ...LAW, level_shown: true, mastery: 0 })).toMatchObject({ levelShown: true, mastery: 0 });
  });

  it('reads each pending count as null', () => {
    expect(parseLaw(EMPTY)).toEqual({
      shown: false,
      levelShown: false,
      lines: [],
      streak: 0,
      xpToday: 0,
      totalXp: 0,
      level: 1,
      dues: null,
      leeches: null,
      mastery: null
    });
  });

  it('refuses a body that is not the law answer, whole', () => {
    // positive controls: the bodies every case below spoils one part of
    expect(parseLaw(LAW)?.lines).toEqual(['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches']);
    expect(parseLaw(EMPTY)?.level).toBe(1);

    const fields: [string, unknown][] = [
      ['shown', 'true'],
      ['level_shown', null],
      ['streak', -1],
      ['xp_today', 1.5],
      ['total_xp', '0'],
      ['level', null],
      ['lines', null],
      ['lines', 'total_xp'],
      ['lines', ['total_xp', 'reviews']],
      ['lines', ['reviews', 'total_xp']],
      ['dues', -1],
      ['dues', null],
      ['dues_pending', 'false'],
      ['dues_pending', true],
      ['leeches', 2.5],
      ['leeches', null],
      ['leeches_pending', true],
      ['mastery', 100.5],
      ['mastery', -0.5],
      ['mastery', '50'],
      ['mastery', null],
      ['mastery_pending', true]
    ];
    const pending: [string, unknown][] = [
      ['dues', 0],
      ['dues_pending', false],
      ['dues_pending', undefined],
      ['leeches', 0],
      ['mastery', 0],
      ['lines', ['total_xp', 'dues']],
      ['lines', ['leeches']],
      ['lines', ['mastery', 'streak']]
    ];
    const spoiled = [
      null,
      undefined,
      'law',
      [],
      {},
      ...fields.map(([key, value]) => ({ ...LAW, [key]: value })),
      ...pending.map(([key, value]) => ({ ...EMPTY, shown: true, [key]: value }))
    ];
    for (const body of examined('spoiled bodies', spoiled)) {
      expect(parseLaw(body), JSON.stringify(body) ?? String(body)).toBeNull();
    }
  });
});

describe('the law tiers reader', () => {
  it('reads the law cards and today\'s law XP by tier', () => {
    expect(parseLawTiers(TIERS)).toEqual({
      cards: { T1: 0, T2: 1, T3: 2, T4: 3, none: 4 },
      xpToday: { T1: 5, T2: 6, T3: 7, T4: 8, none: 0 }
    });
  });

  it('refuses a body that is not the tiers answer, whole', () => {
    // a positive control: the body every case below spoils one table of
    expect(parseLawTiers(TIERS)?.cards).toEqual({ T1: 0, T2: 1, T3: 2, T4: 3, none: 4 });

    const tables: unknown[] = [
      null,
      undefined,
      'T1',
      [0, 1, 2, 3, 4],
      { T1: 0, T2: 1, T3: 2, T4: 3 },
      { T1: 0, T2: 1, T3: 2, T4: 3, none: 4, T5: 5 },
      { T1: 0, T2: 1, T3: 2, T4: 3, T5: 4 },
      { T1: -1, T2: 1, T3: 2, T4: 3, none: 4 },
      { T1: 0, T2: 1, T3: 2, T4: 3, none: 0.5 },
      { T1: 0, T2: 1, T3: '2', T4: 3, none: 4 }
    ];
    const spoiled = [
      null,
      undefined,
      {},
      ...tables.map((table) => ({ ...TIERS, cards: table })),
      ...tables.map((table) => ({ ...TIERS, xp_today: table }))
    ];
    for (const body of examined('spoiled bodies', spoiled)) {
      expect(parseLawTiers(body), JSON.stringify(body) ?? String(body)).toBeNull();
    }
  });
});
