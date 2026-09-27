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
    adapter: adapter({ fallback: 'index.html' })
  }
};

export default config;
