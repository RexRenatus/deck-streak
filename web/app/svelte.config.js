import adapter from '@sveltejs/adapter-static';
import { PAGE_FRAME_SRC } from './src/lib/card/policy.js';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  compilerOptions: {
    // The house rule (packs/stack-selection): runes mode everywhere, so every Svelte 4 construct
    // (`$:`, `export let`, `on:click`) is a compile error rather than a silent legacy component.
    runes: true
  },
  kit: {
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
    csp: {
      mode: 'hash',
      directives: {
        'script-src': ['self', 'https://telegram.org'],
        'object-src': ['none'],
        'base-uri': ['self'],
        'connect-src': ['self'],
        'frame-src': PAGE_FRAME_SRC
      }
    },
    // svelte-check reads the TypeScript project SvelteKit generates, which covers src and tests;
    // the card harness and its two configurations are type-checked too (SPEC-341). Paths are
    // relative to the generated .svelte-kit/tsconfig.json.
    typescript: {
      config: (tsconfig) => {
        tsconfig.include.push(
          '../tests-card/**/*.ts',
          '../vite.card.config.ts',
          '../playwright.card.config.ts',
          '../policy-header.ts'
        );
      }
    }
  }
};

export default config;
