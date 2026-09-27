import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';
import { writeTokens } from './scripts/build-tokens.ts';

// SvelteKit's own configuration, runes mode included, lives in svelte.config.js, where
// svelte-check and the language server read it too.
export default defineConfig({
  plugins: [
    // The design tokens compile into src/lib/design/tokens.css when a build or the dev server
    // starts (ADR-028). Every buildStart hook finishes before any module is transformed, so the
    // stylesheet exists when Tailwind resolves layout.css's import of it.
    { name: 'deck-streak-tokens', buildStart: writeTokens },
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
