# SPEC-628: the web type check reads the engine suite, its harness and its two configurations, and a census holds every suite in it

- **Issue:** none. Refers SPEC-338 R9 (the engine suite and its harness page), SPEC-341 and
  SPEC-350 (the card and study suites' entries in the project), and amends none of them.
  **Context(s):** the web app's type-check project (`web/app/tsconfig.json`) and its tests; no
  workflow, script, dependency or lockfile changes.
- **Decided by:** ADR-650 (the engine suite, its harness and its two configurations join the one
  type-check project by name).
- **Schematic:** `docs/schematics/web-type-check-project.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-628.md`. **Mutation band:** none: no row, no production path changes.
  **Models:** none.

## 1. The problem, measured

`pnpm --dir web/app check` runs `svelte-check --tsconfig ./tsconfig.json --fail-on-warnings`
(`web/app/package.json` line 12), and svelte-check diagnoses only the files its project's
`include` list names. At the base of this pull request, that list (`web/app/tsconfig.json` lines
18-31) names the card suite with its two configurations (lines 24-26, SPEC-341) and the study
suite with its two (lines 28-30, SPEC-350), but no file of the engine suite:

- `git show <base>:web/app/tsconfig.json | grep -c -E 'tests-engine/|playwright.engine.config.ts'`
  reads `0`;
- the list names none of `web/app/tests-engine/` (`engine.spec.ts`, `sync.spec.ts`),
  `web/app/engine-harness/main.ts` (the harness page's script, whose `Harness` type both specs
  import), `web/app/vite.engine.config.ts` (the server the suite runs on) or
  `web/app/playwright.engine.config.ts`.

The engine suite runs only in the `web-engine` job, by
`playwright test --config playwright.engine.config.ts` (`web/app/package.json` line 16), and
Playwright strips TypeScript types without checking them. So a type error in those five files
failed no check, and reached CI only when the suite ran, if at all. Measured at the base:

- the check over the project read 0 errors and 0 warnings over 2058 files;
- with `export const plantedTypeError: number = 'planted';` appended to each of the five files,
  the check still read 0 errors and 0 warnings: the project read none of them;
- with R1's four entries added, the check read 0 errors and 0 warnings over 2063 files, the five
  files more: the widened reading reported no diagnostic in the suite, its harness page or its
  configurations, so none of the five files changes (R2).

## 2. Requirements

- **R1.** `web/app/tsconfig.json` gains exactly five lines, inserted directly after line 27
  (`"policy-header.ts",`), in this order, each indented four spaces: the comment line
  `// The engine suite, its harness page and its two configurations (SPEC-628).`, then
  `"tests-engine/**/*.ts",`, `"engine-harness/**/*.ts",`, `"vite.engine.config.ts",` and
  `"playwright.engine.config.ts",`. No other line changes: `extends`, every compiler option, the
  comment at lines 15-17 and every other entry stay as they are. The file has 37 lines, and
  `grep -c -E 'tests-engine/|playwright.engine.config.ts'` over it reads `2`.
- **R2.** The widened project type-checks: `pnpm --dir web/app check` reads 0 errors and 0
  warnings over it. A diagnostic the widening reports in one of the five engine files is cured in
  that file by a type-level change alone (a type annotation, `import type`, a non-null assertion
  or an `as` cast to a named type), so that the file's JavaScript, transpiled once with comments
  removed and once with comments kept, is byte-equal before and after; no `any`, `as unknown`,
  `@ts-` directive or lint suppression is added, no comment is added or changed, and no compiler
  option, svelte-check flag or `exclude` entry changes for it. At the base the widening reported
  no diagnostic, so no engine file changes.
- **R3.** A census test holds the class. Every top-level entry of `web/app` that is a directory
  named `src`, `tests`, `tests-<name>` or `<name>-harness`, or a file whose name matches
  `^[a-z][a-z.-]*\.(ts|js)$` and does not end `.d.ts`, is named by the `include` list: a directory
  bare, as `<dir>/**/*.ts` or as `<dir>/**/*`, a file by its exact name. The census reads the list
  by TypeScript's own configuration parser (comments allowed), prints
  `examined <n> top-level members` and refuses a zero count, and names, sorted, each member the
  list omits. At the base it examines 16 members (`src`, `tests`, `tests-card`, `tests-engine`,
  `tests-study`, `engine-harness` and the ten top-level `.ts` and `.js` files) and omits
  `engine-harness`, `playwright.engine.config.ts`, `tests-engine` and `vite.engine.config.ts`.
- **R4.** Nothing else changes: no Playwright test, configuration value, script, workflow,
  dependency or lockfile; the engine suite's behaviour is unchanged (R2); `web/app/scripts` (the
  token compile and the docs recorder) stays outside the project and outside the census
  (section 5).

