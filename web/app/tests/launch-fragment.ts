// A Mini App launch, as the browser suites make one (SPEC-400; ADR-414). Telegram opens the Mini
// App with its launch parameters in the URL fragment, and the page loads Telegram's script only
// then. This module is not a spec: the suites import the launch fragment, the stand-in served at
// the script's URL, and the request counts from it.
import type { Page } from '@playwright/test';

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
