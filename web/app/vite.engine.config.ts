// Serves the web engine's harness page for its browser tests (SPEC-338 R9, ADR-348): the page and
// the engine's Worker from `engine-harness/`, the module the build wrote from
// `../../target/web-engine/` under `/engine/`, and every response under the page's own policy,
// read from svelte.config.js, so a test that loads the module proves the policy admits it.
import { randomBytes } from 'node:crypto';
import { createReadStream, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';
import { policyHeader } from './policy-header.ts';
import svelteConfig from './svelte.config.js';

/** The directory `scripts/web-engine-build.sh --out target/web-engine` writes. */
const ENGINE_DIR = fileURLToPath(new URL('../../target/web-engine/', import.meta.url));

/** The page policy as a header, without `frame-src`: this harness's cross-site frame test frames
 * the harness in itself, which the page's `frame-src 'none'` would refuse (SPEC-341 R13). */
const directives = svelteConfig.kit?.csp?.directives;
if (directives === undefined) throw new Error('svelte.config.js names no kit.csp.directives');
const engineDirectives: Record<string, unknown> = { ...directives };
delete engineDirectives['frame-src'];
const POLICY = policyHeader(engineDirectives);

/** The engine's files, by name, with the type each is served as. */
const TYPES: Record<string, string> = {
  'deck_streak_web_engine.js': 'text/javascript',
  'deck_streak_web_engine_bg.wasm': 'application/wasm'
};

/** Serves the built module and its bindings under `/engine/`, never cached, so each run loads
 * the module the build last wrote. */
function engineFiles(): Plugin {
  return {
    name: 'deck-streak-engine-files',
    configureServer(server) {
      server.middlewares.use('/engine', (request, response, next) => {
        const name = (request.url ?? '').split('?')[0].replace(/^\//, '');
        const type = TYPES[name];
        if (type === undefined) return next();
        const file = ENGINE_DIR + name;
        if (!existsSync(file)) {
          response.statusCode = 404;
          response.end(`the build wrote no ${name}: run scripts/web-engine-build.sh first`);
          return;
        }
        response.setHeader('Content-Type', type);
        response.setHeader('Cache-Control', 'no-store');
        response.setHeader('Content-Security-Policy', POLICY);
        createReadStream(file).pipe(response);
      });
    }
  };
}

/** The route the Worker's sync key goes to (credential.ts's `SYNC_ROUTE`), and the route a
 * redirected answer moves to; the proxy forwards both to the engine's own sync server, each prefix
 * stripped (SPEC-364 B1 to B3, ADR-375 D20). */
const SYNC_ROUTE = '/anki-sync/';
const MOVED_ROUTE = '/anki-sync-moved/';
/** The service's release of a sealing key (credential.ts's `RELEASE_ROUTE`), which this server
 * stands in for. */
const RELEASE_ROUTE = '/api/sync/seal-key';
/** The tests' own routes: `POST mode/<mode>` sets how the sync route answers and empties the
 * recorder; `GET seen` answers the sync paths the recorder saw since. */
const TEST_ROUTE = '/test-sync/';
/** The route where the service states the oldest client level it accepts (sync.ts's
 * `STATEMENT_ROUTE`), and the route a redirected statement moves to (SPEC-374 R22). */
const STATEMENT_ROUTE = '/api/sync/minimum-client';
const STATEMENT_MOVED = '/api/sync/minimum-client-moved';
/** How the statement route answers: a level the client meets, a level above any client, or a
 * same-origin 307 whose target would admit, so a followed redirect shows. */
const STATEMENTS = ['admit', 'below', 'moved'] as const;
type Statement = (typeof STATEMENTS)[number];

/** How the sync route answers: forwarded to the server, refused with a 403, moved by a
 * same-origin 307, or its connection dropped with no answer. */
const MODES = ['pass', 'forbid', 'redirect', 'drop'] as const;
type Mode = (typeof MODES)[number];

/** The service's snapshot answer (choice.ts's `SNAPSHOT_ROUTE`, SPEC-377 R12), which this server
 * stands in for until part c2 serves it, and how it answers: a sealed snapshot found an hour ago,
 * none found, a refusal with a 401, or its connection dropped with no answer. `POST
 * snapshot/<answer>` sets it, and `POST mode/<mode>` sets it back to none found. */
const SNAPSHOT_ROUTE = '/api/sync/snapshot';
const SNAPSHOTS = ['found', 'missing', 'refused', 'drop'] as const;
type Snapshot = (typeof SNAPSHOTS)[number];

/** The sync tests' routes, served before Vite's own middlewares and its proxy: the modes of the
 * sync route, the recorder of the sync paths seen, and a stand-in release that answers one random
 * 32-byte key per seal id, drawn when that seal id is first asked for. */
function syncRoutes(): Plugin {
  let mode: Mode = 'pass';
  let statement: Statement = 'admit';
  let snapshot: Snapshot = 'missing';
  const seen: string[] = [];
  const statementAsked: string[] = [];
  const released = new Map<string, string>();
  return {
    name: 'deck-streak-sync-routes',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const url = request.url ?? '';
        const path = url.split('?')[0];
        if (request.method === 'POST' && path.startsWith(`${TEST_ROUTE}mode/`)) {
          const asked = path.slice(`${TEST_ROUTE}mode/`.length);
          const known = MODES.find((each) => each === asked);
          if (known === undefined) {
            response.statusCode = 400;
            response.end(`no sync mode ${asked}: one of ${MODES.join(', ')}`);
            return;
          }
          mode = known;
          seen.length = 0;
          statement = 'admit';
          statementAsked.length = 0;
          snapshot = 'missing';
          response.end(mode);
          return;
        }
        if (request.method === 'GET' && path === `${TEST_ROUTE}seen`) {
          response.setHeader('Content-Type', 'application/json');
          response.end(JSON.stringify(seen));
          return;
        }
        if (request.method === 'POST' && path === RELEASE_ROUTE) {
          const chunks: Buffer[] = [];
          request.on('data', (chunk: Buffer) => chunks.push(chunk));
          request.on('end', () => {
            let seal: unknown;
            try {
              seal = (JSON.parse(Buffer.concat(chunks).toString('utf8')) as { seal_id?: unknown }).seal_id;
            } catch {
              seal = undefined;
            }
            if (typeof seal !== 'string' || seal === '') {
              response.statusCode = 400;
              response.end('a release names one seal_id');
              return;
            }
            let key = released.get(seal);
            if (key === undefined) {
              key = randomBytes(32).toString('base64url');
              released.set(seal, key);
            }
            response.setHeader('Content-Type', 'application/json');
            response.setHeader('Cache-Control', 'no-store');
            response.end(JSON.stringify({ key }));
          });
          return;
        }
        if (request.method === 'POST' && path.startsWith(`${TEST_ROUTE}statement/`)) {
          const wanted = path.slice(`${TEST_ROUTE}statement/`.length);
          const known = STATEMENTS.find((each) => each === wanted);
          if (known === undefined) {
            response.statusCode = 400;
            response.end(`no statement ${wanted}: one of ${STATEMENTS.join(', ')}`);
            return;
          }
          statement = known;
          statementAsked.length = 0;
          response.end(statement);
          return;
        }
        if (request.method === 'POST' && path.startsWith(`${TEST_ROUTE}snapshot/`)) {
          const wanted = path.slice(`${TEST_ROUTE}snapshot/`.length);
          const known = SNAPSHOTS.find((each) => each === wanted);
          if (known === undefined) {
            response.statusCode = 400;
            response.end(`no snapshot answer ${wanted}: one of ${SNAPSHOTS.join(', ')}`);
            return;
          }
          snapshot = known;
          response.end(snapshot);
          return;
        }
        if (request.method === 'GET' && path === SNAPSHOT_ROUTE) {
          if (snapshot === 'drop') {
            request.socket.destroy();
            return;
          }
          response.setHeader('Content-Type', 'application/json');
          response.setHeader('Cache-Control', 'no-store');
          if (snapshot === 'refused') {
            response.statusCode = 401;
            response.end(JSON.stringify({ error: 'no owner session' }));
            return;
          }
          response.end(JSON.stringify(snapshot === 'found' ? { found: true, age_seconds: 3600 } : { found: false }));
          return;
        }
        if (request.method === 'GET' && path === `${TEST_ROUTE}asked`) {
          response.setHeader('Content-Type', 'application/json');
          response.end(JSON.stringify(statementAsked));
          return;
        }
        if (request.method === 'GET' && (path === STATEMENT_ROUTE || path === STATEMENT_MOVED)) {
          statementAsked.push(path);
          if (path === STATEMENT_ROUTE && statement === 'moved') {
            response.statusCode = 307;
            response.setHeader('Location', STATEMENT_MOVED);
            response.end();
            return;
          }
          const level = path === STATEMENT_ROUTE && statement === 'below' ? 9007199254740991 : 1;
          response.setHeader('Content-Type', 'application/json');
          response.setHeader('Cache-Control', 'no-store');
          response.end(JSON.stringify({ minimum_client_level: level }));
          return;
        }
        if (path.startsWith(SYNC_ROUTE) || path.startsWith(MOVED_ROUTE)) seen.push(path);
        if (path.startsWith(SYNC_ROUTE) && mode === 'forbid') {
          response.statusCode = 403;
          response.end('forbidden');
          return;
        }
        if (path.startsWith(SYNC_ROUTE) && mode === 'redirect') {
          response.statusCode = 307;
          response.setHeader('Location', MOVED_ROUTE + url.slice(SYNC_ROUTE.length));
          response.end();
          return;
        }
        if (path.startsWith(SYNC_ROUTE) && mode === 'drop') {
          request.socket.destroy();
          return;
        }
        next();
      });
    }
  };
}

const port = Number(process.env.ENGINE_PORT ?? 4174);
/** The engine's sync server's port, which playwright.engine.config.ts starts it on. */
const syncPort = Number(process.env.ENGINE_SYNC_PORT ?? 4177);
/** The loopback host this server listens on, and the sync server's. */
const HOST = '127.0.0.1';

/** Forwards `route` to the engine's sync server with its prefix stripped. */
function toSyncServer(route: string) {
  return { target: `http://${HOST}:${syncPort}`, rewrite: (path: string) => '/' + path.slice(route.length) };
}

export default defineConfig({
  root: fileURLToPath(new URL('./engine-harness/', import.meta.url)),
  plugins: [engineFiles(), syncRoutes()],
  clearScreen: false,
  optimizeDeps: { noDiscovery: true, include: [] },
  server: {
    host: HOST,
    port,
    strictPort: true,
    hmr: false,
    headers: { 'Content-Security-Policy': POLICY, 'Cache-Control': 'no-store' },
    proxy: { [SYNC_ROUTE]: toSyncServer(SYNC_ROUTE), [MOVED_ROUTE]: toSyncServer(MOVED_ROUTE) }
  }
});
