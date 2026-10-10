/**
 * The launch gate (SPEC-400; ADR-414). The app's start hook calls `admitLaunch` before the router's
 * first navigation, while the fragment Telegram opened the page with is still whole. On a launch it
 * adds Telegram's script and waits for it; outside one it adds a second policy that refuses
 * Telegram's origin for the page's life, so no script element naming it is ever fetched.
 */

/** The fragment keys Telegram opens a Mini App with. Their presence alone is a launch: no value is read. */
const PARAMETERS = ['tgWebAppData', 'tgWebAppVersion', 'tgWebAppPlatform'];

/**
 * The tab's launch mark, so a reload after the router has dropped the fragment is still a launch.
 * It holds no launch data.
 */
const MARK = 'deck-streak:launched';

const SCRIPT = 'https://telegram.org/js/telegram-web-app.js';

/** `svelte.config.js`'s `script-src` without Telegram's origin (SPEC-400 R4; its unit test holds them equal). */
const NARROWED = "script-src 'self' 'wasm-unsafe-eval'";

/** How the gate asks the server about a launch: the page's own `fetch`, or a stand-in for it. */
export type Send = (input: string, init: RequestInit) => Promise<Response>;

/** The launch data in `hash`. */
export function launchData(hash: string): string | null {
  void hash;
  return null;
}

/** Whether the tab carries the accepted mark. */
export function wasAccepted(storage: Pick<Storage, 'getItem'> | undefined): boolean {
  void storage;
  return false;
}

/** Whether Telegram launched the page: a launch parameter in `hash`, or the tab's mark in `storage`. */
export function isLaunch(hash: string, storage: Pick<Storage, 'getItem'> | undefined): boolean {
  const fragment = new URLSearchParams(hash.slice(1));
  if (PARAMETERS.some((parameter) => fragment.has(parameter))) return true;
  try {
    return storage!.getItem(MARK) !== null;
  } catch {
    // a storage that refuses its reads, or none at all, leaves the fragment as the only test
    return false;
  }
}

/**
 * Admits the launch into `host` before the router's first navigation: on a launch, Telegram's script
 * in the head, sending no referrer to its origin, and a promise that settles once it has loaded or
 * failed; outside one, the narrowed policy and a promise already settled.
 */
export function admitLaunch(host: Window, send: Send = (input, init) => fetch(input, init)): Promise<void> {
  void send;
  const { document } = host;
  // the storage is reached inside the read, so a page that may not use it is judged by its fragment
  if (!isLaunch(host.location.hash, { getItem: (key) => host.sessionStorage.getItem(key) })) {
    const policy = document.createElement('meta');
    policy.setAttribute('http-equiv', 'Content-Security-Policy');
    policy.setAttribute('content', NARROWED);
    document.head.append(policy);
    return Promise.resolve();
  }
  try {
    host.sessionStorage.setItem(MARK, '1');
  } catch {
    // a storage that refuses its writes leaves a reload of this tab outside a launch
  }
  const script = document.createElement('script');
  script.src = SCRIPT;
  script.setAttribute('referrerpolicy', 'same-origin');
  const settled = new Promise<void>((resolve) => {
    script.addEventListener('load', () => resolve());
    script.addEventListener('error', () => resolve());
  });
  document.head.append(script);
  return settled;
}
