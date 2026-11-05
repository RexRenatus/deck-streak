---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-613: Coverage runs in its own workflow as an advisory report on every pull request and dev push, and never gates a merge

## Context and Problem Statement

DeckStreak measures its tests by red-first records and mutation testing, and reports no coverage
figure at all. A reader of a pull request cannot see which changed lines no test executed, and a
release carries no test report. A coverage figure is a weak measure of test quality: a line can
execute under a test that asserts nothing about it, so a percentage gate rewards tests written to
touch lines, and a ratchet turns every refactor into a fight with the number. The repository needs
the figure as a reading aid, without letting it hold a merge.

The required checks are `ci`, `fragment` and `dependency-review` on dev and `ci` and `fragment` on
main (`.github/rulesets/dev.json`, `.github/rulesets/main.json`). The aggregate `ci` needs every job
of `ci.yml` (SPEC-038, ADR-055), so a job added there would hold every merge on its result.

## Decision Drivers

- The figure never decides a merge, so no required check and no `needs:` may read it.
- A broken instrument must still be visible: a report that examined nothing reads VOID, never a pass.
- One list says what is not measured, and every exclusion carries its reason.
- The provider is a dependency like any other: its advisories fall under the audits that already gate.
- No secret, no write token and no external coverage service.

## Decisions, and the alternatives each was chosen against

### D1. A workflow of its own, named in no needs and no ruleset

`.github/workflows/coverage.yml` runs on every pull request into dev or main and on every push to
dev. Its four jobs (`coverage-plan`, `coverage-rust`, `coverage-web`, `coverage-python`) are named in
no `needs:` of any other workflow and in no ruleset; the delivery changes no ruleset file and no job
of `ci.yml`. A plan job picks the lanes a pull request touches; a push to dev and the release pull
request (dev into main) run every lane over the whole tree.

- Chosen: a separate workflow, because nothing can then require it by accident and `ci` keeps its
  exact meaning.
- A coverage job inside `ci.yml`: rejected because the aggregate `ci` needs every job of that file,
  so a coverage result would hold every merge.
- A required coverage check with a threshold: rejected because a percentage gate rewards tests that
  touch lines without asserting anything, which red-first and mutation testing already judge better.
- A ratchet that refuses a falling figure: rejected because a refactor that deletes covered code
  lowers the figure for no fault of the change.

### D2. A job exits 1 only when the instrument is broken

Each lane exits 0 on a measured report whatever its figures read, and exits 1 only on VOID: zero
examined lines in a lane the plan says applies, an lcov file the report cannot read, or, in the web
lane, a coverage provider locked at another version than vitest. A failing test inside a lane does
not fail the lane: `ci` judges tests, and the report prints the runner exit beside its figures. Each
lane runs planted controls first and reads VOID if a planted broken input does not read as broken.
No step uses `continue-on-error`.

- Chosen: exit 1 for a broken instrument only, because a red coverage job then always means the
  measurement failed, never that the code is under-tested.
- Fail the lane on a failing test: rejected because it repeats the verdict `ci` owns and makes a
  coverage job red for a reason that is not the instrument.
- Exit 0 always: rejected because an empty report would then read as a pass.

### D3. One exclusion list, applied by one reader

`config/coverage.json` holds every exclusion, one row per glob with its kind and its reason: the
deprecated Telegram code citing ADR-589, the timing binary whose wall-clock budget an instrumented
build distorts, test code, type declarations and generated code. `scripts/coverage-report.py` applies
the list to every lane and prints its row count. The two flags the Rust run takes to save time (the
deprecated bot package left out of the build, and the timing binary left out of the run) are printed
by the same script from the same rows. No inline exclusion hint enters the tree without a row.

- Chosen: one list and one reader, because a glob then means the same thing in every lane and a
  reviewer reads every exclusion in one file.
- Each tool its own exclusion syntax: rejected because three grammars (`--ignore-filename-regex`,
  `coverage.exclude`, `omit`) drift apart and no single file says what is not measured.
- Inline hints in the source: rejected because they hide an exclusion where no list counts it.

### D4. Each lane takes its own tool, pinned

**Rust:** cargo-llvm-cov, installed by the pinned install action at an exact version with no
fallback, over cargo-nextest. `llvm-tools` is added inside the Rust lane only, so
`rust-toolchain.toml` and the shared cache key, `hashFiles(rust-toolchain.toml)`, stay as they are;
the lane restores the Rust cache and never saves one. The lcov holds lines and functions, and
the region figures come from the tool summary, because lcov carries no regions.

