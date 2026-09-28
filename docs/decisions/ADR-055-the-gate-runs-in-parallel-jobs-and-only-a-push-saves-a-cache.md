---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner, who approved the CI-speed package), the DeckStreak builder"
---

# The gate runs in four parallel jobs, and only a push to dev or main saves a cache

## Context and Problem Statement

CI ran the whole local gate as one job: 4m57s for a code pull request, 2m43s for a docs-only one and
3m53s for a push to `dev`, with every Rust dependency compiled from nothing on every run (SPEC-038
section 1). The owner approved a package to make it fast: a Rust cache saved only from `dev` and
`main`, the gate split into parallel jobs, the pack rows run in parallel, superseded pull-request
runs cancelled, Playwright's browser cached, and per-stage timings. The target is a warm code pull
request in about a minute and a half.

How should the gate be split so that `check.sh` stays the one source of truth, which stage belongs
where when a stage's tools span jobs, and how are caches kept safe on a public repository whose pull
requests can come from forks?

## Decision Drivers

- `check.sh` is the gate, locally and in CI (ADR-017); a job runs stages, never its own commands.
- No stage may drop out of CI silently, and a missing tool must fail a stage by name.
- A cache is trusted input to the next run; a pull request's code must never write one that `dev`
  reads.
- A push to `dev` or `main` must always be judged; only a superseded pull-request run may be
  cancelled.
- zizmor stays clean; every action is pinned by its full commit SHA; the token stays read-only.

## Considered Options (the alternatives it was chosen against)

- A split `actions/cache/restore` and `actions/cache/save` pair for the Rust cache: chosen, because the key is written in `ci.yml`, where a test pins it to the toolchain pin and the lockfile, and the save's `if:` states the owner's rule where anyone can read it.
- `Swatinem/rust-cache` with `save-if`: rejected, because its key is composed inside the action, so no test can pin it to the toolchain pin and the lockfile, and it adds a third-party action with write access to the cache.
- `actions/cache` as one step: rejected, because it saves in its post step on every event, pull requests included.
- No Rust cache: rejected, because clippy, nextest and the doctests then compile every dependency on every run, 65 s before Anki's engine and many minutes after it (ADR-022).
- A key with the commit in it, saved by every push: rejected, because each push would upload a new multi-gigabyte entry into a 10 GB repository budget, evicting the entries pull requests restore.
- Saving `target/` whole: rejected, because 1.0 of its 1.9 GiB is the workspace's own artifacts, which a fresh checkout rebuilds anyway (Cargo judges a path package by its sources' modification times), so `cargo clean --workspace` runs first.
- Keeping `setup-node`'s `cache: pnpm`: rejected, because it saves the pnpm store from pull-request runs; a restore and save pair under the same rule as every other cache replaces it.
- The four jobs the owner named, `rust`, `web`, `packs` and `hygiene`, each calling `bash scripts/check.sh` once with its stages: chosen, because each job then installs only its own tools, and `check.sh` still decides what every stage does.
- A matrix over stage groups: rejected, because each group needs different tools, so every step would carry a condition on the matrix value, and the check names would change to `gate (rust)` and so on.
- One workflow file per job: rejected, because `needs` cannot cross workflows, so the aggregate `ci` could not need them.
- Sharding the pack rows across several jobs: rejected, because each shard pays a checkout and a runner start-up, and a bounded pool inside one job gives the same speed.
- Folding each stage's tool check into the stage, and dropping the `toolchain` stage: chosen, because a missing tool then fails the stage that needs it, by name and with its install hint, in whichever job or machine runs it, and every stage fits exactly one job.
- Keeping `toolchain` and running it in every job: rejected, because it checks every tool, and no job holds them all, so it would fail in each of them.
- A `toolchain` stage that takes the stages to check as arguments: rejected, because a second list of which tools each stage needs would drift from the stages themselves.
- Splitting the `audit` stage into `audit-rust` and `audit-web`: chosen, because `cargo deny` belongs with the Rust toolchain and `pnpm audit` with Node's, so each half runs in the job that already has its tool.
- Placing the whole `audit` stage in the job that has both tools: rejected, because that job would have to install the other toolchain too, which is what the split removes.
- A bounded thread pool in `scripts/pack-rows.py`, the smaller of 8 and the CPUs available by default: chosen, because every row is already its own process, so threads only wait on them, and results are gathered in row order.
- A process pool: rejected, because it would add a second process per row for no gain, since each row already runs in a process of its own.
- An unbounded pool: rejected, because about 400 rows would start about 400 interpreters at once.
- Asynchronous subprocesses: rejected, because their timeout and kill behaviour differ from `subprocess.run`'s, which the per-row timeouts rely on.
- One concurrency group per pull request with `cancel-in-progress` for pull requests, and a group of its own for each push: chosen, because a newer push to a pull request cancels the run it supersedes, and a push to `dev` or `main` can never be cancelled or replaced while it waits.
- One group per ref with `cancel-in-progress: false`, the old setting: rejected, because GitHub keeps one pending run per group and cancels the older pending run when a third arrives, so a push to `dev` could be cancelled.
- `queue: max` for pushes: rejected, because it cannot be combined with `cancel-in-progress: true` in one block, and it would still hold each push behind the one before it for no benefit.
- Caching Playwright's browser, keyed on the locked Playwright version, installed on a miss: chosen, because the owner asked for it, and the key moves exactly when the browser build does.
- Downloading the browser on every run, as Playwright's own CI guide advises: rejected, because the owner chose the cache; the guide's reasons (a restore costs about what a download does, and the system libraries cannot be cached) are measured in SPEC-038 section 7.
- The checksum-verified `protoc` 31.1 download #204 wrote for the gate job, carried into every job that compiles Rust, and its check carried into the stages that compile the engine: chosen, because ADR-022 pins that version and digest, the download takes about a second, and a build a guard test runs can reach the engine too.
- `protoc` in the `rust` job only: rejected, because `hygiene`'s guard tests build Rust as well, and a job that compiles Rust without the engine's build tool fails only when a test first reaches the engine.
- Ubuntu's `protobuf-compiler` package: rejected, because Ubuntu 24.04's is 3.21.12, not the 31.1 Anki's build pins (ADR-022).
- A third-party action that installs `protoc`: rejected, because it adds an action with the job's token to do what a checked download already does.
- Caching `protoc`: rejected, because its download takes about a second, less than a cache restore.
- Stage logs under `${{ runner.temp }}/check-logs`: chosen, because `actions/upload-artifact` skips hidden paths by default, which is why `.check-logs/` never uploaded, and a directory outside the checkout is read by no stage's scan.
- Keeping `.check-logs/` and setting `include-hidden-files: true`: rejected, because the logs would still sit inside the checkout, where `gitleaks dir` and the scrub walk the tree.

