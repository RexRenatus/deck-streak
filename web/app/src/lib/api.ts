import { parseBadges, type BadgesView } from './badges/badges';
import {
  CAPTURE_PATH,
  captureBody,
  parseSaved,
  type CaptureRequest,
  type Saved
} from './capture/capture';
import { FEED_PATH, parseFeed, type FeedItem } from './ladder/feed';
import {
  SENSITIVE_DECKS_PATH,
  parseMarked,
  sensitiveBody,
  sensitiveDeckPath
} from './study/ai-decks';
import {
  parseEnvelope,
  parseListings,
  type Envelope,
  type Listing
} from './insights/insights';
import { parseLaw, parseLawTiers, type LawTiersView, type LawView } from './law/law';
import { parseLevel, type LevelView } from './level/level';
import { parseProgress, type ProgressView } from './progress/progress';
import { parseRecords, type RecordsView } from './records/records';
import { parseGovernor, parseStreak, type StreakView } from './streak/streak';
import { parseScore, type ScoreToday } from './score/score';
import { parseWallet, walletPath, type WalletView } from './economy/wallet';
import { parseBoard, type BoardView } from './records/board';
import { parseExchange, type ExchangeView } from './level/exchange';
import { telegram } from './telegram.svelte';

/**
 * The Mini App's one API client (SPEC-028 R7; ADR-006, ADR-007).
 *
 * The API shares the page's origin behind Caddy, so there is no CORS and no token for the page to
 * hold. The client opens a session once, by posting the raw, signed launch data to
 * `POST /api/session`, and every later call carries the session cookie alone. The server validates
 * the launch data; the client trusts nothing it could parse out of it. When a call answers 401 the
 * client opens a new session once; if the server refuses that too, the launch data has aged past
 * its bound, the client stops calling, and the screen asks the owner to reopen DeckStreak from
 * Telegram, which hands the page fresh launch data.
 *
 * The launch data goes into that one request body and nowhere else: never a URL, a header, the
 * device's storage or a log line.
 */

/** What a screen gets from a call: the value, or why there is none. */
export type Answer<T> =
  | { readonly kind: 'ok'; readonly value: T }
  | { readonly kind: 'reopen' }
  | { readonly kind: 'unavailable' };

/** The owner's session as `GET /api/me` answers it. */
export interface Me {
  /** The server's study day, as an ISO date: it turns over at 04:00 on the server's calendar. */
  readonly studyDay: string;
}

/** What the client needs from its caller. */
export interface ApiOptions {
  /** The raw, signed launch data to open a session with, or null outside Telegram. */
  readonly launchData: () => string | null;
  /** The transport; the page's own `fetch` by default. */
  readonly fetch?: typeof globalThis.fetch;
}

/** The Mini App's one API client. */
export interface Api {
  /** The owner's session, with the server's study day. */
  me(): Promise<Answer<Me>>;
  /** The current study day's score (SPEC-071 R20). */
  score(): Promise<Answer<ScoreToday>>;
  /** The owner's level, today's XP and the consistency run (SPEC-072 R23). */
  level(): Promise<Answer<LevelView>>;
  /** The owner's board: best day, today, the language streak and the level (SPEC-075 R3). */
  board(): Promise<Answer<BoardView>>;
  /** The XP exchange readout over every day: each bucket's XP per graduation (SPEC-075 R9). */
  exchange(): Promise<Answer<ExchangeView>>;
  /** Both streak tracks and the governor's verdict (SPEC-076 R20, R21). */
  streak(): Promise<Answer<StreakView>>;
  /** The owner's earned badges, newest first, and the locked ones with progress (SPEC-073 R16). */
  badges(): Promise<Answer<BadgesView>>;
  /** The owner's personal records, today's distance to each, and the chase (SPEC-073 R17). */
  records(): Promise<Answer<RecordsView>>;
  /** Each configured course's Road to C2: its mastery, band, unit and six band cells (SPEC-077 R15). */
  progress(): Promise<Answer<ProgressView>>;
  /** The law block: its shown lines, in the server's order, and its pending counts (SPEC-077 R15). */
  law(): Promise<Answer<LawView>>;
  /** The law cards and today's law XP by tier (SPEC-072 R24). */
  lawTiers(): Promise<Answer<LawTiersView>>;
  /** The owner's unseen in-app celebrations, each with its tier (SPEC-084 R10). */
  feed(): Promise<Answer<FeedItem[]>>;
  /** The instruments the owner can read (SPEC-094 R18). */
  insights(): Promise<Answer<Listing[]>>;
  /** One instrument's latest report; null when it has not run yet. */
  insight(id: string): Promise<Answer<Envelope | null>>;
  /**
   * The wallet and one page of its movements, newest first: the first page, or the page after the
   * movement `before` (SPEC-082 R15).
   */
  wallet(before?: number): Promise<Answer<WalletView>>;
  /** Saves a quick capture into the vault's inbox, once per capture id (SPEC-118 R10). */
  capture(request: CaptureRequest): Promise<Answer<Saved>>;
  /** The decks the learner keeps away from AI, by id as a decimal string (SPEC-381 R7). */
  sensitiveDecks(): Promise<Answer<string[]>>;
  /** Keeps the deck `id` away from AI, or lets AI read it again, and answers the marked decks (SPEC-381 R7). */
  setSensitive(id: string, sensitive: boolean): Promise<Answer<string[]>>;
}

