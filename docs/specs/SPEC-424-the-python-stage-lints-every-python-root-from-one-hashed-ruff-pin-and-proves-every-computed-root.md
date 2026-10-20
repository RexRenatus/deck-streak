# SPEC-424: the python stage lints every Python root from one hashed ruff pin, and proves every computed repository root

- **Wave:** the gate's own tools (SPEC-054). **Issue:** none; the pull request closes nothing, and
  section 5 names the issues its exclusions cite.
  **Context(s):** `repo` (`ruff.toml`, `scripts/`, `scripts/tests/`, `.github/workflows/`, `docs/`).
- **Decided by:** ADR-438 (this SPEC's own: the pin and its readings, the rule set, the cures, the
  population, the guard, the placement, the kill criteria, the pushes, the record's shape), and the
  decisions it extends: SPEC-038 R3 and R4, SPEC-054 R7 and ADR-055.
- **Schematic:** `docs/schematics/ci-jobs-and-caches.md`: its new last section, the python stage's
  ruff arm and the computed-root guard.
- **Status:** built by this delivery, with its tests, its rows and `docs/red-first/SPEC-424.md`.
  **Mutation band:** S42400-S42499 (section 8).

## 1. The problem, measured

Read at DeckStreak `dev` `ac7ebd75`.

1. **Nothing lints the Python.** `ruff.toml:6-15` selects no rule, `scripts/check.sh:31` lists no
   lint stage, and `stage_python` (`scripts/check.sh:179-196`) runs three unittest suites after
   checking `python3` and `cargo` (`:181`). No workflow installs or runs ruff: `git grep -i ruff`
   finds the word only in `ruff.toml`, `CHANGELOG.md` and two documents.
2. **Nothing pins a Python tool.** The tree holds no requirements file, and the `hygiene` job
   (`.github/workflows/ci.yml:298-358`) installs `cargo-deny`, gitleaks and protoc, each pinned, and
   no Python tool.
3. **The settled rule set finds these sites today**, each listed by place:
   - S324: `scripts/ios_lane.py:312` and `scripts/tests/test_ios_lane.py:488`;
   - TID251 (`os.chdir`): `scripts/tests/test_mutation_rows_refusal.py:189`, `:190`, `:195`, `:246`,
     `:278`, `:287`, `:365` and `:369`;
   - PLE2515: `crates/agent/tests/fixtures/fake-probe.py:17`;
   - S104: `scripts/tests/test_deploy_scripts.py:2029`; S604:
     `scripts/tests/test_dispatch_shards.py:1056`;
   - and the formatter changes one file, joining `scripts/tests/test_sync_server_launcher.py:279-281`
     onto one line.
4. **Nine computed repository roots, checked by nothing.** Each is `Path(__file__)` followed by a
   count of parents, and a count off by one reads a sibling directory with no error:
   `scripts/create-issues.py:31`, `scripts/dynamic-imports-check.py:27`,
   `scripts/public-scrub.py:59`, `scripts/threat_model.py:17`, `scripts/tests/_support.py:7`,
   `scripts/tests/test_bot_messages.py:19`,
   `scripts/tests/test_ship_profile.py:14`, `scripts/tests/test_threat_model.py:21` and
   `deploy/host-scrub/inventory.py:226`. All nine resolve to the repository root (`scripts/tests/test_mutation_rows.py:52` and `:467` sit inside the fixture strings `KILLERS` and `TOOL_KILLERS`, which R9 excludes, since a string literal is never a site, so neither is a site). No commit of
   DeckStreak's history rewrote such a count (ADR-438's context names the searches), so the guard
   is preventive.
5. **The stage's cost.** At `a3a052b2`, the `hygiene` job's python stage took
   853 s in job 116733923723 of run 38826135989.

## 2. Requirements

R1. **One hashed pin.** `scripts/ruff-requirements.txt` holds exactly one requirement,
    `ruff==<version>`, and one `--hash=sha256:<64 hex digits>` line for every wheel the package
    index publishes for that release, and no source archive. `ruff.toml` sets
    `required-version = "==<version>"` with the same version.