## Decision Outcome

Chosen options, as SPEC-038's requirements state them:

- **The jobs.** `rust` (`fmt clippy test doctest audit-rust`), `web` (`web audit-web`), `packs`
  (`packs`) and `hygiene` (`python scrub secrets`) need nothing and start together. The aggregate `ci`
  needs them, `workflow-lint` and `base-is-dev`. The required contexts and the rulesets are
  unchanged. `hygiene` and `packs` check out the whole history, because they read it.
- **The stages.** Every stage checks its own tools first; `toolchain` is gone; `audit` is
  `audit-rust` and `audit-web`. `check.sh` appends each stage's seconds, verdict and exit to
  `timings.tsv` in its log directory.
- **The caches.** Rust (`~/.cargo` registry and git database, and `target/`, keyed on the toolchain
  pin and `Cargo.lock`), the pnpm store (keyed on `pnpm-lock.yaml`) and Playwright's browser (keyed on
  the locked Playwright version) are each restored with `actions/cache/restore` and saved with
  `actions/cache/save` only when a push to `dev` or `main` missed its exact key. A pull request
  restores and never saves. The Rust job cleans the workspace's own artifacts before its save, and
  sets `CARGO_INCREMENTAL=0`. `hygiene`, whose python stage builds a Rust example (SPEC-042's rails
  test), restores the Rust cache too and never saves it.
- **The engine's build tool.** `rust` and `hygiene` install `protoc` 31.1 before their stages,
  checked against ADR-022's digest, and `clippy`, `test` and `doctest` check it (or `PROTOC`) by
  name. `engine-measure.yml` is unchanged.
- **The pool.** `pack-rows.py --jobs N`, by default the smaller of 8 and the CPUs available, with
  the same timeouts, verdicts, exit codes and row order as the serial runner.
- **Concurrency.** Pull-request runs group by their ref and cancel the run they supersede; each push
  has a group of its own.

### Consequences

- Good, because the gate's wall time becomes its slowest job's, and a warm Rust job compiles only
  the workspace.
- Good, because a pull request, a fork's included, can read `dev`'s caches and write none.
- Good, because a missing tool now names itself in the stage that needs it, and the stage logs reach
  the run at last.
- Bad, because four jobs pay four start-ups, checkouts and setups.
- Bad, because the first run after a lockfile or toolchain change is cold until a push to `dev`
  saves its cache.

### Confirmation