/** How opening a session ended: a session, a refusal only reopening the app can answer, or no answer. */
type Opened = 'open' | 'refused' | 'failed';

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** A client over `options`: one session, opened on the first call and renewed at most once per expiry. */
export function createApi(options: ApiOptions): Api {
  const send: typeof globalThis.fetch =
    options.fetch ?? ((input, init) => globalThis.fetch(input, init));
  let session: Promise<Opened> | null = null;
  let stopped = false;

  async function handshake(): Promise<Opened> {
    const launch = options.launchData();
    if (!launch) return 'refused';
    try {
      const response = await send('/api/session', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ init_data: launch })
      });
      if (response.ok) return 'open';
      return response.status === 401 || response.status === 403 ? 'refused' : 'failed';
    } catch {
      return 'failed';
    }
  }

  /** The session being opened or already open; concurrent calls share one handshake. */
  function opening(): Promise<Opened> {
    session ??= handshake();
    return session;
  }

  /**
   * A same-origin request that carries the session cookie alone, a GET unless `init` says
   * otherwise: its response, `'reopen'` when only reopening the app can help, or null when no
   * answer came. A renewal repeats the request as it was, body and all.
   */
  async function call(path: string, init: RequestInit = {}): Promise<Response | 'reopen' | null> {
    let renewed = false;
    for (;;) {
      if (stopped) return 'reopen';
      const used = opening();
      const opened = await used;
      if (opened === 'refused') {
        // the launch data is refused: the client stops calling until the owner reopens the app
        stopped = true;
        return 'reopen';
      }
      if (opened === 'failed') {
        // no session came of it: forget the attempt, so the next call tries again
        if (session === used) session = null;
        return null;
      }
      let response: Response;
      try {
        response = await send(path, { ...init, credentials: 'same-origin' });
      } catch {
        return null;
      }
      if (response.status !== 401) return response;
      if (renewed) {
        // a fresh session was refused at once: calling again would only loop
        stopped = true;
        return 'reopen';
      }
      // the session has ended: open a new one, once, and repeat the call
      renewed = true;
      if (session === used) session = null;
    }
  }

  /** A GET of `path` whose JSON body `parse` reads: its value, or why there is none. */
  async function read<T>(path: string, parse: (body: unknown) => T | null): Promise<Answer<T>> {
    const response = await call(path);
    if (response === 'reopen') return { kind: 'reopen' };
    if (response === null || !response.ok) {
      return { kind: 'unavailable' };
    }
    const value = parse(await response.json().catch(() => null));
    return value === null ? { kind: 'unavailable' } : { kind: 'ok', value };
  }

  return {
    me: () => read('/api/me', parseMe),
    score: () => read('/api/score', parseScore),
    level: () => read('/api/level', parseLevel),
    board: () => read('/api/board', parseBoard),
    exchange: () => read('/api/xp/exchange', parseExchange),
    streak: async () => {
      const streak = await read('/api/streak', parseStreak);
      if (streak.kind !== 'ok') return streak;
      const governor = await read('/api/governor', parseGovernor);
      if (governor.kind !== 'ok') return governor;
      return { kind: 'ok', value: { ...streak.value, governor: governor.value } };
    },
    badges: () => read('/api/badges', parseBadges),
    records: () => read('/api/records', parseRecords),
    progress: () => read('/api/progress', parseProgress),
    law: () => read('/api/law', parseLaw),
    lawTiers: () => read('/api/level/law-tiers', parseLawTiers),
    feed: () => read(FEED_PATH, parseFeed),
    wallet: (before) => read(walletPath(before), parseWallet),
    insights: () => read('/api/insights', parseListings),
    insight: (id) =>
      read(`/api/insights/${encodeURIComponent(id)}`, (body) => {
        const parsed = parseEnvelope(body);
        return parsed === undefined ? null : { value: parsed };
      }).then((answer) =>
        answer.kind === 'ok' ? { kind: 'ok', value: answer.value.value } : answer
      ),
    capture: async (request) => {
      const response = await call(CAPTURE_PATH, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: captureBody(request)
      });
      if (response === 'reopen') return { kind: 'reopen' };
      if (response === null) return { kind: 'unavailable' };
      const saved = parseSaved(response.status, await response.json().catch(() => null));
      return saved === null ? { kind: 'unavailable' } : { kind: 'ok', value: saved };
    },
    sensitiveDecks: () => read(SENSITIVE_DECKS_PATH, parseMarked),
    setSensitive: async (id, sensitive) => {
      const response = await call(sensitiveDeckPath(id), {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: sensitiveBody(sensitive)
      });
      if (response === 'reopen') return { kind: 'reopen' };
      if (response === null || !response.ok) return { kind: 'unavailable' };
      const marked = parseMarked(await response.json().catch(() => null));
      return marked === null ? { kind: 'unavailable' } : { kind: 'ok', value: marked };
    }
  };
}

/** The body of `GET /api/me`, or null when it is not one. */
function parseMe(body: unknown): Me | null {
  const day =
    body !== null &&
    typeof body === 'object'
      ? (body as Record<string, unknown>).study_day
      : undefined;
  return typeof day === 'string' && ISO_DATE.test(day) ? { studyDay: day } : null;
}

/** The app's client, opening its session with the launch data the wrapper read. */
export const api: Api = createApi({ launchData: () => telegram.launchData });
