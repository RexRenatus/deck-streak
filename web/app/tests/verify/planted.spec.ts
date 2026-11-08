// The planted controls of the phone-size verification (SPEC-639 R6; ADR-662).
//
// Each gate is shown to refuse what it exists to refuse, at a phone size: a low-contrast pair and a
// small target are each refused by axe under the audit's own tags; a launch in a host palette is
// refused as not the app's own theme; a colour change of one 8 by 8 CSS pixel element (64 pixels)
// is refused by the snapshot gate; and a clean planted page passes axe and matches its baseline.
// The planted values sit in PLANTED alone, so each test's final assertion is fixed and only what
// is planted moves.
import AxeBuilder from '@axe-core/playwright';
import type { Page } from '@playwright/test';
import { THEMES } from '../telegram-palettes';
import { expect, open, schemeOf, test, tideFindings } from './fixtures';
import { STATES } from './states';

const PLANTED = {
  lowContrastInk: '#b0b0b0',
  lowContrastPaper: '#ffffff',
  smallTargetSize: 16,
  smallTargetGap: 2,
  hostPaletteEntries: Infinity
};

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA, as the theme audit runs them.
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

// The project the planted baseline is rendered in, and the colour the change gives its mark.
const SNAPSHOT_PROJECT = 'phone-light';
const CHANGED_MARK = '#c0392b';

type Results = Awaited<ReturnType<AxeBuilder['analyze']>>;

async function audit(page: Page): Promise<Results> {
  return new AxeBuilder({ page })
    .options({ rules: { 'target-size': { enabled: true } } })
    .withTags(WCAG_22_AA)
    .analyze();
}

function ran(results: Results): string[] {
  return [...results.passes, ...results.violations, ...results.incomplete, ...results.inapplicable].map(
    (rule) => rule.id
  );
}

function planted(title: string, body: string): string {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${title}</title>
<style>body { margin: 0; padding: 24px; background: #ffffff; color: #1a1a1a; font: 16px/1.5 sans-serif; }
button { border: 0; padding: 0; background: #1a1a1a; }</style></head>
<body><main><h1>${title}</h1>${body}</main></body></html>`;
}

const CLEAN = planted(
  'A clean planted page',
  '<p>A line of text at full contrast.</p><div id="mark" style="width: 8px; height: 8px; background: #1f4e79"></div>'
);

test('a planted low-contrast pair is refused by axe at a phone size', async ({ page }) => {
  const pair = `color: ${PLANTED.lowContrastInk}; background: ${PLANTED.lowContrastPaper}`;
  await page.setContent(planted('A planted pair', `<p style="${pair}">A line of planted text.</p>`));
  const results = await audit(page);
  expect(results.violations.map((v) => v.id)).toContain('color-contrast');
});

test('a planted small target is refused by axe at a phone size', async ({ page }) => {
  const size = `width: ${PLANTED.smallTargetSize}px; height: ${PLANTED.smallTargetSize}px`;
  const row = `display: flex; gap: ${PLANTED.smallTargetGap}px`;
  const targets = `<div style="${row}"><button type="button" aria-label="A planted target" style="${size}"></button>
<button type="button" aria-label="Its neighbour" style="width: 44px; height: 44px"></button></div>`;
  await page.setContent(planted('A planted target', targets));
  const results = await audit(page);
  expect(ran(results)).toContain('target-size');
  expect(results.violations.map((v) => v.id)).toContain('target-size');
});

test("a planted host palette is refused as not the app's own theme", async ({ page, colorScheme }) => {
  const scheme = schemeOf(colorScheme);
  const palette = Object.fromEntries(Object.entries(THEMES[scheme]).slice(0, PLANTED.hostPaletteEntries));
  await open(page, '/', STATES['/'].first, scheme, palette);
  expect(await tideFindings(page, scheme)).not.toEqual([]);
});

test('a planted colour change is refused by the snapshot gate', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== SNAPSHOT_PROJECT, 'the planted baseline is rendered in phone-light');
  await page.setContent(CLEAN);
  await expect(page).toHaveScreenshot('planted.png');
  await page.locator('#mark').evaluate((mark, colour) => {
    (mark as HTMLElement).style.backgroundColor = colour;
  }, CHANGED_MARK);
  await expect(expect(page).toHaveScreenshot('planted.png', { timeout: 2000 })).rejects.toThrow(/different/);
});

test('the clean planted page passes axe and matches its baseline', async ({ page }, testInfo) => {
  await page.setContent(CLEAN);
  const results = await audit(page);
  expect(ran(results)).toContain('target-size');
  expect(results.violations.map((v) => v.id)).toEqual([]);
  if (testInfo.project.name === SNAPSHOT_PROJECT) {
    await expect(page).toHaveScreenshot('planted.png');
  }
});
