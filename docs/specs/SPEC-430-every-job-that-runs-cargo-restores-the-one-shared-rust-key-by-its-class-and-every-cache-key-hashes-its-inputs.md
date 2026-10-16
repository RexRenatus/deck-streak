# SPEC-430: every job that runs cargo restores the one shared Rust key by its class, only a push or the warm job saves it, and every cache key hashes its inputs and nothing run-unique

- **Issue:** none; the pull request closes nothing. **Context(s):** `repo` (`scripts/tests/test_ci_workflows.py`,
  its rows and `docs/`).
- **Decided by:** ADR-444 (this SPEC's own: D1 to D10, each with what it was chosen against); SPEC-038 R1, R2 and
  R12, SPEC-191 R9, ADR-055 and ADR-191 decided the caches this SPEC holds, and none of them is amended.
- **Status:** built in its pull request into `dev`, which lands after the open release pull request #822. It holds
  `docs/red-first/SPEC-430.md`.

## 1. The problem, measured

Every count below is the output of a read-only reader of R2 to R5 and R13, with its restore, save, key, fallback
and writer counts, run over `git show` exports of `dev` `b25ed04a`: the 15 workflow files and the 14 scripts they
call. Every other figure was read at `b25ed04a` from the tree (`git show <sha>:<path>`), or from a DeckStreak CI job
named by its id and its duration. The workflows, `scripts/check.sh` and the called scripts are those of `ac7ebd75`.

- The 15 workflow files under `.github/workflows/` (each `.yml`) hold 15 `actions/cache/restore` steps (`ci.yml` 10,
  `mutation-weekly.yml` 3, `rust-cache.yml` 2), 4 `actions/cache/save` steps (`ci.yml` 3, `rust-cache.yml` 1) and
  no combined `actions/cache` step (the reader prints `restores 15 saves 4 combined 0`).
- 12 restores carry one Rust key byte for byte,
  `rust-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ hashFiles('Cargo.lock') }}`
  (`ci.yml:62`, `:132`, `:184`, `:328`, `:508`, `:691`, `:960`; `mutation-weekly.yml:143`, `:284`, `:427`;
  `rust-cache.yml:51`, `:78`). The 4 saves key on a restore's primary key (`ci.yml:83`, `:249`, `:259`;
  `rust-cache.yml:99`). The pnpm store's 2 restores key on `pnpm-${{ runner.os }}-${{ hashFiles('pnpm-lock.yaml') }}`
  (`ci.yml:221`, `:829`), and the browser's 1 restore on the version `pnpm-lock.yaml` locks (`ci.yml:240`).
  That is 19 keys and 13 fallback lines (11 Rust, `rust-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-`;
  2 pnpm, `pnpm-${{ runner.os }}-`). No key or fallback names a run-unique context, variable or clock.
- 19 jobs run cargo, directly, after a wrapper's ` -- `, or through a script they call (section 5). 11 of them restore
  the shared key. 8 restore no cache: three only list mutants (`ci.yml` `mutation-plan`, `mutation-weekly.yml` `size`
  and `listing`), two are the measurement workflows (`engine-measure.yml`, whose cold build "restores no cache, on
  purpose", SPEC-038 R12; `fsrs7-measure.yml`, which builds only release and cross-target artifacts), and three
  build releases (`release.yml` `release` and `audit-sync-server`, and `xcframework.yml`, which two tag-pushed
  workflows call).
- One called Python script holds cargo argument lists: `scripts/mutation_rows.py`, `["cargo", "test", ...]` in
  `run_killer` (`:1037`) and `builds` (`:1111`), both reached only through `prove`. Read whole, it would class
  every job that calls it: `ci.yml`'s `mutation-plan` (`legs`, `ci.yml:448`) as shared, and `mutation-verdict`
  (`merge`, `ci.yml:766`) as a twentieth job that runs cargo, two shared jobs that restore nothing. Read along the
  subcommand each call names (R13), neither reaches a list. The reader reads 58 Python calls, 37 scoped by their
  subcommand, 13 unscoped and 8 whole, and 3 scoped reads drop the script's two lists (`legs`, `retired` and
  `merge`).
- The shared entry restores on a push and on a pull request: a dev push's `rust` job hit the exact key and had the
  tree restored about 53 s later (job 116828746605, about 53 s); a pull request's `rust` job hit the same key
  (job 116815921139, about 54 s). That entry was saved by a dev push's `rust` job, which restored by the fallback and
  whose save step took about 92 s (job 116499004794, about 92 s). The warm workflow has not run: a schedule runs only
  from the default branch's copy, which does not hold `rust-cache.yml` yet (`rust-cache.yml:3-7`).
