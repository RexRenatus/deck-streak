# SPEC-655: every tracked shell script is checked by a pinned ShellCheck that starts at warn

Path:
`docs/specs/SPEC-655-every-tracked-shell-script-is-checked-by-a-pinned-shellcheck-that-starts-at-warn.md`.

- **Issue:** #1375.
- **Context(s):** the hygiene job of `.github/workflows/ci.yml` (one inserted step), the guard
  tests under `scripts/tests` (one new census module, its planted fixtures, and one inserted test
  class in `scripts/tests/test_ci_workflows.py`), the rows band and the documents. No shell script,
  `scripts/check.sh` stage, product source or other workflow changes.
- **Decided by:** ADR-677 (ShellCheck runs from a pinned release in the hygiene job, and a census
  judges its saved output at warn). It cites SPEC-038 R12 (a tool from its release archive against
  a pinned digest), SPEC-190 R12 (one loader reads every workflow) and ADR-450 D6 (the kill
  criteria and the promotion rule), and amends none of them.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-655.md`.

## 1. Context

The repository keeps shell scripts beside its Python and Rust: the deploy and rollback scripts, the
gate (`scripts/check.sh`), the release and mutation helpers, and test doubles with no extension.
`# shellcheck disable=` directives already sit in some of them, so ShellCheck has been run by
hand, but no job runs it: a quoting or word-splitting defect in a script reaches dev unread. The
hygiene job already installs `gitleaks` and `protoc` from their release archives against a pinned
digest (SPEC-038 R12 for `protoc`), and the python stage of that job runs every guard test under
`scripts/tests`.

## 2. Requirements

R1. A tracked file is a shell script when its name ends in `.sh`, `.bash`, `.dash` or `.ksh`, or
    when its name holds no dot and its first line is a `#!` line naming `sh`, `bash`, `dash` or
    `ksh`, directly or through `env` (with or without `-S`). A link is no script. The census and
    the step apply this one rule. `.zsh` is left out because ShellCheck reads only sh, bash, dash
    and ksh and refuses zsh, and `.bats` because a Bats file holds `@test` blocks, Bats syntax
    rather than a shell's; the tree tracks 0 `.zsh` and 0 `.bats` files at this delivery's base.
R2. The census prints `examined N shell script(s)` and refuses `examined 0` as VOID.
R3. One step of the hygiene job, before its python stage, downloads the pinned ShellCheck release
    archive, checks it against the full digest held in the step's `SHELLCHECK_SHA256`, lists R1's
    population, runs `shellcheck --norc --format=json1 --severity=style` over it, and saves the
    list (`files.txt`), the tool's `--version` text (`version.txt`), its exit status
    (`status.txt`) and its output (`output.json`) under `$CHECK_LOG_DIR/shellcheck`, which the
    job's existing upload keeps. The step fails only on the download or the digest; it never fails
    on a finding or on the tool's status, which the census judges. `--norc` keeps a configuration
    file from changing what is checked.
R4. The census refuses as VOID a saved output with a file missing, a version other than the pin's,
    an exit status other than 0 or 1, or a status its findings contradict (0 with a finding, 1
    with none). It fails as `DRIFT` when the saved list is not R1's population by name.
R5. `LEVEL` is `warn` or `fail`. For each path and check code whose found count exceeds its listed
    count, the census prints `WARN <path> SC<code> found <n>, listed <m>, at line(s) <lines>` and
    passes at `warn`, and fails naming that line at `fail`. Each listed entry found fewer times
    than listed fails as `STALE <path> SC<code> found <n>, listed <m>` at both levels.
R6. `LISTED` maps each of today's findings, by path and check code, to its count and the reason it
    stands; its entries come from the hygiene job's own saved output. An entry with an empty reason
    fails as `UNREASONED <path> SC<code> listed <m>, with no reason` at both levels. It need not
    hold every finding: an unlisted finding prints at `warn`, and this SPEC claims no exact count
    of findings over the whole population.
