import { defineConfig } from '@playwright/test';
import type { VerifyOptions } from './tests/verify/fixtures';

// The phone sizes of the verification (SPEC-639 R1; ADR-662), in CSS pixels.
const PHONE = { width: 390, height: 844 };
const SMALL_PHONE = { width: 360, height: 740 };

type Size = typeof PHONE;
type Scheme = 'light' | 'dark';

/**
 * A phone project of the verification: Chromium at `viewport` with touch, in `colorScheme`, with
 * the root text size at `textScale` times the default. Only tests/verify runs in it, and the small
 * phone runs the audits without the snapshots, which are taken at the 390-wide size.
 */
function phone(name: string, viewport: Size, colorScheme: Scheme, textScale: number) {
  return {
    name,
    testMatch: '**/verify/**/*.spec.ts',
    testIgnore: viewport === SMALL_PHONE ? '**/verify/snapshots.spec.ts' : [],
    use: { viewport, hasTouch: true, colorScheme, textScale }
  };
}

export default defineConfig<VerifyOptions>({
  testDir: 'tests',
  forbidOnly: !!process.env.CI,
  // The images a run renders, a failed comparison's included, go with the run's stage logs.
  ...(process.env.CHECK_LOG_DIR ? { outputDir: `${process.env.CHECK_LOG_DIR}/playwright` } : {}),
  // A baseline is committed with the platform in its name, and CI never writes one.
  snapshotPathTemplate: '{testDir}/verify/__screenshots__/{projectName}/{testFilePath}/{arg}-{platform}{ext}',
  updateSnapshots: process.env.CI ? 'none' : 'missing',
  // No margin: one changed pixel fails the comparison.
  expect: { toHaveScreenshot: { threshold: 0, maxDiffPixels: 0 } },
  projects: [
    { name: 'chromium', testIgnore: '**/verify/**' },
    phone('phone-light', PHONE, 'light', 1),
    phone('phone-dark', PHONE, 'dark', 1),
    phone('phone-light-text200', PHONE, 'light', 2),
    phone('phone-dark-text200', PHONE, 'dark', 2),
    phone('phone-small-light', SMALL_PHONE, 'light', 1),
    phone('phone-small-dark', SMALL_PHONE, 'dark', 1)
  ],
  // The production build, served by `vite preview`. Preview answers with SvelteKit's built server,
  // hooks included, where Caddy serves build/ as static files. Vite runs directly, not through
  // `pnpm run`, whose scripts get a process group of their own that Playwright's teardown misses.
  webServer: { command: 'vite build && vite preview', port: 4173 }
});
