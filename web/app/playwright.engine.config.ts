import { randomBytes } from 'node:crypto';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { defineConfig, devices } from '@playwright/test';

// The web engine's browser tests (SPEC-338 R9): Chromium and WebKit, each test in a persistent
// profile of its own, as a user's browser has (engine.spec.ts launches it). The harness is served
// by Vite with vite.engine.config.ts over the module scripts/web-engine-build.sh wrote. The port
// is ENGINE_PORT's, else 4174, beside the end-to-end tests' 4173; `strictPort` refuses a taken one.
const port = Number(process.env.ENGINE_PORT ?? 4174);

/** The loopback host both servers listen on, and the only one the tests reach. */
const HOST = '127.0.0.1';

// The engine's own sync server, which the sync tests reach through Vite's proxy (SPEC-364 B1 to
// B3, ADR-375 D20). Its port is ENGINE_SYNC_PORT's, else 4177, beside the card's 4175 and the
// study suite's 4176. Its users are synthetic, one per project (Chromium's the first, WebKit's the
// second, so neither project's uploads reach the other's server collection; SPEC-377 A1 to A3),
// built here from parts with a password drawn for the run, and its base is an empty directory made
// for the run. Playwright loads this file again in each test worker, so each value is set once,
// by the first process, and the workers inherit it through their environment.
const syncPort = Number(process.env.ENGINE_SYNC_PORT ?? 4177);
process.env.ENGINE_SYNC_USER ??= ['engine', 'sync', 'tester'].join('-');
process.env.ENGINE_SYNC_PASSWORD ??= randomBytes(18).toString('base64url');
process.env.ENGINE_SYNC_BASE ??= mkdtempSync(join(tmpdir(), 'deck-streak-engine-sync-'));

export default defineConfig({
  testDir: 'tests-engine',
  forbidOnly: !!process.env.CI,
  // One browser at a time: each test owns the origin's collection and its Web Lock.
  workers: 1,
  fullyParallel: false,
  timeout: 120_000,
  reporter: 'list',
  use: { baseURL: `http://${HOST}:${port}` },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } }
  ],
  // Vite runs directly, not through `pnpm run`, whose scripts get a process group of their own
  // that Playwright's teardown misses.
  webServer: [
    {
      command: 'vite --config vite.engine.config.ts',
      url: `http://${HOST}:${port}/index.html`,
      reuseExistingServer: false,
      env: { ENGINE_PORT: String(port), ENGINE_SYNC_PORT: String(syncPort) }
    },
    {
      // The engine's sync server, ready when it answers its health route. The first run compiles
      // the native engine, so the wait is sized for a cold build (ADR-375 D20).
      command: 'cargo run --locked -p deck-streak-engine-core --example sync_server',
      url: `http://${HOST}:${syncPort}/health`,
      reuseExistingServer: false,
      timeout: 1_200_000,
      env: {
        SYNC_USER1: `${process.env.ENGINE_SYNC_USER}:${process.env.ENGINE_SYNC_PASSWORD}`,
        SYNC_USER2: `${process.env.ENGINE_SYNC_USER}-webkit:${process.env.ENGINE_SYNC_PASSWORD}`,
        SYNC_HOST: HOST,
        SYNC_PORT: String(syncPort),
        SYNC_BASE: process.env.ENGINE_SYNC_BASE
      }
    }
  ]
});
