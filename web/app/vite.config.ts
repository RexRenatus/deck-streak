import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { configDefaults, defineConfig } from 'vitest/config';
import { writeTokens } from './scripts/build-tokens.ts';
import config from './sveltekit.config.js';

// CI's web job names a file here, and the run then writes its JUnit results to it beside
// Vitest's own reporters (SPEC-038 R21). Unset, as on a builder's machine, the reporters stay
// Vitest's own.
const junit = process.env.VITEST_JUNIT_FILE;

// SvelteKit's own configuration, runes mode included, lives in sveltekit.config.js and reaches its
// Vite plugin here, where svelte-check and the language server read it too.
export default defineConfig({
  plugins: [
    // The design tokens compile into src/lib/design/tokens.css when a build or the dev server
    // starts (ADR-028). Every buildStart hook finishes before any module is transformed, so the
    // stylesheet exists when Tailwind resolves layout.css's import of it.
    { name: 'deck-streak-tokens', buildStart: writeTokens },
    tailwindcss(),
    sveltekit(config),
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
    expect: { requireAssertions: true },
    ...(junit
      ? { reporters: [...configDefaults.reporters, ['junit', { outputFile: junit }]] }
      : {})
  }
});
