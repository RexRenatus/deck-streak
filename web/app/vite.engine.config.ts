// Serves the web engine's harness page for its browser tests (SPEC-338 R9, ADR-348): the page and
// the engine's Worker from `engine-harness/`, the module the build wrote from
// `../../target/web-engine/` under `/engine/`, and every response under the page's own policy,
// read from svelte.config.js, so a test that loads the module proves the policy admits it.
import { createReadStream, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';
import svelteConfig from './svelte.config.js';

/** The directory `scripts/web-engine-build.sh --out target/web-engine` writes. */
const ENGINE_DIR = fileURLToPath(new URL('../../target/web-engine/', import.meta.url));

/** The policy's keywords, which a header writes quoted; a host or scheme is written bare. */
const KEYWORDS = new Set([
  'self',
  'none',
  'unsafe-inline',
  'unsafe-eval',
  'wasm-unsafe-eval',
  'unsafe-hashes',
  'strict-dynamic',
  'report-sample'
]);

/** The policy header SvelteKit's directives describe, keywords quoted as a header needs them. */
export function policyHeader(directives: Record<string, string[]>): string {
  return Object.entries(directives)
    .map(([name, values]) =>
      [name, ...values.map((value) => (KEYWORDS.has(value) ? `'${value}'` : value))].join(' ')
    )
    .join('; ');
}

const directives = svelteConfig.kit?.csp?.directives;
if (directives === undefined) throw new Error('svelte.config.js names no kit.csp.directives');
const POLICY = policyHeader(directives as Record<string, string[]>);

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

const port = Number(process.env.ENGINE_PORT ?? 4174);

export default defineConfig({
  root: fileURLToPath(new URL('./engine-harness/', import.meta.url)),
  plugins: [engineFiles()],
  clearScreen: false,
  optimizeDeps: { noDiscovery: true, include: [] },
  server: {
    host: '127.0.0.1',
    port,
    strictPort: true,
    hmr: false,
    headers: { 'Content-Security-Policy': POLICY, 'Cache-Control': 'no-store' }
  }
});
