/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Score from './score/+page.svelte';

// SPEC-071 R22. The score screen shows the current study day's score as the server answers it:
// while it waits, a status; then the breakdown, or the sentence for a day with no score yet; and
// the alert for a server it cannot reach or a session it cannot renew. The wrapper and the API
// client are created when their modules load, so Telegram's object and the server are in place
// before the imports run. The client keeps one session for the whole file, so the case that ends
// it runs last.
const server = vi.hoisted(() => {
  const noop = () => undefined;
  const state: { session: number; score: number; body: unknown; held: Promise<void> | null } = {
    session: 200,
    score: 200,
    body: null,
    held: null
  };
  const fetch = vi.fn(async (input: RequestInfo | URL) => {
    if (String(input) === '/api/session') return new Response(null, { status: state.session });
    if (state.held !== null) await state.held;
    return state.score === 200 ? Response.json(state.body) : new Response(null, { status: state.score });
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

const SCORED = {
  study_day: '2001-02-03',
  score: {
    total: 64,
    grade: { label: 'SOLID', emoji: '✅' },
    pillars: { consistency: 76, retention: null, workload: 70, volume: 60.5, mastery: 55 },
    reviews: 0,
    retention: null
  }
};

describe('the score screen', () => {
  it('waits with a status, then shows the breakdown of the current day', async () => {
    let release = () => undefined as void;
    server.state.held = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.state.body = SCORED;

    render(Score);

    expect((await screen.findByRole('status')).textContent).toBe('Loading…');
    release();
    server.state.held = null;
    expect((await screen.findByRole('heading', { name: "Today's score" })).tagName).toBe('H2');
    expect(screen.getByText('64')).toBeTruthy();
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByRole('link', { name: 'Back to Today' }).getAttribute('href')).toBe('/');
  });

  it('says a day with no score has none yet', async () => {
    server.state.body = { study_day: '2001-02-03', score: null };

    render(Score);

    expect(await screen.findByText(/Today has no score yet/)).toBeTruthy();
    expect(screen.queryByRole('heading', { name: "Today's score" })).toBeNull();
  });

  it('says the server could not be reached when the score cannot be read', async () => {
    server.state.score = 503;

    render(Score);

    expect((await screen.findByRole('alert')).textContent).toContain('could not reach its server');
  });

  it('asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    server.state.score = 401;
    server.state.session = 401;

    render(Score);

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Reopen DeckStreak from Telegram'
    );
    expect(screen.queryByText(/could not reach/)).toBeNull();
  });
});
