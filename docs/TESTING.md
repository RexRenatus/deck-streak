# Testing

## Test pyramid

- Unit and property tests in each crate, many and fast.
- Parity tests that compare the port with goldens v9's own functions produced (`tools/parity-oracle/`).
- Integration tests per crate under `crates/<context>/tests/`.
- Mini App unit tests (Vitest) and end-to-end tests with an axe audit (Playwright, headless Chromium).

## Running the tests

```sh
cargo nextest run --workspace
cargo test --doc --workspace
pnpm -r test
pnpm -r test:e2e
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

## Writing a test

Write it first, run it, and record why it failed in `docs/red-first/SPEC-NNN.md` before the code
that makes it pass. Assert a positive value, never only an absence. A test that enumerates reports
`examined N` and refuses zero. Inject the clock; never sleep.

## Temporary files

A test removes every temporary file and directory it makes, whether it passes or fails, so a green
run leaves nothing behind. Use a form that removes what it makes on drop or exit:

- Rust: `tempfile::TempDir` and `tempfile::NamedTempFile`, dropped at the end of their scope.
- Python: `tempfile.TemporaryDirectory`, or a `mkdtemp` whose removal the same function registers
  with `addCleanup` or a `finally`.
- TypeScript: a `mkdtemp` or `mkdtempSync` result that the same file removes with `rm` or `rmSync`,
  or `mkdtempDisposableSync`.

`TempDir::keep`, `TempDir::into_path`, `NamedTempFile::keep`, `std::env::temp_dir()`, a Python
`mkdtemp`, `mkstemp` or `NamedTemporaryFile(delete=False)` with no removal, and any string literal
that begins with `/tmp/` are refused. `scripts/tests/test_temp_hygiene.py` reads every test file,
names each one it refuses by file and line, and proves itself on the planted leaks under
`scripts/tests/fixtures/temp-hygiene/` (SPEC-030).

## Honest pack states

Every pack is judged on the maintainer's box by `scripts/box-packs.sh` (ADR-069), from a private
checkout of the packs at a pinned commit, with the wiring a private file gives it; the sdd, ddd and
tdd probes run the same way. A pull request shows the run's verdict as its `box/packs` status.

The wiring's packs section says which packs are enforced and which wait for a named issue, and the
run refuses a state the tree has outgrown. A `pending` pack whose every blocking row passes must say
`enforced`. A deferred row that passes must lose its deferral, and one that is still red, VOID or in
error stays deferred and fails nothing.

The wiring's box section names every red row expected on the tree with the open issue that builds
its subject. A red row it does not name fails the run, and a named row that is no longer red is
refused as stale. A pack that examines nothing reads `pending` with its issue, and is never reported
green. Before any pack runs, the driver reads each named issue's state with `gh`: an expectation
whose issue is closed is stale, and a run that cannot read an issue's state is VOID, never green
(SPEC-054). The data DeckStreak keeps from a pack (the vault's rails, layout and gate classes, and
the scrub's shapes) is compared with the pack's own at the pin, so it cannot drift. The driver's
own tests drive it with a fake runner, a synthetic checkout and a fake `gh`
(`scripts/tests/test_box_packs.py`), so they run in CI.

## Continuous integration

CI runs `bash scripts/check.sh` on every pull request into `dev` and `main` and every push to them,
on GitHub-hosted runners, in five jobs that start together (ADR-055):

| job | stages |
|---|---|
| `rust` | `fmt clippy test doctest audit-rust` |
| `engine` | `test-engine`, in two slices of the engine set, one per runner |
| `web` | `web audit-web` |
| `packs` | `packs` |
| `hygiene` | `python scrub secrets` |

Every stage runs in exactly one job, and the aggregate `ci` check, which the rulesets require, needs
all five with `workflow-lint` and `base-is-dev`.
The required `ci` is always the pull request's own run: a push to `dev` or `main` reports it as
`ci (push)`. Each stage checks its own tools first, so a missing tool fails that stage by name
wherever it runs.

`audit-web` audits every package `pnpm-lock.yaml` resolves, the development dependencies too: the
Mini App ships as a static build, so its packages are all development dependencies. It runs
`pnpm audit --json --audit-level low`, and `scripts/audit-web-verdict.py` reads the report. The
verdict prints how many packages pnpm examined, reads VOID when that is none, and fails on any
advisory at `low` or above, the bar `audit-rust` holds (SPEC-058).

`check.sh` defines the engine set once, `ENGINE_TESTS`: a nextest filterset naming SPEC-022's two
test binaries that drive Anki's engine end to end (`sync` and `engine_budget`), whose tests take tens
of seconds each. The `test` stage runs every test outside it and `test-engine` runs it, with one
command that differs only in that filterset, so each test runs in exactly one of the two, and the
engine's slow tests no longer lengthen the `rust` job (SPEC-038 section 8). In CI the `engine` job
runs the set in two slices, one per runner: each leg sets `ENGINE_SLICE` to its slice (`1/2`,
`2/2`), which `test-engine` hands nextest as `--partition slice:1/2`. The local gate with no
arguments runs both stages and the whole set. To run only the fast tests locally:

```sh
bash scripts/check.sh test          # every test outside the engine set
bash scripts/check.sh test-engine   # the engine set
```

`rust` restores a cache of `~/.cargo`'s downloads and `target/`, keyed on the toolchain pin and
`Cargo.lock`, and `engine` and `hygiene` restore it too (a guard test builds a Rust example); only
`rust` saves it. `web` restores the pnpm store and Playwright's browser, keyed on `pnpm-lock.yaml` and
the Playwright version it locks. Only a push to `dev` or `main` saves a cache, and only when it
missed its key; a pull request, a fork's included, restores and never saves. A newer push to a pull
request cancels the run it supersedes; a push to `dev` or `main` is never cancelled.

`check.sh` writes each stage's log, and `timings.tsv` (each stage's seconds, verdict and exit), to
`$CHECK_LOG_DIR`, and each CI job uploads them as its own artifact, whatever the verdict.

## Coverage and mutation

A test proves something only if it fails on a wrong program. Mutation rows (the mutation-rows
pack's practice) are added for game math and gates as they land.
