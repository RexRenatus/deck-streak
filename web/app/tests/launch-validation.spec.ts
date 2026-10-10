// The page asks the server to validate a launch before it loads Telegram's script (SPEC-403 A1 to
// A5; ADR-417). A launch the server refuses, or does not answer with a 204, loads no script and
// adds the narrowed policy, on the load and on its reload. An accepted launch loads the script once,
// after the validation request, and a reload then sends no second validation.
//
// Run: pnpm exec playwright test tests/launch-validation.spec.ts   (CI runs every spec in tests/)
import { expect, test, type Page } from '@playwright/test';
import { ROUTES } from '../src/lib/routes';
import { STAND_IN, TELEGRAM_SDK, answerLaunches, launchFragment, ownRequests, telegramRequests } from './launch-fragment';

/** What the stand-in left in the page: its run count. */
interface StandIn {
  readonly __standInRan?: number;
}

/** The policy the page adds outside a launch (SPEC-400 R4), and the only one carrying it. */
const NARROWED = `meta[http-equiv="Content-Security-Policy"][content="script-src 'self' 'wasm-unsafe-eval'"]`;

/** Serves the stand-in at the script's URL, so nothing reaches Telegram. */
async function standIn(page: Page): Promise<void> {
  await page.route(`${TELEGRAM_SDK}*`, (route) =>
    route.fulfill({ contentType: 'text/javascript', body: STAND_IN })
  );
}

/** Answers the session handshake with a refusal, so a launch that loads the script ends quietly. */
async function sessions(page: Page): Promise<void> {
  await page.route('**/api/session', (route) =>
    route.fulfill({ status: 401, json: { error: 'unauthorized' } })
  );
}

test('a crafted launch the server refuses loads no Telegram script, and neither does its reload', async ({
  page
}) => {
  for (const status of [401, 403]) {
    const fresh = await page.context().newPage();
    await standIn(fresh);
    await sessions(fresh);
    const bodies = await answerLaunches(fresh, status);
    const telegram = telegramRequests(fresh);
    const own = ownRequests(fresh);
    await fresh.goto('/' + launchFragment('auth_date=1&hash=forged'), { waitUntil: 'networkidle' });

    expect(telegram(), `${status}: requests to Telegram's origin`).toBe(0);
    expect(bodies.map((body) => JSON.parse(body)), `${status}: the validations sent`).toEqual([
      { init_data: 'auth_date=1&hash=forged' }
    ]);
    await expect(fresh.locator(NARROWED), `${status}: the narrowed policy`).toHaveCount(1);
    expect(own(), `${status}: requests to the page's own origin`).toBeGreaterThan(0);

    await fresh.reload({ waitUntil: 'networkidle' });

    expect(telegram(), `${status}: requests to Telegram's origin after the reload`).toBe(0);
    expect(bodies, `${status}: the validations sent after the reload`).toHaveLength(2);
    await fresh.close();
  }
});

test('an accepted launch loads Telegram\'s script once, after the server accepted it', async ({ page }) => {
  await standIn(page);
  await sessions(page);
  const bodies = await answerLaunches(page, 204);
  const order: string[] = [];
  page.on('request', (request) => {
    const url = new URL(request.url());
    if (url.pathname === '/api/launch') order.push('launch');
    if (url.origin === new URL(TELEGRAM_SDK).origin) order.push('telegram');
  });
  await page.goto('/' + launchFragment('auth_date=1&hash=synthetic'), { waitUntil: 'networkidle' });

  expect(order, 'the order of the requests').toEqual(['launch', 'telegram']);
  expect(bodies.map((body) => JSON.parse(body))).toEqual([{ init_data: 'auth_date=1&hash=synthetic' }]);
  expect(await page.evaluate(() => (window as unknown as StandIn).__standInRan)).toBe(1);
});

test('a launch whose validation goes unanswered loads no Telegram script', async ({ page }) => {
  for (const answer of ['abort', 429, 503, 200] as const) {
    const fresh = await page.context().newPage();
    await standIn(fresh);
    await sessions(fresh);
    const bodies = await answerLaunches(fresh, answer);
    const telegram = telegramRequests(fresh);
    const own = ownRequests(fresh);
    await fresh.goto('/' + launchFragment('auth_date=1&hash=synthetic'), { waitUntil: 'networkidle' });

    expect(telegram(), `${answer}: requests to Telegram's origin`).toBe(0);
    await expect(fresh.locator(NARROWED), `${answer}: the narrowed policy`).toHaveCount(1);
    expect(own(), `${answer}: requests to the page's own origin`).toBeGreaterThan(0);
    expect(bodies, `${answer}: the validations sent`).toHaveLength(1);
    await fresh.close();
  }
});

test('a reload after an accepted launch loads Telegram\'s script again with no second validation', async ({
  page
}) => {
  await standIn(page);
  await sessions(page);
  const bodies = await answerLaunches(page, 204);
  const telegram = telegramRequests(page);
  await page.goto('/' + launchFragment('auth_date=1&start_param=about&hash=synthetic'));
  await expect.poll(() => new URL(page.url()).pathname).toBe('/about');
  const before = bodies.length;

  await page.reload({ waitUntil: 'networkidle' });

  expect(telegram()).toBe(2);
  expect(await page.evaluate(() => (window as unknown as StandIn).__standInRan)).toBe(1);
  expect(bodies.length - before, 'validations the reload added').toBe(0);
});

test('outside a launch no route sends the launch for validation', async ({ page }) => {
  // every route is a whole document load, each waited on until the network is quiet
  test.setTimeout(ROUTES.length * 10_000);
  await standIn(page);
  const bodies = await answerLaunches(page, 204);
  const telegram = telegramRequests(page);
  const own = ownRequests(page);

  for (const path of ROUTES) {
    const before = own();
    await page.goto(path, { waitUntil: 'networkidle' });

    expect(bodies.length, `${path}: validations sent`).toBe(0);
    expect(telegram(), `${path}: requests to Telegram's origin`).toBe(0);
    // the positive control: the route did load, from its own origin
    expect(own() - before, `${path}: requests to the page's own origin`).toBeGreaterThan(0);
  }
});
