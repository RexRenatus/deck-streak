import { expect, test } from '@playwright/test';

// SPEC-341 R5, A2 (SEC01-F13). The built page's own policy, the meta element SvelteKit writes at
// build, holds frame-src 'none': a card frame cannot navigate itself, nor can any other frame.
const TELEGRAM_SDK = 'https://telegram.org/js/telegram-web-app.js';

test('the built page lets no frame navigate', async ({ page }) => {
  // Telegram's script is answered with an empty one, so the page stays off the network.
  await page.route(`${TELEGRAM_SDK}*`, (route) =>
    route.fulfill({ contentType: 'text/javascript', body: '' })
  );
  await page.goto('/');

  const policy = await page.locator('meta[http-equiv="content-security-policy"]').getAttribute('content');
  const directives = (policy ?? '').split(';').map((directive) => directive.trim());
  expect(directives).toContain("frame-src 'none'");
});
