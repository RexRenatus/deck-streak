import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';

// SvelteKit's own configuration, runes mode included, lives in svelte.config.js, where
// svelte-check and the language server read it too.
export default defineConfig({
  plugins: [
    tailwindcss(),
    sveltekit(),
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/lib/paraglide',
      emitTsDeclarations: true
    }),
    // Testing Library's plugin (ADR-028) acts only under Vitest: a component test that runs in
    // jsdom gets Svelte's browser build, and what each test rendered is unmounted after it.
    svelteTesting()
  ],
  // One Vitest project, in Node; a test file that renders a component, or reads the document,
  // names jsdom in its first comment. The repository root's vitest.config.ts runs it as a project
  // too.
  test: {
    name: 'app',
    environment: 'node',
    include: ['src/**/*.{test,spec}.{js,ts}'],
    exclude: ['src/**/*.svelte.{test,spec}.{js,ts}'],
    expect: { requireAssertions: true }
  }
});
