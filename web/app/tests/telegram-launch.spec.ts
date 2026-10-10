// Telegram's script loads only on a Mini App launch (SPEC-400 A1 to A4; ADR-414). Outside a launch
// no route asks for the script or runs it, and the page's policy refuses a script from Telegram's
// origin. A launch loads the script before the router's first navigation, while the fragment that
// carries the launch is whole, and a reload after the launch's redirect loads it again.
//
// Run: pnpm exec playwright test tests/telegram-launch.spec.ts   (CI runs every spec in tests/)
import { expect, test, type Page } from '@playwright/test';
import { ROUTES } from '../src/lib/routes';
import { STAND_IN, TELEGRAM_SDK, launchFragment, ownRequests, telegramRequestList, telegramRequests } from './launch-fragment';

/** What the stand-in left in the page: its run count and the fragment it saw. */
interface StandIn {
  readonly __standInRan?: number;
  readonly __standInHash?: string;
}

/** Serves the stand-in at the script's URL, so nothing reaches Telegram. */
async function standIn(page: Page): Promise<void> {
  await page.route(`${TELEGRAM_SDK}*`, (route) =>
    route.fulfill({ contentType: 'text/javascript', body: STAND_IN })
  );
}

/** Answers the session handshake with a refusal, and keeps each request body it was sent. */
async function sessions(page: Page): Promise<string[]> {
  const bodies: string[] = [];
  await page.route('**/api/session', (route) => {
    bodies.push(route.request().postData() ?? '');
    return route.fulfill({ status: 401, json: { error: 'unauthorized' } });
  });
  return bodies;
}

test('outside Telegram no route asks for Telegram\'s script or runs it', async ({ page }) => {
  // every route is a whole document load, each waited on until the network is quiet
  test.setTimeout(ROUTES.length * 10_000);
  await standIn(page);
  const telegram = telegramRequests(page);
  const own = ownRequests(page);

  for (const path of ROUTES) {
    const before = own();
    await page.goto(path, { waitUntil: 'networkidle' });

    expect(telegram(), `${path}: requests to Telegram's origin`).toBe(0);
    expect(await page.evaluate(() => (window as unknown as StandIn).__standInRan), `${path}: the script ran`).toBeUndefined();
    expect(
      await page.evaluate(() => document.querySelectorAll('script[src^="https://telegram.org"]').length),
      `${path}: script elements naming Telegram's origin`
    ).toBe(0);
    // the positive control: the route did load, from its own origin
    expect(own() - before, `${path}: requests to the page's own origin`).toBeGreaterThan(0);
  }
});

test('outside Telegram the page\'s policy refuses a script from Telegram\'s origin', async ({ page }) => {
  await standIn(page);
  // registered after the stand-in, so it runs first: it counts each request the stand-in's route
  // receives, then falls back to the stand-in
  let routed = 0;
  await page.route(`${TELEGRAM_SDK}*`, (route) => {
    routed += 1;
    return route.fallback();
  });
  const telegram = telegramRequests(page);
  const requests = telegramRequestList(page);
  await page.goto('/', { waitUntil: 'networkidle' });
  const duringLoad = telegram();

  const outcome = await page.evaluate(
    (src) =>
      new Promise<{ violated: boolean; effectiveDirective: string; blockedURI: string }>((resolve) => {
        const bound = setTimeout(() => resolve({ violated: false, effectiveDirective: '', blockedURI: '' }), 5000);
        document.addEventListener('securitypolicyviolation', (event) => {
          clearTimeout(bound);
          resolve({ violated: true, effectiveDirective: event.effectiveDirective, blockedURI: event.blockedURI });
        });
        const script = document.createElement('script');
        script.src = src;
        document.head.append(script);
      }),
    TELEGRAM_SDK
  );

  expect(outcome.violated, 'the policy refused the script').toBe(true);
  expect(outcome.effectiveDirective).toBe('script-src-elem');
  expect(outcome.blockedURI.startsWith('https://telegram.org'), `blocked ${outcome.blockedURI}`).toBe(true);
  // the browser reports the refused fetch as one request that failed with no response: nothing
  // else asked Telegram's origin, and the stand-in was never served
  expect(await requests(), 'requests to Telegram\'s origin').toEqual([{ url: TELEGRAM_SDK, failure: 'csp', response: null }]);
  expect(routed, 'requests the stand-in\'s route received').toBe(0);
  expect(duringLoad, 'requests to Telegram\'s origin during the page load').toBe(0);
});

test('a launch loads Telegram\'s script before the first navigation and its launch data reaches the session unchanged', async ({
  page
}) => {
  // a launch whose startapp token routes the first navigation: the layout's redirect drops the fragment
  await standIn(page);
  await sessions(page);
  const telegram = telegramRequests(page);
  await page.goto('/' + launchFragment('auth_date=1&start_param=about&hash=synthetic'));

  await expect.poll(() => new URL(page.url()).pathname).toBe('/about');
  expect(new URL(page.url()).hash).toBe('');
  // the script ran before that redirect, while the fragment was whole
  expect(await page.evaluate(() => (window as unknown as StandIn).__standInHash)).toContain('tgWebAppData');
  expect(telegram()).toBe(1);

  // a launch on Today: its first call opens the session with the launch data, byte for byte
  const second = await page.context().newPage();
  await standIn(second);
  const bodies = await sessions(second);
  const opened = second.waitForRequest('**/api/session');
  await second.goto('/' + launchFragment('auth_date=1&hash=synthetic'));
  await opened;
  await second.waitForLoadState('networkidle');

  expect(bodies.map((body) => JSON.parse(body))).toEqual([{ init_data: 'auth_date=1&hash=synthetic' }]);
});

test('a reload after the launch\'s first navigation loads Telegram\'s script again', async ({ page }) => {
  await standIn(page);
  await sessions(page);
  const telegram = telegramRequests(page);
  await page.goto('/' + launchFragment('auth_date=1&start_param=about&hash=synthetic'));
  await expect.poll(() => new URL(page.url()).pathname).toBe('/about');
  expect(new URL(page.url()).hash).toBe('');

  await page.reload({ waitUntil: 'networkidle' });

  expect(telegram()).toBe(2);
  expect(await page.evaluate(() => (window as unknown as StandIn).__standInRan)).toBe(1);
});
