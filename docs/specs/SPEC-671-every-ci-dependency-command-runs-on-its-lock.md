# SPEC-671: every CI command that resolves or builds dependencies runs on its lock

Path: `docs/specs/SPEC-671-every-ci-dependency-command-runs-on-its-lock.md`.

- **Issue:** #1390.
- **Context(s):** a new census module under `scripts/tests/`, its planted cases under
  `scripts/tests/fixtures/lock-flags/`, the rows band and the documents. No workflow, script,
  production file or existing test changes.
- **Decided by:** ADR-694 (a text census over the workflows and the scripts they run, at warn). It
  cites ADR-450 D6 (a census starts at warn and is promoted by its own delivery), SPEC-436 R21
  (the verdict form), SPEC-344 R6 (the Apple build's cargo commands carry `--locked`) and SPEC-332,
  SPEC-416 and SPEC-458 (the jobs that fetch the locked graph), and amends none of them.
- **Schematic:** `docs/schematics/ci-lock-flags.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-671.md`. Read at dev `b9a08e60`.

## 1. The problem, measured

A lock file holds the dependency graph a build was reviewed with. A CI command that resolves
dependencies without its lock flag may resolve a different graph and build it, and nothing reads
that today: the one hold is the Apple workflow's cargo commands (SPEC-344 R6), and the jobs that
fetch the locked graph before a rows proof, a mutant baseline or a cache save (SPEC-332, SPEC-416,
SPEC-458). Every other workflow command and every command in a script a step runs is unread.

A reading of the workflows and of the scripts their steps run by path found these sites, a lower
bound: three `cargo clean` steps, five `cargo mutants --list` steps, four `cargo mutants` runs,
two `cargo codspeed` steps, one `cargo llvm-cov report`, one `cargo nextest run` in
`scripts/mutation-baseline.sh`, two `swift test` steps and `scripts/swift_mutants.py`'s
`swift test`, and eleven xcodebuild actions in `xcframework.yml`. Every `pnpm install` carries
`--frozen-lockfile`, no step runs npm, and the one pip install pins by hash. The census reads the
whole population in CI, so its first run there measures every site; this SPEC claims no exact count
over the whole population.

## 2. Requirements

R1. The population is every `.yml` and `.yaml` file under `.github/workflows/` and every script a
run text names by a repository path ending `.sh` or `.py` that the tree holds, each read once as
text and never run or imported. The census prints `examined N file(s)` and refuses an empty
workflow set, which is VOID. A link at any component of a path is refused.

R2. A workflow's run texts are every `run:` key at any depth (a `with: run:` input included),
block and inline scalars alike, each scoped by its job. Backslash continuations are joined, comment
lines dropped, and a line is split into simple commands at `&&`, `||`, `;`, `|`, `$(`, a backtick,
parentheses and braces; a workflow expression is one word. A shell script is read the same way,
each line scoped by its shell function. A Python script is read by its syntax tree: a list literal,
or a tuple literal passed to a call, whose first element names a tool, scoped by its function.

R3. A tool's word is found at any position of a simple command, so a command run through a wrapper
(`timeout`, `xargs`, a Python wrapper that runs what follows `--`) is read. A tool word whose next
word is not shaped as a subcommand is not a command.

R4. A site is: a cargo subcommand with neither `--locked` nor `--frozen` before ` -- `; for
cargo-mutants, a run with neither `--cargo-arg=--locked` nor `--cargo-arg=--frozen`; a
`cargo install` without `--locked`; a `pnpm install` or `pnpm i` without `--frozen-lockfile`, or
with `--no-frozen-lockfile`, and a `pnpm add`, `update` or `dlx`; an npm `install`, `add`,
`update` or `exec`, and `npx`, where `npm ci` is the locked form; a `swift build`, `test`, `run`,
`package resolve` or `package update` without `--only-use-versions-from-resolved-file` or one of
its two aliases; every xcodebuild action, and every xcodebuild call naming a project, workspace or
scheme, whatever flags it carries, since requiring a resolved-file flag for xcodebuild is its own
delivery's question; a pip install without `--require-hashes`; and a `pnpm/action-setup` step whose
`run_install` is not `false`.

R5. Not a site: `cargo fmt`, `cargo metadata`, `cargo tree`, `cargo version`, `cargo help` and
`cargo --version`, which read no lock or read it on purpose without one (the lock check of
`scripts/check.sh` runs `cargo metadata` unlocked); a command under `--frozen` or with its lock
flag; a pnpm or npm subcommand that installs nothing (`exec`, `run`, a package script); an
xcodebuild call with no action and no project, such as `-create-xcframework`; and a tool a workflow
fetches by a pinned and checked download, which names no tool word in a run text.

R6. Undecided, printed as `UNDECIDED <file>:<line>: <why>` and never judged at either level: a
here-document body that names a tool, unless the body is fed to Python or Node, which is another
language and is not read; a site whose words hold a workflow expression, which may hold the flag;
and a Python argument list whose second element is built by an expression. The census does not
follow a called script's own calls, a script a step writes and runs, or a program named only by a
shell variable.

R7. A site's key is its file, its job or function (`<top>` outside one), its tool, its subcommand
and its ordinal among that file's commands of the same scope, tool and subcommand; a line number is
never part of a key. `LISTED` holds today's sites, each with a reason. At `warn`, an unlisted site
prints `WARN <file>:<line>: <tool> <subcommand> has no lock flag`; at `fail` it fails the test. A
listed entry no current site matches prints `STALE` and fails at both levels. `LEVEL` is `warn`.

R8. The planted cases sit under `scripts/tests/fixtures/lock-flags/` with a `.fixture` suffix, so
no runner collects them, and are read as text: for each tool a command without its lock flag (a
site), the same command with its flag (clean; xcodebuild is asked no flag, so each planted xcodebuild action or project call is a site), a command under `--frozen` (clean), a command split
over continuation lines, a command that reads no lock (not a site), and an empty population (VOID).

R9. Every existing test, assertion, census, pin, mutation row, planted case and refusal stays as it
is, the Apple workflow's `--locked` hold included. Adding a lock flag to any site is outside this
SPEC.

## 3. Acceptance criteria of SPEC-671

| Id | Criterion | Test |
|---|---|---|
| A1 | R1: an empty workflow set is refused as VOID | `TheVerdictKeepsItsLevel.test_an_empty_population_is_void` |
| A2 | R2 to R5: each planted cargo form is a site or clean as its lock flag says, a continued command read whole, a flag after ` -- ` no lock flag, and a wrapped command read | `TheCensusReadsEachPlantedForm.test_each_cargo_form_is_a_site_or_clean_as_its_lock_flag_says` |
| A3 | R4 to R6: each planted pnpm, npm, swift, xcodebuild, pip and setup form is a site or clean, a Python here-document is not read, another here-document and a workflow expression are undecided, and a named script is followed | `TheCensusReadsEachPlantedForm.test_each_node_and_apple_and_pip_form_is_a_site_or_clean_as_its_lock_flag_says` |
| A4 | R2, R3: a planted shell script is read by function and after a wrapper | `TheCensusReadsEachPlantedForm.test_a_shell_script_is_read_by_function_and_after_any_wrapper` |
| A5 | R2, R6: a planted Python script is read by its argument lists, a tuple that is no call argument is not read, and a built list is undecided | `TheCensusReadsEachPlantedForm.test_a_python_script_is_read_by_its_argument_lists` |
| A6 | R7: an unlisted planted site warns at warn and fails at fail | `TheVerdictKeepsItsLevel.test_an_unlisted_site_warns_at_warn_and_fails_at_fail` |
| A7 | R7: a planted stale entry fails at both levels | `TheVerdictKeepsItsLevel.test_a_stale_entry_fails_at_both_levels` |
| A8 | R1, R7: over the repository, every listed entry matches a current site, each has a reason, and each unlisted site prints at warn | `TheRepositoryRunsEachDependencyCommandOnItsLock.test_each_site_is_listed_with_its_reason_and_no_entry_is_stale` |

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_an_empty_population_is_void
A2: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_each_cargo_form_is_a_site_or_clean_as_its_lock_flag_says
A3: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_each_node_and_apple_and_pip_form_is_a_site_or_clean_as_its_lock_flag_says
A4: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_a_shell_script_is_read_by_function_and_after_any_wrapper
A5: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_a_python_script_is_read_by_its_argument_lists
A6: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_an_unlisted_site_warns_at_warn_and_fails_at_fail
A7: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_a_stale_entry_fails_at_both_levels
A8: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_each_site_is_listed_with_its_reason_and_no_entry_is_stale
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_ci_lock_flags.py` | repo | added: the census, its listing and its tests |
| `scripts/tests/fixtures/lock-flags/workflow.yml.fixture` | repo | added: a planted case |
| `scripts/tests/fixtures/lock-flags/script.sh.fixture` | repo | added: a planted case |
| `scripts/tests/fixtures/lock-flags/script.py.fixture` | repo | added: a planted case |
| `docs/specs/SPEC-671-every-ci-dependency-command-runs-on-its-lock.md` | repo | added |
| `docs/decisions/ADR-694-a-text-census-holds-ci-dependency-commands-to-their-lock-at-warn.md` | repo | added |
| `docs/schematics/ci-lock-flags.md` | repo | added |
| `docs/red-first/SPEC-671.md` | repo | added |
| `changelog.d/ci-lock-flags.md` | repo | added |
| `scripts/mutation-rows.d/S67100-S67199.json` | repo | added: the rows of section 9 |

## 5. What this does NOT cover

No lock flag is added to any site. A script's own calls, a script a step writes and runs, a program
named only by a variable, and a tool's own configuration (such as `.cargo/mutants.toml`) are not
read. Whether `cargo clean`, `cargo codspeed` and `cargo llvm-cov report` read the lock is
unanswered by the tools' documentation; they stay sites, each listed with its reason.

## 6. Risks

A text census misses a command whose text it cannot read (R6), so every count is a lower bound. A
form that is not refused needs a reason in R5; any other form stays a site.

## 7. What only CI proves

`TheRepositoryRunsEachDependencyCommandOnItsLock` reads the whole population, so it runs in CI only, in the `hygiene` job's Python stage. Its
first run there lists every unlisted site at warn, and the next push lists each with that job's id.

## 8. Formal model

None: a text census has no protocol, state machine or concurrency to model.

## 9. Mutation rows (S67100-S67199)

Eleven hand-written rows on the census's arms, each killed by a planted test by its full id, in
`scripts/mutation-rows.d/S67100-S67199.json`.
