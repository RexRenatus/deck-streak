import adapter from '@sveltejs/adapter-static';

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
    // frame-ancestors; the Caddy block's header does (SPEC-032).
    csp: {
      mode: 'hash',
      directives: {
        'script-src': ['self', 'https://telegram.org'],
        'object-src': ['none'],
        'base-uri': ['self'],
        'connect-src': ['self']
      }
    }
  }
};

export default config;
