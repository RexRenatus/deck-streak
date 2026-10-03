/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Progress from './progress/+page.svelte';

// SPEC-077 R17, T34. The progress screen shows each course's Road to C2 as the server answers it:
// while it waits, a status; then the ladders, a note when there is no course, or the alert for a
// server it cannot reach or a session it cannot renew.
// The client keeps one session for the whole file, so the case that ends it runs last. Each wait
// is an assertion retried until it holds, so a screen that never shows reads as a failed assertion.
const server = vi.hoisted(() => {
  const noop = () => undefined;
  const state: { session: number; status: number; body: unknown; held: Promise<void> | null } = {
    session: 200,
    status: 200,
    body: null,
    held: null
  };
  const fetch = vi.fn(async (input: RequestInfo | URL) => {
    if (String(input) === '/api/session') return new Response(null, { status: state.session });
    if (state.held !== null) await state.held;
    return state.status === 200 ? Response.json(state.body) : new Response(null, { status: state.status });
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

const BANDS = ['A1', 'A2', 'B1', 'B2', 'C1', 'C2'];

function course(code: string, name: string) {
  return {
    code,
    name,
    flag: code === 'es' ? '🇪🇸' : '🇫🇷',
    mastery_pct: 41.5,
    current_band: 'B1',
    current_unit: 7,
    bands: BANDS.map((band, i) => ({
      band,
      total: 100,
      mature: i < 2 ? 100 : 30,
      pct: i < 2 ? 92 : 30,
      achieved: i < 2
    }))
  };
}

const PROGRESS = { courses: [course('fr', 'French'), course('es', 'Spanish')] };

describe('the progress screen page', () => {
  it('waits with a status, then shows each course and a way back', async () => {
    let release = () => undefined as void;
    server.state.held = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.state.body = PROGRESS;

    render(Progress);

    await vi.waitFor(() => expect(screen.queryByRole('status')?.textContent).toBe('Loading…'));
    release();
    server.state.held = null;
    await vi.waitFor(() =>
      expect(screen.queryByRole('heading', { name: 'Road to C2' })?.tagName).toBe('H2')
    );
    expect(screen.queryByRole('heading', { level: 1 })?.textContent).toBe('DeckStreak');
    // one ladder per course, in the server's order, each under the screen's own heading
    const courses = screen
      .getAllByRole('heading', { level: 3 })
      .map((heading) => heading.textContent?.replace(/\s+/g, ' ').trim());
    expect(courses).toEqual(['🇫🇷 French', '🇪🇸 Spanish']);
    expect(screen.getAllByRole('list', { name: /band by band/ })).toHaveLength(2);
    expect(screen.queryByText(/has no course yet/)).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('link', { name: 'Back to Today' })?.getAttribute('href')).toBe('/');
    expect(server.fetch.mock.calls.map(([input]) => String(input))).toContain('/api/progress');
  });

  it('says there is no course yet when the server answers none', async () => {
    server.state.body = { courses: [] };

    render(Progress);

    await vi.waitFor(() => expect(screen.queryByText(/has no course yet/)?.tagName).toBe('P'));
    expect(screen.queryByRole('heading', { name: 'Road to C2' })?.tagName).toBe('H2');
    expect(screen.queryAllByRole('heading', { level: 3 })).toEqual([]);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('says the server could not be reached when the progress cannot be read', async () => {
    server.state.status = 503;

    render(Progress);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('could not reach its server')
    );
    expect(screen.queryByRole('alert')?.textContent).not.toContain('Reopen');
    expect(screen.queryByRole('heading', { name: 'Road to C2' })).toBeNull();
    expect(screen.queryByRole('link', { name: 'Back to Today' })?.getAttribute('href')).toBe('/');
  });

  it('asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    server.state.status = 401;
    server.state.session = 401;

    render(Progress);

    await vi.waitFor(() =>
      expect(screen.queryByRole('alert')?.textContent ?? '').toContain('Reopen DeckStreak from Telegram')
    );
    expect(screen.queryByText(/could not reach/)).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Road to C2' })).toBeNull();
  });
});
