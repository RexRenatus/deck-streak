// The web engine in the browsers (SPEC-338 R9, R11): the module scripts/web-engine-build.sh wrote,
// in its dedicated Worker over OPFS, driven through the harness page in Chromium and WebKit. Each
// test runs in a persistent profile of its own, as a user's browser has; an empty path is a fresh
// temporary profile. The measurement tests print one `ENGINE-REPORT` line each for section 7.
import {
  expect,
  test,
  type BrowserContext,
  type Page,
  type PlaywrightWorkerArgs,
  type PlaywrightWorkerOptions
} from '@playwright/test';
import type { Harness } from '../engine-harness/main';

/** A card's scheduling fields as the page returns them, its id written out (a bigint does not
 * cross the page boundary as itself). */
interface Fields {
  id: string;
  queue: number;
  type: number;
  due: number;
  interval: number;
  reps: number;
  lapses: number;
}

/** Opens the harness page and waits for its script. */
async function boot(page: Page, url = '/index.html'): Promise<void> {
  await page.goto(url);
  await page.waitForFunction(() => window.harness !== undefined);
}

/** Waits until no Worker of the origin holds the collection's Web Lock: a reloaded or closed
 * page's Worker lets it go when the browser ends that Worker, not before the next page loads. */
async function lockReleased(page: Page): Promise<void> {
  await page.evaluate(async () => {
    for (let attempt = 0; attempt < 200; attempt += 1) {
      const { held = [] } = await navigator.locks.query();
      if (!held.some((lock) => lock.name === 'deck-streak-collection')) return;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error('the collection lock was never released');
  });
}

/** A persistent profile of the test's own browser. */
async function profile(
  playwright: PlaywrightWorkerArgs['playwright'],
  browserName: PlaywrightWorkerOptions['browserName'],
  baseURL: string | undefined
): Promise<BrowserContext> {
  return playwright[browserName].launchPersistentContext('', { baseURL });
}

test('opens, answers and undoes over OPFS', async ({ playwright, browserName, baseURL }) => {
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    const run = await page.evaluate(async () => {
      const read = (fields: unknown) => ({ ...(fields as object), id: String((fields as { id: bigint }).id) });
      const client = window.harness.start();
      const opened = await client.open();
      const seeded = await client.seed(3);
      const card = (await client.next()) as bigint;
      const before = read(await client.snapshot(card));
      const shown = (await client.card()).card!.id;
      await client.rate(shown, 3, 1500);
      const answered = String(shown);
      const after = read(await client.snapshot(card));
      // the undo asks for the offer of the review's own last answer, then confirms it (SPEC-371 R12)
      const offered = await client.undoOffer();
      if (offered.offer === null) throw new Error(`no offer: ${offered.why}`);
      await client.undo(offered.offer.card, offered.offer.step);
      const undone = read(await client.snapshot(card));
      return {
        opened,
        seeded,
        card: String(card),
        answered,
        offered: String(offered.offer.card),
        before,
        after,
        undone,
        persistence: await window.harness.requestPersistence(),
        violations: window.harness.violations
      };
    });
    const before = run.before as Fields;
    const after = run.after as Fields;
    expect(run.opened).toEqual({ existed: false, notes: 0 });
    expect(run.seeded).toBe(3);
    expect(run.answered).toBe(run.card);
    expect(run.offered).toBe(run.card);
    expect(after.reps).toBe(before.reps + 1);
    expect(run.undone).toEqual(run.before);
    expect(['persisted', 'not-persisted', 'unsupported']).toContain(run.persistence);
    expect(run.violations).toEqual([]);
  } finally {
    await context.close();
  }
});

test('buries, flags and undoes each over OPFS', async ({ playwright, browserName, baseURL }) => {
  // SPEC-383 C1: the review's last bury, then its last flag, each offered and undone in the browser
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    const run = await page.evaluate(async () => {
      const read = (fields: unknown) => ({ ...(fields as object), id: String((fields as { id: bigint }).id) });
      const client = window.harness.start();
      await client.open();
      await client.seed(3);
      // a bury: the next card's view offers its undo, and the confirmed undo puts the card back
      const buried = (await client.card()).card!.id;
      const before = read(await client.snapshot(buried));
      await client.bury(buried);
      const away = read(await client.snapshot(buried));
      const afterBury = (await client.card()).card!;
      const buryOffer = await client.undoOffer();
      if (buryOffer.offer === null) throw new Error(`no bury offer: ${buryOffer.why}`);
      await client.undo(buryOffer.offer.card, buryOffer.offer.step);
      const unburied = read(await client.snapshot(buried));
      // a flag: the card's own view offers its undo, and the confirmed undo takes the red flag off
      const shown = (await client.card()).card!;
      const flag = await client.flag(shown.id);
      const flagged = (await client.card()).card!;
      const flagOffer = await client.undoOffer();
      if (flagOffer.offer === null) throw new Error(`no flag offer: ${flagOffer.why}`);
      await client.undo(flagOffer.offer.card, flagOffer.offer.step);
      const unflagged = (await client.card()).card!;
      return {
        buried: String(buried),
        before,
        away,
        unburied,
        buryView: afterBury.undo,
        buryOffer: { ...buryOffer.offer, card: String(buryOffer.offer.card) },
        shown: String(shown.id),
        shownFlag: shown.flag,
        flag,
        flagView: [String(flagged.id), flagged.flag, flagged.undo],
        flagOffer: { ...flagOffer.offer, card: String(flagOffer.offer.card) },
        unflagged: [String(unflagged.id), unflagged.flag],
        violations: window.harness.violations
      };
    });
    const before = run.before as Fields;
    expect((run.away as Fields).queue).not.toBe(before.queue);
    expect(run.buryView).toBe('bury');
    expect(run.buryOffer).toMatchObject({ kind: 'bury', card: run.buried, returns: 'new' });
    expect(run.unburied).toEqual(run.before);
    expect([run.shownFlag, run.flag]).toEqual([0, 1]);
    expect(run.flagView).toEqual([run.shown, 1, 'flag']);
    expect(run.flagOffer).toMatchObject({ kind: 'flag', card: run.shown, flag: 'added' });
    expect(run.unflagged).toEqual([run.shown, 0]);
    expect(run.violations).toEqual([]);
  } finally {
    await context.close();
  }
});

