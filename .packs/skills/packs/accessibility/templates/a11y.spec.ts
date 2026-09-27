// DeckStreak's rendered accessibility audit (packs/accessibility, SPEC-V2-2223).
//
// axe-core runs over every screen of the Mini App, in Telegram's light and dark themes, against the
// WCAG 2.2 A and AA rules. The static rows of packs/accessibility cannot measure a rendered colour
// pair, a target's size or a live region. This audit can, and the pack's `runtime-audit` row checks
// that it exists, names the WCAG 2.2 AA tags and runs in CI.
//
// Install: pnpm add -D @playwright/test @axe-core/playwright
// Run:     pnpm exec playwright test tests/a11y.spec.ts   (a CI workflow runs it on every change)
import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

// Every screen the Mini App routes to. Add a route here when a screen is added.
const ROUTES = ['/', '/review', '/decks', '/stats', '/settings'];

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA. wcag22aa carries target-size (2.5.8).
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

// Telegram's own default palettes. The Mini App paints with --tg-theme-* variables, which exist only
// inside Telegram, so the audit supplies both schemes and contrast is measured in each.
const THEMES = {
	light: {
		bg_color: '#ffffff',
		text_color: '#000000',
		hint_color: '#707579',
		link_color: '#3390ec',
		button_color: '#3390ec',
		button_text_color: '#ffffff',
		secondary_bg_color: '#f4f4f5'
	},
	dark: {
		bg_color: '#212121',
		text_color: '#ffffff',
		hint_color: '#aaaaaa',
		link_color: '#8774e1',
		button_color: '#8774e1',
		button_text_color: '#ffffff',
		secondary_bg_color: '#181818'
	}
} as const;

for (const [scheme, themeParams] of Object.entries(THEMES)) {
	test.describe(`${scheme} theme`, () => {
		test.beforeEach(async ({ page }) => {
			// The Mini App reads Telegram.WebApp when it starts; this stub lets it render in a plain
			// browser with the theme under test.
			await page.addInitScript(
				({ scheme, themeParams }) => {
					const noop = () => {};
					const button = { show: noop, hide: noop, onClick: noop, offClick: noop, setText: noop };
					Object.assign(window, {
						Telegram: {
							WebApp: {
								initData: '',
								initDataUnsafe: {},
								colorScheme: scheme,
								themeParams,
								version: '9.0',
								platform: 'unknown',
								ready: noop,
								expand: noop,
								onEvent: noop,
								offEvent: noop,
								BackButton: button,
								MainButton: button,
								HapticFeedback: {
									impactOccurred: noop,
									notificationOccurred: noop,
									selectionChanged: noop
								}
							}
						}
					});
				},
				{ scheme, themeParams }
			);
			await page.emulateMedia({ colorScheme: scheme as 'light' | 'dark' });
		});

		for (const route of ROUTES) {
			test(`${route} has no WCAG 2.2 AA violation axe can find`, async ({ page }) => {
				await page.goto(route);
				const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
				expect(results.violations).toEqual([]);
			});
		}
	});
}
