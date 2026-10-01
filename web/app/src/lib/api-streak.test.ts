/**
 * @vitest-environment jsdom
 */
import { describe, expect, it, vi } from 'vitest';
import { createApi } from './api';

// SPEC-076 R20, R21, R23. `api.streak()` reads the streak body and then the governor body, both by
// GET with the session cookie alone, and answers one merged view, or why there is none. Each test
// judges the record of what the client sent, not only what it answered.
const LAUNCH = 'auth_date=1&hash=synthetic-launch-data';
const STREAK = {
  study_day: '2001-02-03',
  language: { current: 12, longest: 20, heat: 3, freezes: 2, freeze_cap: 3 },
  law: { current: 4, longest: 6, heat: 1 },
  at_stake: { language: 'freeze', law: 'break' }
};
const GOVERNOR = { verdict: 'lapse', strength: 0.4, lapse_since: '2001-02-01', relight_cards: 9 };

type Reply = number | unknown;

/** A server answering each path from `replies` (a number is a bare status, else a 200 JSON body). */
function server(replies: Record<string, Reply>) {
  const sent: Array<{ line: string; credentials?: RequestCredentials; body?: unknown }> = [];
  const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = String(input);
    sent.push({ line: `${init.method ?? 'GET'} ${url}`, credentials: init.credentials, body: init.body });
    if (url === '/api/session') return new Response(null, { status: 204 });
    const reply = replies[url];
    return typeof reply === 'number' ? new Response(null, { status: reply }) : Response.json(reply);
  });
  return { fetch: fetch as unknown as typeof globalThis.fetch, sent };
}

describe('the streak call', () => {
  it('opens a session, reads the streak then the governor, and merges them', async () => {
    const { fetch, sent } = server({ '/api/streak': STREAK, '/api/governor': GOVERNOR });
    const answer = await createApi({ launchData: () => LAUNCH, fetch }).streak();

    expect(answer).toStrictEqual({
      kind: 'ok',
      value: {
        studyDay: '2001-02-03',
        language: { current: 12, longest: 20, heat: 3, freezes: 2, freezeCap: 3 },
        law: { current: 4, longest: 6, heat: 1 },
        atStake: { language: 'freeze', law: 'break' },
        governor: {
          verdict: 'lapse',
          strength: 0.4,
          lapseSince: '2001-02-01',
          why: 'no_study_days',
          relightCards: 9
        }
      }
    });
    expect(sent.map((request) => request.line)).toEqual([
      'POST /api/session',
      'GET /api/streak',
      'GET /api/governor'
    ]);
    expect(sent[1].credentials).toBe('same-origin');
    expect(sent[2].credentials).toBe('same-origin');
    expect(sent[1].body).toBeUndefined();
    expect(sent[2].body).toBeUndefined();
  });

  it('asks for no governor when the streak is unavailable, and says so', async () => {
    const cases: Array<[Reply, string]> = [
      [503, 'status'],
      [{ study_day: 'today' }, 'malformed']
    ];
    for (const [reply] of cases) {
      const { fetch, sent } = server({ '/api/streak': reply, '/api/governor': GOVERNOR });
      const answer = await createApi({ launchData: () => LAUNCH, fetch }).streak();
      expect(answer).toStrictEqual({ kind: 'unavailable' });
      expect(sent.map((request) => request.line)).toEqual(['POST /api/session', 'GET /api/streak']);
    }
  });

  it('answers unavailable when the governor is a status error or malformed, after both reads', async () => {
    for (const reply of [503, { verdict: 'off' }]) {
      const { fetch, sent } = server({ '/api/streak': STREAK, '/api/governor': reply });
      const answer = await createApi({ launchData: () => LAUNCH, fetch }).streak();
      expect(answer).toStrictEqual({ kind: 'unavailable' });
      expect(sent.map((request) => request.line)).toEqual([
        'POST /api/session',
        'GET /api/streak',
        'GET /api/governor'
      ]);
    }
  });

  it('asks the owner to reopen when there are no launch data', async () => {
    const { fetch, sent } = server({ '/api/streak': STREAK, '/api/governor': GOVERNOR });
    const answer = await createApi({ launchData: () => null, fetch }).streak();
    expect(answer).toStrictEqual({ kind: 'reopen' });
    expect(sent).toEqual([]);
  });
});
