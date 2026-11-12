# SPEC-058: the web audit judges every dependency the repository resolves, and refuses a run that examined nothing

- **Wave:** W0. **Issue:** #260 (epic #1). **Context(s):** `repo` (`scripts/check.sh`,
  `scripts/audit-web-verdict.py`).
- **Decided by:** ADR-055 (the gate's stages, whose audit it split into `audit-rust` and
  `audit-web`), with its note of 2026-09-28, which decides the dependency classes the web audit
  judges, its level, and how its verdict is read from pnpm's report.
- **Status:** judged: written at delivery, because it had no planned copy, and delivered with its
  tests and `docs/red-first/SPEC-058.md` (ADR-016). It waited in `docs/specs/planned/` from its own
  commit until its tests were green.

## 1. The problem, measured

Measured at `dev` 16ed8e2 with pnpm 11.27.1, the version `package.json` pins. Each count is read
from the `metadata` of the command's own JSON report:

| command | exit | `totalDependencies` | `dependencies` | `devDependencies` | `optionalDependencies` |
|---|---|---|---|---|---|
| `pnpm audit --prod --json` | 0 | 0 | 0 | 0 | 0 |
| `pnpm audit --json` | 1 | 428 | 0 | 428 | 87 |

- **Every package the workspace resolves is a development dependency.** The Mini App ships as a
  static build (ADR-005), and neither the root's `package.json` nor `web/app`'s declares a runtime
  dependency, so an audit of the production dependencies has no package to examine. The
  `audit-web` stage runs `pnpm audit --prod` (SPEC-038 R5).
