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
import { TELEGRAM_SDK, answerLaunches, launchFragment } from './launch-fragment';
import { THEMES } from './telegram-palettes';

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA. wcag22aa carries target-size (2.5.8).
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

/**
 * A stand-in for Telegram's script, served at the script's own URL when a launch asks for it: each
 * screen is opened as a launch (SPEC-400), so the app's start loads the script. It does what the
 * real script does when a chat opens the Mini App in `scheme`: it sets the --tg-theme-* variables
 * from the palette, which the design tokens read, and the Mini App object the wrapper reads. The
 * audit so measures the colours Telegram would paint, and nothing reaches the network.
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

// SPEC-505 R9, A6; ADR-523 D6. The review route needs the engine, which this build does not carry,
// so the audit serves a stand-in for the engine's bindings, at the bindings' own URL, with each
// export the session calls. It opens a collection and shows one card whose note type's CSS ends its
// own style element, so the review is audited in its refusal state: the frame empty and marked, the
// status naming the reason, the controls kept.
const REVIEW = '/study/review';
const REFUSED_REASON = 'style-closed';
const ENGINE_STAND_IN = `
const card = {
  id: '1', ordinal: 0, flag: 0, question: '<p>A card</p>', answer: '<p>Its answer</p>',
  css: '.card { color: navy; }</style><base href="https://cards.example/">',
  labels: ['<1m', '<6m', '<10m', '4d'], undo: null, late: false
};
const face = (text) => ({ text, css: card.css, autoplay: [], replay: [], omitted: [] });
export default async function load() {}
export async function install_storage() { return 0; }
export function init() {}
export function open() { return JSON.stringify({ existed: true, notes: 1 }); }
export function close() {}
export function seed() { return 0; }
export function next_card() { return 1n; }
export function undo() {}
export function undo_offer() { return JSON.stringify({ offer: null, why: 'none' }); }
export function snapshot() { return 'null'; }
export function last_panic() { return undefined; }
export function memory_pages() { return 1; }
export function deck_tree() { return '[]'; }
export function set_current_deck() {}
export function current_card() { return JSON.stringify({ counts: { new: 1, learning: 0, review: 0 }, card }); }
export function rate() {}
export function bury() {}
export function flag() { return 1; }
export function faces() { return { question: face(card.question), answer: face(card.answer), wanted: [] }; }
`;

for (const [scheme, themeParams] of Object.entries(THEMES)) {
  test.describe(`${scheme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.route(`${TELEGRAM_SDK}*`, (intercepted) =>
        intercepted.fulfill({ contentType: 'text/javascript', body: standIn(scheme, themeParams) })
      );
      // Each screen opens as a launch (SPEC-400): the fragment a Telegram client opens the Mini App
      // with is in the page's URL before any of its scripts runs, so the app's start loads the
      // script. The fragment is written here because the audit keeps its one navigation to the
      // route itself (a11y-coverage.test.ts reads it).
      await page.addInitScript((fragment) => {
        if (window === window.top && location.hash === '') history.replaceState(history.state, '', fragment);
      }, launchFragment('auth_date=1&hash=synthetic'));
      // The API, answered in place: a session, then the owner's study day.
      await answerLaunches(page, 204);
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
      // The XP exchange readout on the level screen: one bucket with a rate and one no card
      // graduated for, whose undefined rate is text (SPEC-075 R9).
      await page.route('**/api/xp/exchange*', (intercepted) =>
        intercepted.fulfill({
          json: {
            window: null,
            rates: [
              {
                source: 'focus',
                total_xp: 50,
                graduated_cards: 0,
                rate: null,
                rate_defined: false
              },
              { source: 'reviews', total_xp: 10, graduated_cards: 4, rate: 2.5, rate_defined: true }
            ]
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
      // The badge gallery: one earned, one locked with progress, one without (SPEC-073 R16).
      await page.route('**/api/badges', (intercepted) =>
        intercepted.fulfill({
          json: {
            earned: [
              {
                key: 'centurion_day',
                tier: 0,
                name: 'Centurion Day',
                emoji: '\u{1F4AF}',
                study_day: '2001-02-02'
              }
            ],
            locked: [
              {
                key: 'monthly_monk',
                name: 'Monthly Monk',
                emoji: '\u{1F9D8}',
                criteria: '30-day streak',
                family: 'study',
                progress: { value: 3, threshold: 30 }
              },
              {
                key: 'first_page',
                name: 'First Page',
                emoji: '\u{1F4D6}',
                criteria: 'Logged your first reading session',
                family: 'habit',
                progress: null
              }
            ]
          }
        })
      );
      // The personal board on the records screen: a best day, today, the streak and the level
      // (SPEC-075 R3).
      await page.route('**/api/board', (intercepted) =>
        intercepted.fulfill({
          json: {
            rows: [
              {
                kind: 'best_day',
                emoji: '\u{1F3C5}',
                label: 'Best day',
                value: 90,
                study_day: '2001-02-01'
              },
              {
                kind: 'today',
                emoji: '\u{1F4C5}',
                label: 'Today',
                value: 77,
                study_day: '2001-02-03'
              },
              { kind: 'streak', emoji: '\u{1F525}', label: 'Streak', value: 3, longest: 9 },
              { kind: 'level', emoji: '\u26A1', label: 'Level', value: 7, title: 'Adept' }
            ]
          }
        })
      );
      // The records screen: one record still ahead today, one reached, and a chase (SPEC-073 R17).
      await page.route('**/api/records', (intercepted) =>
        intercepted.fulfill({
          json: {
            records: [
              {
                kind: 'best_score',
                label: 'Best daily score',
                value: 120,
                study_day: '2001-02-01',
                previous: 100,
                today: 77,
                distance: 43
              },
              {
                kind: 'most_minutes',
                label: 'Most minutes in a day',
                value: 8,
                study_day: '2001-01-30',
                previous: 5,
                today: 10,
                distance: 0
              }
            ],
            chase: { kind: 'best_score', label: 'Best daily score', gap: 43 }
          }
        })
      );
      // The progress screen: one course with its six band cells, two achieved and the current band
      // marked (SPEC-077 R17, B3).
      await page.route('**/api/progress', (intercepted) =>
        intercepted.fulfill({
          json: {
            courses: [
              {
                code: 'fr',
                name: 'French',
                flag: '\u{1F1EB}\u{1F1F7}',
                mastery_pct: 41.5,
                current_band: 'B1',
                current_unit: 7,
                bands: ['A1', 'A2', 'B1', 'B2', 'C1', 'C2'].map((band, i) => ({
                  band,
                  total: 100,
                  mature: i < 2 ? 100 : 30,
                  pct: i < 2 ? 92 : 30,
                  achieved: i < 2
                }))
              }
            ]
          }
        })
      );
      // The law tab: three shown lines and one pending count (SPEC-077 R17, B3). The level screen's
      // glob ends at /api/level, so the law tiers have their own.
      await page.route('**/api/law', (intercepted) =>
        intercepted.fulfill({
          json: {
            shown: true,
            level_shown: true,
            lines: ['total_xp', 'streak', 'xp_today'],
            streak: 4,
            xp_today: 30,
            total_xp: 900,
            level: 3,
            dues: null,
            dues_pending: true,
            leeches: 2,
            leeches_pending: false,
            mastery: 61,
            mastery_pending: false
          }
        })
      );
      await page.route('**/api/level/law-tiers', (intercepted) =>
        intercepted.fulfill({
          json: {
            cards: { T1: 5, T2: 4, T3: 3, T4: 2, none: 1 },
            xp_today: { T1: 10, T2: 8, T3: 6, T4: 4, none: 2 }
          }
        })
      );
      await page.emulateMedia({ colorScheme: scheme as 'light' | 'dark' });
    });

    for (const route of ROUTES) {
      test(`${route} has no WCAG 2.2 AA violation axe can find`, async ({ page }) => {
        if (route === REVIEW) {
          await page.context().route('**/engine/deck_streak_web_engine.js', (intercepted) =>
            intercepted.fulfill({ contentType: 'text/javascript', body: ENGINE_STAND_IN })
          );
        }
        await page.goto(route);
        // the stand-in ran: the root carries the palette's scheme it sets
        await expect
          .poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--tg-color-scheme')))
          .toBe(scheme);
        // the screen has settled: Today's study day has arrived, or the screen made no call
        await page.waitForLoadState('networkidle');
        if (route === REVIEW) {
          // the review shows the refused card: its frame marked with the reason, the status naming it
          await expect(page.locator('iframe[data-card-refused]')).toHaveAttribute('data-card-refused', REFUSED_REASON);
          await expect(page.locator('p[role="status"]')).toContainText("Its note type's CSS ends its own style block");
        }
        // the refused card's frame is sandboxed with no script, so the legacy mode audits the screen around it
        const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).setLegacyMode().analyze();
        expect(results.violations).toEqual([]);
      });
    }
  });
}
