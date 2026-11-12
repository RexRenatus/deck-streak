import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-643 R5, A1 and A2; ADR-666 D5. The passkey list's audit (tests/sign-in-methods-a11y.spec.ts)
// runs axe-core over the sign-in methods screen with the list answered in the page, once with every
// row closed and once with a confirm step open, in each of Telegram's colour schemes. The test probe
// resolves no Playwright command, so this test holds that spec's shape instead, as
// a11y-coverage.test.ts holds the app's audit and voice.test.ts the voice picker's: the palettes,
// the list's answer, the one visit, each state's steps in order and the app's tags. An audit planted
// without a state's own step, or with its list answered elsewhere, is refused.

/** web/app/tests, where the browser audits live. */
const TESTS = resolve(import.meta.dirname, '../../../tests');
const AUDIT = resolve(TESTS, 'sign-in-methods-a11y.spec.ts');
const APP_AUDIT = resolve(TESTS, 'a11y.spec.ts');

/** The two tests of the audit, by title: the rows closed, and a confirm step open. */
const CLOSED = 'the passkey list with every row closed has no WCAG 2.2 AA violation axe can find';
const OPEN = 'the passkey list with a confirm step open has no WCAG 2.2 AA violation axe can find';

/** Each state's steps, in the order its test takes them. */
const CLOSED_STEPS = [
  'await listed(page, scheme)',
  'toHaveCount(0)',
  '.analyze()',
  'expect(results.violations).toEqual([])'
];
const OPEN_STEPS = [
  'await listed(page, scheme)',
  '.click()',
  'toBeFocused()',
  '.analyze()',
  'expect(results.violations).toEqual([])'
];

function examined(what: string, count: number): void {
  console.log(`examined ${count} ${what}`);
  expect(count, `examined 0 ${what}: nothing was judged`).toBeGreaterThan(0);
}

/** The audit's text, once it is asserted to exist. */
function audit(): string {
  expect(existsSync(AUDIT), 'tests/sign-in-methods-a11y.spec.ts, the passkey list audit, is absent').toBe(true);
  return readFileSync(AUDIT, 'utf8');
}

/** The text of the test titled `title`, up to the next test or the end; empty when it is absent. */
function body(spec: string, title: string): string {
  const start = spec.indexOf(`test('${title}'`);
  if (start < 0) return '';
  const next = spec.indexOf('test(', start + 1);
  return next < 0 ? spec.slice(start) : spec.slice(start, next);
}

/** The axe-core tags an audit names, as voice.test.ts reads them. */
function tags(text: string): string | undefined {
  return /const WCAG_22_AA = (\[[^\]]*\]);/.exec(text)?.[1];
}

/** Each mark both states share, with whether `spec` holds it. */
function sharedMarks(spec: string): [string, boolean][] {
  const appTags = tags(readFileSync(APP_AUDIT, 'utf8'));
  const visits = spec.match(/page\.goto\(/g) ?? [];
  return [
    ['the palette module import', /import\s*\{\s*THEMES\s*\}\s*from\s*'\.\/telegram-palettes'/.test(spec)],
    [
      'the palette loop',
      /for\s*\(\s*const\s+\[\s*scheme\s*,\s*themeParams\s*\]\s+of\s+Object\.entries\(\s*THEMES\s*\)\s*\)/.test(spec)
    ],
    ['the emulated colour scheme', /emulateMedia\(\s*\{\s*colorScheme:\s*scheme/.test(spec)],
    ['the list answered in the page', spec.includes("'**/api/identities'")],
    ['two passkeys on the list', (spec.match(/kind:\s*'passkey'/g) ?? []).length >= 2],
    ['one visit, to the sign-in methods screen', visits.length === 1 && spec.includes("page.goto('/sign-in-methods')")],
    ['axe with the tags', /new AxeBuilder\(\{\s*page\s*\}\)\s*\.withTags\(WCAG_22_AA\)/.test(spec)],
    ["the app audit's tags", appTags !== undefined && tags(spec) === appTags]
  ];
}

/** The shared marks `spec` lacks: empty when it holds them all. */
function shared(spec: string): string[] {
  return sharedMarks(spec)
    .filter(([, held]) => !held)
    .map(([name]) => name);
}

/** The marks the test titled `title` lacks, each looked for after the one before it: empty when it holds them all. */
function steps(spec: string, title: string, marks: readonly string[]): string[] {
  const text = body(spec, title);
  const lacking: string[] = [];
  let at = 0;
  for (const mark of marks) {
    const found = text.indexOf(mark, at);
    if (found < 0) lacking.push(`${mark} in "${title}"`);
    else at = found + mark.length;
  }
  return lacking;
}

describe('the passkey list browser audit', () => {
  it('the passkey list audit runs axe over the list with every row closed in both colour schemes', () => {
    const spec = audit();
    examined('marks', sharedMarks(spec).length + CLOSED_STEPS.length);
    expect(shared(spec)).toEqual([]);
    expect(steps(spec, CLOSED, CLOSED_STEPS)).toEqual([]);

    // planted audits: one without the closed-row step, one with the list answered elsewhere
    expect(steps(spec.replace('toHaveCount(0)', ''), CLOSED, CLOSED_STEPS)).not.toEqual([]);
    expect(shared(spec.replace("'**/api/identities'", "'**/api/elsewhere'"))).not.toEqual([]);
  });

  it('the passkey list audit runs axe over the list with a confirm step open in both colour schemes', () => {
    const spec = audit();
    examined('marks', sharedMarks(spec).length + OPEN_STEPS.length);
    expect(shared(spec)).toEqual([]);
    expect(steps(spec, OPEN, OPEN_STEPS)).toEqual([]);

    // planted audits: one without the press that opens the step, one with the list answered elsewhere
    expect(steps(spec.replace('.click()', ''), OPEN, OPEN_STEPS)).not.toEqual([]);
    expect(shared(spec.replace("'**/api/identities'", "'**/api/elsewhere'"))).not.toEqual([]);
  });
});
