// Serves the Mini App's build for the study suite (SPEC-350 R13, A20; ADR-361): `vite preview` over
// `build/`, adapter-static's output, where scripts/web-engine-stage.sh put the module and its
// bindings under `engine/`. Every route the build has no file for is answered with its fallback
// page, `index.html`, as the deployed site answers it, and the page carries its own policy in that
// page's meta element. Nothing is cached, so each run loads the build and the module the stage last
// wrote.
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const port = Number(process.env.STUDY_PORT ?? 4176);

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  appType: 'spa',
  publicDir: false,
  clearScreen: false,
  build: { outDir: 'build' },
  preview: {
    host: '127.0.0.1',
    port,
    strictPort: true,
    headers: { 'Cache-Control': 'no-store' }
  }
});
