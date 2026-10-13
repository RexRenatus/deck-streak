import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { TELEGRAM_SDK, answerLaunches, launchFragment } from './launch-fragment';

function tagline(locale: string): string {
  const file = new URL(`../messages/${locale}.json`, import.meta.url);
  return JSON.parse(readFileSync(file, 'utf8')).tagline;
}

test.beforeEach(async ({ page }) => {
  // A launch loads Telegram's script from telegram.org (SPEC-400). Answering it with an empty script
  // on a launch keeps the smoke test off the network; where the script is, is asserted below.
  await page.route(`${TELEGRAM_SDK}*`, (route) =>
    route.fulfill({ contentType: 'text/javascript', body: '' })
  );
});

test('the built app renders the DeckStreak heading', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { level: 1 })).toHaveText('DeckStreak');
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.getByRole('paragraph')).toHaveText(tagline('en'));
});

test('telegram-web-app.js is in the head only when Telegram launched the page', async ({ page }) => {
  // outside a launch the head holds no element naming the script
  await page.goto('/', { waitUntil: 'networkidle' });
  await expect(page.locator(`head script[src="${TELEGRAM_SDK}"]`)).toHaveCount(0);

  // a launch, on another path, which the server accepts (SPEC-403): a navigation that changes only
  // the fragment re-runs nothing
  await answerLaunches(page, 204);
  await page.goto('/about' + launchFragment('auth_date=1&hash=synthetic'));
  const script = page.locator(`head script[src="${TELEGRAM_SDK}"]`);
  await expect(script).toHaveCount(1);
  await expect(script).toHaveAttribute('referrerpolicy', 'same-origin');
});

test('the document language follows the locale', async ({ page }) => {
  // A static host serves one fallback page, in the base locale, to everyone. The cookie is set
  // only after that page arrives, so the language has to come from the client, as it does there.
  await page.addInitScript(() => {
    document.cookie = 'PARAGLIDE_LOCALE=ja; path=/';
  });
  await page.goto('/');

  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await expect(page.getByRole('paragraph')).toHaveText(tagline('ja'));
});