- **pnpm states its count only in its JSON report.** Without `--json`, `pnpm audit` prints a table
  of advisories, or `No known vulnerabilities found`, and no count. With it,
  `metadata.totalDependencies` counts each package version pnpm asked the registry about, once
  (pnpm 11's `lockfileToAuditRequest`). The exit is 0 whenever no advisory at or above the level
  was found, a run that examined nothing included. Every check that enumerates prints its examined
  count and refuses zero (CLAUDE.md, the tdd pack).
- **The level can come from the workspace.** `--audit-level` defaults to `low`, and pnpm 11 reads
  `audit.level` from `pnpm-workspace.yaml` when the command line names none. On a scratch copy of
  the workspace with `audit.level: critical`, `pnpm audit --json` exited 0 with no advisory in its
  report, and `pnpm audit --json --audit-level low` exited 1 with every advisory the default level
  reports: the command line's level wins.
- **Two settings shape the report.** `--ignore-registry-errors` makes pnpm exit 0 when the registry
  answers with an error (pnpm's documentation of `pnpm audit`). `audit.ignore` in
  `pnpm-workspace.yaml` drops an advisory from the report's `advisories`, while
  `metadata.vulnerabilities` still counts it (measured on the same scratch copy).
- **The full audit's exit 1** at 16ed8e2 is the advisories #254 and #255 track, in two development
  dependencies; #259 fixes both with scoped overrides. The stage this SPEC builds is therefore red
  on that lockfile and green on a `dev` that holds #259 (section 7).

## 2. Requirements

R1. `audit-web` runs `pnpm audit --json --audit-level "$AUDIT_WEB_LEVEL"`, once. It passes no flag
    that narrows the dependency classes (`--prod` or `-P`, `--dev` or `-D`, `--no-optional`), so
    pnpm audits every package the lockfile resolves, its dependencies, devDependencies and
    optionalDependencies alike. It never passes `--ignore-registry-errors`.
R2. The level is `low`, defined once in `scripts/check.sh` as `AUDIT_WEB_LEVEL` and handed to pnpm
    and to the verdict. It is pnpm's lowest level, so every advisory pnpm grades fails the stage.
    That is the bar `audit-rust` holds, where `deny.toml` makes every RustSec advisory an error. It
    is stated on the command line, so no `audit.level` in `pnpm-workspace.yaml` can raise it.
R3. `scripts/audit-web-verdict.py` reads pnpm's JSON report on its standard input, with pnpm's exit
    code (`--pnpm-exit`) and the level (`--level`). Its last line is the stage's verdict:
    - **no report:** a text that is not a JSON object whose `metadata.totalDependencies` is a whole
      number and whose `advisories` is an object reads `audit-web: VOID: pnpm audit gave no report
      with a package count (exit <code>): <its first line>`, and exits 3;
    - **nothing examined:** a count of 0 reads `audit-web: VOID: examined 0 package(s), so nothing
      was judged`, and exits 3;
    - **otherwise** the verdict line is `audit-web: examined <N> package(s) (dependencies <n>,
      devDependencies <n>, optionalDependencies <n>), advisories at or above <level>: <K>`, its
      counts read from the report's metadata;
    - **an advisory** whose severity is at or above the level, in pnpm's order `info`, `low`,
      `moderate`, `high`, `critical`, or is none of them, is named on a line of its own before the
      verdict (its severity, package, versions found, advisory id and vulnerable range). It counts
      in K, and the stage exits 1, whatever pnpm's exit was;
    - **with K 0, a pnpm that exited non-zero** fails the stage: the verdict line ends
      `, but pnpm audit exited <code>`, and it exits 1;
    - **with K 0 and pnpm's exit 0,** the stage passes with exit 0.
R4. `audit-web` checks `node` (24 or later), then `pnpm`, then `python3` (3.11 or later), which
    runs the verdict, before it runs them. Without each, it fails by name with its install hint
    (SPEC-038 R4).
R5. SPEC-038 takes an insert-only amendment (its section 10) at R4 and R5, where they state the
    stage's tools and its command, naming this SPEC. ADR-055 takes a dated note at its end.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the stage's one pnpm call audits every dependency class at the stated level: `audit --json --audit-level low`, with no flag that narrows the classes | `test_audit_web.py` `the_web_audit_covers_the_development_dependencies_too` |
| A2 | a report that examined 0 packages reads VOID and the stage exits non-zero; so does a run that gave no report: a text that is not JSON, a report with no count, and a count that is not a whole number | `test_audit_web.py` `a_web_audit_that_examined_nothing_is_void` |
| A3 | an advisory at or above the level fails the stage and is named, whatever pnpm's exit; an advisory of no known grade fails too, and one below the level does not; a pnpm that exited non-zero never passes; the verdict judges at each level `--audit-level` takes, and refuses one it does not | `test_audit_web.py` `an_advisory_at_the_failing_level_fails_the_web_audit` |
| A4 | a clean report passes, and the verdict line names how many packages were examined, in each class | `test_audit_web.py` `a_clean_web_audit_passes_and_prints_its_examined_count` |
| A5 | the stage fails by name without each tool it runs, python3 among them | `test_check_gate.py` `every_stage_fails_by_name_without_each_tool_it_runs` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_audit_web.py -k the_web_audit_covers_the_development_dependencies_too
A2: python3 -m unittest discover -s scripts/tests -p test_audit_web.py -k a_web_audit_that_examined_nothing_is_void
A3: python3 -m unittest discover -s scripts/tests -p test_audit_web.py -k an_advisory_at_the_failing_level_fails_the_web_audit
A4: python3 -m unittest discover -s scripts/tests -p test_audit_web.py -k a_clean_web_audit_passes_and_prints_its_examined_count
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k every_stage_fails_by_name_without_each_tool_it_runs
```

A1 to A4 run `check.sh`'s `audit-web` stage as SPEC-038's A5 does: a `PATH` of the shell tools
`check.sh` needs, a `node` stub, the real `python3` and `cat`, and a `pnpm` stub that records the
arguments it is given, prints a planted report and exits with a planted code. They make no registry
query. Their reports are synthetic, with planted package names and advisory ids. A3 also runs the
verdict itself at each of the four levels, over one report holding an advisory of each grade, and
with a level or an option it refuses. A5 grows SPEC-038's table `TOOLS` in `test_check_gate.py` by
one entry, `python3` after `pnpm` for `audit-web`, and changes no assertion (SPEC-038 section 10).

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/check.sh` | repo | changed: R1, R2, R4 |
| `scripts/audit-web-verdict.py` | repo | added: R3 |
| `scripts/tests/test_audit_web.py` | repo | added: A1 to A4 |
| `scripts/tests/test_check_gate.py` | repo | changed: A5, `python3` in `audit-web`'s entry of `TOOLS` |
| `scripts/mutation-rows.d/S05800-S05899.json` | repo | added: the stage's and the verdict's invariants, as hand-proved rows |
| `docs/TESTING.md` | repo | changed: what `audit-web` audits, and when it reads VOID |
| `docs/schematics/web-audit-verdict.md` | repo | added: from pnpm's report to the stage's verdict |
| `docs/decisions/ADR-055-the-gate-runs-in-parallel-jobs-and-only-a-push-saves-a-cache.md` | repo | changed: a note at its end (the ADR is accepted, so it is appended to) |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | repo | changed: an insert-only amendment, its section 10 (R5) |
| `docs/specs/SPEC-058-the-web-audit-judges-every-dependency-and-refuses-an-empty-run.md` | repo | added, from `docs/specs/planned/` |
| `docs/red-first/SPEC-058.md` | repo | added |
| `changelog.d/fix-audit-web-058.md` | repo | added |

## 5. What this does NOT do

- It changes no dependency. An advisory the audit reports is fixed by a dependency change of its
  own, as #259 is for #254 and #255; this delivery merges `dev` in after #259 lands (section 7).
- It adds no exception list. `audit.ignore` is unset in `pnpm-workspace.yaml`, and the verdict
  reads the report pnpm gives after it. An exception would be a decision of its own, naming its
  advisory and its reason as `deny.toml`'s exceptions do (#260).
- It changes no workflow. The `web` job already runs `audit-web`, and its runner image carries the
  `python3` the verdict runs on, as the `hygiene` job's does for the python stage (#260).
- It leaves `audit-rust` as it is: `cargo deny` already refuses every RustSec advisory, and
  SPEC-055's test holds each of its exceptions live (#260).

## 6. Risks

- **A registry outage fails the stage.** pnpm then gives no report, and the stage reads VOID by
  design: an audit that could not ask the registry examined nothing. A re-run after the outage
  decides it.
- **pnpm changes its report's shape.** A report without a whole-number
  `metadata.totalDependencies`, or without `advisories`, reads VOID by name (A2), so the change
  fails loudly and is never read as clean.
- **An advisory in a development dependency fails every pull request until it is fixed,** as one
  in the Rust graph does. The fix is an update or a scoped override, the path #259 took.
- **An advisory published after a merge** is found by the next run of the stage, on the next pull
  request or push: the stage judges what the registry knows when it runs.

## 7. Measurements

The stage on the real tree, `bash scripts/check.sh audit-web` with pnpm 11.27.1, before and after
`dev` with #259 was merged in:

| tree | the stage | its verdict line |
|---|---|---|
| 4ad713c, the last commit on `dev` 16ed8e2's lockfile, run from a `git archive` export | `FAILED`, exit 1 | `audit-web: examined 428 package(s) (dependencies 0, devDependencies 428, optionalDependencies 87), advisories at or above low: 4` |
| 355bbb7, `dev` c3d769b merged in, which holds #259's overrides | `ok` | `audit-web: examined 428 package(s) (dependencies 0, devDependencies 428, optionalDependencies 87), advisories at or above low: 0` |

On the first, the stage named each of the four advisories on a line of its own before its verdict:
the advisories #254 and #255 track, which #259 fixes. Both runs examined the same 428 packages,
because #259 moves two versions and adds none.

## 8. Amendments at delivery

- **A2 and A3 grew after the green,** from reading the verdict's mutants (4ad713c). A2 gained a
  registry error after a blank line; A3 gained an advisory found at two versions, one with no
  findings, the verdict run at each of the four levels over one advisory of each grade, and its
  usage errors. Each kills a mutant the first cases left alive, and each was green on arrival; the
  red-first record names them.
- **The verdict reads an advisory's fields directly** (c4f16f9). pnpm writes every advisory as an
  object, so the guards that read a malformed one were dropped: a malformed advisory stops the
  verdict with a traceback, and the stage fails with it.
