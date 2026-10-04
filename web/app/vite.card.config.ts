// The card harness server (SPEC-341 R8 to R11): Vite in development mode over tests-card/harness,
// on the loopback address and the card port. It serves each harness page with the response
// headers the suite needs, read from where the app declares them:
// - index.html: the page policy svelte.config.js declares, as SvelteKit would write it, and the
//   edge's policy header from the Caddy file, two Content-Security-Policy headers both enforced;
// - open.html, the reference frame: no policy at all;
// - variant.html: the index headers, except that off=W4 leaves frame-src out of the page policy and
//   off=scripts admits the harness's own origin as a script source for the measurement.
import { readFileSync } from 'node:fs';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, type Plugin } from 'vite';
import config from './svelte.config.js';
import { policyHeader } from './policy-header.ts';

const PORT = Number(process.env.CARD_PORT ?? 4175);
const ORIGIN = `http://127.0.0.1:${PORT}`;
const CADDY = readFileSync(new URL('../../deploy/caddy/deck-streak.caddy', import.meta.url), 'utf8');
const EDGE = /Content-Security-Policy\s+"([^"]+)"/.exec(CADDY)?.[1];
if (EDGE === undefined) throw new Error('deploy/caddy/deck-streak.caddy declares no Content-Security-Policy');
const DIRECTIVES: Readonly<Record<string, unknown>> = config.kit?.csp?.directives ?? {};

/** The page policy as a variant of the harness serves it. */
function pagePolicy(off: string | null): string {
  const directives: Record<string, unknown> = { ...DIRECTIVES };
  if (off === 'W4') delete directives['frame-src'];
  if (off === 'scripts') directives['script-src'] = [...((DIRECTIVES['script-src'] as string[] | undefined) ?? []), ORIGIN];
  return policyHeader(directives);
}

function headers(): Plugin {
  return {
    name: 'card-harness-headers',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const url = new URL(request.url ?? '/', ORIGIN);
        const page = url.pathname === '/' ? '/index.html' : url.pathname;
        if (page === '/index.html') response.setHeader('Content-Security-Policy', [pagePolicy(null), EDGE]);
        if (page === '/variant.html') response.setHeader('Content-Security-Policy', [pagePolicy(url.searchParams.get('off')), EDGE]);
        response.setHeader('Cache-Control', 'no-store');
        next();
      });
    }
  };
}

export default defineConfig({
  root: 'tests-card/harness',
  publicDir: false,
  plugins: [svelte({ configFile: false, compilerOptions: config.compilerOptions }), headers()],
  server: { host: '127.0.0.1', port: PORT, strictPort: true, hmr: false },
  optimizeDeps: { noDiscovery: true, include: [] },
  clearScreen: false,
  logLevel: 'warn'
});
