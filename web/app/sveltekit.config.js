import adapter from '@sveltejs/adapter-static';
import { PAGE_FRAME_SRC } from './src/lib/card/policy.js';

// SvelteKit's own options, which vite.config.ts passes to its Vite plugin. SvelteKit 3 refuses a
// svelte.config.js at the Vite root, so they live in this module, where the page policy's tests and
// the card and engine harnesses read them too, and svelte-check reads them through the plugin.
/** @type {import('@sveltejs/kit/vite').Config} */
const config = {
  compilerOptions: {
    // The house rule (packs/stack-selection): runes mode everywhere, so every Svelte 4 construct
    // (`$:`, `export let`, `on:click`) is a compile error rather than a silent legacy component.
    runes: true
  },
  // A single-page app: every route is answered by the fallback page, which Caddy serves with
  // `try_files {path} {path}.html /index.html`, and the client renders the route.
  adapter: adapter({ fallback: 'index.html' }),
  // The page's own policy (SPEC-028 R14). A script runs only from this origin, from Telegram, or
  // inline when SvelteKit generated it: in hash mode SvelteKit writes the sha256 of each inline
  // script it emits into the fallback page's meta policy at build. Styles stay unrestricted,
  // because Svelte's transitions insert inline style elements. A meta policy cannot carry
  // frame-ancestors; the Caddy block's header does (SPEC-032). A frame's own navigation, one the
  // card frame starts itself included, is checked against this page's frame-src, so the card
  // module's 'none' holds every frame to the document it was given (SPEC-341 R5, SEC01-F13).
  // `wasm-unsafe-eval` admits WebAssembly compilation for the web engine (SPEC-338 R7, ADR-349), and neither `eval` nor
  // `new Function`, which `unsafe-eval` would.
  csp: {
    mode: 'hash',
    directives: {
      'script-src': ['self', 'https://telegram.org', 'wasm-unsafe-eval'],
      'object-src': ['none'],
      'base-uri': ['self'],
      'connect-src': ['self'],
      'frame-src': PAGE_FRAME_SRC
    }
  }
};

export default config;
