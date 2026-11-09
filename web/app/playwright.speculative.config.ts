// The speculative-connection measurement (#941): tests-card/speculative.measure.ts in a stock build of
// each engine the planted card suite runs, beside the suite's own Chromium headless shell. Chromium
// runs as its full build (the `chromium` channel, the real browser in headless mode), and WebKit and
// Firefox as the builds Playwright installs, the only builds it drives. One worker and no parallelism,
// as the card suite runs, since the listeners are module state of the one worker. The card suite's
// own configuration, playwright.card.config.ts, and its three projects are unchanged.
import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.CARD_PORT ?? 4175);

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
    { name: 'chromium-full', use: { ...devices['Desktop Chrome'], channel: 'chromium' } },
    { name: 'chromium-shell', use: { ...devices['Desktop Chrome'] } },
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