R2. **The settled selection.** `ruff.toml`'s `[lint]` table sets `extend-select` to `S`, `B`,
    `ASYNC`, `PGH`, `PLE`, `FIX`, `T10`, `DTZ`, `RUF034` and `TID251`; `ignore` to `S603`, `S607`,
    `S105`, `S106` and `S107` (each with its reason in a comment) and `S101`, `S102`, `S108`,
    `S311`, `B904`, `B905`, `B023`, `B007`, `B008` and `DTZ001`; `per-file-ignores` to `S104` and
    `S604` for `scripts/tests/**`, `TID251` for `scripts/tests/test_mutation_rows_refusal.py` and
    `PLE2515` for `crates/agent/tests/fixtures/fake-probe.py`; and
    `[lint.flake8-tidy-imports.banned-api]` to `os.system`, `os.popen`, `subprocess.getoutput`,
    `subprocess.getstatusoutput`, `pickle`, `marshal`, `shelve`, `tempfile.mktemp`, `telnetlib`,
    `ftplib` and `os.chdir`, each with a message. It sets no `select`, so ruff's default set stays
    enabled, and no preview; `unfixable` keeps `F401` and `F841`, and every line of `ruff.toml`
    before the `[lint]` table is kept.
R3. **The install in CI.** Each job that runs the python stage (today `hygiene`) installs ruff
    before its gate step, in one step: a new virtual environment under the runner's temporary
    directory, `pip install --require-hashes --only-binary :all: -r scripts/ruff-requirements.txt`
    there, and a directory holding only that ruff appended to the job's path. No other step of any
    workflow installs ruff.
R4. **The tool check.** `stage_python` checks `ruff` after `python3` and `cargo`; a missing ruff
    fails the stage with `missing tool: ruff (<install hint>)`, and the hint names the pin file and
    the install command (SPEC-038 R4, as section 17 of SPEC-038 amends it).
R5. **The arm's VOID readings.** After the three suites, whatever they found, the stage's ruff arm
    reads the pin from `scripts/ruff-requirements.txt`, compares `ruff --version` with
    `ruff <pin>`, lists the files with `ruff check --show-files`, and then runs `ruff check` and
    `ruff format --check`. The listing, lint and format calls each pass `--no-cache` and the roots
    `scripts tools deploy agent crates`, which `scripts/check.sh` names once, as `RUFF_ROOTS`, and
    `ruff check` prints its findings in the concise form. Each of these is VOID and
    fails the stage, its reason on the stage's last line as `ruff VOID: <reason>`: a pin file with no
    `ruff==` line; a version that is not the pin (no lint or format call is then made); an exit of 2
    from the listing, `ruff check` or `ruff format --check`; a listing of no file; a format summary
    that names no count; and a format count that differs from the listing's.
R6. **A finding fails by count.** `ruff check` exiting 1, or `ruff format --check` exiting 1, fails
    the stage, and the stage's last line names both arms' examined counts and exits; it does not
    read VOID.
R7. **The stage's last line.** It is SPEC-054 R7's line, then `; ruff check examined <n> file(s),
    exit <c>; ruff format examined <n> file(s), exit <c>`, or `; ruff VOID: <reason>`. A red suite
    never stops the arm, and a red arm never stops a suite.
R8. **The cures.** At the delivery's head the gate's ruff finds nothing: `scripts/ios_lane.py:312`
    and `scripts/tests/test_ios_lane.py:488` pass `usedforsecurity=False` to `hashlib.sha1`, each
    in the formatter's wrapping; `scripts/tests/test_sync_server_launcher.py:279-281` is in the
    formatter's form; every other site of section 1 item 3 stays as it is under R2's per-file
    ignores. No behaviour changes.
R9. **The computed-root guard.** `scripts/tests/test_computed_roots.py` walks every `.py` file under
    the five roots and parses it with `ast`. Each assignment, at any scope, plain or annotated, whose
    one target is a name holding `ROOT` or `REPO` in any case and whose value is `Path(__file__)`
    or `pathlib.Path(__file__)`, optionally `.resolve()`, then `.parents[N]` with an integer literal
    or `.parent` repeated, must resolve to the repository root, or to the directory `LISTED_ROOTS`
    names for it. Such a name with any other chain fails as unresolvable; a file that does not parse
    fails by name. Each failure names the path, the line, the name and the directory it resolves to.
    A string literal is never a site.
