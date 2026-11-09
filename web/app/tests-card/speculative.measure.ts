// The speculative-connection measurement (#941). Each planted card a link predictor could act on (the
// two preconnect hints, and every card whose link points at the TCP listener) is opened in a stock
// build of each engine the card suite runs (playwright.speculative.config.ts): in the reference frame
// (open.html, every layer off) and in the card frame (index.html, every layer on), then hovered at the
// frame's centre, then clicked there when the card acts on a click. After each step the suite's TCP
// listener, which counts every connection it accepts with a request or without one (SPEC-461 R2), is
// read. What each step reads is the measurement: it is printed and written to the test's output, and
// never judged. The controls are judged: a bare connection this file opens reads `tcp` in the same run,
// the sentinel arrives after every settle window, and nav-self's followed link reads its connection.
import { once } from 'node:events';
import { mkdirSync, writeFileSync } from 'node:fs';
import { connect } from 'node:net';
import { dirname } from 'node:path';
import { expect, test, type Page } from '@playwright/test';
import { Listeners, type Reading } from './listeners';
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
}

let listeners: Listeners;

test.beforeAll(async () => {
  listeners = await Listeners.start();
});

test.afterAll(async () => {
  await listeners.close();
});

/** Waits the settle window, proves the listener still counts with the sentinel, and returns what arrived. */
async function settled(page: Page): Promise<Reading> {
  await page.waitForTimeout(SETTLE);
  await listeners.sentinel(page, REACH_TIMEOUT);
  return listeners.reading(page);
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
      steps.push({ frame, step: 'load', reading: await settled(page) });
      const box = await element.boundingBox();
      expect(box, 'the frame has no box to hover').not.toBeNull();
      if (box !== null) {
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        steps.push({ frame, step: 'hover', reading: await settled(page) });
        if (card.click) {
          await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
          steps.push({ frame, step: 'click', reading: await settled(page) });
        }
      }
      await context.close();
    }
    for (const { frame, step, reading } of steps) {
      console.log(`speculative ${info.project.name} ${card.id} ${frame} ${step}: ${JSON.stringify(reading)}`);
    }
    const file = info.outputPath('measurement.json');
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, `${JSON.stringify({ engine: info.project.name, build: browser.version(), card: card.id, steps }, null, 2)}\n`);

    // the control: a followed link's connection is counted in this build
    if (card.id === 'nav-self') {
      const followed = steps.find((entry) => entry.frame === 'open.html' && entry.step === 'click');
      expect(Listeners.holds(followed?.reading ?? {}, card.paths), `nav-self's link read nothing in ${info.project.name}`).toBe(true);
    }
  });
}
