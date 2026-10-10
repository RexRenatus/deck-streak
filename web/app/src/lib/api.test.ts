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

  it('a launch the server refuses at once asks the owner to reopen, and stops', async () => {
    for (const status of [401, 403]) {
      const { fetch, sent } = server([status]);
      const api = createApi({ launchData: () => LAUNCH, fetch });

      expect(await api.me(), `status ${status}`).toEqual({ kind: 'reopen' });
      expect(await api.me(), `status ${status}`).toEqual({ kind: 'reopen' });
      expect(lines(sent), `status ${status}`).toEqual(['POST /api/session']);
    }
  });

  it('a handshake that cannot reach the server says so, and the next call tries again', async () => {
    const sent: string[] = [];
    let reachable = false;
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push(`${init.method ?? 'GET'} ${String(input)}`);
      if (!reachable) throw new TypeError('network down');
      return String(input) === '/api/me'
        ? Response.json({ study_day: STUDY_DAY })
        : new Response(null, { status: 200 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
    // no session came of it, so nothing was asked of the API
    expect(sent).toEqual(['POST /api/session']);
    reachable = true;
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(sent).toEqual(['POST /api/session', 'POST /api/session', 'GET /api/me']);
  });

  it('a call that cannot reach the server says so, and the session stays open', async () => {
    const sent: string[] = [];
    let reachable = false;
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push(`${init.method ?? 'GET'} ${String(input)}`);
      if (String(input) === '/api/session') return new Response(null, { status: 200 });
      if (!reachable) throw new TypeError('network down');
      return Response.json({ study_day: STUDY_DAY });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
    reachable = true;
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(sent).toEqual(['POST /api/session', 'GET /api/me', 'GET /api/me']);
  });

  it('an answer that is not a success is unavailable whatever its body says', async () => {
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

  it('a body that is not a study day is refused rather than shown', async () => {
    const bodies: readonly string[] = [
      'null',
      'not json at all',
      '"2001-02-03"',
      '5',
      '[]',
      '{}',
      JSON.stringify({ study_day: null }),
      JSON.stringify({ study_day: 20010203 }),
      // an array that reads as a date once it is written out as text
      JSON.stringify({ study_day: [STUDY_DAY] }),
      // a date with anything before or after it is not an ISO date
      JSON.stringify({ study_day: `x${STUDY_DAY}` }),
      JSON.stringify({ study_day: `${STUDY_DAY}x` }),
      JSON.stringify({ study_day: `${STUDY_DAY}\n` })
    ];
    for (const body of bodies) {
      const fetch = vi.fn(async (input: RequestInfo | URL) =>
        String(input) === '/api/me'
          ? new Response(body, { status: 200 })
          : new Response(null, { status: 200 })
      );
      const api = createApi({
        launchData: () => LAUNCH,
        fetch: fetch as unknown as typeof globalThis.fetch
      });

      expect(await api.me(), `body ${body}`).toEqual({ kind: 'unavailable' });
    }
  });

  it('a call that meets the ended session after another renewed it joins the new session', async () => {
    const gates: Array<(status: number) => void> = [];
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const url = String(input);
      sent.push(`${init.method ?? 'GET'} ${url}`);
      if (url === '/api/session') return new Response(null, { status: 200 });
      // the first two calls wait for the test to answer them; later ones succeed
      if (sent.filter((line) => line === 'GET /api/me').length <= 2) {
        const status = await new Promise<number>((resolve) => gates.push(resolve));
        return new Response(null, { status });
      }
      return Response.json({ study_day: STUDY_DAY });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    const both = Promise.all([api.me(), api.me()]);
    await vi.waitFor(() => expect(gates.length).toBe(2));
    // both calls learn at once that the session has ended
    gates[0](401);
    gates[1](401);

    const answers = await both;
    expect(answers).toEqual([
      { kind: 'ok', value: { studyDay: STUDY_DAY } },
      { kind: 'ok', value: { studyDay: STUDY_DAY } }
    ]);
    // one session at first, one renewal shared by both: two handshakes, not three
    expect(sent.filter((line) => line === 'POST /api/session')).toHaveLength(2);
  });

  it('a call that meets a handshake another call already abandoned keeps the session opened since', async () => {
    // A first call's handshake fails. A second call joins that failed handshake before the first
    // has abandoned it, and a third opens a new session in between. However those three fall
    // against each other, the second must not discard the third's session: a fourth call opens none.
    const handshakes: number[] = [];
    for (let delay = 0; delay < 24; delay += 1) {
      let fail: ((response: Response) => void) | undefined;
      let opened = 0;
      const fetch = vi.fn((input: RequestInfo | URL) => {
        if (String(input) !== '/api/session') {
          return Promise.resolve(Response.json({ study_day: STUDY_DAY }));
        }
        opened += 1;
        if (opened > 1) return Promise.resolve(new Response(null, { status: 200 }));
        return new Promise<Response>((resolve) => {
          fail = resolve;
        });
      });
      const api = createApi({
        launchData: () => LAUNCH,
        fetch: fetch as unknown as typeof globalThis.fetch
      });
      const first = api.me();
      for (let tick = 0; tick < 3; tick += 1) await Promise.resolve();
      expect(fail).toBeTypeOf('function');
      fail?.(new Response(null, { status: 503 }));
      let later: Promise<unknown> = Promise.resolve();
      let joined: Promise<unknown> = Promise.resolve();
      let step: Promise<void> = Promise.resolve();
      for (let tick = 0; tick < delay; tick += 1) step = step.then(() => undefined);
      await step.then(() => {
        queueMicrotask(() => {
          later = api.me();
        });
        joined = api.me();
      });
      await Promise.all([first, joined, later]);
      expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
      handshakes.push(opened);
    }
    // the failed handshake and the one that opened: never a third
    expect(handshakes).toEqual(Array.from({ length: 24 }, () => 2));
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

describe("the API client's insights", () => {
  function reading(bodies: Record<string, unknown>) {
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push(`${init.method ?? 'GET'} ${String(input)}`);
      return String(input) in bodies
        ? Response.json(bodies[String(input)])
        : new Response(null, { status: 200 });
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });
    return { api, sent };
  }

  it('lists the instruments from /api/insights', async () => {
    const { api, sent } = reading({
      '/api/insights': { instruments: [{ id: 'dark_fields', cadence: 'weekly', study_day: null }] }
    });
    expect(await api.insights()).toEqual({
      kind: 'ok',
      value: [{ id: 'dark_fields', cadence: 'weekly', studyDay: null }]
    });
    expect(sent).toEqual(['POST /api/session', 'GET /api/insights']);
  });

  it('answers unavailable when the listing is not one', async () => {
    const { api } = reading({ '/api/insights': { nope: 1 } });
    expect(await api.insights()).toEqual({ kind: 'unavailable' });
  });

  it('reads one instrument by its encoded id, and a not-yet-run report as null', async () => {
    const stored = { study_day: 3, failed_reads: [], report: { a: 1 } };
    const { api, sent } = reading({
      '/api/insights/a%20b': { instrument: 'a b', report: stored },
      '/api/insights/fresh': { instrument: 'fresh', report: null }
    });
    expect(await api.insight('a b')).toEqual({
      kind: 'ok',
      value: { instrument: 'a b', studyDay: 3, failedReads: [], report: { a: 1 } }
    });
    expect(await api.insight('fresh')).toEqual({ kind: 'ok', value: null });
    expect(sent).toContain('GET /api/insights/a%20b');
  });

  it('answers unavailable when an instrument body is not an envelope', async () => {
    const { api } = reading({ '/api/insights/x': { oops: true } });
    expect(await api.insight('x')).toEqual({ kind: 'unavailable' });
  });
});

// SPEC-118 R10; ruling (l). The quick capture is the client's one POST beside its reads: the same
// session, opened once and renewed at most once, carries a JSON body from the same origin, and a
// renewal repeats the same capture with the same retry key.
describe("the API client's capture", () => {
  const CAPTURE = { captureId: '0123456789abcdef0123456789abcdef', kind: 'journal', text: 'x' } as const;

  /** A server answering the n-th request with the n-th of `answers`, recording each request. */
  function capturing(answers: readonly (() => Response)[]) {
    const sent: Sent[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push({
        url: String(input),
        method: init.method ?? 'GET',
        credentials: init.credentials,
        headers: Object.fromEntries(new Headers(init.headers).entries()),
        body: typeof init.body === 'string' ? init.body : undefined
      });
      return (answers[sent.length - 1] ?? (() => new Response(null, { status: 599 })))();
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });
    return { api, sent };
  }

  const open = () => new Response(null, { status: 200 });

  it('posts the capture as JSON, from the same origin, in the session', async () => {
    const { api, sent } = capturing([open, () => Response.json({ name: 'n.md' }, { status: 201 })]);

    expect(await api.capture(CAPTURE)).toEqual({
      kind: 'ok',
      value: { name: 'n.md', alreadyCaptured: false }
    });
    expect(lines(sent)).toEqual(['POST /api/session', 'POST /api/inbox/captures']);
    expect(sent[1].headers).toEqual({ 'content-type': 'application/json' });
    expect(sent[1].credentials).toBe('same-origin');
    expect(JSON.parse(sent[1].body ?? 'null')).toEqual({
      capture_id: CAPTURE.captureId,
      kind: 'journal',
      text: 'x'
    });
    // the launch data travels only in the handshake
    expect(sent.filter((request) => JSON.stringify(request).includes(LAUNCH))).toEqual([sent[0]]);
  });

  it('renews an ended session once and repeats the same capture', async () => {
    const { api, sent } = capturing([
      open,
      () => new Response(null, { status: 401 }),
      open,
      () => Response.json({ name: 'n.md', already_captured: true }, { status: 200 })
    ]);

    expect(await api.capture(CAPTURE)).toEqual({
      kind: 'ok',
      value: { name: 'n.md', alreadyCaptured: true }
    });
    expect(lines(sent)).toEqual([
      'POST /api/session',
      'POST /api/inbox/captures',
      'POST /api/session',
      'POST /api/inbox/captures'
    ]);
    expect(sent[3].body).toBe(sent[1].body);
  });

  it('answers unavailable for a refusal or a body that names no capture, and reopen for a refused session', async () => {
    for (const answer of [
      () => Response.json({ reason: 'vault_missing' }, { status: 503 }),
      () => Response.json({ reason: 'text_out_of_bounds' }, { status: 422 }),
      () => Response.json({ oops: true }, { status: 201 })
    ]) {
      const { api } = capturing([open, answer]);
      expect(await api.capture(CAPTURE)).toEqual({ kind: 'unavailable' });
    }
    const { api, sent } = capturing([
      open,
      () => new Response(null, { status: 401 }),
      () => new Response(null, { status: 401 })
    ]);
    expect(await api.capture(CAPTURE)).toEqual({ kind: 'reopen' });
    expect(lines(sent)).toEqual([
      'POST /api/session',
      'POST /api/inbox/captures',
      'POST /api/session'
    ]);
  });

  it('answers unavailable when no answer came: no session opened, or the connection dropped', async () => {
    const unopened = capturing([() => new Response(null, { status: 500 })]);
    expect(await unopened.api.capture(CAPTURE)).toEqual({ kind: 'unavailable' });
    expect(lines(unopened.sent)).toEqual(['POST /api/session']);

    const dropped = capturing([
      open,
      () => {
        throw new TypeError('the connection dropped');
      }
    ]);
    expect(await dropped.api.capture(CAPTURE)).toEqual({ kind: 'unavailable' });
    expect(lines(dropped.sent)).toEqual(['POST /api/session', 'POST /api/inbox/captures']);
  });
});

// SPEC-385 R9, A5; ADR-399 D2. Outside Telegram there is no launch data to post, so the client
// sends each owner call with the session cookie alone; a call refused 401 asks for sign-in once
// and answers reopen to its screen, and the answer kinds stay three.
describe('the API client outside Telegram', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('outside telegram a refused call asks for sign-in', async () => {
    const askSignIn = vi.fn();
    const { fetch, sent } = server([200, 401]);
    const options = { launchData: () => null, fetch, askSignIn };
    const api = createApi(options);

    // a call in a session answers as inside Telegram, and asks for nothing
    expect(await api.me()).toEqual({ kind: 'ok', value: { studyDay: STUDY_DAY } });
    expect(askSignIn).not.toHaveBeenCalled();
    // a call refused 401 asks for sign-in, once, and answers reopen
    expect(await api.me()).toEqual({ kind: 'reopen' });
    expect(askSignIn).toHaveBeenCalledOnce();
    // the session cookie alone: no handshake and no renewal
    expect(lines(sent)).toEqual(['GET /api/me', 'GET /api/me']);
    expect(sent.map((request) => request.credentials)).toEqual(['same-origin', 'same-origin']);
  });
});

// MUTATION COVERAGE (SPEC-385 §7): the app's own client outside Telegram, a call in a sign-in
// session that meets no answer, and the methods list. Each was green when its test was written,
// and none is a criterion of §3. The router is replaced, so the sign-in page opens nowhere.
const navigation = vi.hoisted(() => ({ goto: vi.fn() }));
vi.mock('$app/navigation', () => ({ goto: navigation.goto }));

describe("the API client's sign-in surface", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    navigation.goto.mockReset();
  });

  it("the app's own client opens the sign-in page for a refused call outside telegram", async () => {
    const { fetch, sent } = server([401]);
    vi.stubGlobal('fetch', fetch);
    const { api } = await import('./api');

    expect(await api.me()).toEqual({ kind: 'reopen' });
    await vi.waitFor(() => expect(navigation.goto).toHaveBeenCalledOnce());
    expect(navigation.goto.mock.calls).toEqual([['/signin']]);
    // with the session cookie alone: no handshake was posted
    expect(lines(sent)).toEqual(['GET /api/me']);
  });

  it('a call in a sign-in session that meets no answer is unavailable, and asks for nothing', async () => {
    const askSignIn = vi.fn();
    const fetch = vi.fn(async () => {
      throw new TypeError('the network dropped the request');
    });
    const api = createApi({
      launchData: () => null,
      fetch: fetch as unknown as typeof globalThis.fetch,
      askSignIn
    });

    expect(await api.me()).toEqual({ kind: 'unavailable' });
    expect(fetch).toHaveBeenCalledOnce();
    expect(askSignIn).not.toHaveBeenCalled();
  });

  it('reads the sign-in methods from /api/identities', async () => {
    const ADDED = Date.UTC(2001, 1, 3, 12);
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      sent.push(`${init.method ?? 'GET'} ${String(input)}`);
      if (String(input) !== '/api/identities') return new Response(null, { status: 200 });
      return Response.json([
        { id: 1, kind: 'telegram' },
        { id: 4, kind: 'passkey', created_at: ADDED }
      ]);
    });
    const api = createApi({
      launchData: () => LAUNCH,
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await api.identities()).toEqual({
      kind: 'ok',
      value: [
        { id: 1, kind: 'telegram' },
        { id: 4, kind: 'passkey', createdAt: ADDED }
      ]
    });
    expect(sent).toEqual(['POST /api/session', 'GET /api/identities']);
  });
});
