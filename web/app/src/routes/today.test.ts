/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Today from './+page.svelte';

// SPEC-028 R7, R8, A11. Today shows the study day the server answers, which turns over at 04:00
// on the server's calendar, never a date the phone computes. The wrapper and the API client are
// created when their modules load, so Telegram's object and the server are in place before the
// imports run.
const server = vi.hoisted(() => {
  const noop = () => undefined;
  const statuses = { session: 200, me: 200 };
  const fetch = vi.fn(async (input: RequestInfo | URL) => {
    if (String(input) === '/api/session') return new Response(null, { status: statuses.session });
    return statuses.me === 200
      ? Response.json({ study_day: '2001-02-03' })
      : new Response(null, { status: statuses.me });
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
  return { fetch, statuses };
});

describe('Today', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("Today shows the server's study day, not the device date", async () => {
    // The phone's clock reads 01:30 on 4 February. The study day turns over at 04:00, so the
    // server still answers the 3rd.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date(2001, 1, 4, 1, 30));

    render(Today);

    const day = await screen.findByText('Saturday, February 3, 2001');
    expect([day.tagName, day.getAttribute('datetime')]).toEqual(['TIME', '2001-02-03']);
    expect(document.body.textContent).not.toMatch(/February 4|2001-02-04|Sunday/);
    expect(server.fetch.mock.calls.map(([input]) => String(input))).toEqual([
      '/api/session',
      '/api/me'
    ]);
  });

  it('Today asks the owner to reopen DeckStreak when the session cannot be renewed', async () => {
    // The session has ended and the launch data is past its bound: the server refuses both.
    server.statuses.me = 401;
    server.statuses.session = 401;

    render(Today);

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('Reopen DeckStreak from Telegram');
    expect(document.querySelector('time')).toBeNull();
  });
});
