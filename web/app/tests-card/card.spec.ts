// The planted card suite (SPEC-341 R8 to R11, A9 to A12; ADR-352 D7). Every channel the schematic's
// web table names has a planted card (planted.ts). Each card is opened in the reference frame
// (open.html, every layer off), where it must reach its listener, and in the card frame
// (index.html, every layer on), where it must reach nothing; then with each layer removed alone
// (variant.html?off=W1 to W4), where exactly the channels the schematic gives that layer open. The
// scripts-on variant measures what card scripts would reach, and the render proof shows the card
// frame renders the card. Arrivals are read from listeners.ts, never from the browser's own events.
import { expect, test, type Browser, type BrowserContext, type Page, type TestInfo } from '@playwright/test';
import { Listeners, type Reading } from './listeners';
import { LAYERS, PLANTED, RENDER, UNOBSERVABLE, type Engine } from './planted';

/** A settle window is never shorter than this, in milliseconds. */
const SETTLE_FLOOR = 1500;
/** A settle window is at least this many times the card's measured reference latency. */
const SETTLE_FACTOR = 3;
/** The settle window of a card whose reference latency this worker has not measured. */
const SETTLE_UNMEASURED = 3000;
/** How long a frame has to reach a listener when it should. */
const REACH_TIMEOUT = 5000;

let listeners: Listeners;
/** Each card's reference latency in this worker, in milliseconds. */
const latency = new Map<string, number>();

test.beforeAll(async () => {
  listeners = await Listeners.start();
});

test.afterAll(async () => {
  await listeners.close();
});

interface Visit {
  context: BrowserContext;
  page: Page;
  started: number;
}

function engineOf(info: TestInfo): Engine {
  return info.project.name as Engine;
}

/** Whether `engine` cannot see the card's channel open even with every layer off. */
function blind(engine: Engine, id: string): boolean {
  return UNOBSERVABLE.some((entry) => entry.engine === engine && entry.id === id);
}

function settle(id: string): number {
  const measured = latency.get(id);
  return measured === undefined ? SETTLE_UNMEASURED : Math.max(SETTLE_FLOOR, SETTLE_FACTOR * measured);
}

/**
 * Opens the harness page `path` with `card` in a fresh browser context, so no cache, socket or
 * storage carries over from another visit, waits for the frame's document, and clicks the centre
 * of the frame, where a clicking card lays its link or button, when the card acts on a click.
 */
async function visit(browser: Browser, path: string, card: string, click: boolean): Promise<Visit> {
  listeners.reset();
  const context = await browser.newContext();
  const page = await context.newPage();
  const started = Date.now();
  await page.goto(`${path}${path.includes('?') ? '&' : '?'}${listeners.query(card)}`);
  const frame = page.locator('#card iframe');
  await frame.waitFor();
  const content = await (await frame.elementHandle())?.contentFrame();
  await content?.waitForLoadState('load');
  if (click) {
    const box = await frame.boundingBox();
    expect(box, 'the frame has no box to click').not.toBeNull();
    if (box !== null) await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  }
  return { context, page, started };
}

/** Waits until every one of `paths` has arrived, and returns the reading that held them. */
async function reaches(at: Visit, paths: readonly string[]): Promise<Reading> {
  let reading: Reading = {};
  await expect
    .poll(
      async () => {
        reading = await listeners.reading(at.page);
        return Listeners.holds(reading, paths);
      },
      { timeout: REACH_TIMEOUT, message: `nothing reached ${paths.join(', ')}` }
    )
    .toBe(true);
  return reading;
}

/**
 * Waits the card's settle window, proves the listener still counts with a sentinel from the
 * harness page, and returns everything that arrived: a frame that reaches nothing returns `{}`.
 */
async function arrivals(at: Visit, id: string): Promise<Reading> {
  await at.page.waitForTimeout(settle(id));
  await listeners.sentinel(at.page, REACH_TIMEOUT);
  return listeners.reading(at.page);
}

test('the suite examines every planted card, pair and variant', ({}, info) => {
  const variants = LAYERS.length * PLANTED.length;
  console.log(`examined ${PLANTED.length} planted cards, ${PLANTED.length} pairs, ${variants} variants in ${info.project.name}`);
  expect(PLANTED.length).toBeGreaterThan(0);
  expect(LAYERS.length).toBeGreaterThan(0);
});

