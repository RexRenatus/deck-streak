import { parseLevel, type LevelView } from './level/level';
import { parseScore, type ScoreToday } from './score/score';
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
   * A same-origin GET that carries the session cookie alone: its response, `'reopen'` when only
   * reopening the app can help, or null when no answer came.
   */
  async function get(path: string): Promise<Response | 'reopen' | null> {
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
        response = await send(path, { credentials: 'same-origin' });
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
    const response = await get(path);
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
    level: () => read('/api/level', parseLevel)
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
