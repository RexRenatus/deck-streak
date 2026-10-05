import { defineConfig, devices } from '@playwright/test';

// The web engine's browser tests (SPEC-338 R9): Chromium and WebKit, each test in a persistent
// profile of its own, as a user's browser has (engine.spec.ts launches it). The harness is served
// by Vite with vite.engine.config.ts over the module scripts/web-engine-build.sh wrote. The port
// is ENGINE_PORT's, else 4174, beside the end-to-end tests' 4173; `strictPort` refuses a taken one.
const port = Number(process.env.ENGINE_PORT ?? 4174);

export default defineConfig({
  testDir: 'tests-engine',
  forbidOnly: !!process.env.CI,
  // One browser at a time: each test owns the origin's collection and its Web Lock.
  workers: 1,
  fullyParallel: false,
  timeout: 120_000,
  reporter: 'list',
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } }
  ],
  // Vite runs directly, not through `pnpm run`, whose scripts get a process group of their own
  // that Playwright's teardown misses.
  webServer: {
    command: 'vite --config vite.engine.config.ts',
    url: `http://127.0.0.1:${port}/index.html`,
    reuseExistingServer: false,
    env: { ENGINE_PORT: String(port) }
  }
});