- What holds the shape today: `OnlyAPushSavesACache` reads the 4 `ci.yml` jobs whose `check.sh` stages compile Rust
  for a restore (`test_ci_workflows.py:2575-2622`), every workflow's saves for their guard (`:2624-2642`,
  `cache_problems`, SPEC-038 R2, SPEC-191 R9) and the browser key (`:2661-2697`). It reads no restore in
  `mutation-rust`, `mutation-rows`, `web-engine`, `mutation-weekly.yml` or `rust-cache.yml`, asserts that the Rust key
  contains its two hashes rather than that it is the shared key, and reads no key for a run-unique part.

## 2. Requirements

R1. A new test class, `TheCachesHoldTheirShape`, in `scripts/tests/test_ci_workflows.py`, sits directly above the
    line `class TheEngineBuildsInEveryRustJob(unittest.TestCase):`, with its constants named `SHAPE_*` and its
    helpers `shape_*` beside it. The delivery changes no existing line of the module. The new code reads every file
    through the module's loader `workflow_file_text`, so `NOT_WORKFLOW_READS` and `DYNAMIC_IMPORTS` are unchanged, and
    it calls the module's helpers read-only (ADR-444 D5).
R2. **The cargo reader** (ADR-444 D1). A step runs cargo when its `run:` script, comments removed and backslash
    continuations joined, holds `cargo <subcommand>` at a command position: the start of a line; after `;`, `&&`,
    `||`, `|`, `(`, `$(` or a backtick; after `if`, `then`, `do`, `else`, `elif`, `while`, `until`, `!`, `exec` or
    `time`; after one or more `NAME=value` assignments; or after a ` -- ` separator. `<subcommand>` is the first
    word after `cargo` that is not an option (a word starting `-` or `+`), and a call needs it to match
    `[a-z][a-z0-9-]*` at its start: so `cargo = x` is no call, and `cargo build` and `cargo --locked test` are calls.
    A step also runs what it calls: a `bash` or `sh` call of a path under `scripts/` is read with the same reader; a
    `python3` call of a path under `scripts/` is read for each argument list that starts `["cargo", "<subcommand>"`,
    along the subcommand the call names (R13); and a `bash scripts/check.sh` call (the module's `GATE_CALL`) is read
    stage by stage from the body of each named stage's function in `scripts/check.sh`, every stage of `STAGES_ALL`
    when it names none. A stage body that calls `need_cargo` and holds no cargo command counts as one call that
    builds the cache's profile. A called script the tree lacks is a problem by name. The reader goes one script
    deep, and takes the root it reads scripts from as a parameter.
R3. **Kinds of call.** A call compiles nothing when its subcommand is `fmt`, `fetch`, `deny`, `clean` or `metadata`,
    or it is a listing (`cargo mutants` with `--list`, `cargo nextest list`). It builds another profile or target
    when its arguments hold `--release`, `-r`, `--profile` with any profile but `dev` or `test`, or `--target`, or its
    subcommand is `install`. Every other call, an unknown subcommand included, builds the cache's profile.
R4. **Classes of a job that runs cargo**, the first that fits: *release*, a job of a workflow whose `on` holds
    `release`, or `push` with `tags` or `tags-ignore`, or of a reusable workflow such a workflow calls
    (`LOCAL_CALL`, or a `./.github/workflows/<file>` call), at any depth; *another runner*, a `runs-on` that is a plain
    string starting `macos-` or `windows-`; *shared*, a job with at least one call that builds the cache's profile;
    *cache-free*, every other.
R5. **What each class asserts.** A shared job holds exactly one `actions/cache/restore` step whose `path` lines equal
    `RUST_CACHE`, whose `key` equals the shared key byte for byte and whose `lookup-only` is not `true`; it sits at a
    lower step index than the first step that builds the cache's profile, and its `if:` is absent or equal to that
    step's. A cache-free job's every Rust-path restore carries the shared key. A release-class job uses no
    `actions/cache/restore`, no `actions/cache`, no action of `CACHE_BY_THEMSELVES`, no `actions/setup-node` with a
    `cache` input or with `package-manager-cache` other than `false`, no `pnpm/action-setup` with `cache` other than
    `false`, and no other action whose path names `cache`. A job on another runner holds no Rust-path restore.
R6. **Who saves** (ADR-444 D2). A writer is a step using `actions/cache/save`, `actions/cache`, an action of
    `CACHE_BY_THEMSELVES`, `actions/setup-node`, `pnpm/action-setup`, or any other action whose path names `cache`
    (case-insensitive) other than `actions/cache/restore`; an action of that last kind is a problem by name. Each
    workflow's writers are judged by `cache_problems(name, workflow)` as the module defines it, read and not edited.
    A Rust save (its `path` lines `RUST_CACHE`) keys on `${{ steps.<id>.outputs.cache-primary-key }}`, `<id>` an
    earlier `actions/cache/restore` of the same job with the same `path` lines and the shared key.
R7. **Keys hash their inputs** (ADR-444 D3). Every key of every `actions/cache/restore`, `actions/cache/save` and
    `actions/cache` step is examined; a key of the form `${{ steps.<id>.outputs.cache-primary-key }}` is resolved to
    the key of `<id>`, an earlier restore of the same job with the same `path` lines, and is a problem when no such
    step exists. By its `path` lines: the Rust cache's key is the shared key and its fallback is absent or exactly
    `rust-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-`; the pnpm store's (one line
    `${{ steps.<p>.outputs.path }}`, `<p>` an earlier step of the job whose `run:` holds `pnpm store path`) is
    `pnpm-${{ runner.os }}-${{ hashFiles('pnpm-lock.yaml') }}`, its fallback absent or exactly `pnpm-${{ runner.os }}-`;
    the browser's (`BROWSERS`) matches
    `playwright-${{ runner.os }}-${{ steps.<v>.outputs.version }}-chromium-headless-shell`, `<v>` an earlier step of
    the job whose `run:` names `pnpm-lock.yaml`, with no fallback. Any other `path` lines are a problem: a cache this
    rule does not know.
