import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

const TELEGRAM_SDK = 'https://telegram.org/js/telegram-web-app.js';

function tagline(locale: string): string {
  const file = new URL(`../messages/${locale}.json`, import.meta.url);
  return JSON.parse(readFileSync(file, 'utf8')).tagline;
}

test.beforeEach(async ({ page }) => {
  // The shell loads Telegram's script from telegram.org. Answering it with an empty script keeps
  // the smoke test off the network; the script's presence and position are asserted below.
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

test('telegram-web-app.js is the first script in the head', async ({ page }) => {
  await page.goto('/');

  await expect(page.locator('head script').first()).toHaveAttribute('src', TELEGRAM_SDK);
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
