// A Mini App launch, as the browser suites make one (SPEC-400; ADR-414). Telegram opens the Mini
// App with its launch parameters in the URL fragment, and the page loads Telegram's script only
// then. This module is not a spec: the suites import the launch fragment, the stand-in served at
// the script's URL, the request counts and the request list, and the answer to the launch's
// validation (SPEC-403; ADR-417) from it.
import type { Page, Request } from '@playwright/test';

/** Telegram's Mini App script, at the URL the page loads it from on a launch. */
export const TELEGRAM_SDK = 'https://telegram.org/js/telegram-web-app.js';

/** The origin Telegram's script is served from. */
const TELEGRAM_ORIGIN = 'https://telegram.org';

/**
 * The fragment a Telegram client opens the Mini App with: the signed launch data and a platform.
 * No version is written; the page reads a launch by the presence of a launch parameter alone.
 */
export function launchFragment(launchData: string): string {
  return `#tgWebAppData=${encodeURIComponent(launchData)}&tgWebAppPlatform=unknown`;
}

/** Every request `page` issues from now on, by URL, read when the count is asked for. */
function issued(page: Page): string[] {
  const urls: string[] = [];
  page.on('request', (request) => urls.push(request.url()));
  return urls;
}

/** How many requests `page` has issued to Telegram's origin since this call, read on each call. */
export function telegramRequests(page: Page): () => number {
  const urls = issued(page);
  return () => urls.filter((url) => new URL(url).origin === TELEGRAM_ORIGIN).length;
}

/** A request to Telegram's origin as the browser reported it. */
export interface TelegramRequest {
  /** The URL the page asked for. */
  readonly url: string;
  /** The browser's failure text (`csp` for a fetch the page's policy refused), or `null` if it did not fail. */
  readonly failure: string | null;
  /** The response's status, or `null` if no response came back. */
  readonly response: number | null;
}

/**
 * Every request `page` issues to Telegram's origin from now on, kept whole and read when asked for:
 * its URL, its failure and its response. The browser reports a fetch the page's policy refuses as
 * a request that failed with no response, so a count alone cannot tell a refused attempt from an
 * answered one, and this list can.
 */
export function telegramRequestList(page: Page): () => Promise<TelegramRequest[]> {
  const requests: Request[] = [];
  page.on('request', (request) => {
    if (new URL(request.url()).origin === TELEGRAM_ORIGIN) requests.push(request);
  });
  return () =>
    Promise.all(
      requests.map(async (request) => {
        // a request settles with a response or a failure, so its failure is read after that
        const response = await request.response();
        return { url: request.url(), failure: request.failure()?.errorText ?? null, response: response?.status() ?? null };
      })
    );
}

/** How many requests `page` has issued to its own origin since this call, read on each call. */
export function ownRequests(page: Page): () => number {
  const urls = issued(page);
  return () => {
    const own = new URL(page.url()).origin;
    return urls.filter((url) => new URL(url).origin === own).length;
  };
}

/**
 * A stand-in for Telegram's script, served at the script's URL when a launch asks for it. Like the
 * real script, it keeps the fragment's launch data in the tab's session storage, under a key of its
 * own, and reads it back when a reload has no fragment; it sets the Mini App object the wrapper
 * reads, with that launch data as its signed launch data. It also records the fragment it saw when
 * it ran (`__standInHash`) and counts its runs in the document (`__standInRan`), so a suite can
 * tell when it ran and whether it ran at all. Nothing reaches the network.
 */
export const STAND_IN = `(() => {
  window.__standInHash = location.hash;
  window.__standInRan = (window.__standInRan ?? 0) + 1;
  const fromFragment = new URLSearchParams(location.hash.slice(1)).get('tgWebAppData');
  if (fromFragment !== null) sessionStorage.setItem('stand-in:launch', fromFragment);
  const launchData = fromFragment ?? sessionStorage.getItem('stand-in:launch') ?? '';
  const noop = () => {};
  const inset = { top: 0, bottom: 0, left: 0, right: 0 };
  const button = { show: noop, hide: noop, onClick: noop, offClick: noop, setText: noop };
  window.Telegram = {
    WebApp: {
      initData: launchData,
      version: '9.0',
      platform: 'unknown',
      colorScheme: 'light',
      themeParams: {},
      viewportStableHeight: window.innerHeight,
      safeAreaInset: inset,
      contentSafeAreaInset: inset,
      isVersionAtLeast: () => true,
      ready: noop,
      expand: noop,
      openLink: noop,
      onEvent: noop,
      offEvent: noop,
      BackButton: button,
      MainButton: button,
      HapticFeedback: { impactOccurred: noop, notificationOccurred: noop, selectionChanged: noop }
    }
  };
})();`;

/**
 * Answers the page's launch validation, `POST /api/launch` (SPEC-403; ADR-417), and keeps each body
 * it was sent, read when asked for. `answer` is the status to give, or `'abort'` for a request
 * that fails with no response. A 401 and a 403 carry the server's reason alone, a 200 is the
 * fallback document's status (an HTML page, as the static host answers an unknown path), and any
 * other status has no body.
 */
export async function answerLaunches(page: Page, answer: number | 'abort'): Promise<string[]> {
  const bodies: string[] = [];
  await page.route('**/api/launch', (route) => {
    bodies.push(route.request().postData() ?? '');
    if (answer === 'abort') return route.abort();
    if (answer === 401) return route.fulfill({ status: 401, json: { reason: 'init_data_invalid' } });
    if (answer === 403) return route.fulfill({ status: 403, json: { reason: 'not_owner' } });
    if (answer === 200) return route.fulfill({ status: 200, contentType: 'text/html', body: '<!doctype html><title>DeckStreak</title>' });
    return route.fulfill({ status: answer });
  });
  return bodies;
}
