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
run leaves nothing on the small shared host. Use a form that removes what it makes on drop or exit:

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

`.packs/wiring.json` says which packs are enforced and which wait for a named issue, and the gate
refuses a state the tree has outgrown. A `pending` pack whose every blocking row passes must say
`enforced`. Every deferred row runs in a pass of its own: one that passes must lose its deferral,
and one that is still red, VOID or in error stays deferred and fails nothing
(`scripts/pack-rows.py`). The runner runs rows in a bounded pool, `--jobs N`, by default the
smaller of 8 and the CPUs available, and reports them in row order with the verdicts one row at a
time would give.

The packs built into phxd run on the maintainer's box with `scripts/box-packs.sh` (ADR-030). The
wiring's `box` section names every red row expected on the tree with the open issue that builds its
subject. A red row it does not name fails the run, and a named row that is no longer red is refused
as stale. A pack that examines nothing reads `pending` with its issue, and is never reported green.
The runner's own tests drive it with a fake phxd (`scripts/tests/test_box_packs.py`), so they run in
CI.

## Continuous integration

CI runs `bash scripts/check.sh` on every pull request into `dev` and `main` and every push to them,
on GitHub-hosted runners, in four jobs that start together (ADR-055):

| job | stages |
|---|---|
| `rust` | `fmt clippy test doctest audit-rust` |
| `web` | `web audit-web` |
| `packs` | `packs` |
| `hygiene` | `python scrub secrets` |

Every stage runs in exactly one job, and the aggregate `ci` check, which the rulesets require, needs
all four with `workflow-lint` and `base-is-dev`. Each stage checks its own tools first, so a missing
tool fails that stage by name wherever it runs.

`rust` restores a cache of `~/.cargo`'s downloads and `target/`, keyed on the toolchain pin and
`Cargo.lock`; `web` restores the pnpm store and Playwright's browser, keyed on `pnpm-lock.yaml` and
the Playwright version it locks. Only a push to `dev` or `main` saves a cache, and only when it
missed its key; a pull request, a fork's included, restores and never saves. A newer push to a pull
request cancels the run it supersedes; a push to `dev` or `main` is never cancelled.

`check.sh` writes each stage's log, and `timings.tsv` (each stage's seconds, verdict and exit), to
`$CHECK_LOG_DIR`, and each CI job uploads them as its own artifact, whatever the verdict.

## Coverage and mutation

A test proves something only if it fails on a wrong program. Mutation rows (the mutation-rows
pack's practice) are added for game math and gates as they land.
