/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Streak from './streak/+page.svelte';

// SPEC-076 R23. The streak page asks the server once, with a status while it waits, then shows both
// tracks and the governor, or the alert for a server it cannot reach or a session it cannot renew.
// The client keeps one session for the whole file, so the case that ends it runs last.
const server = vi.hoisted(() => {
  const noop = () => undefined;
  const state: {
    session: number;
    streak: number;
    governor: number;
    held: Promise<void> | null;
    sent: Array<{ line: string; credentials?: RequestCredentials; body?: unknown }>;
  } = { session: 200, streak: 200, governor: 200, held: null, sent: [] };
  const bodies = {
    streak: {
      study_day: '2001-02-03',
      language: { current: 12, longest: 20, heat: 3, freezes: 2, freeze_cap: 3 },
      law: { current: 4, longest: 6, heat: 1 },
      at_stake: { language: 'freeze', law: 'break' }
    },
    governor: { verdict: 'armed', strength: 0.9, lapse_since: null, relight_cards: null }
  };
  const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const url = String(input);
    state.sent.push({ line: `${init.method ?? 'GET'} ${url}`, credentials: init.credentials, body: init.body });
    if (url === '/api/session') return new Response(null, { status: state.session });
    if (state.held !== null) await state.held;
    const status = url === '/api/streak' ? state.streak : state.governor;
    return status === 200
      ? Response.json(url === '/api/streak' ? bodies.streak : bodies.governor)
      : new Response(null, { status });
  });
  const inset = { top: 0, bottom: 0, left: 0, right: 0 };
  Object.assign(globalThis, {
    fetch,
    Telegram: {
      WebApp: {
        initData: 'auth_date=1&hash=synthetic',
        version: '9.0',
        platform: 'tdesktop',
        colorScheme: 'light',
        themeParams: {},
        viewportStableHeight: 640,
        safeAreaInset: inset,
        contentSafeAreaInset: inset,
        isVersionAtLeast: () => true,
        ready: noop,
        expand: noop,
        openLink: noop,
        onEvent: noop,
        offEvent: noop
      }
    }
  });
  return { fetch, state };
});

describe('the streak screen page', () => {
  it('waits with a status, then shows the streaks, the governor and a way back', async () => {
    let release = () => undefined as void;
    server.state.held = new Promise<void>((resolve) => {
      release = resolve;
    });

    const { container } = render(Streak);

    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('DeckStreak');
    expect((await screen.findByRole('status')).textContent).toBe('Loading…');
    expect(screen.queryByRole('heading', { name: 'Your streaks' })).toBeNull();
    release();
    server.state.held = null;
    expect((await screen.findByRole('heading', { name: 'Your streaks' })).tagName).toBe('H2');
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(container.querySelectorAll('article').length).toBe(2);
    expect(screen.getByText('Armed: your habit is strong.').getAttribute('data-verdict')).toBe('armed');
    expect(screen.getByRole('link', { name: 'Back to Today' }).getAttribute('href')).toBe('/');
    expect(server.state.sent.map((request) => request.line)).toEqual([
      'POST /api/session',
      'GET /api/streak',
      'GET /api/governor'
    ]);
    expect(server.state.sent[0].body).toBe(JSON.stringify({ init_data: 'auth_date=1&hash=synthetic' }));
    expect(server.state.sent[1].credentials).toBe('same-origin');
    expect(server.state.sent[2].credentials).toBe('same-origin');
  });

  it('asks the server once when the screen is on the page', async () => {
    server.state.sent = [];

    render(Streak);
    await screen.findByRole('heading', { name: 'Your streaks' });

    expect(server.state.sent.map((request) => request.line)).toEqual(['GET /api/streak', 'GET /api/governor']);
  });

  it('says the server could not be reached when the governor cannot be read', async () => {
    server.state.governor = 503;

    render(Streak);

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('could not reach its server');
    expect(alert.textContent).not.toContain('Reopen');
    expect(screen.queryByRole('heading', { name: 'Your streaks' })).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('says the server could not be reached when the streak cannot be read', async () => {
    server.state.governor = 200;
    server.state.streak = 503;
    server.state.sent = [];

    render(Streak);

    expect((await screen.findByRole('alert')).textContent).toContain('could not reach its server');
    expect(server.state.sent.map((request) => request.line)).toEqual(['GET /api/streak']);
  });

  it('asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    server.state.streak = 401;
    server.state.session = 401;

    render(Streak);

    expect((await screen.findByRole('alert')).textContent).toContain('Reopen DeckStreak from Telegram');
    expect(screen.queryByText(/could not reach/)).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });
});
