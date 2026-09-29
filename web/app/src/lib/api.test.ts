/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createApi, type Answer, type Me } from './api';

// SPEC-028 R7, A5, A6; ADR-006. The client opens a session by posting the raw, signed launch data
// once, then calls with the session cookie alone. A synthetic launch string stands in for
// Telegram's: the tests read only whether and where it travels.
const LAUNCH = 'auth_date=1&hash=synthetic-launch-data';
const STUDY_DAY = '2001-02-03';

interface Sent {
  readonly url: string;
  readonly method: string;
  readonly credentials: RequestCredentials | undefined;
  readonly headers: Record<string, string>;
  readonly body: string | undefined;
}

/**
 * A server that answers the n-th request with the n-th status of `statuses` (599 once they run
 * out), and records every request it was sent. The tests judge the record afterwards, so a request
 * the client should not have made fails the test even when the client swallows its answer.
 */
function server(statuses: readonly number[]) {
  const sent: Sent[] = [];
  const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = String(input);
    const status = statuses[sent.length] ?? 599;
    sent.push({
      url,
      method: init.method ?? 'GET',
      credentials: init.credentials,
      headers: Object.fromEntries(new Headers(init.headers).entries()),
      body: typeof init.body === 'string' ? init.body : undefined
    });
    if (url === '/api/me' && status === 200) return Response.json({ study_day: STUDY_DAY });
    return new Response(null, { status });
  });
  return { fetch: fetch as unknown as typeof globalThis.fetch, sent };
}

function lines(sent: readonly Sent[]): string[] {
  return sent.map((request) => `${request.method} ${request.url}`);
}

/** Runs `run` once `turns` more turns of the microtask queue have passed. */
function after(turns: number, run: () => void): void {
  if (turns === 0) run();
  else queueMicrotask(() => after(turns - 1, run));
}

/** Yields `turns` turns of the microtask queue, so every call a test placed in it has started. */
async function settle(turns = 64): Promise<void> {
  for (let turn = 0; turn < turns; turn += 1) await Promise.resolve();
}

