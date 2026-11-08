// The phone-size snapshots of every screen (SPEC-639 R4; ADR-662).
//
// Every route and state of the state table is compared with its committed baseline in the four
// 390-wide phone projects: Tide light and dark, at the default and at twice the text size. The
// comparison allows no margin (the config's screenshot expectation), the clock is fixed, and the
// images are taken at CSS scale. The baselines are rendered by CI and committed; CI never writes
// one.
import { ROUTES } from '../../src/lib/routes';
import { expect, open, schemeOf, test } from './fixtures';
import { STATES, slug } from './states';

for (const route of ROUTES) {
  for (const [state, opened] of Object.entries(STATES[route])) {
    test(`${route} ${state} matches its baseline`, async ({ page, colorScheme }) => {
      await open(page, route, opened, schemeOf(colorScheme));
      await expect(page).toHaveScreenshot(`${slug(route)}-${state}.png`, { fullPage: true });
    });
  }
}
