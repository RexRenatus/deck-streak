// The planted card suite (SPEC-341 R8 to R12): tests-card/card.spec.ts in Chromium, WebKit and Firefox, one
// worker, against the card harness server (vite.card.config.ts). The listeners are module state of
// the one worker, and the suite's visits are timed, so nothing runs in parallel. Vite runs directly,
// not through `pnpm run`, as the main e2e configuration does.
import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.CARD_PORT ?? 4175);

// CI names a file here, and the run then writes its JUnit results to it beside the list
// reporter (SPEC-038 R21). Unset, as on a builder's machine, the list reporter stays alone.
const JUNIT = process.env.PLAYWRIGHT_JUNIT_OUTPUT_FILE;

export default defineConfig({
  testDir: 'tests-card',
  forbidOnly: !!process.env.CI,
  workers: 1,
  fullyParallel: false,
  timeout: 60_000,
  reporter: JUNIT ? [['list'], ['junit', { outputFile: JUNIT }]] : 'list',
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } }
  ],
  webServer: {
    command: 'vite --config vite.card.config.ts',
    url: `http://127.0.0.1:${port}/index.html`,
    reuseExistingServer: false,
    env: { CARD_PORT: String(port) }
  }
});
