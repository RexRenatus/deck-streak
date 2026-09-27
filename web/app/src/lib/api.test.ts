/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createApi } from './api';

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
});
