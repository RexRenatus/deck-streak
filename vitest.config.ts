import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const root = fileURLToPath(new URL('.', import.meta.url));
const app = fileURLToPath(new URL('./web/app', import.meta.url));

// Runs web/app's tests from the repository root, so an acceptance command can name a test by its
// path from here: `pnpm exec vitest run web/app/src/lib/smoke.test.ts -t "<title>"`.
//
// web/app's Vite plugins resolve their paths from the working directory (Paraglide's project and
// output, and SvelteKit's root when no root is given), so the run moves into web/app before those
// plugins load. Vitest then matches a path filter
// against each test file's path relative to the project's `dir`, which is set back to the
// repository root, so a filter written from here still names the file.
process.chdir(app);

export default defineConfig({
  test: {
    // Coverage (SPEC-592 R9, ADR-613 D4) stays off unless a run passes --coverage. Its patterns are
    // read from the run root, which is the repository or web/app, so both spellings are listed; every
    // app source file is included, and what is not measured is listed in config/coverage.json, which
    // the report applies.
    coverage: {
      provider: 'v8',
      enabled: false,
      include: ['src/**/*.{ts,js,svelte}', 'web/app/src/**/*.{ts,js,svelte}'],
      reporter: ['lcovonly', 'json-summary', 'text-summary'],
      reportsDirectory: fileURLToPath(new URL('./target/coverage/web', import.meta.url)),
      reportOnFailure: true
    },
    projects: [
      {
        extends: `${app}/vite.config.ts`,
        test: {
          name: 'app',
          root: app,
          dir: root,
          include: ['web/app/src/**/*.{test,spec}.{js,ts}'],
          exclude: ['web/app/src/**/*.svelte.{test,spec}.{js,ts}']
        }
      }
    ]
  }
});
