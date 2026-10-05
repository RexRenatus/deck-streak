import { defineConfig, devices } from '@playwright/test';

// The study suite (SPEC-350 R13, A20; ADR-361): the built Mini App over the real module, which
// scripts/web-engine-stage.sh put beside the build under `/engine/`, in Chromium and WebKit, each
// test in a persistent profile of its own (study.spec.ts launches it). `vite preview` serves the
// build with vite.study.config.ts. The port is STUDY_PORT's, else 4176, beside the end-to-end
// tests' 4173, the engine's 4174 and the card's 4175; `strictPort` refuses a taken one.
const port = Number(process.env.STUDY_PORT ?? 4176);

export default defineConfig({
  testDir: 'tests-study',
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
    command: 'vite preview --config vite.study.config.ts',
    url: `http://127.0.0.1:${port}/index.html`,
    reuseExistingServer: false,
    env: { STUDY_PORT: String(port) }
  }
});
