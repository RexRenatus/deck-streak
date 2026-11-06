# Red-first record: SPEC-628

SPEC-628 (R1 to R4, A1 and A2). The SPEC, ADR-650, the schematic and the changelog fragment were
committed first, then the census test of A1 and A2 alone, over the base's project unchanged: the
`include` list of `web/app/tsconfig.json` names none of `tests-engine/**/*.ts`,
`engine-harness/**/*.ts`, `vite.engine.config.ts` and `playwright.engine.config.ts`, so the census
reads `engine-harness`, `playwright.engine.config.ts`, `tests-engine` and `vite.engine.config.ts`
omitted. A1's red is therefore its criterion's assertion and never a missing name. The tests run
in CI's `web` job, which runs the whole Vitest suite, and each red below is quoted from that job's
log over the red commit. With a type error planted in each of the five engine files, the check
over the base's project read 0 errors and 0 warnings: the project read none of them.

```text
A1: pnpm exec vitest run web/app/src/lib/type-check-project.test.ts -t "the type-check project names every browser suite, harness and configuration"
A2: pnpm exec vitest run web/app/src/lib/type-check-project.test.ts -t "the census names a member the project omits"
```

```red-first
A1: red at e6a5777cbcac3fa0ee3271d64d0ea6325d4b767d: AssertionError: the project omits engine-harness, playwright.engine.config.ts, tests-engine, vite.engine.config.ts: expected [ 'engine-harness', …(3) ] to deeply equal []
A1: green at 8e0a17718600f5b641197e8d5b20668328001f11
A2: not red: guards the census's own selection and forms, which C2 holds
```

With a type error planted in each of the five engine files, the check read 0 errors at `877b3dfb` and 5 at `8e0a1771`, one in each file, and the check over the project read 0 errors and 0 warnings at `8e0a1771`.
