// The Worker's sync in the browsers (SPEC-364 B1 to B3, ADR-375 D20): the engine's own transport
// from its dedicated Worker, through the page's own origin, to the engine's own sync server, which
// playwright.engine.config.ts starts and vite.engine.config.ts forwards `/anki-sync/` to. The
// tests set how that route answers through the server's test routes (pass, forbid, redirect, drop)
// and read the sync paths it saw. Each test runs in a persistent profile of its own, so each opens a
// collection of its own; the sync server keeps one account per browser for the run, its base empty
// at the start.
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

/** How the statement route answers: see vite.engine.config.ts's `STATEMENTS`. */
type Statement = 'admit' | 'below' | 'moved';

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

/** Sets how the statement route answers, and empties the recorder of the statements asked. */
async function stating(request: APIRequestContext, answer: Statement): Promise<void> {
  const response = await request.post(`/test-sync/statement/${answer}`);
  expect(response.status(), `the statement route's answer ${answer}`).toBe(200);
}

/** The statement paths asked since the statement route's answer was last set. */
async function asked(request: APIRequestContext): Promise<string[]> {
  return (await (await request.get('/test-sync/asked')).json()) as string[];
}

/** The sync paths the route saw since its mode was last set. */
async function seen(request: APIRequestContext): Promise<string[]> {
  return (await (await request.get('/test-sync/seen')).json()) as string[];
}

/** The sync server's account for `browserName`'s project, which playwright.engine.config.ts made
 * for the run: Chromium's the first, WebKit's the second, named as the config names it. */
function account(browserName: PlaywrightWorkerOptions['browserName']): { user: string; password: string } {
  const user = process.env.ENGINE_SYNC_USER;
  const password = process.env.ENGINE_SYNC_PASSWORD;
  if (!user || !password) throw new Error('playwright.engine.config.ts makes the sync account');
  return { user: browserName === 'webkit' ? `${user}-webkit` : user, password };
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
  const { user, password } = account(browserName);
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
  const { user, password } = account(browserName);
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
  const { user, password } = account(browserName);
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

test('a statement that does not admit the client stops the sync before any sync request and says why', async ({
  playwright,
  browserName,
  baseURL,
  request
}) => {
  await answering(request, 'pass');
  const { user, password } = account(browserName);
  const below =
    'This version of DeckStreak is older than the oldest the sync service accepts. Update DeckStreak to sync.';
  const undecodable =
    "The sync service's statement of the oldest version it accepts could not be read, so nothing was synced.";
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    await opened(page);
    // a statement above this client's level stops the login and the sync, each with the core's sentence
    await stating(request, 'below');
    expect(await login(page, user, password)).toEqual({ code: 'engine-failed', message: below });
    expect(await synced(page)).toEqual({ answer: { code: 'engine-failed', message: below }, status: 'absent' });
    expect(await seen(request)).toEqual([]);
    expect(await asked(request)).toEqual(['/api/sync/minimum-client', '/api/sync/minimum-client']);
    // a redirected statement is not followed: it reads as no statement, and the login stops
    await stating(request, 'moved');
    expect(await login(page, user, password)).toEqual({ code: 'engine-failed', message: undecodable });
    expect(await asked(request)).toEqual(['/api/sync/minimum-client']);
    // a statement that admits lets the login through
    await stating(request, 'admit');
    expect(await login(page, user, password)).toBe('held');
    expect(await page.evaluate(() => window.harness.violations)).toEqual([]);
  } finally {
    await context.close();
  }
});

declare global {
  interface Window {
    harness: Harness;
  }
}

// SPEC-377 A1 to A3 (ADR-388 D6, D8 to D10): the full sync's choice from the Worker, through the
// engine's own transport to the engine's own sync server. Each test makes the server's collection
// it needs from a browser of its own first, so each holds whatever the server held before it.

/** How the snapshot stand-in answers: see vite.engine.config.ts's `SNAPSHOTS`. */
type SnapshotAnswer = 'found' | 'missing' | 'refused' | 'drop';

/** Sets how the snapshot route answers an upload's question. */
async function snapshotting(request: APIRequestContext, answer: SnapshotAnswer): Promise<void> {
  const response = await request.post(`/test-sync/snapshot/${answer}`);
  expect(response.status(), `the snapshot route's answer ${answer}`).toBe(200);
}

/** Starts the page's Worker client and opens its collection; seeds `notes` notes and answers the
 * first card good, when there are notes. Answers the open's answer and the answered card's id. */
async function reviewed(page: Page, notes: number): Promise<{ opened: unknown; card: string | null }> {
  return page.evaluate(async (notes) => {
    const client = window.harness.start();
    const opened = await client.open();
    if (notes === 0) return { opened, card: null };
    await client.seed(notes);
    await client.next();
    const shown = (await client.card()).card!.id;
    await client.rate(shown, 3, 1500);
    return { opened, card: String(shown) };
  }, notes);
}

/** The choice's counts and then the owner's tap on `direction`, each as data. */
async function chosen(page: Page, direction: 'upload' | 'download'): Promise<{ counted: unknown; confirmed: unknown }> {
  return page.evaluate(async (direction) => {
    const client = window.harness.client!;
    const counted = await client.choiceCount().catch((error: unknown) => window.harness.refusal(error));
    const confirmed = await client.choiceConfirm(direction).catch((error: unknown) => window.harness.refusal(error));
    return { counted, confirmed };
  }, direction);
}

/** A card's review count in the page's collection, or null when the collection lacks the card. */
async function reps(page: Page, card: string): Promise<number | null> {
  return page.evaluate(async (card) => {
    const fields = await window.harness.client!.snapshot(BigInt(card));
    return fields === null ? null : fields.reps;
  }, card);
}

