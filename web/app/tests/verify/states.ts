// The states a learner sees on each screen, as the phone-size audit and snapshots open them
// (SPEC-639 R2). Each route has `first` (the screen with its data), `empty` (no data) and `error`
// (its calls answered with a server error), and the review screen has its `question` and `answer`.
//
// A state is the answer to each call the screen makes: the screen's own reads in its page source,
// and the layout's, the wallet header and the celebration feed, on every screen. A screen that
// opens the study engine is given the engine's stand-in with one card, with no card, or a server
// error for the engine's script. Every value here is synthetic: no real learner's data enters a
// fixture or a baseline, and tests/verify holds no email-shaped value and no account-sized number.
import type { RoutePath } from '../../src/lib/routes';

/** One call's answer: a JSON body, or a status with no body. */
export type Reply = { readonly json: unknown } | { readonly status: number };

/** What the study engine's stand-in holds: one card, no card, or a script that does not load. */
export type Engine = 'card' | 'none' | 'down';

/** One state of one screen. */
export interface State {
  /** Each call the screen makes, by its URL glob, and its answer. */
  readonly calls: Readonly<Record<string, Reply>>;
  /** The engine's stand-in, for a screen that opens the engine. */
  readonly engine?: Engine;
  /** Whether the review shows its answer side, after its show-answer control. */
  readonly reveal?: boolean;
}

/** Every route of the route table, each with its states by name. */
export type StateTable = Readonly<Record<RoutePath, Readonly<Record<string, State>>>>;

const DAY = '2001-02-03';
const SERVER_ERROR: Reply = { status: 500 };

const WALLET = {
  study_day: DAY,
  balance: 103,
  loss_cap: 30,
  loss_cap_left: 25,
  movements: [
    { id: 4, study_day: DAY, source: 'fine', amount: -5 },
    { id: 3, study_day: DAY, source: 'mint', amount: 8 }
  ],
  next: 3
};
const NO_WALLET = { study_day: DAY, balance: 0, loss_cap: 30, loss_cap_left: 30, movements: [], next: null };

// The layout's calls on every screen: the wallet header and the celebration feed. The feed is
// empty in every state, so no celebration moves on a settled screen.
const LAYOUT_FIRST = { '**/api/wallet': { json: WALLET }, '**/api/notifications/feed*': { json: { items: [] } } };
const LAYOUT_EMPTY = { '**/api/wallet': { json: NO_WALLET }, '**/api/notifications/feed*': { json: { items: [] } } };
const LAYOUT_ERROR = { '**/api/wallet': SERVER_ERROR, '**/api/notifications/feed*': { json: { items: [] } } };

const SCORE = {
  study_day: DAY,
  score: {
    total: 64,
    grade: { label: 'SOLID', emoji: '✅' },
    pillars: { consistency: 76, retention: null, workload: 70, volume: 60.5, mastery: 55 },
    reviews: 0,
    retention: null
  }
};

const LEVEL = {
  study_day: DAY,
  level: 7,
  title: 'Adept',
  emoji: '✨',
  total_xp: 1234,
  xp_into_level: 40,
  xp_for_next: 200,
  today: [
    { source: 'reviews', track: 'language', amount: 24, state: 'settled' },
    { source: 'streak', track: 'language', amount: 10, state: 'provisional' }
  ],
  run: 5,
  multiplier: 1.25,
  multiplier_after_a_miss: 1.1,
  ascendant: true
};
const NO_LEVEL = {
  study_day: DAY,
  level: 1,
  title: 'Novice',
  emoji: '✨',
  total_xp: 0,
  xp_into_level: 0,
  xp_for_next: 100,
  today: [],
  run: 0,
  multiplier: 1,
  multiplier_after_a_miss: 1,
  ascendant: false
};

const EXCHANGE = {
  window: null,
  rates: [
    { source: 'focus', total_xp: 50, graduated_cards: 0, rate: null, rate_defined: false },
    { source: 'reviews', total_xp: 10, graduated_cards: 4, rate: 2.5, rate_defined: true }
  ]
};

