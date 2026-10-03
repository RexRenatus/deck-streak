/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Level from './level/+page.svelte';

// SPEC-072 R26. The level screen shows the owner's level as the server answers it: while it waits,
// a status; then the level, or the alert for a server it cannot reach or a session it cannot renew.
// The client keeps one session for the whole file, so the case that ends it runs last.
const server = vi.hoisted(() => {
  const noop = () => undefined;
  const state: { session: number; score: number; body: unknown; held: Promise<void> | null } = {
    session: 200,
    score: 200,
    body: null,
    held: null
  };
  // The XP exchange readout (SPEC-075 R9) answers at once, with a fixed body or the status
  // `exchange.status` names, so every case above it reads the level alone.
  const exchange: { status: number; body: unknown; held: Promise<void> | null } = {
    status: 200,
    held: null,
    body: {
      window: null,
      rates: [{ source: 'reviews', total_xp: 10, graduated_cards: 4, rate: 2.5, rate_defined: true }]
    }
  };
  const fetch = vi.fn(async (input: RequestInfo | URL) => {
    if (String(input) === '/api/session') return new Response(null, { status: state.session });
    if (String(input) === '/api/xp/exchange') {
      if (exchange.held !== null) await exchange.held;
      return exchange.status === 200
        ? Response.json(exchange.body)
        : new Response(null, { status: exchange.status });
    }
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
  return { fetch, state, exchange };
});

const LEVELED = {
  study_day: '2001-02-03',
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

describe('the level screen page', () => {
  it('waits with a status, then shows the level and a way back', async () => {
    let release = () => undefined as void;
    server.state.held = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.state.body = LEVELED;

    render(Level);

    expect((await screen.findByRole('status')).textContent).toBe('Loading…');
    release();
    server.state.held = null;
    expect((await screen.findByRole('heading', { name: 'Your level' })).tagName).toBe('H2');
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('link', { name: 'Back to Today' }).getAttribute('href')).toBe('/');
  });

  it('says the server could not be reached when the level cannot be read', async () => {
    server.state.score = 503;

    render(Level);

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('could not reach its server');
    expect(alert.textContent).not.toContain('Reopen');
    expect(screen.queryByRole('heading', { name: 'Your level' })).toBeNull();
  });

  it('shows the XP exchange readout below the level', async () => {
    server.state.score = 200;
    server.state.body = LEVELED;

    render(Level);

    await vi.waitFor(() => {
      expect(screen.queryByRole('heading', { name: 'XP exchange rates' })?.tagName).toBe('H2');
      expect(screen.queryByRole('heading', { name: 'Your level' })?.tagName).toBe('H2');
    });
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(server.fetch.mock.calls.map(([input]) => String(input))).toContain('/api/xp/exchange');
  });

  it('waits with a status in place of the readout, then shows it', async () => {
    server.state.score = 200;
    server.state.body = LEVELED;
    let release = () => undefined as void;
    server.exchange.held = new Promise<void>((resolve) => {
      release = resolve;
    });

    render(Level);

    await vi.waitFor(() => {
      expect(screen.queryByRole('heading', { name: 'Your level' })?.tagName).toBe('H2');
      expect(screen.queryByRole('status')?.textContent).toBe('Loading…');
    });
    expect(screen.queryByRole('heading', { name: 'XP exchange rates' })).toBeNull();
    release();
    server.exchange.held = null;
    await vi.waitFor(() =>
      expect(screen.queryByRole('heading', { name: 'XP exchange rates' })?.tagName).toBe('H2')
    );
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('says the server could not be reached in place of the readout, and keeps the level', async () => {
    server.state.score = 200;
    server.state.body = LEVELED;
    server.exchange.status = 503;

    render(Level);

    await vi.waitFor(() => {
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('could not reach its server');
      expect(screen.queryByRole('heading', { name: 'Your level' })?.tagName).toBe('H2');
    });
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('heading', { name: 'XP exchange rates' })).toBeNull();
    server.exchange.status = 200;
  });

  it('asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    server.state.score = 401;
    server.state.session = 401;

    render(Level);

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Reopen DeckStreak from Telegram'
    );
    expect(screen.queryByText(/could not reach/)).toBeNull();
  });

  it('asks to reopen DeckStreak once, with no readout alert beside it', async () => {
    server.exchange.status = 401;

    render(Level);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('Reopen DeckStreak from Telegram')
    );
    expect(screen.queryAllByRole('alert')).toHaveLength(1);
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByText(/could not reach/)).toBeNull();
    expect(screen.queryByRole('heading', { name: 'XP exchange rates' })).toBeNull();
  });
});