for (const card of PLANTED) {
  test(`${card.id}: the reference frame reaches its listener and the card frame reaches nothing`, async ({ browser }, info) => {
    const engine = engineOf(info);

    // the reference frame, every layer off: the channel opens, unless this engine cannot see it
    const reference = await visit(browser, 'open.html', card.id, card.click);
    if (blind(engine, card.id)) {
      const reading = await arrivals(reference, card.id);
      expect(reading, `${card.id} reached a listener in ${engine}, so it is observable there`).toEqual({});
    } else {
      await reaches(reference, card.paths);
      latency.set(card.id, Date.now() - reference.started);
    }
    await reference.context.close();

    // the card frame, every layer on: nothing arrives
    const shipped = await visit(browser, 'index.html', card.id, card.click);
    expect(await arrivals(shipped, card.id), `${card.id} reached out of the card frame in ${engine}`).toEqual({});
    await shipped.context.close();
  });
}

for (const layer of LAYERS) {
  for (const card of PLANTED) {
    const opens = card.alone === layer;
    test(`${layer} off: ${card.id} ${opens ? 'opens' : 'stays closed'}`, async ({ browser }, info) => {
      const engine = engineOf(info);
      const variant = await visit(browser, `variant.html?off=${layer}`, card.id, card.click);
      if (opens && !blind(engine, card.id)) await reaches(variant, card.paths);
      else expect(await arrivals(variant, card.id), `${card.id} opened with ${layer} off in ${engine}`).toEqual({});
      await variant.context.close();
    });
  }
}

test('the host policy alone holds the image-set forms in every engine', async ({ browser }, info) => {
  const engine = engineOf(info);
  const card = PLANTED.find((planted) => planted.id === 'srcset');
  expect(card, 'planted.ts plants no srcset card').toBeDefined();
  if (card === undefined) return;
  const reference = await visit(browser, 'open.html', card.id, card.click);
  await reaches(reference, ['/srcset/1', '/srcset/2']);
  await reference.context.close();
  const inherited = await visit(browser, 'variant.html?off=W3meta', card.id, card.click);
  expect(await arrivals(inherited, card.id), `srcset opened under the host policy alone in ${engine}`).toEqual({});
  await inherited.context.close();
});

test('scripts on: a peer connection from the card reaches the UDP listener', async ({ browser }, info) => {
  const engine = engineOf(info);
  const scripted = await visit(browser, 'variant.html?off=scripts', 'webrtc', false);
  if (blind(engine, 'webrtc')) {
    const reading = await arrivals(scripted, 'webrtc');
    console.log(`scripts on, ${engine}: webrtc is UNOBSERVABLE; ${reading.udp ?? 0} datagram(s)`);
    expect(reading).toEqual({});
  } else {
    const reading = await reaches(scripted, ['udp']);
    console.log(`scripts on, ${engine}: ${reading.udp} datagram(s) reached the UDP listener`);
    expect(reading.udp).toBeGreaterThan(0);
  }
  await scripted.context.close();
});

test("scripts on: the bridge card's parent message arrives from origin null and reaches nothing else", async ({ browser }, info) => {
  const engine = engineOf(info);
  const scripted = await visit(browser, 'variant.html?off=scripts', 'bridge', false);
  await reaches(scripted, ['bridge:message']);
  const reading = await arrivals(scripted, 'bridge');
  const counts = await listeners.bridge(scripted.page);
  console.log(`scripts on, ${engine}: ${counts?.message} parent message(s), origins ${JSON.stringify(counts?.origins)}`);

  // the messages arrive, every one from origin "null" and from the card frame's own window
  expect(counts?.origins.length).toBeGreaterThan(0);
  expect([...new Set(counts?.origins)]).toEqual(['null']);
  expect(counts?.fromFrame.every((own) => own)).toBe(true);
  // and nothing else: not the bridge, the BroadcastChannel, the storage, the port or a listener
  expect(reading).toEqual({ 'bridge:message': counts?.message });
  await scripted.context.close();
});

/** A screenshot of the harness page's frame, with `card` in it. */
async function shot(browser: Browser, path: string, card: string): Promise<Buffer> {
  // the frame's load event has waited for its image; a short pause lets that paint settle
  const at = await visit(browser, path, card, false);
  await at.page.waitForTimeout(250);
  const image = await at.page.locator('#card iframe').screenshot({ animations: 'disabled' });
  await at.context.close();
  return image;
}

test('render-proof: the card frame shows the card as the reference frame does, unlike a blank frame', async ({ browser }) => {
  expect(RENDER, 'planted.ts exports no render-proof card').not.toBeNull();
  const shipped = await shot(browser, 'index.html', 'render');
  const reference = await shot(browser, 'open.html', 'render');
  const blank = await shot(browser, 'open.html', 'blank');

  expect(shipped.equals(reference), 'the card frame renders the card unlike the reference frame').toBe(true);
  expect(shipped.equals(blank), 'the card frame renders a blank frame').toBe(false);
  expect(reference.equals(blank)).toBe(false);
});
