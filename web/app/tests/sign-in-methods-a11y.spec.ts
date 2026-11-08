// The passkey list's rendered accessibility audit (SPEC-643 R1 to R4; ADR-666).
//
// axe-core runs over the sign-in methods screen while its list shows Telegram and two passkeys added
// at two instants, so the screen shows three rows and two Removes, each named for its place: once
// with every row closed, and once after a press of the first passkey's Remove has opened its confirm
// step, an alert dialog named by its question, with Keep holding the focus. It runs in Telegram's
// light and dark themes, against the WCAG 2.2 A and AA rules, as the app's audit (a11y.spec.ts) runs
// every route. The app's audit visits this screen with no list answered, so it reads no passkey row;
// here the list is answered in the page, with the session, the study day and the wallet as the voice
// picker's audit answers them, and the root layout's feed read is left to the local preview server,
// as both of those audits leave it.
//
// src/lib/settings/sign-in-methods-a11y.test.ts holds this spec's shape, since the test probe
// resolves no Playwright command.
//
// Run: pnpm exec playwright test tests/sign-in-methods-a11y.spec.ts
import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';
import { TELEGRAM_SDK, answerLaunches, launchFragment } from './launch-fragment';
import { THEMES } from './telegram-palettes';

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA, as the app's audit names them.
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

/**
 * A stand-in for Telegram's script in `scheme`, as the app's audit serves it: it sets the
 * --tg-theme-* variables from the palette, which the design tokens read, and the Mini App object the
 * wrapper reads.
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

// The confirm step's question, which names its alert dialog.
const QUESTION = 'Remove this passkey? You can still sign in with Telegram.';

// The owner's sign-in methods, as GET /api/identities lists them: Telegram, then two passkeys added
// at two instants, so the screen shows two Removes with distinct names.
const LISTED = [
  { id: 1, kind: 'telegram' },
  { id: 4, kind: 'passkey', created_at: Date.UTC(2001, 1, 3, 12) },
  { id: 5, kind: 'passkey', created_at: Date.UTC(2001, 1, 4, 12) }
];

/**
 * Opens the sign-in methods screen and waits for its list: three rows, and two Removes each named for
 * its passkey's place. A Remove is matched by its place and the word before its day, never by the
 * formatted day, which the browser's time zone and locale data set.
 */
async function listed(page: Page, scheme: string): Promise<void> {
  await page.goto('/sign-in-methods');
  // the stand-in ran: the root carries the palette's scheme it sets
  await expect
    .poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--tg-color-scheme')))
    .toBe(scheme);
  // the screen has settled: the list and the wallet have arrived
  await page.waitForLoadState('networkidle');
  const list = page.getByRole('list', { name: 'Sign-in methods' });
  await expect(list.getByRole('listitem')).toHaveCount(3);
  await expect(list.getByRole('button', { name: /^Remove passkey [12], added / })).toHaveCount(2);
}

for (const [scheme, themeParams] of Object.entries(THEMES)) {
  test.describe(`${scheme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.route(`${TELEGRAM_SDK}*`, (intercepted) =>
        intercepted.fulfill({ contentType: 'text/javascript', body: standIn(scheme, themeParams) })
      );
      await page.addInitScript((fragment) => {
        if (window === window.top && location.hash === '') history.replaceState(history.state, '', fragment);
      }, launchFragment('auth_date=1&hash=synthetic'));
      // the API, answered in place, as the app's audit answers it: a session, the study day, the wallet, the list
      await answerLaunches(page, 204);
      await page.route('**/api/session', (intercepted) => intercepted.fulfill({ status: 200 }));
      await page.route('**/api/me', (intercepted) => intercepted.fulfill({ json: { study_day: '2001-02-03' } }));
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
      await page.route('**/api/identities', (intercepted) => intercepted.fulfill({ json: LISTED }));
      await page.emulateMedia({ colorScheme: scheme as 'light' | 'dark' });
    });

    // every row closed: the list as it first shows, with no confirm step open (SPEC-643 R2)
    test('the passkey list with every row closed has no WCAG 2.2 AA violation axe can find', async ({ page }) => {
      await listed(page, scheme);
      await expect(page.getByRole('alertdialog')).toHaveCount(0);
      const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
      expect(results.violations).toEqual([]);
    });

    // a confirm step open: the first passkey's Remove pressed, its question shown, Keep focused (SPEC-643 R3)
    test('the passkey list with a confirm step open has no WCAG 2.2 AA violation axe can find', async ({ page }) => {
      await listed(page, scheme);
      await page.getByRole('button', { name: /^Remove passkey 1, added / }).click();
      await expect(page.getByRole('alertdialog', { name: QUESTION })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Keep' })).toBeFocused();
      const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
      expect(results.violations).toEqual([]);
    });
  });
}