R7. The repository test reads `$CHECK_LOG_DIR/shellcheck`; with no `$CHECK_LOG_DIR` it is VOID,
    which fails (ADR-450 D6's tooling rule). The step writes it only in the hygiene job.
R8. `LEVEL` is `warn`. Moving it to `fail` is its own delivery under ADR-677 D4.
R9. `scripts/tests/test_ci_workflows.py` pins R3's step: one hygiene step runs ShellCheck, before
    the python stage, from the pinned archive URL, with a digest `sha256_checked` admits, the
    arguments of R3 and `SHELLCHECK_DIR` under the job's log folder.

## 3. Acceptance criteria of SPEC-655

| id | criterion | decided by |
|---|---|---|
| A1 | R1: planted `clean.sh`, `direct-tool` (`#!/bin/sh`) and `fake-tool` (`#!/usr/bin/env bash`) are counted; `not-shell` (a `python3` shebang), `notes.txt` (a dotted name with a shell shebang) and the link `linked.sh` are not | `test_shell_script_census.PlantedTreesAreJudged.test_a_script_is_counted_by_its_extension_or_its_shell_shebang_and_nothing_else` |
| A2 | R5, R8: `LEVEL` is `warn`; a planted unlisted SC2086 at `finding.sh` line 3 returns its exact `WARN` line and no problem at `warn`, and that line as the one problem at `fail` | `test_shell_script_census.PlantedTreesAreJudged.test_an_unlisted_finding_prints_a_warn_line_at_warn_and_fails_at_fail` |
| A3 | R5, R6: the same finding listed once passes at both levels; listed once and found 0 times fails with its exact `STALE` line at both levels; listed once with an empty reason fails with its exact `UNREASONED` line at both levels | `test_shell_script_census.PlantedTreesAreJudged.test_a_listed_finding_passes_and_one_no_longer_found_is_stale_at_both_levels` |
| A4 | R2, R4: an empty tracked list and a tree whose one file is not a script are each refused as `examined 0`; a missing `output.json`, a version off the pin, status 2, status 0 with a finding and status 1 with none are each refused as VOID by their message; a saved list short of the population fails as `DRIFT` | `test_shell_script_census.PlantedTreesAreJudged.test_an_empty_population_and_a_saved_output_off_its_pin_or_its_list_are_refused` |
| A5 | R1, R2, R5 to R7: over the repository in the hygiene job, the population holds `scripts/check.sh`, the saved output is the pin's and over that population, and no problem is found | `test_shell_script_census.TheRepositorysScriptsAreChecked.test_every_tracked_shell_script_is_judged_from_the_pinned_shellchecks_saved_output` |
| A6 | R3, R9: the workflow pin of R9 | `test_ci_workflows.TheHygieneJobSavesThePinnedShellCheckOutput.test_hygiene_runs_the_pinned_shellcheck_once_before_its_stages` |

## 4. File manifest

| path | role | change |
|---|---|---|
| `.github/workflows/ci.yml` | R3's step in the hygiene job | one step inserted |
| `scripts/tests/test_shell_script_census.py` | the census, R1 to R8 | added |
| `scripts/tests/fixtures/shell-scripts/` | nine planted files, each ending in `.fixture` | added |
| `scripts/tests/test_ci_workflows.py` | R9's pin | one class inserted |
| `docs/specs/SPEC-655-every-tracked-shell-script-is-checked-by-a-pinned-shellcheck-that-starts-at-warn.md` | this SPEC | added |
| `docs/decisions/ADR-677-shellcheck-runs-from-a-pinned-release-in-the-hygiene-job-and-a-census-judges-its-output-at-warn.md` | the decision | added |
| `docs/red-first/SPEC-655.md` | the red-first record | added |
| `changelog.d/shell-scripts-655.md` | the changelog fragment | added |
| `docs/schematics/shell-script-census.md` | the data flow of R3 to R7: the step, its saved files and the census | added |
| `scripts/mutation-rows.d/S65500-S65599.json` | the rows of section 6 | added |

## 5. What it does not cover

- No finding is fixed and no script is edited; a fix is its own delivery.
- No `# shellcheck disable=` directive is counted or refused; those in the tree stay as they are.
- ShellCheck runs on no machine but the hygiene job's runner; no local gate stage runs it.
- `zsh` scripts and the shell blocks inside workflow files are not in R1's population.

## 6. Mutation rows

Each row replaces one line of `scripts/tests/test_shell_script_census.py`, whose find occurs
exactly once in it, and is killed by its planted test by full id.

| id | find | killer |
|---|---|---|
| S65501 | `.sh` is a member of `EXTENSIONS` | A1 |
| S65502 | `sh` is a member of `SHELLS` | A1 |
| S65503 | the `env` arm of the shebang | A1 |
| S65504 | a link or a non-file is no script | A1 |
| S65505 | a name with no dot is judged by its first line | A1 |
| S65506 | a name with a dot is never judged by its first line | A1 |
| S65507 | `LEVEL` is `warn` | A2 |
| S65508 | an unlisted finding fails at `fail` | A2 |
| S65509 | an unlisted finding is returned to print at `warn` | A2 |
| S65510 | a count above the listed count, not equal to it | A3 |
| S65511 | a listed entry found fewer times is `STALE` | A3 |
| S65512 | the `examined` count refuses an empty population | A4 |
| S65513 | a missing saved file is VOID | A4 |
| S65514 | the version pin | A4 |
| S65515 | the admitted exit statuses are 0 and 1 | A4 |
| S65516 | the exit status agrees with the findings | A4 |
| S65517 | the saved list is the population | A4 |
| S65518 | a listed entry with an empty reason fails | A3 |
