// The Worker's sync in the browsers (SPEC-364 B1 to B3, ADR-375 D20): the engine's own transport
// from its dedicated Worker, through the page's own origin, to the engine's own sync server, which
// playwright.engine.config.ts starts and vite.engine.config.ts forwards `/anki-sync/` to. The
// tests set how that route answers through the server's test routes (pass, forbid, redirect, drop)
// and read the sync paths it saw. Each test runs in a persistent profile of its own, so each opens a
// collection of its own; the sync server keeps one account for the run, its base empty at the start.
import {
  expect,
  test,
  type APIRequestContext,
  type BrowserContext,
  type Page,
  type PlaywrightWorkerArgs,
  type PlaywrightWorkerOptions
} from '@playwright/test';
import type { Harness } from '../engine-harness/main';

/** How the sync route answers: see vite.engine.config.ts's `MODES`. */
type Mode = 'pass' | 'forbid' | 'redirect' | 'drop';

/** Opens the harness page and waits for its script. */
async function boot(page: Page): Promise<void> {
  await page.goto('/index.html');
  await page.waitForFunction(() => window.harness !== undefined);
}

/** A persistent profile of the test's own browser; an empty path is a fresh temporary one. */
async function profile(
  playwright: PlaywrightWorkerArgs['playwright'],
  browserName: PlaywrightWorkerOptions['browserName'],
  baseURL: string | undefined
): Promise<BrowserContext> {
  return playwright[browserName].launchPersistentContext('', { baseURL });
}

/** Sets how the sync route answers, and empties the recorder of the paths it saw. */
async function answering(request: APIRequestContext, mode: Mode): Promise<void> {
  const response = await request.post(`/test-sync/mode/${mode}`);
  expect(response.status(), `the sync route's mode ${mode}`).toBe(200);
}

/** The sync paths the route saw since its mode was last set. */
async function seen(request: APIRequestContext): Promise<string[]> {
  return (await (await request.get('/test-sync/seen')).json()) as string[];
}

/** The sync server's one account, which playwright.engine.config.ts made for the run. */
function account(): { user: string; password: string } {
  const user = process.env.ENGINE_SYNC_USER;
  const password = process.env.ENGINE_SYNC_PASSWORD;
  if (!user || !password) throw new Error('playwright.engine.config.ts makes the sync account');
  return { user, password };
}

/** Starts the page's Worker client and opens its collection. */
async function opened(page: Page): Promise<unknown> {
  return page.evaluate(async () => {
    const client = window.harness.start();
    return { opened: await client.open(), seeded: await client.seed(3) };
  });
}

/** The page's sync login, or its refusal as data. */
async function login(page: Page, user: string, password: string): Promise<unknown> {
  return page.evaluate(
    async ({ user, password }) =>
      window.harness.client!.syncLogin(user, password).catch((error: unknown) => window.harness.refusal(error)),
    { user, password }
  );
}

/** The page's normal sync and the store's status after it, or a refusal as data. */
async function synced(page: Page): Promise<unknown> {
  return page.evaluate(async () => {
    const client = window.harness.client!;
    const answer = await client.sync().catch((error: unknown) => window.harness.refusal(error));
    return { answer, status: await client.credentialStatus() };
  });
}

test.afterEach(async ({ request }) => {
  await answering(request, 'pass');
});

test('a login and a normal sync from the worker reach the server', async ({
  playwright,
  browserName,
  baseURL,
  request
}) => {
  await answering(request, 'pass');
  const { user, password } = account();
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    // the collection the test opens and seeds: three notes, so the empty server needs an upload
    expect(await opened(page)).toEqual({ opened: { existed: false, notes: 0 }, seeded: 3 });
    expect(await login(page, user, password)).toBe('held');
    expect(await synced(page)).toEqual({ answer: { status: 'held', required: 'full-upload' }, status: 'held' });
    // the server saw the login and a sync request, each through the page's own origin
    const paths = await seen(request);
    expect(paths).toContain('/anki-sync/sync/hostKey');
    expect(paths.filter((path) => path !== '/anki-sync/sync/hostKey').length).toBeGreaterThan(0);
    expect(paths.filter((path) => !path.startsWith('/anki-sync/'))).toEqual([]);
    expect(await page.evaluate(() => window.harness.violations)).toEqual([]);
  } finally {
    await context.close();
  }
});

test('a refused key is dropped and a lost network keeps it', async ({
  playwright,
  browserName,
  baseURL,
  request
}) => {
  await answering(request, 'pass');
  const { user, password } = account();
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    await opened(page);
    expect(await login(page, user, password)).toBe('held');
    // a lost network is a failure: the key is kept
    await answering(request, 'drop');
    expect(await synced(page)).toEqual({ answer: { status: 'held', required: null }, status: 'held' });
    expect(await seen(request)).toContain('/anki-sync/sync/meta');
    // the server's 403 is the engine's auth refusal: the key is dropped
    await answering(request, 'forbid');
    expect(await synced(page)).toEqual({ answer: { status: 'absent', required: null }, status: 'absent' });
    expect(await seen(request)).toContain('/anki-sync/sync/meta');
  } finally {
    await context.close();
  }
});

test('a redirected sync answer is refused', async ({ playwright, browserName, baseURL, request }) => {
  await answering(request, 'pass');
  const { user, password } = account();
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    await opened(page);
    // a redirected login is answered from the moved route, and refused: no key is kept
    await answering(request, 'redirect');
    expect(await login(page, user, password)).toBe('offline');
    expect(await page.evaluate(() => window.harness.client!.credentialStatus())).toBe('absent');
    expect(await seen(request)).toContain('/anki-sync-moved/sync/hostKey');
    // a redirected sync is answered from the moved route, and refused: nothing synced, key kept
    await answering(request, 'pass');
    expect(await login(page, user, password)).toBe('held');
    await answering(request, 'redirect');
    expect(await synced(page)).toEqual({ answer: { status: 'held', required: null }, status: 'held' });
    expect(await seen(request)).toContain('/anki-sync-moved/sync/meta');
  } finally {
    await context.close();
  }
});

declare global {
  interface Window {
    harness: Harness;
  }
}
