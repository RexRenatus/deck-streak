// The phone-size snapshots of every screen (SPEC-639 R4; ADR-662).
//
// Every route and state of the state table is compared with its committed baseline in the four
// 390-wide phone projects: Tide light and dark, at the default and at twice the text size. The
// comparison allows no margin (the config's screenshot expectation), the clock is fixed, and the
// images are taken at CSS scale. The baselines are rendered by CI and committed; CI never writes
// one outside the render mode. In the render mode each screen is captured twice: the first
// capture writes its image, the screen is opened again, and the second must match it, so an
// input the fixtures leave unfixed is refused in the run that renders.
import { ROUTES } from '../../src/lib/routes';
import { expect, open, schemeOf, test } from './fixtures';
import { STATES, slug } from './states';

// The config's render mode, set by one CI step for the push that renders.
const RENDER = process.env.VERIFY_RENDER === '1' && Boolean(process.env.CHECK_LOG_DIR);

for (const route of ROUTES) {
  for (const [state, opened] of Object.entries(STATES[route])) {
    test(`${route} ${state} matches its baseline`, async ({ page, colorScheme }) => {
      const name = `${slug(route)}-${state}.png`;
      await open(page, route, opened, schemeOf(colorScheme));
      if (RENDER) {
        await expect.soft(page).toHaveScreenshot(name, { fullPage: true });
        await open(page, route, opened, schemeOf(colorScheme));
      }
      await expect(page).toHaveScreenshot(name, { fullPage: true });
    });
  }
}