R10. **The listed set and the fence.** `LISTED_ROOTS`, in the guard's module, maps `<path>::<name>`
    to a repository-relative directory and its reason, and is empty at delivery. The walk skips the
    name patterns `FENCED` lists (`*drill*`, `*readings_tree*`, `test_deploy_standin.py` and
    `_standin_checks.py`, matched case-blind) before any open, and counts them as skipped. The guard
    prints the files and the sites it examined, and fails at zero.
R11. **Kill criteria.** ADR-438 D7 holds them; no test reads them.
R12. **Rows.** `scripts/mutation-rows.d/S42400-S42499.json` holds the rows of section 8.

## 3. Acceptance criteria of SPEC-424

| id | criterion | decided by |
|---|---|---|
| A1 | `ruff.toml` requires exactly `==` and the version `scripts/ruff-requirements.txt` pins; the pin file holds one requirement, `ruff==<version>`, every other line a well-formed `--hash=sha256:` line, at least one, and the count is printed | `scripts/tests/test_ruff_gate.py` |
| A2 | `ruff.toml`'s `[lint]` table is exactly R2's selection: the added families and rules, the two kinds of ignore, the per-file table, the banned names each with a message; no `select`, no preview; `unfixable` kept | `scripts/tests/test_ruff_gate.py` |
| A3 | each job that runs the python stage installs ruff from the hashed pin, into a virtual environment, with `--require-hashes` and `--only-binary :all:`, before its gate step, and appends a ruff-only directory to its path; no other step of any workflow installs ruff | `scripts/tests/test_ci_workflows.py` |
| A4 | the python stage fails by name, with its install hint, without `ruff` as without `python3` or `cargo` | `scripts/tests/test_check_gate.py` |
| A5 | a ruff off its pin, or a pin file with no `ruff==` line, reads `ruff VOID` and fails the stage, and no lint or format call is made | `scripts/tests/test_check_gate.py` |
| A6 | a listing, a `ruff check` or a `ruff format --check` that exits 2 reads `ruff VOID` and fails the stage | `scripts/tests/test_check_gate.py` |
| A7 | a listing of no file, a format summary with no count, and a format count unequal to the listing's each read `ruff VOID` and fail the stage | `scripts/tests/test_check_gate.py` |
| A8 | a lint finding, or a file the formatter would change, fails the stage with both arms' examined counts and exits on its last line, and not VOID | `scripts/tests/test_check_gate.py` |
| A9 | a clean ruff passes the stage: the last line names the three suites and both arms with the planted count, each listing, lint and format call carried `--no-cache` and the five roots in order, and a red suite still leaves both arms run and named | `scripts/tests/test_check_gate.py` |
| A10 | over a planted tree, a root at the wrong depth is refused by path, line, name and directory for each chain shape (`parents[N]` and repeated `.parent`, with and without `.resolve()`, `pathlib.Path`, annotated, inside a function); the right depth passes; a listed root passes at its listed directory and is refused elsewhere; an unresolvable chain and a file that does not parse are refused by name | `scripts/tests/test_computed_roots.py` |
| A11 | over a planted tree, exactly the ROOT and REPO assignments are judged, in any case: another name, a string literal holding the assignment's text, and an expression statement are not | `scripts/tests/test_computed_roots.py` |
| A12 | over a planted tree, a fenced name that does not parse and holds a wrong root is skipped by name, never parsed, and counted as skipped | `scripts/tests/test_computed_roots.py` |
| A13 | every computed root of the tree resolves: the guard prints its examined files and sites, both above zero, `LISTED_ROOTS` is empty, and nothing is refused | `scripts/tests/test_computed_roots.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ruff_gate.py -k test_ruff_toml_requires_exactly_the_version_the_hashed_pin_names
A2: python3 -m unittest discover -s scripts/tests -p test_ruff_gate.py -k test_the_lint_table_is_exactly_the_settled_selection
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_job_that_runs_the_python_stage_installs_ruff_from_the_hashed_pin_first
A4: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_every_stage_fails_by_name_without_each_tool_it_runs
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_a_ruff_off_its_pin_or_with_no_pin_reads_void_before_it_lints
A6: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_a_ruff_that_refuses_its_configuration_reads_void
A7: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_a_run_that_examined_nothing_or_two_populations_reads_void
A8: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_a_lint_finding_or_a_format_change_fails_the_stage_with_its_counts
A9: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_a_clean_run_passes_over_the_five_roots_beside_every_suite
A10: python3 -m unittest discover -s scripts/tests -p test_computed_roots.py -k test_a_planted_root_at_the_wrong_depth_is_refused_by_place
A11: python3 -m unittest discover -s scripts/tests -p test_computed_roots.py -k test_only_root_and_repo_assignments_are_judged_and_strings_are_not
A12: python3 -m unittest discover -s scripts/tests -p test_computed_roots.py -k test_the_walk_never_opens_a_fenced_file
A13: python3 -m unittest discover -s scripts/tests -p test_computed_roots.py -k test_every_computed_root_in_the_tree_resolves
```

