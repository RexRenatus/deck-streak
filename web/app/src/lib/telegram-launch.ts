/**
 * The launch gate (SPEC-400, SPEC-403; ADR-414, ADR-417). The app's start hook calls `admitLaunch`
 * before the router's first navigation, while the fragment Telegram opened the page with is still
 * whole. The fragment's launch data is sent to `POST /api/launch` first, and Telegram's script is
 * added only when the server accepts it with a 204. A launch the server refuses or does not answer,
 * and a page opened with no launch, add a second policy that refuses Telegram's origin for the
 * page's life, so no script element naming it is ever fetched.
 */

/** The fragment key Telegram opens a Mini App with, whose value is the signed launch data. */
const LAUNCH_DATA = 'tgWebAppData';

/**
 * The tab's accepted mark, so a reload after the router has dropped the fragment still loads the
 * script with no second request. It is written only after the server's 204, and it holds no launch
 * data. The mark SPEC-400 wrote on presence alone (`deck-streak:launched`) is never read.
 */
const MARK = 'deck-streak:launch-accepted';

const SCRIPT = 'https://telegram.org/js/telegram-web-app.js';

/** The server's validation of a launch (SPEC-403 R8): 204 accepts, 401 and 403 refuse. */
const LAUNCH_PATH = '/api/launch';

/** How long the server may take to answer, as long as its own request timeout (SPEC-403 R4), in milliseconds. */
const DEADLINE = 10_000;

/** `svelte.config.js`'s `script-src` without Telegram's origin (SPEC-400 R4; its unit test holds them equal). */
const NARROWED = "script-src 'self' 'wasm-unsafe-eval'";

/** How the gate asks the server about a launch: the page's own `fetch`, or a stand-in for it in a test. */
export type Send = (input: string, init: RequestInit) => Promise<Response>;

/**
 * The launch data in `hash`: the fragment's `tgWebAppData` value, decoded once, which is the
 * decoding Telegram's script applies. An absent or empty value is none, and so is any other launch
 * parameter on its own (SPEC-403 R1).
 */
export function launchData(hash: string): string | null {
  const value = new URLSearchParams(hash.slice(1)).get(LAUNCH_DATA);
  return value === '' ? null : value;
}

/** Whether `storage` holds the tab's accepted mark; a storage that refuses its reads, or none, holds none. */
export function wasAccepted(storage: Pick<Storage, 'getItem'> | undefined): boolean {
  try {
    return storage!.getItem(MARK) !== null;
  } catch {
    // a storage that refuses its reads, or none at all, is no acceptance
    return false;
  }
}

/**
 * Whether the server accepts `data` as a launch (SPEC-403 R2 to R5). The answer and a deadline race
 * in one `Promise.race`, so one outcome settles and a later answer changes nothing; the deadline
 * aborts the request. Only a 204 accepts. A 401 or a 403 refuses the launch, and any other status,
 * a failed request or no answer by the deadline leaves it unanswered; the page treats all three
 * as a launch that is not accepted.
 */
export async function accepts(data: string, send: Send): Promise<boolean> {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const deadline = new Promise<boolean>((resolve) => {
    timer = setTimeout(() => {
      controller.abort();
      resolve(false);
    }, DEADLINE);
  });
  const answer = send(LAUNCH_PATH, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ init_data: data }),
    signal: controller.signal
  }).then(
    (response) => response.status === 204,
    () => false
  );
  try {
    return await Promise.race([answer, deadline]);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Admits the launch into `host` before the router's first navigation. With launch data in the
 * fragment it asks the server first (`send` defaults to the page's own `fetch`); with none, the
 * tab's accepted mark stands for an earlier acceptance. When the launch is accepted it marks the
 * tab and adds Telegram's script to the head, sending no referrer to its origin, and settles once
 * the script has loaded or failed. Otherwise it clears the mark and adds the narrowed policy, and
 * settles at once (SPEC-403 R2 to R6).
 */
export async function admitLaunch(host: Window, send: Send = (input, init) => fetch(input, init)): Promise<void> {
  const { document } = host;
  const data = launchData(host.location.hash);
  // the storage is reached inside the read, so a page that may not use it is judged by its fragment
  const accepted = data === null ? wasAccepted({ getItem: (key) => host.sessionStorage.getItem(key) }) : await accepts(data, send);
  if (accepted) {
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
  try {
    host.sessionStorage.removeItem(MARK);
  } catch {
    // a storage that refuses its writes holds no mark this page could clear
  }
  const policy = document.createElement('meta');
  policy.setAttribute('http-equiv', 'Content-Security-Policy');
  policy.setAttribute('content', NARROWED);
  document.head.append(policy);
}