## 3. Acceptance criteria of SPEC-628

Both are top-level tests of `web/app/src/lib/type-check-project.test.ts`, in Vitest's node
environment, each command run from the repository root.

| id | criterion | decided by |
|---|---|---|
| A1 | the type-check project names every browser suite, harness and configuration: the members R3 selects from the `web/app` listing are counted, a zero count refused, the `include` list omits none of them, and a planted `tests-planted` directory beside them is named as the one member omitted | `pnpm exec vitest run web/app/src/lib/type-check-project.test.ts -t "the type-check project names every browser suite, harness and configuration"` |
| A2 | the census names a member the project omits: over a planted listing, R3's selection keeps exactly a planted suite, harness, Vite and Playwright configuration and `src`, and each of four planted `include` lists names, sorted, exactly the members it leaves out | `pnpm exec vitest run web/app/src/lib/type-check-project.test.ts -t "the census names a member the project omits"` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/type-check-project.test.ts -t "the type-check project names every browser suite, harness and configuration"
A2: pnpm exec vitest run web/app/src/lib/type-check-project.test.ts -t "the census names a member the project omits"
```

A1 is red at the base by its assertion: the census examines 16 members and names the four R3
lists as omitted. A2 is not red: it guards the census's own selection and forms, which the red
commit already holds.

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/tsconfig.json` | type-check project | changed: the five lines of R1 |
| `web/app/src/lib/type-check-project.test.ts` | tests | added: A1 and A2 |
| `docs/specs/SPEC-628-the-web-type-check-reads-the-engine-suite-its-harness-and-its-two-configurations-and-a-census-holds-every-suite-in-it.md` | records | added: this SPEC |
| `docs/decisions/ADR-650-the-engine-suite-its-harness-and-its-two-configurations-join-the-one-type-check-project-by-name.md` | records | added: its decision |
| `docs/schematics/web-type-check-project.md` | records | added: the project's component and data flow |
| `docs/red-first/SPEC-628.md` | records | added: the red-first record |
| `changelog.d/web-type-check-engine-628.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- `web/app/scripts`, the token compile and the docs recorder: its scripts run in Node beside the
  app, and none is a browser suite, harness page or configuration, so it stays outside the
  project and outside the census.
- Any type check inside a Playwright run: the `web` job's check holds the suites before any of
  them runs.
- Any compiler option, svelte-check flag or `exclude` entry.
- The suites' behaviour: no test, assertion, wait or server setting of the card, study or engine
  suite changes.

## 6. Risks

- **A later top-level suite, harness or configuration** reads A1 red until it joins the project,
  which is the intent: the census names it.
- **A type error in the engine suite** now fails the `web` job's check before any test runs,
  where it used to pass the check and surface, if at all, only when Playwright ran the suite.
- **An open change to a file of the engine suite** must type-check when it lands: a change written
  while the project did not read the suite can read red in the check at its merge.

## 7. What only CI proves

- The `web` job's check over the widened project at the delivery head, reading 0 errors and 0
  warnings.
- The census over CI's checkout: A1 and A2 run in the `web` job, which runs the whole Vitest
  suite, and every red and green is recorded in `docs/red-first/SPEC-628.md`.

## 8. Formal model

None: the change adds four entries to a type-check project's file list and a census test over it,
with no protocol, concurrency or state surface.

## 9. Mutation rows

None. `web/app/tsconfig.json` and the five engine files are outside the web production class, the
new file is a test, and no engine file changes (R2), so no emitted JavaScript changes.