test('the collection survives a page reload', async ({ playwright, browserName, baseURL }) => {
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    const card = await page.evaluate(async () => {
      const client = window.harness.start();
      await client.open();
      await client.seed(5);
      const shown = (await client.card()).card!.id;
      await client.rate(shown, 3, 1500);
      return String(shown);
    });
    await page.reload();
    await page.waitForFunction(() => window.harness !== undefined);
    await lockReleased(page);
    const reopened = await page.evaluate(async (id) => {
      const client = window.harness.start();
      const opened = await client.open();
      const fields = await client.snapshot(BigInt(id));
      return { opened, reps: fields?.reps };
    }, card);
    expect(reopened).toEqual({ opened: { existed: true, notes: 5 }, reps: 1 });
  } finally {
    await context.close();
  }
});

test('a second tab is refused', async ({ playwright, browserName, baseURL }) => {
  const context = await profile(playwright, browserName, baseURL);
  try {
    const first = await context.newPage();
    await boot(first);
    await first.evaluate(async () => {
      const client = window.harness.start();
      await client.open();
      await client.seed(2);
    });
    const second = await context.newPage();
    await boot(second);
    const refused = await second.evaluate(async () => {
      const client = window.harness.start();
      const answers = [];
      for (const call of [() => client.open(), () => client.next()]) {
        answers.push(await call().then(() => null, (error) => window.harness.refusal(error)));
      }
      return answers;
    });
    expect(refused).toEqual([
      { code: 'collection-busy', message: 'another tab holds the collection' },
      { code: 'collection-busy', message: 'another tab holds the collection' }
    ]);
    // The first tab's session is untouched by the second's refusal.
    const answered = await first.evaluate(async () => {
      const client = window.harness.client!;
      const shown = (await client.card()).card!.id;
      await client.rate(shown, 3, 900);
      return String(shown);
    });
    expect(answered).toMatch(/^\d+$/);
  } finally {
    await context.close();
  }
});

test('a context that refuses OPFS is refused', async ({ browser, browserName, baseURL }) => {
  test.skip(browserName !== 'webkit', "WebKit's ephemeral context refuses OPFS; Chromium's serves it");
  const context = await browser.newContext({ baseURL });
  try {
    const page = await context.newPage();
    await boot(page);
    const refused = await page.evaluate(async () => {
      const client = window.harness.start();
      return client.open().then(
        () => null,
        (error) => window.harness.refusal(error)
      );
    });
    expect(refused?.code).toBe('storage-refused');
    expect(refused?.message).not.toBe('');
  } finally {
    await context.close();
  }
});

test('a collection lost to eviction opens as new and says so', async ({ playwright, browserName, baseURL }) => {
  test.skip(browserName !== 'chromium', 'only Chromium offers a protocol call that clears an origin');
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    await page.evaluate(async () => {
      const client = window.harness.start();
      await client.open();
      await client.seed(3);
    });
    await page.close();
    const after = await context.newPage();
    await boot(after);
    await lockReleased(after);
    const cdp = await context.newCDPSession(after);
    await cdp.send('Storage.clearDataForOrigin', { origin: new URL(baseURL ?? '').origin, storageTypes: 'all' });
    await boot(after);
    const opened = await after.evaluate(async () => window.harness.start().open());
    expect(opened).toEqual({ existed: false, notes: 0 });
  } finally {
    await context.close();
  }
});

