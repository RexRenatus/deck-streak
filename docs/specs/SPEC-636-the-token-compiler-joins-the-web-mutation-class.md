# SPEC-636: the token compiler joins the web mutation class

Path:
`docs/specs/SPEC-636-the-token-compiler-joins-the-web-mutation-class.md`.

- **Issue:** none; this pull request carries the change.
- **Context(s):** the mutation plan's classifier, StrykerJS's `mutate` globs and the two checks
  that hold them, in `scripts/mutation-verdict.py` and `web/app/stryker.config.json`, with the tests
  of both. No workflow, crate, web source, deploy or iOS change.
- **Decided by:** ADR-660 (the token compiler joins by its exact path; the Mermaid recorder stays
  out, with its reason).
- **Supersedes:** SPEC-039 section 2, R2, for its `web` class alone (R3 below). Neither SPEC-039
  nor ADR-057 is edited.
- **Schematic:** `docs/schematics/mutation-testing.md` (the plan's edge into the `web` class).
- **Red-first record:** `docs/red-first/SPEC-636.md`.
- **Status:** delivered by the pull request that adds this file, in four commits: the documents;
  A1 and A2, red against the base code; the class change that turns them green; and the record.

## 1. The problem, measured

Every figure below was read at the base, dev at the cut.

- `scripts/mutation-verdict.py` sorts each changed path into a mutation class (SPEC-039 R2). Its
  `web` class is `WEB` (`mutation-verdict.py:117`, `web/app/src/.+\.(?:ts|js|svelte)`) less
  `WEB_NOT_PRODUCTION` (`:118`), so `classify` (`:265-275`) returns `other` for every path under
  `web/app/scripts`.
- `web/app/scripts` holds two files, `build-tokens.ts` and `record-docs-mermaid-fences.js`
  (`git ls-tree --name-only HEAD web/app/scripts/` prints 2 lines).
- `build-tokens.ts` is a production build input. `web/app/vite.config.ts:6` imports its
  `writeTokens`, and the `deck-streak-tokens` plugin (`:16`) calls it when a build starts. Its
  header says `tokens.test.ts` judges its `compileTokens`, and `src/lib/design/tokens.test.ts:3`
  imports it, inside the Vitest `include` (`vite.config.ts:34`, `src/**/*.{test,spec}.{js,ts}`).
- So a pull request that changes `build-tokens.ts` gets no StrykerJS leg. The plan lists the file
  in class `other`, its `stryker_mutate` list (`mutation-verdict.py:887`) never names it, and
  `web/app/stryker.config.json`'s `mutate` (`:7-15`) holds seven globs, all under `src/`, so the
  weekly battery never mutates it either. The mutation legs measure nothing in its scope.
- `record-docs-mermaid-fences.js` is a maintainer tool. Its header says it is run by hand from the
  repository root with an authenticated `gh`, and that it renders through GitHub's Markdown API.
  No Vitest test runs it.

## 2. Requirements

R1. **The token compiler is web production code.** `web/app/scripts/build-tokens.ts`, by its exact
    path and never by a glob over `web/app/scripts` or `scripts/**`, is in class `web` at every
    site that spells the class:
    - the plan's classifier: `classify` in `scripts/mutation-verdict.py` returns `web` for it, so
      the plan lists it in class `web`, and its `stryker_mutate` list names
      `scripts/build-tokens.ts` when the file's change adds a code line;
    - StrykerJS's `mutate` globs: `web/app/stryker.config.json`'s `mutate` gains
      `scripts/build-tokens.ts`, so the weekly battery mutates it;
    - the configuration check's copy of them: `STRYKER_MUTATE` in `scripts/mutation-verdict.py`
      gains the same entry, and `configs` refuses a `mutate` list that lacks it;
    - the exclusions walk: `exclusions` reads the file beside `web/app/src`, so a
      `Stryker disable` comment in it is refused by name (SPEC-057 R11).
R2. **The Mermaid recorder stays outside, with its reason.**
    `web/app/scripts/record-docs-mermaid-fences.js` stays in class `other`, out of `mutate` and out
    of the exclusions walk. It is a maintainer tool run by hand against GitHub's Markdown API, and
    no Vitest test runs it, so no test could kill its mutants. This SPEC claims no StrykerJS run
    over either file.
R3. **SPEC-039 R2 is superseded for `web` alone.** From this SPEC on, SPEC-039 R2's `web` class
    is its `web/app/src` globs plus R1's one path. Its `rust` and `oracle` classes, and every
    other requirement of SPEC-039 and ADR-057, stand as written.
R4. **Nothing is weakened.** No assertion is removed and no test is skipped. No workflow job, step
    or leg, threshold, timeout or path-gate entry changes. `PASSING_MUTATE` in
    `scripts/tests/test_mutation_workflows.py` gains the same entry, in step with `STRYKER_MUTATE`,
    so its fixture trees keep passing the configuration check.
    The bench-exclusion test in `scripts/tests/test_perf_benches.py` (SPEC-618 A24) admits R1's
    one path beside the `src/` root by an exact list, and still refuses a bench file anywhere
    `mutate` reaches.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | over a fixture whose head changes both files of `web/app/scripts` and one `web/app/src` file, the plan classes `build-tokens.ts` `web` and `record-docs-mermaid-fences.js` `other`, and its `stryker_mutate` lists `scripts/build-tokens.ts` and never the recorder; red at the base classifier (R1, R2) | `test_mutation_verdict.py` `TheTokenCompilerJoinsTheWebClass.test_the_plan_classes_the_token_compiler_web_and_the_recorder_other` |
| A2 | the configuration check passes a `mutate` that holds the widened globs and refuses the seven base globs alone, and the exclusions walk names a `Stryker disable` comment planted in the token compiler and none planted in the recorder; red at the base check and walk (R1, R2) | `test_mutation_verdict.py` `TheTokenCompilerJoinsTheWebClass.test_the_configuration_check_and_the_walk_read_the_token_compiler` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_plan_classes_the_token_compiler_web_and_the_recorder_other
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_configuration_check_and_the_walk_read_the_token_compiler
```

Each test prints the population it examined as `examined N`, and N is above 0.

## 4. File manifest

Part 1 holds the documents; part 2 the tests and the record; part 3 the class change; part 4 the
record's reds and greens; part 5 the bench-exclusion test's one admitted path, with the record's
note on it.

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-636-the-token-compiler-joins-the-web-mutation-class.md` | 1 | new: this file |
| `docs/decisions/ADR-660-the-token-compiler-joins-the-web-class-by-its-exact-path.md` | 1 | new |
| `changelog.d/web-scripts-mutation-636.md` | 1 | new fragment |
| `docs/schematics/mutation-testing.md` | 1 | the edge into the `web` class names the token compiler |
| `scripts/tests/test_mutation_verdict.py` | 2 | new: A1 and A2, in one new test class |
| `docs/red-first/SPEC-636.md` | 2, 4, 5 | new: the record; part 4 adds the reds and greens; part 5 a note on the test file changed after them |
| `scripts/mutation-verdict.py` | 3 | the classifier, `STRYKER_MUTATE` and the exclusions walk gain the token compiler |
| `web/app/stryker.config.json` | 3 | `mutate` gains `scripts/build-tokens.ts` |
| `scripts/tests/test_mutation_workflows.py` | 3 | `PASSING_MUTATE` gains `scripts/build-tokens.ts`, in step with `STRYKER_MUTATE` |
| `scripts/tests/test_perf_benches.py` | 5 | the bench-exclusion test admits the one exact path `scripts/build-tokens.ts` beside the `src/` root, by an exact list, and checks that the path is no bench file |

## 5. What this does NOT do

- It runs no StrykerJS and claims no mutation score for `build-tokens.ts`; the token compiler
  itself does not change here, so this pull request's plan lists no web file.
- It changes no workflow job, step or leg, no threshold, timeout or gate figure, and no entry of
  the path gate.
- It edits neither SPEC-039 nor ADR-057: R3 supersedes SPEC-039 R2's `web` class from this file.
- It adds no mutation row and claims no band.

## 6. Risks

- **The first later change to `build-tokens.ts` meets the file whole.** At the base, the web
  judge counts every mutant StrykerJS reports for a file it mutated, with no filter on changed
  lines (`mutation-verdict.py:2271-2307`), and the shard leg runs StrykerJS over its files whole
  (`ci.yml:991`). So the first pull request that changes the file after this one is judged on all
  of its mutants, any older survivor included; the weekly battery reports them first.
- **A new script under `web/app/scripts`** stays in class `other` until a SPEC names it: the class
  takes the one exact path, so no file joins without a decision.
- **The configuration and its check could drift.** `configs` compares the two lists whole
  (`mutation-verdict.py:2778` at the base), so a `mutate` that lacks the entry, or holds another,
  is refused.

## 7. What only CI proves

The pull request's mutation legs: the plan job classes this pull request's paths, and the scripts
class mutates this change's lines of `scripts/mutation-verdict.py` under the Python runner, whose
population maps the script to its test modules, `test_mutation_verdict` and
`test_mutation_workflows` among them. A local run proves A1, A2, the two test modules and
`test_perf_benches.py` only.

## 8. Mutation rows

None in this delivery. The changed lines of `scripts/mutation-verdict.py` are mutated by the
Python runner in its population (`scripts/mutation-python.json`).

## 9. Rollback

Revert the pull request. The classifier, the `mutate` list, its two copies and the walk return to
the `web/app/src` class together.

## Amendment: the older survivors (tests only)

A later pull request adds tests only. `web/app/src/lib/design/token-compiler.test.ts` pins the token
compiler's contract: each refusal by its exact message, a small document's whole stylesheet, each
Telegram theme parameter and each CSS generic family by name, and the one read and one write of
`writeTokens`, with the file system calls stood in. `web/app/src/lib/settings/colour-mode.test.ts`
compiles the tokens inside each test that reads them, never at module scope, so StrykerJS no longer
counts the compiler's mutants as static. No product file, StrykerJS, Vitest or package setting, and
no assertion changes. A3 is the proof: a ranged StrykerJS run over the compiler's older lines, before
and after the change.

| id | criterion | decided by |
|---|---|---|
| A3 | a ranged StrykerJS run over the token compiler's older lines leaves no mutant without coverage, and no survivor but the two equivalent mutants named below; not red at the base, where every new test passes | the ranged StrykerJS run, before and after |

The two equivalent mutants: the default `seen` list of `hexOf` holding a planted string (an alias
path matches `[\w.-]+`, so no path equals a string with a space), and `false ||` in place of the
`typeof parameter !== 'string'` test (a non-string is never a member of the set of parameter names,
so the set test refuses it either way).