describe('the API client', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('the handshake sends initData once and later calls carry only the session', async () => {
    const stored = vi.spyOn(Storage.prototype, 'setItem');
    const logged = [
      vi.spyOn(console, 'log'),
      vi.spyOn(console, 'info'),
      vi.spyOn(console, 'warn'),
      vi.spyOn(console, 'error'),
      vi.spyOn(console, 'debug')
    ];
    const { fetch, sent } = server([200, 200, 200]);
    const api = createApi({ launchData: () => LAUNCH, fetch });

    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });

    expect(lines(sent)).toEqual(['POST /api/session', 'GET /api/me', 'GET /api/me']);
    // the handshake: the launch data raw, as JSON, from the same origin
    expect(JSON.parse(sent[0].body ?? 'null')).toEqual({ init_data: LAUNCH });
    expect(sent[0].headers).toEqual({ 'content-type': 'application/json' });
    expect(sent.map((request) => request.credentials)).toEqual([
      'same-origin',
      'same-origin',
      'same-origin'
    ]);
    // the launch data travels in that one request body, and in no other request, no URL, no
    // header, none of the device's storage and no log line
    const carrying = sent.filter((request) => JSON.stringify(request).includes(LAUNCH));
    expect(carrying).toEqual([sent[0]]);
    expect(sent.map((request) => request.url)).toEqual(['/api/session', '/api/me', '/api/me']);
    expect(stored.mock.calls.length).toBe(0);
    expect(logged.map((spy) => spy.mock.calls.length)).toEqual([0, 0, 0, 0, 0]);
  });

  it('an expired session re-handshakes once, then asks the owner to reopen', async () => {
    // the session ends; the launch data is past its freshness bound, so the server refuses it too
    const { fetch, sent } = server([200, 401, 401]);
    const api = createApi({ launchData: () => LAUNCH, fetch });

    expect(await api.me()).toEqual({ kind: 'reopen' });
    // and it stops calling: the next call answers the same and sends nothing
    expect(await api.me()).toEqual({ kind: 'reopen' });

    expect(lines(sent)).toEqual(['POST /api/session', 'GET /api/me', 'POST /api/session']);
    expect(JSON.parse(sent[2].body ?? 'null')).toEqual({ init_data: LAUNCH });
  });

  it('an expired session that re-handshakes carries on', async () => {
    const { fetch, sent } = server([200, 401, 200, 200]);
    const api = createApi({ launchData: () => LAUNCH, fetch });

    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(lines(sent)).toEqual([
      'POST /api/session',
      'GET /api/me',
      'POST /api/session',
      'GET /api/me'
    ]);
  });

  it('a call the new session refuses too asks the owner to reopen, and stops', async () => {
    const { fetch, sent } = server([200, 401, 200, 401]);
    const api = createApi({ launchData: () => LAUNCH, fetch });

    expect(await api.me()).toEqual({ kind: 'reopen' });
    expect(await api.me()).toEqual({ kind: 'reopen' });
    expect(lines(sent)).toEqual([
      'POST /api/session',
      'GET /api/me',
      'POST /api/session',
      'GET /api/me'
    ]);
  });

  it('a page opened outside Telegram asks the owner to reopen and sends nothing', async () => {
    const { fetch, sent } = server([]);
    const api = createApi({ launchData: () => null, fetch });

    expect(await api.me()).toEqual({ kind: 'reopen' });
    expect(lines(sent)).toEqual([]);
  });

  it('a server that fails says so, and the next call tries again', async () => {
    const { fetch, sent } = server([503, 200, 200]);
    const api = createApi({ launchData: () => LAUNCH, fetch });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(lines(sent)).toEqual(['POST /api/session', 'POST /api/session', 'GET /api/me']);
  });

  it('a study day that is not an ISO date is refused rather than shown', async () => {
    const fetch = vi.fn(async (input: RequestInfo | URL) =>
      String(input) === '/api/me'
        ? Response.json({ study_day: 'the next day' })
        : new Response(null, { status: 200 })
    );
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
  });

  // SPEC-071 §10: the older paths StrykerJS found untested once this delivery touched the client.
  it('a handshake the network drops says so, calls nothing, and the next call tries again', async () => {
    const sent: string[] = [];
    let dropped = false;
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const line = `${init.method ?? 'GET'} ${String(input)}`;
      sent.push(line);
      if (line === 'POST /api/session' && !dropped) {
        dropped = true;
        throw new TypeError('Failed to fetch');
      }
      return line === 'GET /api/me'
        ? Response.json({ study_day: STUDY_DAY })
        : new Response(null, { status: 200 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
    expect(sent).toEqual(['POST /api/session']);
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(sent).toEqual(['POST /api/session', 'POST /api/session', 'GET /api/me']);
  });

  it('a call the network drops says so, and the next call carries on in the session', async () => {
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const line = `${init.method ?? 'GET'} ${String(input)}`;
      sent.push(line);
      if (line === 'GET /api/me' && sent.filter((one) => one === line).length === 1) {
        throw new TypeError('Failed to fetch');
      }
      return line === 'GET /api/me'
        ? Response.json({ study_day: STUDY_DAY })
        : new Response(null, { status: 200 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(sent).toEqual(['POST /api/session', 'GET /api/me', 'GET /api/me']);
  });

  it('a handshake the server forbids asks the owner to reopen, and stops', async () => {
    const { fetch, sent } = server([403, 200, 200]);
    const api = createApi({ launchData: () => LAUNCH, fetch });

    expect(await api.me()).toEqual({ kind: 'reopen' });
    expect(await api.me()).toEqual({ kind: 'reopen' });
    expect(lines(sent)).toEqual(['POST /api/session']);
  });

  it('calls whose session ended together share one new handshake', async () => {
    const sent: string[] = [];
    let sessions = 0;
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const line = `${init.method ?? 'GET'} ${String(input)}`;
      sent.push(line);
      if (line === 'POST /api/session') {
        sessions += 1;
        return new Response(null, { status: 200 });
      }
      // the first session ends before either call is answered; the second serves both
      return sessions < 2
        ? new Response(null, { status: 401 })
        : Response.json({ study_day: STUDY_DAY });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    const answers = await Promise.all([api.me(), api.me()]);

    const ok = { kind: 'ok', value: { studyDay: STUDY_DAY } };
    expect(answers).toEqual([ok, ok]);
    expect(sent).toEqual([
      'POST /api/session',
      'GET /api/me',
      'GET /api/me',
      'POST /api/session',
      'GET /api/me',
      'GET /api/me'
    ]);
  });

  it('a failed handshake forgets its own attempt, never a newer session another call opened', async () => {
    // The first handshake is dropped by hand. At each offset of the microtask queue after the
    // drop, one call joins the failing attempt and queues another call, which opens a newer
    // session. Where the joining call resumes only after that newer session opened, it must leave
    // the newer session alone, so a later call carries on in it and sends no handshake. The sweep
    // must reach that interleaving at least once, or it proves nothing (SPEC-071 §10).
    const ok = { kind: 'ok', value: { studyDay: STUDY_DAY } };
    let interleaved = 0;
    for (let offset = 0; offset < 8; offset += 1) {
      const sent: string[] = [];
      const late: { joined?: Promise<Answer<Me>>; opener?: Promise<Answer<Me>> } = {};
      let drop: (reason: Error) => void = () => undefined;
      const fetch = (input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> => {
        const line = `${init.method ?? 'GET'} ${String(input)}`;
        sent.push(line);
        if (sent.length > 1) {
          return Promise.resolve(
            line === 'GET /api/me'
              ? Response.json({ study_day: STUDY_DAY })
              : new Response(null, { status: 200 })
          );
        }
        const dropped = new Promise<Response>((_resolve, reject) => {
          drop = reject;
        });
        // attached before the handshake awaits the drop, so each offset counts from the drop
        void dropped.catch(() =>
          after(offset, () => {
            queueMicrotask(() => {
              late.opener = api.me();
            });
            late.joined = api.me();
          })
        );
        return dropped;
      };
      const api = createApi({
        launchData: () => LAUNCH,
        fetch: fetch as unknown as typeof globalThis.fetch
      });

      const first = api.me();
      drop(new TypeError('Failed to fetch'));
      await settle();
      expect(await first, `offset ${offset}`).toEqual({ kind: 'unavailable' });
      const joined = await late.joined;
      const opener = await late.opener;
      if (joined?.kind === 'unavailable' && opener?.kind === 'ok') interleaved += 1;

      const before = sent.length;
      expect(await api.me(), `offset ${offset}`).toEqual(ok);
      if (opener?.kind === 'ok') {
        // a session is open, so the later call carries on in it
        expect(sent.slice(before), `offset ${offset}`).toEqual(['GET /api/me']);
      }
    }
    expect(interleaved, 'offsets that reached the interleaving').toBeGreaterThan(0);
  });

  it('a /api/me the server fails is unavailable, even when its body reads as a session', async () => {
    const fetch = vi.fn(async (input: RequestInfo | URL) =>
      String(input) === '/api/me'
        ? Response.json({ study_day: STUDY_DAY }, { status: 500 })
        : new Response(null, { status: 200 })
    );
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
  });

  it('a /api/me body that is no JSON object, or a malformed study day, is unavailable', async () => {
    let body: BodyInit | null = null;
    const fetch = vi.fn(async (input: RequestInfo | URL) =>
      String(input) === '/api/me'
        ? new Response(body, { status: 200 })
        : new Response(null, { status: 200 })
    );
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });
    const malformed: readonly [string, BodyInit | null][] = [
      ['no body', null],
      ['a body that is not JSON', 'not json'],
      ['JSON null', 'null'],
      ['a JSON string', JSON.stringify(STUDY_DAY)],
      ['a study day in an array', JSON.stringify({ study_day: [STUDY_DAY] })],
      ['text before the date', JSON.stringify({ study_day: `x${STUDY_DAY}` })],
      ['text after the date', JSON.stringify({ study_day: `${STUDY_DAY}x` })]
    ];

    for (const [what, given] of malformed) {
      body = given;
      expect(await api.me(), what).toEqual({ kind: 'unavailable' });
    }
    // and the same client reads a well-formed body
    body = JSON.stringify({ study_day: STUDY_DAY });
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
  });
});

// SPEC-071 R20, R22. The score screen reads the current study day's score through the same
// session: one GET of `/api/score`, whose body is read by the score module.
describe("the API client's score", () => {
  const SCORE = {
    study_day: STUDY_DAY,
    score: {
      total: 72,
      grade: { label: 'SOLID', emoji: '✅' },
      pillars: { consistency: 76, retention: null, workload: 70, volume: 60.5, mastery: 55 },
      reviews: 40,
      retention: null
    }
  };

  function scoring(body: unknown) {
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push(`${init.method ?? 'GET'} ${String(input)}`);
      return String(input) === '/api/score' ? Response.json(body) : new Response(null, { status: 200 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });
    return { api, sent };
  }

  it('reads the current study day and its score from /api/score', async () => {
    const { api, sent } = scoring(SCORE);
    expect(await api.score()).toEqual({
      kind: 'ok',
      value: {
        studyDay: STUDY_DAY,
        score: {
          total: 72,
          grade: { label: 'SOLID', emoji: '✅' },
          pillars: { consistency: 76, retention: null, workload: 70, volume: 60.5, mastery: 55 }
        }
      }
    });
    expect(sent).toEqual(['POST /api/session', 'GET /api/score']);
  });

  it('answers unavailable when /api/score answers something that is not a score', async () => {
    const { api } = scoring({ study_day: STUDY_DAY });
    expect(await api.score()).toEqual({ kind: 'unavailable' });
  });
});

// SPEC-072 R23. The level screen reads the owner's level through the same session: one GET of
// `/api/level`, whose body is read by the level module.
describe("the API client's level", () => {
  const LEVEL = {
    study_day: STUDY_DAY,
    level: 7,
    title: 'Adept',
    emoji: '🌿',
    total_xp: 1234,
    xp_into_level: 40,
    xp_for_next: 200,
    today: [],
    run: 5,
    multiplier: 1.25,
    multiplier_after_a_miss: 1.1,
    ascendant: false
  };

  function leveling(body: unknown) {
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push(`${init.method ?? 'GET'} ${String(input)}`);
      return String(input) === '/api/level' ? Response.json(body) : new Response(null, { status: 200 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });
    return { api, sent };
  }

  it('reads the level view from /api/level', async () => {
    const { api, sent } = leveling(LEVEL);
    const answer = await api.level();
    expect(answer.kind).toBe('ok');
    expect(answer.kind === 'ok' && answer.value.title).toBe('Adept');
    expect(sent).toEqual(['POST /api/session', 'GET /api/level']);
  });

  it('answers unavailable when /api/level answers something that is not a level view', async () => {
    const { api } = leveling({ study_day: STUDY_DAY });
    expect(await api.level()).toEqual({ kind: 'unavailable' });
  });
});
