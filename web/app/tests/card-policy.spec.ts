import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

// SPEC-341 R5, A2 (SEC01-F13). The built page's own policy, the meta element SvelteKit writes into
// the fallback page at build, holds frame-src 'none': a card frame cannot navigate itself, nor can
// any other frame. The edge serves that fallback page for every route. `vite preview` renders the
// page on SvelteKit's server instead, which sends the policy as a header and writes no meta
// element, so the test reads the page the build wrote and lets the browser parse it, inertly.
const FALLBACK = new URL('../build/index.html', import.meta.url);

test('the built page lets no frame navigate', async ({ page }) => {
  const built = readFileSync(FALLBACK, 'utf8');
  const policy = await page.evaluate(
    (html) =>
      new DOMParser()
        .parseFromString(html, 'text/html')
        .querySelector('meta[http-equiv="content-security-policy"]')
        ?.getAttribute('content') ?? null,
    built
  );
  expect(policy, 'the built page carries no meta policy').not.toBeNull();
  const directives = (policy ?? '').split(';').map((directive) => directive.trim());
  expect(directives).toContain("frame-src 'none'");
});
