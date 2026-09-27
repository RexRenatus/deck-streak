import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'tests',
  forbidOnly: !!process.env.CI,
  // The production build, served by `vite preview`. Preview answers with SvelteKit's built server,
  // hooks included, where Caddy serves build/ as static files. Vite runs directly, not through
  // `pnpm run`, whose scripts get a process group of their own that Playwright's teardown misses.
  webServer: { command: 'vite build && vite preview', port: 4173 }
});
