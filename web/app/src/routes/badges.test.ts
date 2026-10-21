/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Badges from './badges/+page.svelte';

// SPEC-073 R16, R19. The badge gallery shows the owner's badges as the server answers them:
// while it waits, a status; then the gallery, or the alert for a server it cannot reach or a
// session it cannot renew.
// The client keeps one session for the whole file, so the case that ends it runs last. Each wait
// is an assertion retried until it holds, so a screen that never shows reads as a failed assertion.
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

const BADGES = {
  earned: [
    { key: 'centurion_day', tier: 0, name: 'Centurion Day', emoji: '💯', study_day: '2025-01-13' }
  ],
  locked: [
    {
      key: 'monthly_monk',
      name: 'Monthly Monk',
      emoji: '🧘',
      criteria: '30-day streak',
      family: 'study',
      progress: { value: 3, threshold: 30 }
    }
  ]
};

describe('the badges screen page', () => {
  it('waits with a status, then shows the screen and a way back', async () => {
    let release = () => undefined as void;
    server.state.held = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.state.body = BADGES;

    render(Badges);

    await vi.waitFor(() => expect(screen.queryByRole('status')?.textContent).toBe('Loading…'));
    release();
    server.state.held = null;
    await vi.waitFor(() =>
      expect(screen.queryByRole('heading', { name: 'Earned badges' })?.tagName).toBe('H2')
    );
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('link', { name: 'Back to Today' })?.getAttribute('href')).toBe('/');
    expect(server.fetch.mock.calls.map(([input]) => String(input))).toContain('/api/badges');
  });

  it('says the server could not be reached when the badges cannot be read', async () => {
    server.state.score = 503;

    render(Badges);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('could not reach its server')
    );
    expect(screen.queryByRole('alert')?.textContent).not.toContain('Reopen');
    expect(screen.queryByRole('heading', { name: 'Earned badges' })).toBeNull();
  });

  it('asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    server.state.score = 401;
    server.state.session = 401;

    render(Badges);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('Reopen DeckStreak from Telegram')
    );
    expect(screen.queryByText(/could not reach/)).toBeNull();
  });
});
