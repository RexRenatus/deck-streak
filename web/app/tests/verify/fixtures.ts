// The phone-size verification's launch (SPEC-639 R2, R3, R4; ADR-662).
//
// Each screen opens as a Telegram launch whose stand-in for Telegram's script sets no theme: the
// client paints nothing, so the app paints its own Tide theme in the project's colour scheme. The
// clock is fixed, every call answers from the state table, and a project's text scale sets the
// root text size before the first paint, as a phone's larger text setting does.
import { expect, test as base, type Page } from '@playwright/test';
import { TELEGRAM_SDK, answerLaunches, launchFragment } from '../launch-fragment';
import type { Engine, State } from './states';

/** The colour schemes the phone projects emulate. */
export type Scheme = 'light' | 'dark';

/** The phone projects' own option: the root text size, as a multiple of the default. */
export interface VerifyOptions {
  readonly textScale: number;
}

/** The launch data a launch carries, synthetic. */
export const LAUNCH_DATA = 'auth_date=1&hash=synthetic';

/** The fixed time every page sees. */
export const STUDY_TIME = new Date(Date.UTC(2001, 1, 3, 10));

const ENGINE_SCRIPT = '**/engine/deck_streak_web_engine.js';

/**
 * A stand-in for Telegram's script with `themeParams` as the client's theme. The verification
 * launches with none, so no --tg-theme-* variable is set.
 */
export function standIn(scheme: Scheme, themeParams: Readonly<Record<string, string>> = {}): string {
  return `(() => {
    const noop = () => {};
    const inset = { top: 0, bottom: 0, left: 0, right: 0 };
    const button = { show: noop, hide: noop, onClick: noop, offClick: noop, setText: noop };
    const themeParams = ${JSON.stringify(themeParams)};
    const style = document.documentElement.style;
    for (const [key, value] of Object.entries(themeParams)) {
      style.setProperty('--tg-theme-' + key.split('_').join('-'), value);
    }
    style.setProperty('--tg-color-scheme', ${JSON.stringify(scheme)});
    window.Telegram = {
      WebApp: {
        initData: ${JSON.stringify(LAUNCH_DATA)},
        version: '9.0',
        platform: 'unknown',
        colorScheme: ${JSON.stringify(scheme)},
        themeParams,
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
}

/** A stand-in for the study engine's bindings: a collection with one card, or with none. */
function engineStandIn(engine: Exclude<Engine, 'down'>): string {
  const notes = engine === 'card' ? 1 : 0;
  const current = engine === 'card' ? 'JSON.stringify({ counts: { new: 1, learning: 0, review: 0 }, card })' : "'null'";
  return `
const card = {
  id: '1', ordinal: 0, flag: 0, question: '<p>A card</p>', answer: '<p>Its answer</p>',
  css: '.card { color: #1a1a1a; background: #ffffff; }',
  labels: ['soon', 'later', 'tomorrow', 'next week'], undo: null, late: false
};
const face = (text) => ({ text, css: card.css, autoplay: [], replay: [], omitted: [] });
export default async function load() {}
export async function install_storage() { return 0; }
export function init() {}
export function open() { return JSON.stringify({ existed: true, notes: ${notes} }); }
export function close() {}
export function seed() { return 0; }
export function next_card() { return ${notes}n; }
export function undo() {}
export function undo_offer() { return JSON.stringify({ offer: null, why: 'none' }); }
export function snapshot() { return 'null'; }
export function last_panic() { return undefined; }
export function memory_pages() { return 1; }
export function deck_tree() { return '[]'; }
export function set_current_deck() {}
export function current_card() { return ${current}; }
export function rate() {}
export function bury() {}
export function flag() { return 1; }
export function faces() { return { question: face(card.question), answer: face(card.answer), wanted: [] }; }
`;
}

/** The project's colour scheme as the stand-in names it. */
export function schemeOf(colorScheme: string | null | undefined): Scheme {
  return colorScheme === 'dark' ? 'dark' : 'light';
}

/**
 * Opens `route` in `state` as a launch with `themeParams` as the client's theme (none for the
 * verification), and waits until the screen has settled.
 */
export async function open(
  page: Page,
  route: string,
  state: State,
  scheme: Scheme,
  themeParams: Readonly<Record<string, string>> = {}
): Promise<void> {
  // A call the state does not name is answered as not found, so no state reaches a server.
  await page.route('**/api/**', (intercepted) => intercepted.fulfill({ status: 404 }));
  await page.route(`${TELEGRAM_SDK}*`, (intercepted) =>
    intercepted.fulfill({ contentType: 'text/javascript', body: standIn(scheme, themeParams) })
  );
  await page.addInitScript((fragment) => {
    if (window === window.top && location.hash === '') history.replaceState(history.state, '', fragment);
  }, launchFragment(LAUNCH_DATA));
  await answerLaunches(page, 204);
  await page.route('**/api/session', (intercepted) => intercepted.fulfill({ status: 200 }));
  for (const [glob, reply] of Object.entries(state.calls)) {
    await page.route(glob, (intercepted) => intercepted.fulfill(reply));
  }
  const engine = state.engine;
  if (engine === 'down') {
    await page.context().route(ENGINE_SCRIPT, (intercepted) => intercepted.fulfill({ status: 500 }));
  } else if (engine !== undefined) {
    await page.context().route(ENGINE_SCRIPT, (intercepted) =>
      intercepted.fulfill({ contentType: 'text/javascript', body: engineStandIn(engine) })
    );
  }
  await page.goto(route);
  await expect
    .poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--tg-color-scheme')))
    .toBe(scheme);
  await page.waitForLoadState('networkidle');
  if (state.reveal) {
    await page.getByRole('button', { name: 'Show answer' }).click();
    await page.waitForLoadState('networkidle');
  }
}

/**
 * What keeps the page from being the app's own theme: each --tg-theme-* variable set on the root,
 * and a body background that is not the Tide background of `scheme`. Empty on the app's own theme.
 */
export async function tideFindings(page: Page, scheme: Scheme): Promise<string[]> {
  return page.evaluate((scheme) => {
    const found: string[] = [];
    const root = document.documentElement;
    const computed = getComputedStyle(root);
    const names = new Set([...Array.from(root.style), ...Array.from(computed)]);
    for (const name of [...names].filter((n) => n.startsWith('--tg-theme-')).sort()) {
      found.push(`${name} is set`);
    }
    const tide = computed.getPropertyValue(`--tide-${scheme}-background`).trim();
    if (tide === '') return [...found, `the root has no --tide-${scheme}-background`];
    const probe = document.createElement('div');
    probe.style.backgroundColor = tide;
    document.body.append(probe);
    const wanted = getComputedStyle(probe).backgroundColor;
    probe.remove();
    const painted = getComputedStyle(document.body).backgroundColor;
    if (painted !== wanted) found.push(`the body paints ${painted}, not the Tide ${scheme} background ${wanted}`);
    return found;
  }, scheme);
}

/** The verification's test: the fixed clock and the project's text scale on every page. */
export const test = base.extend<VerifyOptions>({
  textScale: [1, { option: true }],
  page: async ({ page, textScale }, use) => {
    await page.clock.setFixedTime(STUDY_TIME);
    if (textScale !== 1) {
      await page.addInitScript((percent) => {
        const apply = () => document.documentElement.style.setProperty('font-size', percent);
        if (document.documentElement) apply();
        else new MutationObserver((_, watcher) => {
          if (document.documentElement) {
            apply();
            watcher.disconnect();
          }
        }).observe(document, { childList: true });
      }, `${textScale * 100}%`);
    }
    await use(page);
  }
});

export { expect };
