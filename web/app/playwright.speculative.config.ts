// The speculative-connection measurement (#941): tests-card/speculative.measure.ts in a stock build of
// each engine the planted card suite runs, beside the suite's own Chromium headless shell. Chromium
// runs as its full build (the `chromium` channel, the real browser in headless mode), and WebKit and
// Firefox as the builds Playwright installs, the only builds it drives. One worker and no parallelism,
// as the card suite runs, since the listeners are module state of the one worker. The card suite's
// own configuration, playwright.card.config.ts, and its three projects are unchanged.
//
// SPEC-650's amendment: the full build runs in three projects. `chromium-full` is the build as it
// was; `chromium-full-twin` adds one `--disable-features` switch naming a feature no build registers;
// `chromium-full-preconnect-off` adds the same switch naming the link-preconnect feature. Playwright
// appends a project's arguments after its own switches, its own `--disable-features` among them, so
// the twin holds whatever a second switch does, and the variant differs from the twin by one feature.
// Each of the three keeps Chromium's net log at its default capture mode in test-results/netlog/,
// which the workflow uploads with the measurement, and names in its metadata the feature it records.
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.CARD_PORT ?? 4175);

/** Chromium's link preconnect on pointer-down: the feature the variant turns off, and the one each full-build project records. */
const PRECONNECT = 'AnchorElementInteraction';
/** A feature name no Chromium build registers, so the twin's switch turns nothing off. */
const UNREGISTERED = 'CardSpeculativeTwin';

/**
 * The net logs' folder, inside the uploaded results, from the working directory the workflow runs in
 * (`web/app`). The runner clears test-results before its worker starts, and the worker loads this file
 * again before it launches a browser, so the folder is there when the browser opens its log.
 */
const NETLOGS = resolve('test-results', 'netlog');
mkdirSync(NETLOGS, { recursive: true });

/** A full-build project: its net log, and a second `--disable-features` switch when `off` names a feature. */
function full(name: string, off?: string) {
  const args = [`--log-net-log=${resolve(NETLOGS, `${name}.json`)}`];
  if (off !== undefined) args.push(`--disable-features=${off}`);
  return {
    name,
    metadata: { feature: PRECONNECT },
    use: { ...devices['Desktop Chrome'], channel: 'chromium', launchOptions: { args } }
  };
}

export default defineConfig({
  testDir: 'tests-card',
  testMatch: /speculative\.measure\.ts$/,
  forbidOnly: !!process.env.CI,
  workers: 1,
  fullyParallel: false,
  timeout: 120_000,
  reporter: 'list',
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    full('chromium-full'),
    { name: 'chromium-shell', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    full('chromium-full-twin', UNREGISTERED),
    full('chromium-full-preconnect-off', PRECONNECT)
  ],
  webServer: {
    command: 'vite --config vite.card.config.ts',
    url: `http://127.0.0.1:${port}/index.html`,
    reuseExistingServer: false,
    env: { CARD_PORT: String(port) }
  }
});
