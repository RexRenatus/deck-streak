import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const root = fileURLToPath(new URL('.', import.meta.url));
const app = fileURLToPath(new URL('./web/app', import.meta.url));

// Runs web/app's tests from the repository root, so an acceptance command can name a test by its
// path from here: `pnpm exec vitest run web/app/src/lib/smoke.test.ts -t "<title>"`.
//
// SvelteKit's Vite plugin reads svelte.config.js from the working directory and roots Vite there,
// so the run moves into web/app before that plugin loads. Vitest then matches a path filter
// against each test file's path relative to the project's `dir`, which is set back to the
// repository root, so a filter written from here still names the file.
process.chdir(app);

export default defineConfig({
  test: {
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
