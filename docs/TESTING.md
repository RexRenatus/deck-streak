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

## Continuous integration

CI runs `bash scripts/check.sh` on every pull request into `dev` and `main`, on GitHub-hosted
runners, and requires the aggregate `ci` check.

## Coverage and mutation

A test proves something only if it fails on a wrong program. Mutation rows (the mutation-rows
pack's practice) are added for game math and gates as they land.
