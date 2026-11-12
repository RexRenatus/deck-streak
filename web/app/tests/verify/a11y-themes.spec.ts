// The phone-size audit of the app's own Tide theme (SPEC-639 R3; ADR-662).
//
// axe-core runs over every route and state of the state table in each phone project: Tide light
// and dark at 390 by 844 and 360 by 740, and at twice the text size. Before axe runs, the test
// asserts that the page is the app's own theme: the body paints the Tide background of its scheme
// and no --tg-theme-* variable is set. axe ships its target-size rule (2.5.8) turned off, so the
// audit turns it on by name and asserts that it ran. A rule is turned off only through the
// recorded exceptions in exceptions.ts.
import AxeBuilder from '@axe-core/playwright';
import { ROUTES } from '../../src/lib/routes';
import { disabledFor } from './exceptions';
import { expect, open, schemeOf, test, tideFindings } from './fixtures';
import { STATES } from './states';

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA.
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

type Results = Awaited<ReturnType<AxeBuilder['analyze']>>;

/** Every rule axe ran, whatever it found. */
function ran(results: Results): string[] {
  return [...results.passes, ...results.violations, ...results.incomplete, ...results.inapplicable].map(
    (rule) => rule.id
  );
}

for (const route of ROUTES) {
  for (const [state, opened] of Object.entries(STATES[route])) {
    test(`${route} ${state} has no WCAG 2.2 AA violation axe can find on Tide`, async ({ page, colorScheme }) => {
      const scheme = schemeOf(colorScheme);
      await open(page, route, opened, scheme);
      expect(await tideFindings(page, scheme)).toEqual([]);
      // the review's card frame is sandboxed with no script, so the legacy mode audits the screen around it
      const results = await new AxeBuilder({ page })
        .options({ rules: { 'target-size': { enabled: true } } })
        .withTags(WCAG_22_AA)
        .disableRules(disabledFor(route))
        .setLegacyMode()
        .analyze();
      expect(ran(results)).toContain('target-size');
      expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
    });
  }
}
