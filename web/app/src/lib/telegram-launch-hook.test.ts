/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { getLocale, getTextDirection } from '$lib/paraglide/runtime.js';
import { init } from '../hooks.client';

// SPEC-400 R2, R3; ADR-414 D2. SvelteKit awaits the client `init` hook before the router's first
// navigation, so the hook is where the launch is admitted while the fragment is whole. jsdom fetches
// no external script, so the test dispatches the script's `load` itself. A launch is validated by the
// page's own `fetch` first (SPEC-403 R2; ADR-417 D2), which the tests stub.
const SCRIPT = 'https://telegram.org/js/telegram-web-app.js';

/** Whether `promise` has settled, raced against a sentinel that resolves after a macrotask. */
function state(promise: Promise<void>): Promise<'settled' | 'pending'> {
  return Promise.race([
    promise.then(() => 'settled' as const),
    new Promise<'pending'>((resolve) => setTimeout(() => resolve('pending'), 0))
  ]);
}

/** The script elements in the head. */
function scripts(): HTMLScriptElement[] {
  return [...document.head.querySelectorAll('script')];
}

/** Clears what a start left: the head's additions, the tab's mark and the fragment. */
function reset(): void {
  for (const element of document.head.querySelectorAll('script, meta[http-equiv]')) element.remove();
  sessionStorage.clear();
  history.replaceState(null, '', location.pathname);
}

afterEach(() => {
  vi.unstubAllGlobals();
  reset();
});

/** Stubs the page's `fetch` to answer `status`, once `answer` is called, and returns the stub. */
function stubFetch(): { fetch: ReturnType<typeof vi.fn>; answer: (status: number) => void } {
  let answer: (status: number) => void = () => {};
  const fetch = vi.fn(
    () =>
      new Promise<Response>((resolve) => {
        answer = (status) => resolve({ status } as Response);
      })
  );
  vi.stubGlobal('fetch', fetch);
  return { fetch, answer: (status) => answer(status) };
}

describe("the app's start", () => {
  it("the app's start validates the launch through the page's fetch before it adds the script", async () => {
    history.replaceState(null, '', '#tgWebAppData=x');
    const { fetch, answer } = stubFetch();

    const launched = Promise.resolve(init());

    expect(scripts()).toEqual([]);
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith('/api/launch', expect.objectContaining({ method: 'POST' }));
    answer(204);
    expect(await state(launched)).toBe('pending');
    const added = scripts();
    expect(added.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    added[0]?.dispatchEvent(new Event('load'));
    expect(await state(launched)).toBe('settled');
  });

  it("the app's start admits the launch before its first navigation", async () => {
    // a launch the server accepts: the page language and direction are set, and the start waits on the script
    document.documentElement.lang = '';
    document.documentElement.dir = '';
    history.replaceState(null, '', '#tgWebAppData=x');
    const { fetch, answer } = stubFetch();
    const launched = Promise.resolve(init());

    expect(document.documentElement.lang).toBe(getLocale());
    expect(document.documentElement.dir).toBe(getTextDirection());
    answer(204);
    expect(await state(launched)).toBe('pending');
    const added = scripts();
    expect(added.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    added[0]?.dispatchEvent(new Event('load'));
    expect(await state(launched)).toBe('settled');

    // outside a launch: the start settles with the narrower policy and no script, and asks nothing
    fetch.mockClear();
    reset();
    const outside = Promise.resolve(init());

    expect(await state(outside)).toBe('settled');
    expect(fetch).not.toHaveBeenCalled();
    expect(scripts()).toEqual([]);
    expect(document.head.querySelectorAll('meta[http-equiv="Content-Security-Policy"]')).toHaveLength(1);
  });
});
