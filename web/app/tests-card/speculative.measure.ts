// The speculative-connection measurement (#941). Each planted card a link predictor could act on (the
// two preconnect hints, and every card whose link points at the TCP listener) is opened in a stock
// build of each engine the card suite runs (playwright.speculative.config.ts): in the reference frame
// (open.html, every layer off) and in the card frame (index.html, every layer on), then hovered at the
// frame's centre, then clicked there when the card acts on a click. After each step the suite's TCP
// listener, which counts every connection it accepts with a request or without one (SPEC-461 R2), is
// read. What each step reads is the measurement: it is printed and written to the test's output, and
// never judged. The controls are judged: a bare connection this file opens reads `tcp` in the same run,
// the sentinel arrives after every settle window, and nav-self's followed link reads its connection.
// SPEC-650's amendment: the test's output also keeps, for each step, every connection the TCP listener
// accepted by its two addresses and the time of the reading, and, in a project whose metadata names a
// feature, that feature's state as the browser reports it; the bare connection's control also reads
// its own two addresses in that record.
import { once } from 'node:events';
import { mkdirSync, writeFileSync } from 'node:fs';
import { connect } from 'node:net';
import { dirname } from 'node:path';
import { expect, test, type Browser, type Page } from '@playwright/test';
import { Listeners, type Accepted, type Reading } from './listeners';
import { PLANTED, type Listener } from './planted';

/** How long each step waits for what a predictor may open, in milliseconds. */
const SETTLE = 3000;
/** How long an arrival has to reach a listener when it should. */
const REACH_TIMEOUT = 5000;
/** The two resource-hint cards from whose reference frame Chromium and Firefox reach nothing under the suite. */
const HINTS: readonly string[] = ['preconnect', 'shadow-link'];
/** Addresses no listener holds: a card's markup is read with them only to find its links. */
const PROBE: Listener = { http: 'http://listener.invalid', tcp: 'http://listener.invalid:2', udp: 3 };

/** The cards measured: the two hints, and every card that acts on a click through a link to the TCP listener. */
const MEASURED = PLANTED.filter(
  (card) => HINTS.includes(card.id) || (card.click && /<(?:a|area)\b[^>]*\bhref="http:/.test(card.html(PROBE)))
);

/** One step's reading: the frame, the step, and everything that arrived since the frame was opened. */
interface Step {
  frame: string;
  step: 'load' | 'hover' | 'click';
  reading: Reading;
  /** Every connection the TCP listener accepted since the frame was opened, by its two addresses. */
  sockets: Accepted[];
  /** When the reading was taken, in milliseconds since the epoch. */
  at: number;
}

/** What the browser answers when asked for a feature's state. */
interface FeatureState {
  featureEnabled?: boolean;
}

let listeners: Listeners;

test.beforeAll(async () => {
  listeners = await Listeners.start();
});

test.afterAll(async () => {
  await listeners.close();
});

/** Waits the settle window, proves the listener still counts with the sentinel, and returns what arrived. */
async function settled(page: Page): Promise<Pick<Step, 'reading' | 'sockets' | 'at'>> {
  await page.waitForTimeout(SETTLE);
  await listeners.sentinel(page, REACH_TIMEOUT);
  return { reading: await listeners.reading(page), sockets: listeners.sockets, at: Date.now() };
}

/**
 * The state of each feature a project's metadata names (one name, or a list of names), as the
 * browser reports it over a browser session of the Chrome DevTools Protocol, or the error that call
 * returned for that name; nothing when the project names none. It is recorded, never judged.
 */
async function featureState(browser: Browser, feature: unknown): Promise<Record<string, boolean | string>> {
  const names =
    typeof feature === 'string'
      ? [feature]
      : Array.isArray(feature)
        ? feature.filter((name): name is string => typeof name === 'string')
        : [];
  if (names.length === 0) return {};
  const session = await browser.newBrowserCDPSession();
  const send = session.send.bind(session) as unknown as (method: string, params: object) => Promise<FeatureState>;
  const states: Record<string, boolean | string> = {};
  try {
    for (const name of names) {
      try {
        const { featureEnabled } = await send('SystemInfo.getFeatureState', { featureState: name });
        states[name] = featureEnabled ?? 'no state in the answer';
      } catch (error) {
        states[name] = error instanceof Error ? error.message : String(error);
      }
    }
  } finally {
    await session.detach();
  }
  return states;
}

test('the measurement examines every planted card a link predictor could act on', ({}, info) => {
  console.log(`examined ${MEASURED.length} planted cards in ${info.project.name}`);
  expect(MEASURED.map((card) => card.id)).toEqual([
    'preconnect',
    'shadow-link',
    'nav-self',
    'nav-top',
    'nav-blank',
    'nav-blank-area',
    'nav-blank-svg',
    'nav-show-new',
    'download'
  ]);
});

test('control: a bare connection to the TCP listener reads tcp in this run', async ({ page }) => {
  listeners.reset();
  const { hostname, port } = new URL(listeners.address.tcp);
  const socket = connect(Number(port), hostname);
  socket.on('error', () => undefined);
  await once(socket, 'connect');
  await expect.poll(async () => (await listeners.reading(page)).tcp ?? 0, { timeout: REACH_TIMEOUT }).toBe(1);
  // the connection record names this connection: the socket's own address, and the listener's
  const own = `${socket.localAddress ?? ''}:${socket.localPort ?? 0}`;
  expect(listeners.sockets.map(({ peer, target }) => [peer, target])).toEqual([[own, `${hostname}:${port}`]]);
  socket.destroy();
});

for (const card of MEASURED) {
  test(`${card.id}: what this build opens toward the card, by frame and step`, async ({ browser }, info) => {
    const steps: Step[] = [];
    for (const frame of ['open.html', 'index.html']) {
      listeners.reset();
      const context = await browser.newContext();
      const page = await context.newPage();
      await page.goto(`${frame}?${listeners.query(card.id)}`);
      const element = page.locator('#card iframe');
      await element.waitFor();
      const content = await (await element.elementHandle())?.contentFrame();
      await content?.waitForLoadState('load');
      steps.push({ frame, step: 'load', ...(await settled(page)) });
      const box = await element.boundingBox();
      expect(box, 'the frame has no box to hover').not.toBeNull();
      if (box !== null) {
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        steps.push({ frame, step: 'hover', ...(await settled(page)) });
        if (card.click) {
          await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
          steps.push({ frame, step: 'click', ...(await settled(page)) });
        }
      }
      await context.close();
    }
    for (const { frame, step, reading } of steps) {
      console.log(`speculative ${info.project.name} ${card.id} ${frame} ${step}: ${JSON.stringify(reading)}`);
    }
    const features = await featureState(browser, info.project.metadata.features ?? info.project.metadata.feature);
    if (Object.keys(features).length > 0) {
      console.log(`feature state ${info.project.name} ${card.id}: ${JSON.stringify(features)}`);
    }
    const file = info.outputPath('measurement.json');
    mkdirSync(dirname(file), { recursive: true });
    const record = { engine: info.project.name, build: browser.version(), card: card.id, features, steps };
    writeFileSync(file, `${JSON.stringify(record, null, 2)}\n`);

    // the control: a followed link's connection is counted in this build
    if (card.id === 'nav-self') {
      const followed = steps.find((entry) => entry.frame === 'open.html' && entry.step === 'click');
      expect(Listeners.holds(followed?.reading ?? {}, card.paths), `nav-self's link read nothing in ${info.project.name}`).toBe(true);
    }
  });
}