R8, the cures, is decided by the gate's own run in CI, never by a test of the repository: section 7.
R11 has no test, and R12 is judged by the `mutation-rows` check-run.

## 4. File manifest

| path | part | change |
|---|---|---|
| `ruff.toml` | `repo` | changed: R1's `required-version` and R2's `[lint]` selection |
| `scripts/ruff-requirements.txt` | `repo` | added: R1's hashed pin |
| `scripts/check.sh` | `repo` | changed: `RUFF_ROOTS`, the tool check and the ruff arm of `stage_python` (R4 to R7) |
| `.github/workflows/ci.yml` | `repo` | changed: the `hygiene` job's install step (R3) |
| `scripts/ios_lane.py` | `repo` | changed: R8, the fingerprint call of line 312 |
| `scripts/tests/test_ios_lane.py` | `repo` | changed: R8, the fingerprint call of line 488 |
| `scripts/tests/test_sync_server_launcher.py` | `repo` | changed: R8, lines 279-281 in the formatter's form |
| `scripts/tests/test_check_gate.py` | `repo` | changed: `TOOLS` names `ruff` for `python`, the python-stage fixtures carry a ruff stand-in and a pin file, and A5 to A9 (A4 reads the widened map) |
| `scripts/tests/test_ruff_gate.py` | `repo` | added: A1 and A2 |
| `scripts/tests/test_computed_roots.py` | `repo` | added: the guard (R9, R10) and A10 to A13 |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed: A3, one new test class |
| `scripts/mutation-rows.d/S42400-S42499.json` | `repo` | added: section 8's rows |
| `docs/specs/SPEC-424-the-python-stage-lints-every-python-root-from-one-hashed-ruff-pin-and-proves-every-computed-root.md` | `repo` | added: this SPEC |
| `docs/decisions/ADR-438-ruff-runs-in-the-python-stage-from-one-hashed-pin-and-a-run-off-its-pin-or-over-nothing-is-void.md` | `repo` | added: its ADR |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | `repo` | changed: section 17, insert-only (R4's tools) |
| `docs/schematics/ci-jobs-and-caches.md` | `repo` | changed: a new last section, insert-only |
| `docs/red-first/SPEC-424.md` | `repo` | added: the red-first record |
| `changelog.d/ruff-house-rules-424.md` | `repo` | added: the fragment |

## 5. What this does NOT cover

- It enables no rule family beyond R2's, and none of R2's not-admitted rules; each is its own
  decision (#853).
- It adds no allow-list arm for an individual site, of ruff or of the guard; `LISTED_ROOTS` names a
  directory a root must reach, and exempts nothing (#854).
- It adds no Rust lint and no measurement of clippy's rules beyond the `clippy` stage's own
  `-D warnings` (#855).
- The guard judges only `Path(__file__)` chains assigned to ROOT or REPO names; an `os.path`
  spelling, another name, and the files `FENCED` lists are not judged (#856).

## 6. Risks

- **The pinned release lacks a wheel for a maintainer's platform.** The install refuses, which is
  fail-closed; detected by that maintainer's install, and cured by a pin that has one.
- **The runner's Python cannot make a virtual environment.** The install step fails on the
  delivery's own first run; detected there, before any land.
- **A later release changes ruff's default set or its formatter.** Only a pin change can bring one
  in, and that delivery's own run re-reads the tree.
- **The format summary changes shape.** The arm then reads `ruff VOID` (A7), never a pass.
- **A new tree under the roots holds generated Python.** The gate lints it like any other file;
  an exclusion is its own decision, written in `ruff.toml`.
- **A site of a fenced file is wrong.** The guard does not see it (section 5); the gate's ruff still
  lints those files.
- **The gate's summary line cuts a failing stage's last line at 160 characters**
  (`scripts/check.sh:230`), so with three suites ahead of it the ruff fields may not show there.
  The stage log keeps the whole line, and each job uploads it; read the log before naming a cause.

## 7. Decided by the gate's own run, in CI

No repository test decides these. The `hygiene` check-run at the delivery's head records each, and
`docs/red-first/SPEC-424.md` quotes the lines by job id beside their durations.

| id | criterion | the record |
|---|---|---|
| G1 | R8: the python stage's last line at the head reads `ruff check examined <n> file(s), exit 0; ruff format examined <n> file(s), exit 0`, with `<n>` above zero and equal in both arms | the stage log `python.log` in the `check-stage-logs-hygiene` artifact |
| G2 | R3: the install step succeeds, and its duration and the python stage's are recorded | the `hygiene` job's log |

## 8. Mutation rows

The band is `scripts/mutation-rows.d/S42400-S42499.json`, table `SCRIPT_MUTATIONS`. Each row's find
occurs exactly once in its file at the head, and each killer is a test named in section 3.

| row | file | mutant | killer |
|---|---|---|---|
| S42400 | `scripts/check.sh` | the tool check drops `need_ruff` | A4 |
| S42401 | `scripts/check.sh` | the version comparison accepts any version | A5 |
| S42402 | `scripts/check.sh` | an empty pin reads as a pin | A5 |
| S42403 | `scripts/check.sh` | an exit of 2 from `ruff check` is not VOID | A6 |
| S42404 | `scripts/check.sh` | a listing of no file is admitted | A7 |
| S42405 | `scripts/check.sh` | unequal counts are admitted | A7 |
| S42406 | `scripts/check.sh` | a lint finding leaves the stage green | A8 |
| S42407 | `scripts/check.sh` | the format arm drops `--check` | A9 |
| S42408 | `scripts/check.sh` | `RUFF_ROOTS` drops `crates` | A9 |
| S42409 | `scripts/check.sh` | a ruff call drops `--no-cache` | A9 |
| S42410 | `ruff.toml` | `required-version` names another version | A1 |
| S42411 | `ruff.toml` | `extend-select` drops a family | A2 |
| S42412 | `ruff.toml` | `ignore` drops a permanent ignore | A2 |
| S42413 | `ruff.toml` | a per-file ignore moves to another rule | A2 |
| S42414 | `ruff.toml` | the banned list drops `os.chdir` | A2 |
| S42415 | `scripts/ruff-requirements.txt` | the pin loosens `==` to `>=` | A1 |
| S42416 | `.github/workflows/ci.yml` | the install drops `--require-hashes` | A3 |
| S42417 | `.github/workflows/ci.yml` | the install drops `--only-binary :all:` | A3 |

No row covers R8's two `usedforsecurity=False` keywords. `scripts/ios_lane.py` is in the python
mutation runner's population (`scripts/mutation-python.json`), so its `mutation-python` legs judge
whatever mutants the runner generates on line 312 by `test_ios_lane`. The keyword changes no byte
of the fingerprint, so no test observes it, and the gate's ruff arm (S324) is what refuses a change
to it. If the runner lists a mutant there, its disposition is decided before this pull request
lands, and recorded in it. `scripts/tests/test_ios_lane.py` is a test, outside the runner's
population, and so is the guard's own module.

## 9. Red-first

`docs/red-first/SPEC-424.md` records one line per criterion. A1 to A12 are RED-FIRST: each is red at
the red commit, read in that push's `hygiene` check-run, for its criterion's reason (the stage has
no ruff arm, `ruff.toml` selects nothing, the workflow installs no ruff, the guard's stub judges
nothing). A12's fence is in the guard from its first commit, so its red is the stub's empty skip
list. A13 is `not red`: it pins the tree's nine computed roots, which resolve at the base, and
arrives with the green commit.