test('measures the engine over a seeded collection', async ({ playwright, browserName, baseURL }, info) => {
  // ENGINE_MEASURE_NOTES sets the collection's size (ADR-022's is 250,000); ENGINE_CPU_THROTTLE
  // slows Chromium's renderer by its factor, an approximation of a phone's CPU, never a phone.
  const notes = Number(process.env.ENGINE_MEASURE_NOTES ?? 300);
  const throttle = browserName === 'chromium' ? Number(process.env.ENGINE_CPU_THROTTLE ?? 1) : 1;
  test.setTimeout(notes > 10_000 ? 3_600_000 : 120_000);
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    const slow = async () => {
      if (throttle === 1) return;
      const cdp = await context.newCDPSession(page);
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: throttle });
    };
    const start = async () => {
      await boot(page);
      await lockReleased(page);
      await slow();
      // One timed call, in milliseconds, and the module's memory after it: the memory never
      // shrinks, so each reading is the Worker's high-water so far.
      await page.evaluate(() => {
        window.report = { steps: [] };
        window.measure = async (name, call) => {
          const begun = performance.now();
          const value = await call();
          const ms = Math.round((performance.now() - begun) * 10) / 10;
          const memory = (await window.harness.client?.memory()) ?? 0;
          window.report.steps.push({ name, ms, memory });
          return value;
        };
        window.harness.start();
      });
    };
    await start();
    const seeded = await page.evaluate(async (count) => {
      const client = window.harness.client!;
      const opened = await window.measure('open (load, pool, engine)', () => client.open());
      const added = await window.measure('seed', () => client.seed(count));
      await client.close();
      return { opened, added, steps: window.report.steps };
    }, notes);
    await page.reload();
    await start();
    const studied = await page.evaluate(async () => {
      const client = window.harness.client!;
      const read = (fields: unknown) => JSON.stringify({ ...(fields as object), id: String((fields as { id: bigint }).id) });
      const opened = await window.measure('open existing (load, pool, engine)', () => client.open());
      const card = (await window.measure('queue', () => client.next())) as bigint;
      const before = read(await client.snapshot(card));
      const shown = (await window.measure('show', () => client.card())) as { card: { id: bigint } };
      await window.measure('answer', () => client.rate(shown.card.id, 3, 1500));
      const offered = await client.undoOffer();
      if (offered.offer === null) throw new Error(`no offer: ${offered.why}`);
      const { card: answered, step } = offered.offer;
      await window.measure('undo', () => client.undo(answered, step));
      const undone = read(await client.snapshot(card));
      await client.close();
      return { opened, restored: before === undone, steps: window.report.steps };
    });
    await page.reload();
    await start();
    const reloaded = await page.evaluate(async () => {
      const opened = await window.measure('open after reload (load, pool, engine)', () => window.harness.client!.open());
      return { opened, steps: window.report.steps };
    });
    const report = { browser: browserName, notes, throttle, seeded, studied, reloaded };
    process.stdout.write(`ENGINE-REPORT ${JSON.stringify(report)}\n`);
    await info.attach('engine-report', { body: JSON.stringify(report, null, 2), contentType: 'application/json' });
    expect(seeded.opened).toEqual({ existed: false, notes: 0 });
    expect(seeded.added).toBe(notes);
    expect(studied.opened).toEqual({ existed: true, notes });
    expect(studied.restored).toBe(true);
    expect(reloaded.opened).toEqual({ existed: true, notes });
  } finally {
    await context.close();
  }
});

test('measures the engine in a cross-site frame', async ({ playwright, browserName, baseURL }) => {
  // The Mini App runs framed by its host's page on another site. The harness frames itself from
  // the other loopback name, a different site to the browser, and reports what the frame gets.
  const context = await profile(playwright, browserName, baseURL);
  try {
    const page = await context.newPage();
    await boot(page);
    const framed = new URL(baseURL ?? '');
    framed.hostname = 'localhost';
    await page.evaluate(async (src) => {
      const frame = document.createElement('iframe');
      frame.src = src;
      const loaded = new Promise((resolve) => frame.addEventListener('load', resolve));
      document.body.append(frame);
      await loaded;
    }, new URL('/index.html', framed).href);
    const frame = page.frames().find((candidate) => candidate.url().startsWith(framed.origin));
    expect(frame?.url()).toContain('localhost');
    await frame!.waitForFunction(() => window.harness !== undefined);
    const outcome = await frame!.evaluate(async () => {
      const client = window.harness.start();
      try {
        const opened = await client.open();
        await client.seed(2);
        const shown = (await client.card()).card!.id;
        await client.rate(shown, 3, 1200);
        const card = String(shown);
        const offered = await client.undoOffer();
        if (offered.offer === null) throw new Error(`no offer: ${offered.why}`);
        await client.undo(offered.offer.card, offered.offer.step);
        return { ok: true, opened, card };
      } catch (error) {
        return { ok: false, ...window.harness.refusal(error) };
      }
    });
    process.stdout.write(`ENGINE-REPORT ${JSON.stringify({ browser: browserName, framed: outcome })}\n`);
    // Either the frame studies over its own partition of the origin's storage, or it is refused by
    // name; nothing in between.
    const code = 'code' in outcome ? outcome.code : '';
    expect(outcome.ok || ['storage-refused', 'collection-busy'].includes(code)).toBe(true);
  } finally {
    await context.close();
  }
});

declare global {
  interface Window {
    harness: Harness;
    measure(name: string, call: () => Promise<unknown>): Promise<unknown>;
    report: { steps: { name: string; ms: number; memory: number }[] };
  }
}