/** A browser of the test's own that answers one card and uploads its collection over whatever the
 * server holds, the snapshot found. Answers the answered card's id. */
async function uploaded(
  playwright: PlaywrightWorkerArgs['playwright'],
  browserName: PlaywrightWorkerOptions['browserName'],
  baseURL: string | undefined,
  request: APIRequestContext
): Promise<string> {
  const { user, password } = account(browserName);
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    const { card } = await reviewed(page, 3);
    expect(await login(page, user, password)).toBe('held');
    const { answer } = (await synced(page)) as { answer: { required: string } };
    expect(['full-upload', 'full-sync']).toContain(answer.required);
    await snapshotting(request, 'found');
    expect((await chosen(page, 'upload')).confirmed).toEqual({ status: 'held', outcome: 'written' });
    return card!;
  } finally {
    await context.close();
  }
}

test('an upload waits for its snapshot', async ({ playwright, browserName, baseURL, request }) => {
  // SPEC-377 A1, SPEC-364 section 17 (i): an upload with no snapshot found, or with an answer the
  // Worker could not read, is refused before the engine writes; with one found it is written, and
  // the review made in the browser reaches the server
  await answering(request, 'pass');
  const { user, password } = account(browserName);
  const context = await profile(playwright, browserName, baseURL);
  let card: string;
  try {
    const page = await context.newPage();
    await boot(page);
    card = (await reviewed(page, 3)).card!;
    expect(await login(page, user, password)).toBe('held');
    const { answer } = (await synced(page)) as { answer: { required: string } };
    expect(['full-upload', 'full-sync']).toContain(answer.required);
    for (const unread of ['missing', 'refused', 'drop'] as const) {
      await snapshotting(request, unread);
      const waiting = await chosen(page, 'upload');
      expect(waiting.counted, unread).toMatchObject({ status: 'held', snapshot: { found: unread === 'missing' ? false : null } });
      expect(waiting.confirmed, unread).toEqual({ status: 'held', outcome: 'refused', why: 'no-snapshot' });
    }
    await snapshotting(request, 'found');
    const written = await chosen(page, 'upload');
    expect(written.counted).toMatchObject({ status: 'held', snapshot: { found: true, age: 3600 } });
    expect(written.confirmed).toEqual({ status: 'held', outcome: 'written' });
    // the device and the server now hold one collection
    expect(await synced(page)).toEqual({ answer: { status: 'held', required: 'no-changes' }, status: 'held' });
    expect(await page.evaluate(() => window.harness.violations)).toEqual([]);
  } finally {
    await context.close();
  }
  // a second browser downloads the server's copy, and finds the first browser's review in it
  const second = await profile(playwright, browserName, baseURL);
  try {
    const page = await second.newPage();
    await boot(page);
    await reviewed(page, 0);
    expect(await login(page, user, password)).toBe('held');
    expect(await synced(page)).toMatchObject({ answer: { required: 'full-download' } });
    expect((await chosen(page, 'download')).confirmed).toEqual({ status: 'held', outcome: 'written' });
    expect(await reps(page, card)).toBe(1);
  } finally {
    await second.close();
  }
});

test('a download is written after its backup', async ({ playwright, browserName, baseURL, request }) => {
  // SPEC-377 A2: a device with a review of its own downloads the server's copy; the core writes
  // only after the backup it made reads back with every device review, and the device then holds
  // the server's collection
  await answering(request, 'pass');
  const kept = await uploaded(playwright, browserName, baseURL, request);
  const { user, password } = account(browserName);
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    const own = (await reviewed(page, 2)).card!;
    expect(await login(page, user, password)).toBe('held');
    expect(await synced(page)).toMatchObject({ answer: { required: 'full-sync' } });
    // a download needs no snapshot answer, so the route's answer does not hold it
    await snapshotting(request, 'refused');
    const written = await chosen(page, 'download');
    expect(written.counted).toMatchObject({ status: 'held', counts: { download: { reviews: 1, notes: 2 } } });
    expect(written.confirmed).toEqual({ status: 'held', outcome: 'written' });
    expect([await reps(page, kept), await reps(page, own)]).toEqual([1, null]);
    expect(await synced(page)).toEqual({ answer: { status: 'held', required: 'no-changes' }, status: 'held' });
    expect(await page.evaluate(() => window.harness.violations)).toEqual([]);
  } finally {
    await context.close();
  }
});

test('an evicted collection is restored from the server', async ({ playwright, browserName, baseURL, request }) => {
  // SPEC-377 A3, R10: a browser whose collection is gone opens an empty one, is offered the download
  // alone, and the owner's tap restores every server review
  await answering(request, 'pass');
  const kept = await uploaded(playwright, browserName, baseURL, request);
  const { user, password } = account(browserName);
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    expect((await reviewed(page, 0)).opened).toEqual({ existed: false, notes: 0 });
    expect(await login(page, user, password)).toBe('held');
    expect(await synced(page)).toMatchObject({ answer: { required: 'full-download' } });
    await snapshotting(request, 'missing');
    const restored = await chosen(page, 'download');
    expect(restored.counted).toMatchObject({ status: 'held', counts: { upload: null, download: { reviews: 0, cards: 0, notes: 0 } } });
    expect(restored.confirmed).toEqual({ status: 'held', outcome: 'written' });
    expect(await reps(page, kept)).toBe(1);
    expect(await page.evaluate(() => window.harness.violations)).toEqual([]);
  } finally {
    await context.close();
  }
});
