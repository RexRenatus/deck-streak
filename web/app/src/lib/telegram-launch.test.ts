/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import config from '../../svelte.config.js';
import { policyHeader } from '../../policy-header';
import { accepts, admitLaunch, launchData, wasAccepted, type Send } from './telegram-launch';

// SPEC-400 R2, R3, R4 as SPEC-403 R1 to R7 restate them (ADR-414 D2, D3; ADR-417 D2). The app's
// start hook reads the launch data from the fragment's `tgWebAppData` value, or the tab's accepted
// mark; it adds Telegram's script and waits for it only when the server answers 204 to the launch
// data, and otherwise adds a policy that refuses Telegram's origin. jsdom fetches no external
// script, so each test dispatches the script's `load` or `error` itself, and the answer to the
// validation is a fake `send` that returns a status alone.
const SCRIPT = 'https://telegram.org/js/telegram-web-app.js';
const MARK = 'deck-streak:launch-accepted';
const OLD_MARK = 'deck-streak:launched';
const DEADLINE = 10_000;

/** A storage holding `items`, read by key the way the session storage is. */
function storage(items: Readonly<Record<string, string>>): Pick<Storage, 'getItem'> {
  return { getItem: (key) => items[key] ?? null };
}

/** A storage that refuses every read, as a browser's storage does when the page may not use it. */
const REFUSING: Pick<Storage, 'getItem'> = {
  getItem: () => {
    throw new DOMException('the storage refused', 'SecurityError');
  }
};

/** The script elements in the head. */
function scripts(): HTMLScriptElement[] {
  return [...document.head.querySelectorAll('script')];
}

/** The content of each policy meta element in the head. */
function policies(): (string | null)[] {
  return [...document.head.querySelectorAll('meta[http-equiv="Content-Security-Policy"]')].map((meta) =>
    meta.getAttribute('content')
  );
}

/** Whether `promise` has settled, raced against a sentinel that resolves after a macrotask. */
function state(promise: Promise<void>): Promise<'settled' | 'pending'> {
  return Promise.race([
    promise.then(() => 'settled' as const),
    new Promise<'pending'>((resolve) => setTimeout(() => resolve('pending'), 0))
  ]);
}

/** A `send` that answers every launch with `status`, and keeps each call it was given. */
function answering(status: number): ReturnType<typeof vi.fn<Send>> {
  return vi.fn<Send>(() => Promise.resolve({ status } as Response));
}

/** A `send` that stays pending until `answer` is called, and keeps each call it was given. */
function pending(): { send: ReturnType<typeof vi.fn<Send>>; answer: (status: number) => void } {
  let answer: (status: number) => void = () => {};
  const send = vi.fn<Send>(
    () =>
      new Promise<Response>((resolve) => {
        answer = (status) => resolve({ status } as Response);
      })
  );
  return { send, answer: (status) => answer(status) };
}

/** Whether `promise` has settled once the fake clock has run the timers and microtasks due now. */
async function settledNow(promise: Promise<void>): Promise<boolean> {
  let settled = false;
  void promise.then(() => {
    settled = true;
  });
  await vi.advanceTimersByTimeAsync(0);
  return settled;
}

afterEach(() => {
  vi.useRealTimers();
  for (const element of document.head.querySelectorAll('script, meta[http-equiv]')) element.remove();
  sessionStorage.clear();
  history.replaceState(null, '', location.pathname);
});

