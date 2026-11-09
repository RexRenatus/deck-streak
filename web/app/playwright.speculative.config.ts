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
//
// The corrected variant (SPEC-650's amendment): a project that turns features off passes a single
// `--disable-features` switch, which holds Playwright's own list and the project's own names, in
// place of Playwright's switch. Two projects
// follow, each with its net log. `chromium-full-readable` is the full build as Playwright launches
// it, and records the state of each name the corrected variant turns off beside two names whose
// state Playwright sets, so its records show first whether the browser answers for a name at all.
// `chromium-full-candidates-off` drops Playwright's own `--disable-features` switch and passes one
// switch holding Playwright's list and those names, so it differs from `chromium-full` by them alone.
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.CARD_PORT ?? 4175);

/** Chromium's link preconnect on pointer-down: the feature the variant turns off, and the one each full-build project records. */
const PRECONNECT = 'AnchorElementInteraction';
/** A feature name no Chromium build registers, so the twin's switch turns nothing off. */
const UNREGISTERED = 'CardSpeculativeTwin';
/**
 * Playwright's own `--disable-features` list, as the full build's net log records the switch it
 * passes. `ignoreDefaultArgs` drops that switch only when this copy matches it exactly; the net log
 * of each corrected project records the switches the browser was given, so a drift shows there.
 */
const PLAYWRIGHT_DISABLED: readonly string[] = [
  'AvoidUnnecessaryBeforeUnloadCheckSync',
  'DestroyProfileOnBrowserClose',
  'DialMediaRouteProvider',
  'GlobalMediaControls',
  'HttpsUpgrades',
  'LensOverlay',
  'MediaRouter',
  'PaintHolding',
  'ThirdPartyStoragePartitioning',
  'BlockOriginHeaderModificationOnRedirect',
  'AvoidCorsURLLoaderRestartOnRedirect',
  'Translate',
  'AutoDeElevate',
  'OptimizationHints',
  'NetworkTimeServiceQuerying',
  'AimEnabled',
  'msForceBrowserSignIn',
  'msEdgeUpdateLaunchServicesPreferredVersion'
];
/** The registered preloading and preconnect features the corrected variant turns off together. */
const CANDIDATES: readonly string[] = [
  'NavigationPredictor',
  'PreloadingHeuristicsMLModel',
  'PreloadingEagerHoverHeuristics',
  'PreloadingModerateViewportHeuristics',
  'PreloadingEagerViewportHeuristics',
  'SpeculativeServiceWorkerWarmUp',
  'PrerenderFallbackToPreconnect',
  'PreconnectFromKeyedService',
  'PreconnectManagerDirectFastPath',
  'LoadingPredictorUseLocalPredictions',
  'LoadingPreconnectToRedirectTarget'
];
/** Two names whose state Playwright sets: one it turns off, one it turns on. */
const KNOWN: readonly string[] = ['NetworkTimeServiceQuerying', 'CDPScreenshotNewSurface'];

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

/**
 * A full-build project that records the state of each of `names`, and turns `off` off by one
 * `--disable-features` switch that keeps Playwright's own list, in place of Playwright's switch.
 */
function corrected(name: string, names: readonly string[], off: readonly string[]) {
  const args = [`--log-net-log=${resolve(NETLOGS, `${name}.json`)}`];
  const ignoreDefaultArgs: string[] = [];
  if (off.length > 0) {
    ignoreDefaultArgs.push(`--disable-features=${PLAYWRIGHT_DISABLED.join(',')}`);
    args.push(`--disable-features=${[...PLAYWRIGHT_DISABLED, ...off].join(',')}`);
  }
  return {
    name,
    metadata: { features: [...names] },
    use: { ...devices['Desktop Chrome'], channel: 'chromium', launchOptions: { args, ignoreDefaultArgs } }
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
    full('chromium-full-preconnect-off', PRECONNECT),
    corrected('chromium-full-readable', [...CANDIDATES, ...KNOWN], []),
    corrected('chromium-full-candidates-off', [...CANDIDATES, ...KNOWN], CANDIDATES)
  ],
  webServer: {
    command: 'vite --config vite.card.config.ts',
    url: `http://127.0.0.1:${port}/index.html`,
    reuseExistingServer: false,
    env: { CARD_PORT: String(port) }
  }
});
