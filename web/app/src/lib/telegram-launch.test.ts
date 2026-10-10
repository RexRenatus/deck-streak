/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it } from 'vitest';
import config from '../../svelte.config.js';
import { policyHeader } from '../../policy-header';
import { admitLaunch, isLaunch } from './telegram-launch';

// SPEC-400 R2, R3, R4; ADR-414 D2, D3. The app's start hook reads a launch from the fragment's
// launch parameters, by presence, or from the tab's own mark; on a launch it adds Telegram's script
// and waits for it, and outside one it adds a policy that refuses Telegram's origin. jsdom fetches
// no external script, so each test dispatches the script's `load` or `error` itself.
const SCRIPT = 'https://telegram.org/js/telegram-web-app.js';
const MARK = 'deck-streak:launched';

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

afterEach(() => {
  for (const element of document.head.querySelectorAll('script, meta[http-equiv]')) element.remove();
  sessionStorage.clear();
  history.replaceState(null, '', location.pathname);
});

describe('the launch gate', () => {
  it("a launch is read from the fragment's launch parameters and from the tab's mark", () => {
    const none = storage({});

    // each launch parameter, by its presence alone
    for (const hash of ['#tgWebAppData=x', '#tgWebAppVersion=x', '#tgWebAppPlatform=x', '#tgWebAppData']) {
      expect(isLaunch(hash, none), hash).toBe(true);
    }
    // nothing else in the fragment is a launch: a value is not a key, and a longer key is another key
    for (const hash of ['', '#other=1', '#start=tgWebAppData', '#tgWebAppDataX=1']) {
      expect(isLaunch(hash, none), hash).toBe(false);
    }
    // the tab's mark is a launch with no fragment
    expect(isLaunch('', storage({ [MARK]: '1' }))).toBe(true);
    expect(isLaunch('', none)).toBe(false);
    // a storage that refuses, or none at all, leaves the fragment as the only test
    expect(isLaunch('#tgWebAppData=x', REFUSING)).toBe(true);
    expect(isLaunch('', REFUSING)).toBe(false);
    expect(isLaunch('#tgWebAppData=x', undefined)).toBe(true);
    expect(isLaunch('', undefined)).toBe(false);
  });

  it("Telegram's script is added to the head with no referrer and awaited until it loads or fails", async () => {
    // a launch from the fragment: the script is added once, the mark is written, and no policy
    history.replaceState(null, '', '#tgWebAppData=x');
    const launched = admitLaunch(window);
    const added = scripts();

    expect(added.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    expect(added[0]?.getAttribute('referrerpolicy')).toBe('same-origin');
    expect(sessionStorage.getItem(MARK)).toBe('1');
    expect(policies()).toEqual([]);
    expect(await state(launched)).toBe('pending');
    added[0]?.dispatchEvent(new Event('load'));
    expect(await state(launched)).toBe('settled');

    // a reload, the fragment gone and the mark kept: the script is added again, and a failed load
    // settles the start as a loaded one does
    for (const element of scripts()) element.remove();
    history.replaceState(null, '', location.pathname);
    const reloaded = admitLaunch(window);
    const again = scripts();

    expect(again.map((script) => script.getAttribute('src'))).toEqual([SCRIPT]);
    expect(policies()).toEqual([]);
    expect(await state(reloaded)).toBe('pending');
    again[0]?.dispatchEvent(new Event('error'));
    expect(await state(reloaded)).toBe('settled');
  });

  it("outside a launch the page adds a policy that admits its own scripts and not Telegram's", async () => {
    const directives = config.kit?.csp?.directives?.['script-src'] ?? [];
    const oracle = policyHeader({ 'script-src': directives.filter((source) => source !== 'https://telegram.org') });

    const admitted = admitLaunch(window);

    expect(await state(admitted)).toBe('settled');
    expect(scripts()).toEqual([]);
    expect(sessionStorage.getItem(MARK)).toBeNull();
    expect(policies()).toEqual([oracle]);
  });
});
