// DeckStreak's rendered accessibility audit (the accessibility pack; SPEC-028 R10).
//
// axe-core runs over every screen of the Mini App, in Telegram's light and dark themes, against the
// WCAG 2.2 A and AA rules. The static rows of packs/accessibility cannot measure a rendered colour
// pair, a target's size or a live region. This audit can, and the pack's `runtime-audit` row checks
// that it exists, names the WCAG 2.2 AA tags and runs in CI.
//
// The screens come from the app's route table and the palettes from telegram-palettes.ts, so a new
// screen is audited from the day it exists: src/lib/a11y-coverage.test.ts holds the route table
// equal to the screens under src/routes.
//
// Install: pnpm add -D @playwright/test @axe-core/playwright
// Run:     pnpm exec playwright test tests/a11y.spec.ts   (a CI workflow runs it on every change)
import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';
import { ROUTES } from '../src/lib/routes';
import { THEMES } from './telegram-palettes';

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA. wcag22aa carries target-size (2.5.8).
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

const TELEGRAM_SDK = 'https://telegram.org/js/telegram-web-app.js';

/**
 * A stand-in for Telegram's script, served at the script's own URL, first in the head. It does what
 * the real script does when a chat opens the Mini App in `scheme`: it sets the --tg-theme-*
 * variables from the palette, which the design tokens read, and the Mini App object the wrapper
 * reads. The audit so measures the colours Telegram would paint, and nothing reaches the network.
 */
function standIn(scheme: string, themeParams: Readonly<Record<string, string>>): string {
  return `(() => {
    const noop = () => {};
    const inset = { top: 0, bottom: 0, left: 0, right: 0 };
    const button = { show: noop, hide: noop, onClick: noop, offClick: noop, setText: noop };
    const themeParams = ${JSON.stringify(themeParams)};
    const style = document.documentElement.style;
    for (const [key, value] of Object.entries(themeParams)) {
      style.setProperty('--tg-theme-' + key.split('_').join('-'), value);
    }
    style.setProperty('--tg-color-scheme', ${JSON.stringify(scheme)});
    window.Telegram = {
      WebApp: {
        initData: 'auth_date=1&hash=synthetic',
        version: '9.0',
        platform: 'unknown',
        colorScheme: ${JSON.stringify(scheme)},
        themeParams,
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
}

for (const [scheme, themeParams] of Object.entries(THEMES)) {
  test.describe(`${scheme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.route(`${TELEGRAM_SDK}*`, (intercepted) =>
        intercepted.fulfill({ contentType: 'text/javascript', body: standIn(scheme, themeParams) })
      );
      // The API, answered in place: a session, then the owner's study day.
      await page.route('**/api/session', (intercepted) => intercepted.fulfill({ status: 200 }));
      await page.route('**/api/me', (intercepted) =>
        intercepted.fulfill({ json: { study_day: '2001-02-03' } })
      );
      // The score screen's breakdown, with a retention the day does not have (SPEC-071 R22).
      await page.route('**/api/score', (intercepted) =>
        intercepted.fulfill({
          json: {
            study_day: '2001-02-03',
            score: {
              total: 64,
              grade: { label: 'SOLID', emoji: '\u2705' },
              pillars: { consistency: 76, retention: null, workload: 70, volume: 60.5, mastery: 55 },
              reviews: 0,
              retention: null
            }
          }
        })
      );
      // The level screen, on an Ascendant day with one provisional source (SPEC-072 R26).
      await page.route('**/api/level', (intercepted) =>
        intercepted.fulfill({
          json: {
            study_day: '2001-02-03',
            level: 7,
            title: 'Adept',
            emoji: '\u{1F33F}',
            total_xp: 1234,
            xp_into_level: 40,
            xp_for_next: 200,
            today: [
              { source: 'reviews', track: 'language', amount: 24, state: 'settled' },
              { source: 'streak', track: 'language', amount: 10, state: 'provisional' }
            ],
            run: 5,
            multiplier: 1.25,
            multiplier_after_a_miss: 1.1,
            ascendant: true
          }
        })
      );
      // The wallet, for the header on every screen and the /wallet screen: a balance, today's loss
      // limit, a mint and a fine, and an older page, so the button that asks for it is audited too
      // (SPEC-082 R15, R17).
      await page.route('**/api/wallet', (intercepted) =>
        intercepted.fulfill({
          json: {
            study_day: '2001-02-03',
            balance: 103,
            loss_cap: 30,
            loss_cap_left: 25,
            movements: [
              { id: 4, study_day: '2001-02-03', source: 'fine', amount: -5 },
              { id: 3, study_day: '2001-02-03', source: 'mint', amount: 8 }
            ],
            next: 3
          }
        })
      );
      await page.emulateMedia({ colorScheme: scheme as 'light' | 'dark' });
    });

    for (const route of ROUTES) {
      test(`${route} has no WCAG 2.2 AA violation axe can find`, async ({ page }) => {
        await page.goto(route);
        // the screen has settled: Today's study day has arrived, or the screen made no call
        await page.waitForLoadState('networkidle');
        const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
        expect(results.violations).toEqual([]);
      });
    }
  });
}