const STREAK = {
  study_day: DAY,
  language: { current: 12, longest: 40, heat: 2, freezes: 1, freeze_cap: 3 },
  law: { current: 3, longest: 9, heat: 0 },
  at_stake: { language: 'freeze', law: 'break' }
};
const NO_STREAK = {
  study_day: DAY,
  language: { current: 0, longest: 0, heat: 0, freezes: 0, freeze_cap: 3 },
  law: { current: 0, longest: 0, heat: 0 },
  at_stake: { language: 'break', law: 'break' }
};
const GOVERNOR = { verdict: 'armed', strength: 0.9, lapse_since: null, relight_cards: null };
const NO_GOVERNOR = { verdict: 'lapse', strength: 0, lapse_since: DAY, relight_cards: 0 };

const BADGES = {
  earned: [{ key: 'centurion_day', tier: 0, name: 'Centurion Day', emoji: '⭐', study_day: '2001-02-02' }],
  locked: [
    {
      key: 'monthly_monk',
      name: 'Monthly Monk',
      emoji: '⭐',
      criteria: '30-day streak',
      family: 'study',
      progress: { value: 3, threshold: 30 }
    },
    {
      key: 'first_page',
      name: 'First Page',
      emoji: '⭐',
      criteria: 'Logged your first reading session',
      family: 'habit',
      progress: null
    }
  ]
};

const BOARD = {
  rows: [
    { kind: 'best_day', emoji: '⭐', label: 'Best day', value: 90, study_day: '2001-02-01' },
    { kind: 'today', emoji: '⭐', label: 'Today', value: 77, study_day: DAY },
    { kind: 'streak', emoji: '⭐', label: 'Streak', value: 3, longest: 9 },
    { kind: 'level', emoji: '⚡', label: 'Level', value: 7, title: 'Adept' }
  ]
};

const RECORDS = {
  records: [
    {
      kind: 'best_score',
      label: 'Best daily score',
      value: 120,
      study_day: '2001-02-01',
      previous: 100,
      today: 77,
      distance: 43
    },
    {
      kind: 'most_minutes',
      label: 'Most minutes in a day',
      value: 8,
      study_day: '2001-01-30',
      previous: 5,
      today: 10,
      distance: 0
    }
  ],
  chase: { kind: 'best_score', label: 'Best daily score', gap: 43 }
};

const PROGRESS = {
  courses: [
    {
      code: 'fr',
      name: 'French',
      flag: '⭐',
      mastery_pct: 41.5,
      current_band: 'B1',
      current_unit: 7,
      bands: ['A1', 'A2', 'B1', 'B2', 'C1', 'C2'].map((band, i) => ({
        band,
        total: 100,
        mature: i < 2 ? 100 : 30,
        pct: i < 2 ? 92 : 30,
        achieved: i < 2
      }))
    }
  ]
};

const LAW = {
  shown: true,
  level_shown: true,
  lines: ['total_xp', 'streak', 'xp_today'],
  streak: 4,
  xp_today: 30,
  total_xp: 900,
  level: 3,
  dues: null,
  dues_pending: true,
  leeches: 2,
  leeches_pending: false,
  mastery: 61,
  mastery_pending: false
};
const NO_LAW = { ...LAW, lines: [], streak: 0, xp_today: 0, total_xp: 0, level: 0, leeches: 0, mastery: 0 };
const LAW_TIERS = {
  cards: { T1: 5, T2: 4, T3: 3, T4: 2, none: 1 },
  xp_today: { T1: 10, T2: 8, T3: 6, T4: 4, none: 2 }
};
const NO_LAW_TIERS = {
  cards: { T1: 0, T2: 0, T3: 0, T4: 0, none: 0 },
  xp_today: { T1: 0, T2: 0, T3: 0, T4: 0, none: 0 }
};

const INSIGHTS = { instruments: [{ id: 'weekly-review', cadence: 'weekly', study_day: 11355 }] };
const IDENTITIES = [
  { id: 1, kind: 'telegram' },
  { id: 2, kind: 'passkey', created_at: 86400 }
];