**Web:** `@vitest/coverage-v8` at exactly the version the lockfile resolves for vitest, in the
pnpm catalog beside vitest, with a `coverage` block in the root `vitest.config.ts` that stays off
unless `--coverage` is passed. The provider only warns when its version differs from vitest, so
the lane checks the lockfile itself and reads a mismatch as VOID. No required job reads the
provider version.

**Python:** coverage.py at the newest release of its current major line, from the hash-pinned
`tools/coverage/requirements.txt` installed with `--require-hashes` into a virtual environment
under the runner temp directory, with branch measurement on the `ctrace` core (the
`sys.monitoring` core records no branches on the runner image Python) and the subprocess patch,
over the three suites the gate python stage runs. A weekly Dependabot `pip` entry for
`/tools/coverage` in `.github/dependabot.yml` offers each new release of that line as a pull
request into dev, so a release that fixes an advisory reaches the lane without anyone changing
the file by hand.

That pull request passes the required `fragment` check without a hand-written fragment. SPEC-190
R13 exempts a Dependabot pull request that changes only files its ecosystems update, and SPEC-592
R12 applies that exemption, unchanged in kind, to the ecosystem this update path adds: the step
admits the one exact path `tools/coverage/requirements.txt` beside the cargo, npm and workflow
files, its author check and its whole-line match stay, and no glob is added. So no protection
goes: the step still refuses every path it refused before but the new requirements file, and
that file only from Dependabot and only alone.

- Chosen: each ecosystem native tool, because each reads its own language exactly and writes lcov,
  the one format the report reads.
- One cross-language service: rejected because it needs a token or an upload to a third party.
- An unpinned install: rejected because the figures would change with the tool, not the code.
- A hash pin with no update entry, left to `dependency-review`: rejected because that check reads
  only a pull request that changes the file, so a fixed release would never reach the lane alone.
- A hand-written fragment on each weekly pip pull request: rejected because `fragment`, required
  on dev and main, would then read red on every such pull request until a person added one, which
  R13 spares every other ecosystem.
- The whole `/tools/coverage` directory, or a glob, in the arm: rejected because the step would
  then pass a Dependabot pull request that changes a file beside the requirements file that no
  update writes, such as the coverage settings.

### D5. The changed-line figure, and where the report goes

On a pull request the report intersects the lines `git diff -U0 HEAD^1 HEAD` adds in each lane
production files with that lane lcov `DA` records and prints "changed executable lines N, executed
E, not executed: file:line ...". Every figure is an integer count, never a float percentage. The job
step summary carries a table per crate, module or package. Each lane uploads `coverage-<lane>` (its
lcov, its summary and the report), kept 30 days for a pull request and 90 days for a dev push.

- Chosen: the step summary and an artifact, because both need only the read-only token.
- A pull request comment: rejected because it needs a write token, which a fork pull request never
  gets.
- A percentage per file: rejected because an integer count says how many lines and a rounded
  percentage hides them.

### D6. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani: the delivery adds a workflow, a configuration file, a
standard-library report script and tests; it adds no protocol, no state machine, no interleaving
and no engine path, and no Rust source changes.

## Consequences

- Good, because every pull request shows which changed lines no test ran, in counts.
- Good, because a release pull request and every dev push carry a whole-tree test report.
- Good, because the coverage provider dependencies are audited where every dependency is: the
  `dependency-review` check and the gate web audit judge them under the advisories rule (ADR-005
  and its amendments, ADR-517), never under a coverage figure.
- Bad, because instrumented runs take runner time on every pull request that touches a lane.
- Bad, because some lines stay unmeasured: Swift (until the harness split), doctests, and Python
  files run only by subprocesses whose environment drops the coverage variables.

## Confirmation

- `scripts/tests/test_coverage_report.py`, run by the gate python stage, holds the VOID arms, the
  planted uncovered changed line, the provider mismatch, the exclusion list and the workflow shape
  (SPEC-592).
- `scripts/tests/test_changelog_fragment.py`, run by the same stage, holds the `fragment` arm for
  the requirements file with its planted refusals (SPEC-592 A11).
- Each lane own run on the delivery pull request is read by name before it lands.

## What would make this wrong

- A coverage job becomes a required check or a `needs:` of a required job: the figure would then
  gate, and this ADR would need replacing.
- The provider learns to refuse mixed versions itself: the lane check could then go.
- The coverage tools stop writing lcov: the report would need a second reader.

## More Information

SPEC-592; ADR-589 (the deprecated Telegram code the list excludes); SPEC-038 and ADR-055 (the gate
jobs and the aggregate `ci`); ADR-017 (the branch model and hosted runners).
