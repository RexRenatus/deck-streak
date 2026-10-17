// The TCP listener's three facts (SPEC-461 R11, A5 to A7; ADR-478 D7). Restated as a request-reading
// listener on its own port, it counts every connection it accepts as `tcp` and records each
// request's path under its own key, and the harness's sentinel never reaches it. Each fact is read
// from the listeners' own reading, as the card suite reads every arrival.
import { once } from 'node:events';
import { get } from 'node:http';
import { connect } from 'node:net';
import { expect, test } from '@playwright/test';
import { Listeners } from './listeners';

/** How long an arrival has to reach a listener when it should. */
const REACH_TIMEOUT = 10_000;

let listeners: Listeners;

test.beforeAll(async () => {
  listeners = await Listeners.start();
});

test.afterAll(async () => {
  await listeners.close();
});

test.beforeEach(() => {
  listeners.reset();
});

test('the TCP listener: a connection that sends no request reads tcp and no path key', async ({ page }) => {
  // a bare connection, as an engine opens one ahead of a request: it sends no byte
  const { hostname, port } = new URL(listeners.address.tcp);
  const socket = connect(Number(port), hostname);
  socket.on('error', () => undefined);
  await once(socket, 'connect');
  await expect.poll(async () => (await listeners.reading(page)).tcp ?? 0, { timeout: REACH_TIMEOUT }).toBe(1);
  expect(await listeners.reading(page)).toEqual({ tcp: 1 });
  socket.destroy();
});

test('the TCP listener: a served request reads tcp and its path key', async ({ page }) => {
  // one request on a connection of its own; an error's code stands in for a status never sent
  const served = await new Promise<number | string>((resolve) => {
    get(`${listeners.address.tcp}/served/1`, { agent: false }, (response) => {
      response.resume();
      resolve(response.statusCode ?? 0);
    }).on('error', (error: NodeJS.ErrnoException) => resolve(error.code ?? error.message));
  });
  expect(served, 'the TCP listener served no request').toBe(200);
  expect(await listeners.reading(page)).toEqual({ tcp: 1, '/served/1': 1 });
});

test('the TCP listener: the harness sentinel never reaches it', async ({ page }) => {
  // a page on the HTTP listener's own origin sends the sentinel, as the harness page does
  await page.goto(`${listeners.address.http}/page/1`);
  listeners.reset();
  await listeners.sentinel(page, REACH_TIMEOUT);
  const reading = await listeners.reading(page);
  expect(reading.tcp, 'the sentinel reached the TCP listener').toBeUndefined();
  expect(Object.keys(reading).filter((key) => key.startsWith('/sentinel/'))).toEqual([]);
});
