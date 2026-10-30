/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Law from './law/+page.svelte';

// SPEC-077 R17, T34. The law tab shows the law block and SPEC-072's law cards by tier as the server
// answers them: while it waits, a status; then the block, with a note in place of the tier table
// when only the tiers cannot be read, or the alert for a server it cannot reach or a session it
// cannot renew.
// The client keeps one session for the whole file, so the case that ends it runs last. Each wait
// is an assertion retried until it holds, so a screen that never shows reads as a failed assertion.
const server = vi.hoisted(() => {
  const noop = () => undefined;
  const state: {
    session: number;
    status: Record<string, number>;
    body: Record<string, unknown>;
    held: Promise<void> | null;
  } = {
    session: 200,
    status: {},
    body: {},
    held: null
  };
  const fetch = vi.fn(async (input: RequestInfo | URL) => {
    const path = String(input);
    if (path === '/api/session') return new Response(null, { status: state.session });
    if (state.held !== null) await state.held;
    const status = state.status[path] ?? 200;
    return status === 200 ? Response.json(state.body[path] ?? null) : new Response(null, { status });
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

const LAW = {
  shown: true,
  level_shown: false,
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

const TIERS = {
  cards: { T1: 5, T2: 4, T3: 3, T4: 2, none: 1 },
  xp_today: { T1: 10, T2: 8, T3: 6, T4: 4, none: 2 }
};

describe('the law tab page', () => {
  it('waits with a status, then shows the law block, its tiers and a way back', async () => {
    let release = () => undefined as void;
    server.state.held = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.state.body = { '/api/law': LAW, '/api/level/law-tiers': TIERS };

    render(Law);

    await vi.waitFor(() => expect(screen.queryByRole('status')?.textContent).toBe('Loading…'));
    release();
    server.state.held = null;
    await vi.waitFor(() => expect(screen.queryByRole('heading', { name: 'Law' })?.tagName).toBe('H2'));
    expect(screen.queryByRole('heading', { level: 1 })?.textContent).toBe('DeckStreak');
    expect(screen.queryByRole('list', { name: 'Law' })?.textContent).toContain('Lifetime law XP: 900');
    expect(screen.queryByText('Law cards due: pending')?.tagName).toBe('LI');
    expect(screen.queryByRole('table', { name: 'Law cards by tier' })).not.toBeNull();
    expect(screen.queryByText(/cannot be shown right now/)).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('link', { name: 'Back to Today' })?.getAttribute('href')).toBe('/');
    const paths = server.fetch.mock.calls.map(([input]) => String(input));
    expect(paths).toContain('/api/law');
    expect(paths).toContain('/api/level/law-tiers');
  });

  it('shows the law block with a note in place of the tiers when only the tiers cannot be read', async () => {
    server.state.body = { '/api/law': LAW };
    server.state.status = { '/api/level/law-tiers': 503 };

    render(Law);

    await vi.waitFor(() =>
      expect(screen.queryByText('The law cards by tier cannot be shown right now.')?.tagName).toBe('P')
    );
    expect(screen.queryByRole('heading', { name: 'Law' })?.tagName).toBe('H2');
    expect(screen.queryByRole('table')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('says the server could not be reached when the law block cannot be read', async () => {
    server.state.body = { '/api/level/law-tiers': TIERS };
    server.state.status = { '/api/law': 503 };

    render(Law);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('could not reach its server')
    );
    expect(screen.queryByRole('alert')?.textContent).not.toContain('Reopen');
    expect(screen.queryByRole('heading', { name: 'Law' })).toBeNull();
    expect(screen.queryByRole('link', { name: 'Back to Today' })?.getAttribute('href')).toBe('/');
  });

  it('asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    server.state.status = { '/api/law': 401, '/api/level/law-tiers': 401 };
    server.state.session = 401;

    render(Law);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('Reopen DeckStreak from Telegram')
    );
    expect(screen.queryByText(/could not reach/)).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Law' })).toBeNull();
  });
});