- `scripts/tests/test_ci_workflows.py`: A1 to A4, A7, A10, A11 and A13 of SPEC-038.
- `scripts/tests/test_check_gate.py`: A5, A6 and A12.
- `scripts/tests/test_pack_wiring.py`: A8 and A9.
- zizmor over `.github/workflows`, in the `workflow-lint` job.

## What would make this wrong

- GitHub lets a pull request's run write a cache that its base branch reads. The save rule would
  still hold, but the scope that backs it up would be gone.
- GitHub changes how a concurrency group treats pending runs. A10's scenarios model today's rule.
- A runner image drops a library the headless shell needs, and `--with-deps` must return.

## More Information

SPEC-038; issue #207; ADR-017 (hosted CI); ADR-035 (required checks from GitHub Actions); ADR-004
(the pack runner); ADR-022 (the engine's build budget).

## Note, 2026-09-28: the engine's slow tests run in a job of their own

Recorded by SPEC-038's amendment (its section 8, R13 to R15, A16 to A18). This decision is
unchanged; the note adds a fifth gate job to it.

With Anki's engine in the workspace (SPEC-022), a warm code pull request's `rust` job took 4m19s and
4m22s (runs 36368711222, attempt 2, and 36371239856), and 139 s of each was one run: SPEC-022's sync
and budget tests, eight of them taking 20 to 81 s each. The other 116 tests of the workspace took
under 6 s together. It was decided, under the owner's delegation, that a parallel `engine` job runs
those tests and `rust` runs the rest, and that every test stays required, because `ci` needs both.

Chosen, and what each was chosen against:

- One nextest filterset, `ENGINE_TESTS` in `scripts/check.sh`, defined once: `test` runs
  `not (<the set>)` and a new `test-engine` stage runs the set, with commands that differ in nothing
  else. It was chosen because the partition is then complementary by construction, and one test
  proves that both stages read the one definition (A16).
  - A nextest profile per stage, each with its own `default-filter`: rejected, because the set
    would be written twice, once negated, and the copies could drift apart.
  - A test group assigned by an override, selected with `group(engine)`: rejected, because a test
    group also caps its tests' concurrency, and `group()` cannot be judged per binary, so every
    binary would be run to list its tests.
  - Building `test-engine` for `deck-streak-ingest` alone (`-p`): rejected, because Cargo unifies
    features over the packages it builds. That build resolves `tokio` and what depends on it
    differently from the `--workspace` build the cache holds, so it would recompile them on every
    warm run (SPEC-038 section 8).
  - Naming the eight slow tests one by one: rejected, because a test added to those binaries would
    land in `rust`, and a renamed one would move without a word. Whole binaries keep their tests.
  - The whole `deck-streak-ingest` package: rejected, because its other tests take under a second
    and gain nothing from a second runner.
- A fifth gate job, `engine`, that starts with the other four, restores the Rust cache and never
  saves it, and is a need of `ci`. It was chosen because a warm pull request then waits for the
  slower of `rust` and `engine`, not for their sum.
  - Leaving the tests in `rust`: rejected, because that is the 4m19s path.
  - Skipping `engine` when a pull request leaves the engine alone, or moving its tests out of the
    gate: rejected, because every test stays required, and `ci` fails on a skipped need.
  - Letting `engine` save the cache as well: rejected, because two jobs saving one key race, and
    `rust` builds every target, so its save already holds what `engine` restores.
- Two slices of the engine set, one per runner, from one matrix: each leg hands `test-engine` its
  own slice, `m/N` with N the matrix's size, and `test-engine` passes it to nextest's
  `--partition slice:m/N`. It was chosen by measurement (SPEC-038 section 8). Measured twice each,
  one engine job took 3m39s and 3m38s, and the slower of two slices 2m32s both times, which took the
  gate from 3m43s to 2m39s and 2m48s.
  - One engine job: rejected, because its run of the whole set, about 140 s, stayed the floor.
  - Three or more slices: rejected, because a slice cannot finish before its slowest test, about
    75 s, and each slice costs another runner.
  - A slice stated per job, in two job blocks: rejected, because the count would be written in two
    places; a matrix derives it (`strategy.job-total`), and A20 holds it to 1 to N.

### Consequences of the note

- Good, because a warm code pull request's gate took 2m39s and 2m48s, against 4m29s before the
  engine job and 3m43s with one, and every test still decides `ci`.
- Bad, because each run pays two more runners, each with its own setup, restore and test build: the
  Rust jobs took 6m23s and 7m00s of runner time together, against 4m22s for the single `rust` job,
  for a gate 1m41s to 1m50s shorter.
- Bad, because the local gate runs one more cargo command, which recompiles the engine once more
  (ADR-022's finding).