export const STATES: StateTable = {
  '/': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/me': { json: { study_day: DAY } } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/me': { json: { study_day: null } } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/me': SERVER_ERROR } }
  },
  '/about': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/insights': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/insights': { json: INSIGHTS }, '**/api/insights/*': { json: null } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/insights': { json: { instruments: [] } } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/insights': SERVER_ERROR, '**/api/insights/*': SERVER_ERROR } }
  },
  '/score': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/score': { json: SCORE } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/score': { json: { study_day: DAY, score: null } } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/score': SERVER_ERROR } }
  },
  '/level': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/level': { json: LEVEL }, '**/api/xp/exchange*': { json: EXCHANGE } } },
    empty: {
      calls: { ...LAYOUT_EMPTY, '**/api/level': { json: NO_LEVEL }, '**/api/xp/exchange*': { json: { window: null, rates: [] } } }
    },
    error: { calls: { ...LAYOUT_ERROR, '**/api/level': SERVER_ERROR, '**/api/xp/exchange*': SERVER_ERROR } }
  },
  '/streak': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/streak': { json: STREAK }, '**/api/governor': { json: GOVERNOR } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/streak': { json: NO_STREAK }, '**/api/governor': { json: NO_GOVERNOR } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/streak': SERVER_ERROR, '**/api/governor': SERVER_ERROR } }
  },
  '/wallet': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/badges': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/badges': { json: BADGES } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/badges': { json: { earned: [], locked: [] } } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/badges': SERVER_ERROR } }
  },
  '/records': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/board': { json: BOARD }, '**/api/records': { json: RECORDS } } },
    empty: {
      calls: { ...LAYOUT_EMPTY, '**/api/board': { json: { rows: [] } }, '**/api/records': { json: { records: [], chase: null } } }
    },
    error: { calls: { ...LAYOUT_ERROR, '**/api/board': SERVER_ERROR, '**/api/records': SERVER_ERROR } }
  },
  '/progress': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/progress': { json: PROGRESS } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/progress': { json: { courses: [] } } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/progress': SERVER_ERROR } }
  },
  '/law': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/law': { json: LAW }, '**/api/level/law-tiers': { json: LAW_TIERS } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/law': { json: NO_LAW }, '**/api/level/law-tiers': { json: NO_LAW_TIERS } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/law': SERVER_ERROR, '**/api/level/law-tiers': SERVER_ERROR } }
  },
  '/capture': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/remote': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/study': {
    first: { calls: LAYOUT_FIRST, engine: 'card' },
    empty: { calls: LAYOUT_EMPTY, engine: 'none' },
    error: { calls: LAYOUT_ERROR, engine: 'down' }
  },
  '/study/review': {
    first: { calls: LAYOUT_FIRST, engine: 'card' },
    empty: { calls: LAYOUT_EMPTY, engine: 'none' },
    error: { calls: LAYOUT_ERROR, engine: 'down' },
    question: { calls: LAYOUT_FIRST, engine: 'card' },
    answer: { calls: LAYOUT_FIRST, engine: 'card', reveal: true }
  },
  '/study/mapping': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/study/ai-decks': {
    first: { calls: LAYOUT_FIRST, engine: 'card' },
    empty: { calls: LAYOUT_EMPTY, engine: 'none' },
    error: { calls: LAYOUT_ERROR, engine: 'down' }
  },
  '/sync': {
    first: { calls: LAYOUT_FIRST, engine: 'card' },
    empty: { calls: LAYOUT_EMPTY, engine: 'none' },
    error: { calls: LAYOUT_ERROR, engine: 'down' }
  },
  '/data': {
    first: { calls: LAYOUT_FIRST, engine: 'card' },
    empty: { calls: LAYOUT_EMPTY, engine: 'none' },
    error: { calls: LAYOUT_ERROR, engine: 'down' }
  },
  '/settings': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/link': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/signin': {
    first: { calls: LAYOUT_FIRST },
    empty: { calls: LAYOUT_EMPTY },
    error: { calls: LAYOUT_ERROR }
  },
  '/sign-in-methods': {
    first: { calls: { ...LAYOUT_FIRST, '**/api/identities': { json: IDENTITIES } } },
    empty: { calls: { ...LAYOUT_EMPTY, '**/api/identities': { json: [] } } },
    error: { calls: { ...LAYOUT_ERROR, '**/api/identities': SERVER_ERROR } }
  }
};

/** A route's name in a snapshot's file name: `today` for `/`, else its path with `-` for `/`. */
export function slug(route: string): string {
  return route === '/' ? 'today' : route.replace(/^\//, '').split('/').join('-');
}