R8. **No run-unique part** (ADR-444 D4). Every key and every fallback line, resolved as in R7, is examined. Each
    `${{ }}` in one is `runner.os`, `hashFiles(...)` over literal paths, `steps.<id>.outputs.<name>`, `env.<NAME>` or
    `matrix.<name>`, and any other expression is a problem; `github.run_id`, `github.run_number`, `github.run_attempt`
    and `github.sha` are named in theirs. An `env.<NAME>` is resolved through the step's, the job's and the workflow's
    `env:`, in that order, and its value is judged by the same rule. A step output is judged by its producing step's
    `run:`, which names none of `GITHUB_RUN_ID`, `GITHUB_RUN_NUMBER`, `GITHUB_RUN_ATTEMPT`, `GITHUB_SHA`, `date`,
    `EPOCHSECONDS`, `EPOCHREALTIME`, `RANDOM` or `uuidgen`; a producing step the job lacks is a problem.
R9. **Counts.** Each of the four tree tests judges its controls first, then prints `examined N <what>` through
    `_support.examined`, which refuses zero, and prints each class's count (a class may hold none, so its count is
    printed and not refused). Its controls are read by code the check did not build: the `ci.yml` jobs `stage_calls`
    names with a stage of `COMPILES_RUST` other than `test-release`, the steps `action` and `paths` read, and the
    fallback lines `lines_of` reads.
R10. **Planted fixtures.** Each refusal test writes its workflows into a temporary root under both suffixes, `.yml`
    and `.yaml`, with the scripts they call (`scripts/check.sh` and the rest) planted beside them, spells every key
    and command literally (never from a `SHAPE_` constant), and asserts the exact list of problems, in order. Section
    8's rows name the planted shape each one is killed by.
R11. **The tip.** The four checks pass at `dev` with no workflow change: 0 sites break a rule under R4's classes
    (the reader's census at `b25ed04a`, section 5), so none is cured here. A tip that breaks one when this is built
    is a stop, and its cure is the delivery's.
R12. **Kill criteria** are ADR-444 D6's: live precision over the last 30 merged PRs (below 0.8 with at least 5 hits,
    a demotion is proposed), 2 of a new zero-hit rule's first 5 hits false (a demotion is proposed), a demotion is a
    weakening that goes to the owner as a signed ruling, and `examined 0` or a reader that cannot read what a
    workflow names is VOID, which blocks.
