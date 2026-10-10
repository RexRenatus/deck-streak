// The streak calendar at phone width (SPEC-076 section 27; ADR-302 D2). The Mini App lays the
// served window out as whole weeks, Monday first, each day in exactly one cell, and the page never
// scrolls sideways at 360 px, the narrowest phone the Mini App is laid out for. Only a browser
// lays a page out, so this is measured here and not in the unit tests.
//
// Run: pnpm exec playwright test tests/streak-calendar.spec.ts   (CI runs every spec in tests/)
import { expect, test } from '@playwright/test';
import { TELEGRAM_SDK, launchFragment } from './launch-fragment';

// Telegram's script stood in for at its own URL, as the accessibility audit does, and loaded by
// opening the page as a launch (SPEC-400): launch data for the session handshake, and the Mini App
// object the wrapper reads. Nothing reaches the network.
const STAND_IN = `(() => {
  const noop = () => {};
  const inset = { top: 0, bottom: 0, left: 0, right: 0 };
  const button = { show: noop, hide: noop, onClick: noop, offClick: noop, setText: noop };
  window.Telegram = {
    WebApp: {
      initData: 'auth_date=1&hash=synthetic',
      version: '9.0',
      platform: 'unknown',
      colorScheme: 'light',
      themeParams: {},
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

const DAY_MS = 86_400_000;
// Served on 2025-01-14, a Tuesday (epoch day 20102): the predecessor's window runs from Monday
// 2024-07-15 (epoch day 19919), the Monday on or before the day 181 days back, through it.
const FIRST = 19_919;
const LENGTH = 184;
const MARKERS = ['skip', 'freeze', 'break'] as const;

function iso(epochDay: number): string {
  return new Date(epochDay * DAY_MS).toISOString().slice(0, 10);
}

/** A track's served days, with every marker on days of every weekday, two on some days. */
function served(shift: number) {
  return Array.from({ length: LENGTH }, (_, index) => ({
    day: iso(FIRST + index),
    studied: (index + shift) % 3 !== 0,
    markers: MARKERS.filter((_, which) => (index + shift + which) % 4 === 0 || index % 9 === which)
  }));
}

test.use({ viewport: { width: 360, height: 740 } });

test.beforeEach(async ({ page }) => {
  await page.route(`${TELEGRAM_SDK}*`, (intercepted) =>
    intercepted.fulfill({ contentType: 'text/javascript', body: STAND_IN })
  );
  await page.route('**/api/session', (intercepted) => intercepted.fulfill({ status: 200 }));
  await page.route('**/api/streak', (intercepted) =>
    intercepted.fulfill({
      json: {
        study_day: iso(FIRST + LENGTH - 1),
        language: { current: 12, longest: 40, heat: 2, freezes: 1, freeze_cap: 3 },
        law: { current: 3, longest: 9, heat: 0 },
        at_stake: { language: 'freeze', law: 'break' },
        calendar: { language: served(0), law: served(1) }
      }
    })
  );
  await page.route('**/api/governor', (intercepted) =>
    intercepted.fulfill({
      json: { verdict: 'armed', strength: 0.9, lapse_since: null, relight_cards: null }
    })
  );
});

test('the streak calendar lays out whole weeks, Monday first, and does not scroll sideways at 360 px', async ({
  page
}) => {
  await page.goto('/streak' + launchFragment('auth_date=1&hash=synthetic'));
  // the stand-in ran: the Mini App object it sets is there for the wrapper
  await expect
    .poll(() => page.evaluate(() => typeof (window as unknown as { Telegram?: { WebApp?: object } }).Telegram?.WebApp))
    .toBe('object');
  let examined = 0;
  for (const track of ['language', 'law']) {
    const cells = page.locator(`article[data-track="${track}"] li[data-day]`);
    await expect(cells).toHaveCount(LENGTH);
    const boxes = await cells.evaluateAll((drawn) =>
      drawn.map((cell) => {
        const box = cell.getBoundingClientRect();
        return { x: box.x, y: box.y, right: box.right };
      })
    );
    boxes.forEach((box, index) => {
      // The window opens on a Monday, so cell `index` is weekday index % 7 of week index / 7: it
      // shares its column with the first week's cell of that weekday and its row with its Monday.
      const column = boxes[index % 7];
      const monday = boxes[index - (index % 7)];
      expect(Math.abs(box.x - (column?.x ?? Number.NaN)), `${track} ${index}: column`).toBeLessThan(0.5);
      expect(Math.abs(box.y - (monday?.y ?? Number.NaN)), `${track} ${index}: row`).toBeLessThan(0.5);
      if (index % 7 > 0) expect(box.x, `${track} ${index}: after the day before`).toBeGreaterThan(boxes[index - 1]?.x ?? Infinity);
      if (index >= 7) expect(box.y, `${track} ${index}: below the week before`).toBeGreaterThan(boxes[index - 7]?.y ?? Infinity);
      expect(box.right, `${track} ${index}: inside the viewport`).toBeLessThanOrEqual(360);
      examined += 1;
    });
  }
  console.log(`examined ${examined} calendar cell(s) at 360 px`);
  expect(examined).toBe(2 * LENGTH);
  const sideways = await page.evaluate(() =>
    Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth
  );
  expect(sideways, 'the page scrolls sideways').toBeLessThanOrEqual(0);
});