describe('the launch gate', () => {
  it("a launch is read from the fragment's launch parameters and from the tab's mark", () => {
    const none = storage({});

    // the launch data is the fragment's data value; no other launch parameter makes one
    expect(launchData('#tgWebAppData=x')).toBe('x');
    for (const hash of ['#tgWebAppVersion=x', '#tgWebAppPlatform=x', '#tgWebAppData']) {
      expect(launchData(hash), hash).toBeNull();
    }
    // nothing else in the fragment is a launch: a value is not a key, and a longer key is another key
    for (const hash of ['', '#other=1', '#start=tgWebAppData', '#tgWebAppDataX=1']) {
      expect(launchData(hash), hash).toBeNull();
    }
    // the tab's accepted mark is a launch with no fragment
    expect(wasAccepted(storage({ [MARK]: '1' }))).toBe(true);
    expect(wasAccepted(none)).toBe(false);
    // a storage that refuses, or none at all, is no acceptance
    expect(wasAccepted(REFUSING)).toBe(false);
    expect(wasAccepted(undefined)).toBe(false);
  });

  it("the launch data is the fragment's tgWebAppData value, decoded once", () => {
    expect(launchData('#tgWebAppData=a%3D1%26b%3D2')).toBe('a=1&b=2');
    // `+` reads as a space, and an encoded plus as a plus
    expect(launchData('#tgWebAppData=a+b')).toBe('a b');
    expect(launchData('#tgWebAppData=a%2Bb')).toBe('a+b');
    // the value is read wherever it sits among the keys
    expect(launchData('#tgWebAppPlatform=unknown&tgWebAppData=x')).toBe('x');
    // an absent or empty value is none, and so is another launch parameter alone
    for (const hash of [
      '#tgWebAppData',
      '#tgWebAppData=',
      '#tgWebAppVersion=x',
      '#tgWebAppPlatform=x',
      '',
      '#start=tgWebAppData',
      '#tgWebAppDataX=1'
    ]) {
      expect(launchData(hash), hash).toBeNull();
    }
  });

  it('a launch is sent for validation before any script is added', async () => {
    vi.useFakeTimers();
    history.replaceState(null, '', '#tgWebAppData=a%3D1%26b%3D2');
    const { send, answer } = pending();

    const launched = admitLaunch(window, send);

    expect(scripts()).toEqual([]);
    expect(policies()).toEqual([]);
    expect(send).toHaveBeenCalledTimes(1);
    const [input, init] = send.mock.calls[0]!;
    expect(input).toBe('/api/launch');
    expect(init.method).toBe('POST');
    expect(init.credentials).toBe('same-origin');
    expect(init.headers).toEqual({ 'content-type': 'application/json' });
    expect(init.body).toBe('{"init_data":"a=1&b=2"}');
    expect(init.signal).toBeInstanceOf(AbortSignal);
    expect(await settledNow(launched)).toBe(false);
    expect(scripts()).toEqual([]);
    expect(policies()).toEqual([]);

    answer(401);
    expect(await settledNow(launched)).toBe(true);
  });

  it('only a 204 accepts the launch, and then the script is added once', async () => {
    vi.useFakeTimers();
    history.replaceState(null, '', '#tgWebAppData=x');

    // a 200 is the fallback document's status, so it accepts nothing
    const fallback = admitLaunch(window, answering(200));
    expect(await settledNow(fallback)).toBe(true);
    expect(scripts()).toEqual([]);
    expect(policies()).toHaveLength(1);
    expect(sessionStorage.getItem(MARK)).toBeNull();
    for (const element of document.head.querySelectorAll('meta[http-equiv]')) element.remove();

    // a 204 accepts: the mark is written, the script follows once, and the start waits on it; the
    // deadline's timer is cleared, so it never aborts an answered request
    const send = answering(204);
    const launched = admitLaunch(window, send);
    expect(await settledNow(launched)).toBe(false);
    const added = scripts();
    expect(vi.getTimerCount()).toBe(0);
    await vi.advanceTimersByTimeAsync(DEADLINE);
    expect(send.mock.calls[0]![1].signal?.aborted).toBe(false);

    expect(send).toHaveBeenCalledTimes(1);
    expect(added.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    expect(added[0]?.getAttribute('referrerpolicy')).toBe('same-origin');
    expect(sessionStorage.getItem(MARK)).toBe('1');
    expect(policies()).toEqual([]);
    added[0]?.dispatchEvent(new Event('load'));
    expect(await settledNow(launched)).toBe(true);

    // a script that fails to load settles the start as a loaded one does
    for (const element of scripts()) element.remove();
    const failing = admitLaunch(window, answering(204));
    expect(await settledNow(failing)).toBe(false);
    scripts()[0]?.dispatchEvent(new Event('error'));
    expect(await settledNow(failing)).toBe(true);
    expect(scripts()).toHaveLength(1);
  });

  it("a refused launch adds no script, narrows the policy and clears the tab's mark", async () => {
    vi.useFakeTimers();
    for (const status of [401, 403]) {
      for (const element of document.head.querySelectorAll('meta[http-equiv]')) element.remove();
      history.replaceState(null, '', '#tgWebAppData=x');
      sessionStorage.setItem(MARK, '1');

      const launched = admitLaunch(window, answering(status));

      expect(await settledNow(launched), String(status)).toBe(true);
      expect(scripts(), String(status)).toEqual([]);
      expect(policies(), String(status)).toHaveLength(1);
      expect(sessionStorage.getItem(MARK), String(status)).toBeNull();
    }
  });

  it('an unanswered launch adds no script: an error answer, a failed request or the deadline', async () => {
    vi.useFakeTimers();
    history.replaceState(null, '', '#tgWebAppData=x');
    const unanswered: [string, Send][] = [
      ['429', answering(429)],
      ['500', answering(500)],
      ['a failed request', vi.fn<Send>(() => Promise.reject(new TypeError('the request failed')))]
    ];
    for (const [what, send] of unanswered) {
      for (const element of document.head.querySelectorAll('meta[http-equiv]')) element.remove();
      sessionStorage.setItem(MARK, '1');

      const launched = admitLaunch(window, send);

      expect(await settledNow(launched), what).toBe(true);
      expect(scripts(), what).toEqual([]);
      expect(policies(), what).toHaveLength(1);
      expect(sessionStorage.getItem(MARK), what).toBeNull();
    }

    // no answer at all: the start waits to the deadline, which aborts the request
    for (const element of document.head.querySelectorAll('meta[http-equiv]')) element.remove();
    const { send } = pending();
    const waiting = admitLaunch(window, send);
    const { signal } = send.mock.calls[0]![1];

    await vi.advanceTimersByTimeAsync(DEADLINE - 1);
    expect(await settledNow(waiting)).toBe(false);
    expect(signal?.aborted).toBe(false);
    expect(policies()).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(await settledNow(waiting)).toBe(true);
    expect(signal?.aborted).toBe(true);
    expect(scripts()).toEqual([]);
    expect(policies()).toHaveLength(1);
  });

  it('the server accepts a launch only with a 204, and says so with a boolean', async () => {
    vi.useFakeTimers();
    expect(await accepts('x', answering(204))).toBe(true);
    for (const status of [200, 401, 403, 429, 500]) {
      expect(await accepts('x', answering(status)), String(status)).toBe(false);
    }
    expect(await accepts('x', vi.fn<Send>(() => Promise.reject(new TypeError('the request failed'))))).toBe(false);
    const { send } = pending();
    const waiting = accepts('x', send);
    await vi.advanceTimersByTimeAsync(DEADLINE);
    expect(await waiting).toBe(false);
  });

  it('an acceptance after the deadline adds no script', async () => {
    vi.useFakeTimers();
    history.replaceState(null, '', '#tgWebAppData=x');
    const { send, answer } = pending();

    const launched = admitLaunch(window, send);
    await vi.advanceTimersByTimeAsync(DEADLINE);
    expect(await settledNow(launched)).toBe(true);
    expect(policies()).toHaveLength(1);

    answer(204);
    await vi.advanceTimersByTimeAsync(0);

    expect(scripts()).toEqual([]);
    expect(policies()).toHaveLength(1);
    expect(sessionStorage.getItem(MARK)).toBeNull();
  });

  it('a reload with the accepted mark loads the script with no request, and the earlier mark is not an acceptance', async () => {
    vi.useFakeTimers();

    // the accepted mark, no fragment: the script is added and nothing is sent
    sessionStorage.setItem(MARK, '1');
    const send = answering(204);
    const reloaded = admitLaunch(window, send);
    expect(await settledNow(reloaded)).toBe(false);

    expect(send).not.toHaveBeenCalled();
    expect(scripts().map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    expect(scripts()[0]?.getAttribute('referrerpolicy')).toBe('same-origin');
    expect(policies()).toEqual([]);
    scripts()[0]?.dispatchEvent(new Event('load'));
    expect(await settledNow(reloaded)).toBe(true);

    // the mark an earlier start wrote on presence alone is no acceptance
    for (const element of scripts()) element.remove();
    sessionStorage.clear();
    sessionStorage.setItem(OLD_MARK, '1');
    const earlier = admitLaunch(window, send);
    expect(await settledNow(earlier)).toBe(true);

    expect(send).not.toHaveBeenCalled();
    expect(scripts()).toEqual([]);
    expect(policies()).toHaveLength(1);
  });

  it("Telegram's script is added to the head with no referrer and awaited until it loads or fails", async () => {
    // a launch from the fragment, accepted: the script is added once, the mark is written, and no policy
    history.replaceState(null, '', '#tgWebAppData=x');
    const launched = admitLaunch(window, answering(204));
    await state(launched);
    const added = scripts();

    expect(added.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    expect(added[0]?.getAttribute('referrerpolicy')).toBe('same-origin');
    expect(sessionStorage.getItem(MARK)).toBe('1');
    expect(policies()).toEqual([]);
    expect(await state(launched)).toBe('pending');
    added[0]?.dispatchEvent(new Event('load'));
    expect(await state(launched)).toBe('settled');

    // a reload, the fragment gone and the mark kept: the script is added again with no request, and
    // a failed load settles the start as a loaded one does
    for (const element of scripts()) element.remove();
    history.replaceState(null, '', location.pathname);
    const send = answering(204);
    const reloaded = admitLaunch(window, send);
    const again = scripts();

    expect(send).not.toHaveBeenCalled();
    expect(again.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    expect(policies()).toEqual([]);
    expect(await state(reloaded)).toBe('pending');
    again[0]?.dispatchEvent(new Event('error'));
    expect(await state(reloaded)).toBe('settled');
  });

  it("outside a launch the page adds a policy that admits its own scripts and not Telegram's", async () => {
    const directives = config.kit?.csp?.directives?.['script-src'] ?? [];
    const oracle = policyHeader({ 'script-src': directives.filter((source) => source !== 'https://telegram.org') });
    const send = answering(204);

    const admitted = admitLaunch(window, send);

    expect(await state(admitted)).toBe('settled');
    expect(send).not.toHaveBeenCalled();
    expect(scripts()).toEqual([]);
    expect(sessionStorage.getItem(MARK)).toBeNull();
    expect(policies()).toEqual([oracle]);
  });
});