R13. **The Python read follows the subcommand** (ADR-444 D10). A `python3` call of a script is read by an `ast`
    walk of the script. The walk starts at the module's top-level statements and reads in turn each top-level
    function or class, and each method of a top-level class, that a body it reads names by a `Name` or an
    attribute, until no new name is reached; the argument lists are those in the bodies read. When the call names a
    subcommand (the word after the script's path, matching `[a-z][a-z0-9-]*` and ending at a space, a shell
    separator or the line's end) and the dispatch of the script's last top-level `main` compares exactly one
    expression, the walk follows that subcommand: the dispatch is the `if` statements of `main` whose test compares
    an expression with a string by `==`, or with a tuple, list or set of strings by `in`, and each gives way to its
    body when its strings hold the subcommand and to its `else` otherwise, at any `elif` depth; every other
    statement of `main` is kept. A call that names no subcommand, and a dispatch over more than one expression,
    keep every branch. A script that names `getattr`, `globals`, `vars`, `locals`, `eval`, `exec`, `__import__` or
    `importlib` is read whole. A1 prints the Python calls it read, scoped, unscoped and whole, and names each call
    whose read drops an argument list the whole script holds.

## 3. Acceptance criteria of SPEC-430

| id | criterion | decided by |
|---|---|---|
| A1 | every job of every workflow that runs cargo is examined and classed, and each meets its class (R2 to R5, R9, R13): every `ci.yml` job whose stages compile the cache's profile is shared, and every job holding a non-lookup shared restore runs cargo | `test_ci_workflows.py` `test_every_job_that_runs_cargo_restores_the_one_shared_rust_key` |
| A2 | the cargo census refuses each planted shape of section 8's A2 rows and passes each planted negative control | `test_ci_workflows.py` `test_the_cargo_census_refuses_each_planted_shape` |
| A3 | only the warm job or a push to `dev` or `main` that missed saves the Rust key, every writer is admitted by the module's save scenarios, and every Rust save keys on a shared restore (R6, R9): the saves read equal the `actions/cache/save` steps `action` reads, and the Rust saves those whose `paths` are `RUST_CACHE` | `test_ci_workflows.py` `test_only_the_warm_job_or_a_push_to_dev_or_main_saves_the_rust_key` |
| A4 | the writer census refuses each planted shape of section 8's A4 rows | `test_ci_workflows.py` `test_the_writer_census_refuses_each_planted_shape` |
| A5 | every cache key hashes its lock and toolchain inputs (R7, R9): the keys read by class equal the cache steps `paths` reads by class | `test_ci_workflows.py` `test_every_cache_key_hashes_its_lock_and_toolchain_inputs` |
| A6 | the key census refuses each planted shape of section 8's A6 rows | `test_ci_workflows.py` `test_the_key_census_refuses_each_planted_shape` |
| A7 | no key or fallback carries a run-unique part (R8, R9): the texts examined equal the keys plus the fallback lines `lines_of` reads, every save's key resolved | `test_ci_workflows.py` `test_no_cache_key_carries_a_run_unique_part` |
| A8 | the run-unique census refuses each planted shape of section 8's A8 rows | `test_ci_workflows.py` `test_the_run_unique_census_refuses_each_planted_shape` |
| A9 | the Python read follows the subcommand the step names (R13): a planted subcommand whose handler reaches a cargo argument list only through a function it calls is a job that runs cargo (the negative control), a planted subcommand that reaches none is not, a call that names no subcommand and a dispatch over two expressions keep every branch, and a script that names a run-time lookup is read whole, each as section 8's A9 rows plant it | `test_ci_workflows.py` `test_the_python_read_follows_the_subcommand_the_step_names` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_job_that_runs_cargo_restores_the_one_shared_rust_key
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_cargo_census_refuses_each_planted_shape
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_only_the_warm_job_or_a_push_to_dev_or_main_saves_the_rust_key
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_writer_census_refuses_each_planted_shape
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_cache_key_hashes_its_lock_and_toolchain_inputs
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_key_census_refuses_each_planted_shape
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_no_cache_key_carries_a_run_unique_part
A8: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_run_unique_census_refuses_each_planted_shape
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_python_read_follows_the_subcommand_the_step_names
```

The module runs in CI's `hygiene` job, through `check.sh`'s `python` stage, on every pull request into `dev` or `main`
and every push to them; the aggregate `ci` needs that job, so no required check changes. Each criterion is red before
its code: the red commit holds the tests and the four census helpers as stubs that return no problems and no
population, so A1, A3, A5 and A7 fail on their controls before their counts, and A2, A4, A6, A8 and A9 on their
expected refusals.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_ci_workflows.py` | 1 | changed, insert-only: R1's class, its `SHAPE_*` constants and `shape_*` helpers above `class TheEngineBuildsInEveryRustJob`; no existing line changes |
| `scripts/mutation-rows.d/S43000-S43099.json` | 1 | added: rows S43000-S43060 (section 8) |
| `docs/specs/SPEC-430-every-job-that-runs-cargo-restores-the-one-shared-rust-key-by-its-class-and-every-cache-key-hashes-its-inputs.md` | 1 | added |
| `docs/decisions/ADR-444-the-cache-shape-is-four-checks-in-the-workflow-tests-each-cargo-job-classed-by-what-it-builds-and-where-it-runs.md` | 1 | added |
| `docs/schematics/ci-jobs-and-caches.md` | 1 | changed: one insert-only section after its last line |
| `docs/red-first/SPEC-430.md` | 1 | added |
| `changelog.d/cache-shape-430.md` | 1 | added |

No workflow file changes. The module's own censuses read the new code: its file-read census
(`WorkflowFilesAreReadAsBytes`) and its import census find no new site, because every read goes through the loader;
the 11 rows already on `scripts/tests/test_ci_workflows.py` keep their anchors, each occurring exactly once.

## 5. The census at `b25ed04a`, by the reader

The reader of R2 to R5 and R13, run over the tree at `b25ed04a`, prints this table: each job that runs cargo, its
class, what it runs, and its shared restore.

| workflow | job | class | what builds or runs cargo | its shared restore |
|---|---|---|---|---|
| `ci.yml` | `rust` | shared | `check.sh` `clippy`, `test`, `doctest` | `ci.yml:55`, key `:62` |
| `ci.yml` | `engine` | shared | `check.sh` `test-engine` | `:125`, key `:132` |
| `ci.yml` | `release` | cache-free | `check.sh` `test-release` (`--release`); `cargo fetch` | `:177`, key `:184` |
| `ci.yml` | `hygiene` | shared | `check.sh` `python` (`need_cargo`, guard tests that build) | `:321`, key `:328` |
| `ci.yml` | `mutation-plan` | cache-free | `cargo mutants --list`, twice; `scripts/mutation_rows.py legs` reaches no cargo list | none |
| `ci.yml` | `mutation-rust` | shared | `cargo mutants --in-place`, after `memory_scope.py`'s ` -- ` | `:501`, key `:508`, under the build's `if:` |
| `ci.yml` | `mutation-rows` | shared | `scripts/mutation_rows.py prove` (`["cargo", "test", ...]`, `:1037` and `:1111`); `retired` reaches none | `:684`, key `:691`, under the build's `if:` |
| `ci.yml` | `web-engine` | cache-free | `scripts/web-engine-build.sh` (`--release --target`) | `:953`, key `:960` |
| `mutation-weekly.yml` | `size` | cache-free | `cargo mutants --list` | none |
| `mutation-weekly.yml` | `rust` | shared | `cargo mutants --in-place`, after ` -- ` | `:136`, key `:143` |
| `mutation-weekly.yml` | `rows` | shared | `scripts/mutation_rows.py prove` | `:277`, key `:284` |
| `mutation-weekly.yml` | `listing` | cache-free | `cargo mutants --list` | none |
| `mutation-weekly.yml` | `rehearsal` | shared | `cargo mutants -f ... --in-place`, after ` -- `; `scripts/mutation_rows.py prove`; its `cargo = json.load(...)` line (`:480`) is no call | `:420`, key `:427` |
| `rust-cache.yml` | `warm` | shared | `cargo clippy`, `cargo nextest run --no-run` | `:71`, key `:78`; the lookup `:43` is `lookup-only` |
| `engine-measure.yml` | `measure` | cache-free | `cargo build --release`, `cargo test --release`, `cargo deny` | none (SPEC-038 R12) |
| `fsrs7-measure.yml` | `measure` | cache-free | `cargo run --release`, `cargo build --release --target`, `cargo check --target` | none |
| `release.yml` | `release` | release | `cargo build --release`, `cargo install`, `scripts/web-engine-build.sh` | none |
| `release.yml` | `audit-sync-server` | release | `scripts/audit-sync-server.sh` (`cargo deny`) | none |
| `xcframework.yml` | `xcframework` | release (called by `apple-on-tag.yml` and `testflight-release.yml`; on a `macos-` runner too) | `cargo rustc --release --target`, `cargo run` | none |

That is 19 jobs: 9 shared, 7 cache-free, 3 release and 0 on another runner alone, and no job breaks its class
(the reader prints `jobs that run cargo: 19` and `sites that break: 0`). `ci.yml`'s `mutation-verdict` runs
`scripts/mutation_rows.py merge`, which reaches no cargo list, so it is not a job that runs cargo. The
writers are 19 steps (4 `actions/cache/save`, 8 `actions/setup-node`, 7 `pnpm/action-setup`), of which 4 save and 2
save the Rust key (`ci.yml:76`, `rust-cache.yml:92`), each under its guard (ADR-444 D2). Sites cured: none.

## 6. What this does NOT cover

- It changes no workflow and no job's cache use: the three listings, `fsrs7-measure.yml` and the release builds keep
  restoring nothing, and `engine-measure.yml`'s cold build stays cold (#15).
- It changes no key's shape: a cache-free job does not gain a restore of the registry part alone, which would be a
  second key (#207).
- It adds no cache: `card-sandbox` and `web-engine` keep installing their dependencies and browsers without one (#207).
- It changes no cache's size, no eviction and no storage limit, and no scope a save lands in (#370).
- It edits no line of `OnlyAPushSavesACache`, `cache_problems` or the scenarios: SPEC-038 R2 and SPEC-191 R9 are judged
  as before (#207).
- It adds no lint package beyond the cache-shape rule: a further package is a delivery of its own, with its own
  requirements, criteria and kill criteria, tracked in #869
- It adds no allow-list arm to the cache-shape rule: no job, step or key passes by its name, and an arm is the
  owner's decision, tracked in #870

## 7. Risks

- **A class mislabels a job.** A build whose profile or target is set outside its command line reads as the cache's
  profile and is refused, which fails closed; a cargo call two scripts deep, a `runs-on` expression, and a bare
  `on: push` workflow are read too leniently (ADR-444 D1). Detected by: the planted controls of A2, and ADR-444 D6's
  precision over merged pull requests.
- **A reader drifts from the module it reads.** The checks reuse `cache_problems`, `stage_calls` and the loader; an
  edit to one moves what these checks see. Detected by: A1's, A3's, A5's and A7's controls, which compare with counts
  those helpers do not compute.
- **A row's anchor is not unique.** New code that repeats an anchor of the 11 rows already on the module VOIDs them.
  Detected by: the anchor census of every row on the module before each commit, and CI's rows verdict by id.
- **The checks pass while examining less.** Detected by: each test's `examined` count, VOID on zero, after its
  controls.
- **The Python read scopes too narrowly.** A subcommand that reaches cargo through a name the walk cannot follow
  passes. Detected by: the whole-script read of every script that names a run-time lookup (R13), A1's print of each
  call whose read drops a list the whole script holds, and A9's negative control.

## 8. Hand rows S43000-S43060

Each row's mutant is one helper change, its target `scripts/tests/test_ci_workflows.py`, its find written from the
landed line and occurring exactly once, and its killer the named test of `test_ci_workflows.TheCachesHoldTheirShape`.
A planted job named below is part of that test's fixtures, with the verdict the test expects for it.

| row | the mutant | killed through |
|---|---|---|
| S43000-THE-READER-FINDS-CARGO-AFTER-A-WRAPPER-SEPARATOR | drops the ` -- ` position | A2: a `-- cargo mutants` job with no restore, refused |
| S43001-THE-READER-FINDS-CARGO-AFTER-AN-ENV-ASSIGNMENT | drops the `NAME=value` prefix | A2: a `NAME=1 cargo test` job, refused |
| S43002-THE-READER-FINDS-CARGO-AFTER-IF | drops `if` from the command words | A2: an `if cargo test; then` job, refused |
| S43003-THE-READER-JOINS-CONTINUATIONS | drops the backslash join | A2: a `cargo build \` / `--release` job, passed |
| S43004-A-SHELL-SCRIPT-THE-STEP-CALLS-IS-READ | drops the shell-script read | A2: a `bash scripts/build.sh` job, refused |
| S43005-A-PYTHON-CARGO-ARGV-IS-READ | drops the argument-list read | A2: a `python3 scripts/prove.py` job, refused |
| S43006-A-CHECK-STAGE-IS-READ-FROM-ITS-BODY | reads a `check.sh` call as naming no stage | A2: a `check.sh clippy` job, refused, and a `check.sh test-release` job, passed |
| S43007-A-STAGE-THAT-NEEDS-CARGO-UNSEEN-BUILDS | the `need_cargo` arm counts no call | A2: a `check.sh python` job, refused |
| S43008-A-SCRIPT-THE-TREE-LACKS-IS-A-PROBLEM | drops that problem | A2: a `bash scripts/absent.sh` job, refused by name |
| S43009-A-LISTING-COMPILES-NOTHING | drops the `--list` arm | A2: a `cargo mutants --list` job, passed |
| S43010-FMT-COMPILES-NOTHING | drops `fmt` | A2: a `cargo fmt` job, passed |
| S43011-FETCH-COMPILES-NOTHING | drops `fetch` | A2: a `cargo fetch` job, passed |
| S43012-DENY-COMPILES-NOTHING | drops `deny` | A2: a `cargo deny` job, passed |
| S43013-CLEAN-COMPILES-NOTHING | drops `clean` | A2: a `cargo clean` job, passed |
| S43014-METADATA-COMPILES-NOTHING | drops `metadata` | A2: a `cargo metadata` job, passed |
| S43015-RELEASE-BUILDS-ANOTHER-PROFILE | drops `--release` | A2: a `cargo build --release` job, passed |
| S43016-A-TARGET-BUILDS-ANOTHER-TARGET | drops `--target` | A2: a `cargo check --target` job, passed |
| S43017-A-LOCKED-BUILD-IS-THE-CACHE-PROFILE | adds `--locked` to the other-profile flags | A2: a `cargo build --locked` job, refused |
| S43018-AN-UNKNOWN-SUBCOMMAND-BUILDS | an unknown subcommand compiles nothing | A2: a `cargo frobnicate` job, refused |
| S43019-A-TAG-PUSH-MAKES-THE-RELEASE-CLASS | drops the `tags` arm | A2: a tag-pushed job with a restore, refused |
| S43020-A-RELEASE-EVENT-MAKES-THE-RELEASE-CLASS | drops the `release` arm | A2: a `release`-event job with a restore, refused |
| S43021-A-RELEASE-WORKFLOWS-CALLEE-IS-RELEASE-CLASS | drops the callee walk | A2: a called workflow's job with a restore, refused |
| S43022-A-MACOS-RUNNER-IS-ANOTHER-RUNNER | drops the `macos-` arm | A2: a `macos-` job with no restore, passed |
| S43023-A-RELEASE-JOB-RESTORES-NO-CACHE | drops the release-class assertion | A2: the tag-pushed job with a restore, refused |
| S43024-THE-RESTORE-PRECEDES-THE-FIRST-BUILD | admits a restore after the build | A2: a build-then-restore job, refused |
| S43025-A-LOOKUP-ONLY-STEP-RESTORES-NOTHING | drops the `lookup-only` filter | A2: a lookup-only job, refused |
| S43026-THE-RESTORE-SHARES-THE-BUILDS-CONDITION | drops the `if:` comparison | A2: a restore under another `if:`, refused |
| S43027-THE-SHARED-KEY-IS-RESTORED-ONCE | admits two restores | A2: a job restoring twice, refused |
| S43028-THE-RESTORED-KEY-IS-THE-SHARED-KEY | admits any Rust-path key | A2: a restore whose key ends `-v2`, refused |
| S43029-A-CACHE-NAMED-ACTION-IS-A-WRITER | drops the `cache`-name arm | A4: a planted `*-cache-action` step, refused |
| S43030-A-RUST-SAVE-KEYS-ON-A-SHARED-RESTORE | drops the Rust save's key check | A4: a save keyed `rust-literal`, refused |
| S43031-THE-SAVE-SCENARIOS-JUDGE-EVERY-WRITER | drops the `cache_problems` call | A4: a save on any push, refused on a pushed tag |
| S43032-THE-RUST-KEY-IS-MATCHED-WHOLE | equality becomes containment | A6: a key ending `-v2`, refused |
| S43033-THE-RUST-FALLBACK-KEEPS-THE-TOOLCHAIN | drops the fallback check | A6: the fallback `rust-${{ runner.os }}-`, refused |
| S43034-THE-PNPM-KEY-HASHES-ITS-LOCKFILE | drops the pnpm key check | A6: `pnpm-${{ runner.os }}`, refused |
| S43035-THE-BROWSER-VERSION-STEP-READS-THE-LOCKFILE | drops the lockfile read | A6: a version step that reads no lockfile, refused |
| S43036-AN-UNKNOWN-CACHE-IS-A-PROBLEM | drops that problem | A6: a `~/.npm` cache, refused |
| S43037-A-SAVE-RESOLVES-THROUGH-A-RESTORE-OF-ITS-PATHS | drops the path comparison | A6: a save resolved through another cache's restore, refused |
| S43038-AN-UNPLACED-EXPRESSION-IS-A-PROBLEM | admits any expression | A8: a key with `${{ inputs.salt }}`, refused |
| S43039-RUN-ID-IS-NAMED | drops `github.run_id` | A8: its key, refused by that name |
| S43040-RUN-NUMBER-IS-NAMED | drops `github.run_number` | A8: its key, refused by that name |
| S43041-RUN-ATTEMPT-IS-NAMED | drops `github.run_attempt` | A8: its key, refused by that name |
| S43042-THE-COMMIT-SHA-IS-NAMED | drops `github.sha` | A8: its key and its fallback line, refused by that name |
| S43043-AN-ENV-INDIRECTION-IS-RESOLVED | stops resolving `env.<NAME>` | A8: `${{ env.STAMP }}` over `${{ github.run_id }}`, refused |
| S43044-GITHUB-RUN-ID-IN-A-PRODUCING-STEP | drops `GITHUB_RUN_ID` | A8: a step output from `$GITHUB_RUN_ID`, refused |
| S43045-GITHUB-RUN-NUMBER-IN-A-PRODUCING-STEP | drops `GITHUB_RUN_NUMBER` | A8: its step output, refused |
| S43046-GITHUB-RUN-ATTEMPT-IN-A-PRODUCING-STEP | drops `GITHUB_RUN_ATTEMPT` | A8: its step output, refused |
| S43047-GITHUB-SHA-IN-A-PRODUCING-STEP | drops `GITHUB_SHA` | A8: its step output, refused |
| S43048-DATE-IN-A-PRODUCING-STEP | drops `date` | A8: a step output from `$(date +%s)`, refused |
| S43049-EPOCHSECONDS-IN-A-PRODUCING-STEP | drops `EPOCHSECONDS` | A8: its step output, refused |
| S43050-EPOCHREALTIME-IN-A-PRODUCING-STEP | drops `EPOCHREALTIME` | A8: its step output, refused |
| S43051-RANDOM-IN-A-PRODUCING-STEP | drops `RANDOM` | A8: its step output, refused |
| S43052-UUIDGEN-IN-A-PRODUCING-STEP | drops `uuidgen` | A8: its step output, refused |
| S43053-A-PYTHON-READ-FOLLOWS-THE-NAMED-SUBCOMMAND | keeps both branches of every dispatch `if` | A9: a `python3 scripts/verbs.py legs` job with no restore, passed and not a job that runs cargo |
| S43054-A-SUBCOMMAND-REACHES-WHAT-ITS-HANDLER-CALLS | stops the walk at the names the kept statements hold, so no function a handler calls is read | A9, the negative control: a `python3 scripts/verbs.py prove` job with no restore, whose cargo list sits in `run_tests`, which `prove` calls, refused |
| S43055-A-DISPATCH-TUPLE-NAMES-ITS-SUBCOMMANDS | drops the `in` arm of the dispatch test | A9: the `verbs.py legs` job, passed, beside a `("plan", "list")` branch that reaches the cargo list |
| S43056-A-CALL-WITH-NO-SUBCOMMAND-KEEPS-EVERY-BRANCH | follows the dispatch when the call names no subcommand | A9: a `python3 scripts/verbs.py --package core` job with no restore, refused |
| S43057-A-DISPATCH-OVER-TWO-EXPRESSIONS-KEEPS-EVERY-BRANCH | follows a dispatch over two expressions | A9: a `python3 scripts/modes.py go --mode fast` job with no restore, refused |
| S43058-A-RUN-TIME-LOOKUP-READS-THE-WHOLE-SCRIPT | drops the run-time lookup arm | A9: a `python3 scripts/table.py prove` job with no restore, whose handler `getattr` reaches, refused |
| S43059-A-CARGO-CALL-NEEDS-A-SUBCOMMAND-WORD | drops the subcommand-word test | A2: a `cargo = x` job, passed and not a job that runs cargo, beside a `cargo build` job with no restore, refused |
| S43060-AN-OPTION-BEFORE-THE-SUBCOMMAND-IS-SKIPPED | drops the option skip | A2: a `cargo --locked test` job with no restore, refused |

Each A2, A4, A6, A8 and A9 fixture also holds a positive control that passes (a shared job that restores first, a
Rust save under the dev-push guard, the three good keys, a key of `runner.os` and `hashFiles` alone, a
`verbs.py prove` job that restores first), so a check that refuses everything fails too. A9's planted scripts are
text the walk parses and never runs; A9 asserts the exact problems, in order, and the exact jobs that run cargo with
their classes. No row is planned on a `SHAPE_` constant's value: every constant is read by a planted
test that fails on any change to it.

## 9. References

ADR-444. SPEC-038 (R1, R2, R12), SPEC-191 (R9), ADR-055, ADR-191, and the schematic
`docs/schematics/ci-jobs-and-caches.md`.
