import { fileURLToPath } from 'node:url';
import codspeedPlugin from '@codspeed/vitest-plugin';
import { defineConfig } from 'vitest/config';

const root = fileURLToPath(new URL('.', import.meta.url));
const app = fileURLToPath(new URL('./web/app', import.meta.url));

// SPEC-618 R9, R13: the web benches, run from the repository root by
// `pnpm exec vitest bench --run --config vitest.bench.config.ts`. Each `*.bench.ts` file under
// web/app/src is a bench file, and it runs in jsdom, since the card frame and the review screen read
// a document. The CodSpeed plugin measures the run when CodSpeed's runner drives it (the bench-web
// job); anywhere else it stands aside and Vitest's own timings are printed.
//
// As in vitest.config.ts, web/app's Vite plugins resolve their paths from the working directory, so
// the run moves into web/app before those plugins load, and the project's `dir` is set back to the
// repository root. The project runs no test file: only its benchmark include matches a file.
process.chdir(app);

export default defineConfig({
  test: {
    projects: [
      {
        extends: `${app}/vite.config.ts`,
        plugins: [codspeedPlugin()],
        test: {
          name: 'bench',
          root: app,
          dir: root,
          environment: 'jsdom',
          include: [],
          benchmark: {
            include: ['web/app/src/**/*.bench.ts']
          },
          // a bench measures its function and asserts nothing about what it returns
          expect: { requireAssertions: false }
        }
      }
    ]
  }
});
