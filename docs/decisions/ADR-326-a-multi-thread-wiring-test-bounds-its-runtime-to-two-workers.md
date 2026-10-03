---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak builder for #577, under the DeckStreak architect seat's brief"
---

# A multi-thread wiring test bounds its runtime to two workers

## Context and Problem Statement

Five tests in `crates/daemon/src/wiring.rs`'s `wiring::tests` are `#[tokio::test(flavor =
"multi_thread")]` with no `worker_threads`, so each builds a runtime of as many workers as the host
has cores, and libtest runs them at once. Each then opens its own ledger, and the SQLite driver
starts a thread per connection. When the process's task budget is spent, the OS refuses that thread
(`EAGAIN`, error 11) and the test fails at `expect("the database opens")` with `Database(Io(Os {
code: 11, kind: WouldBlock }))` (#577). The tests share no database path and no lock file (SPEC-325
section 1). How should the module stop failing on default parallel threads without weakening what
its tests run under?

## Decision Drivers

- #577's acceptance runs the module on default parallel threads, so serialising it weakens it.
- The demand that spends the budget is the tests' own: five runtimes sized to the host at once.
- The production open path refuses loudly when the OS refuses a thread; that is right for a service
  and is not the cause here.
- No new dependency and no `Cargo.toml` edge.

## Considered Options (the alternatives it was chosen against)

- Bound each multi-thread test's runtime to two workers with `worker_threads = 2` (chosen).
- Give each test its own temporary directory or database path: lost, because each test already has one (SPEC-325 section 1), so it would cure nothing.
- Serialise the module (`--test-threads=1` in a gate, a test mutex, a serial attribute): lost, because it weakens #577's acceptance, which runs on parallel threads, hides the demand instead of bounding it, and a serial attribute is a new dependency.
- Switch the five tests to the current-thread flavor: lost, because it changes what they exercise, one thread for the test body and every task it spawns, where the bound keeps the flavor and fixes only its size.
- One worker (`worker_threads = 1`): lost, because a single worker leaves nothing to steal work from another; two is the smallest runtime that still schedules across workers.
- Retry the open in `Db::open` when the OS refuses a thread: lost, because the cause is not in the production path, and a retry would hide a real resource refusal from a running service.
- Commit a test that re-runs the module in a child process under a lowered `ulimit -u`: lost, because that limit counts every task the user runs, so the verdict would depend on whatever else runs at that moment; a flaky test would replace a flaky test. The budgeted run stays a measurement, quoted in the red-first record.

## Decision Outcome

Chosen option: "bound each multi-thread test's runtime to two workers", because it removes the
growth with the host while keeping every test, its flavor and its parallel run.

- Each multi-thread attribute in `wiring.rs` reads `#[tokio::test(flavor = "multi_thread",
  worker_threads = 2)]`. Five at once draw ten runtime workers, whatever the host.
- A test reads its own runtime's worker count from the runtime's metrics and asserts two (A1).
- A census over the file refuses any multi-thread attribute that is not that spelling, by its line,
  beside a planted unbounded attribute it must refuse (A2).

### Consequences

- Good, because the module's peak thread count no longer grows with the host's cores, and the
  module passes on default parallel threads under the budget that failed the base.
- Good, because no test is serialised, skipped or filtered, and the production path is unchanged.
- Bad, because a multi-thread test elsewhere in the workspace still sizes its runtime to the host;
  each test target is its own binary, and SPEC-325 section 5 leaves them as they are.

### Confirmation

A1 and A2 in `wiring::tests`, red at the base for their own reasons and green at the head; rows
S32500 to S32502 prove the census, the behaviour test and the census's control each fail on their
mutant; the red-first record quotes ten consecutive default-thread runs and the budgeted runs at
the red and green commits.

## What would make this wrong

- A wiring test that needs more than two workers to show what it asserts, for example one that
  asserts an interleaving only many workers produce. None of the five asserts an interleaving: each
  asserts a refusal code, a log line or a ledger read after one cycle.
- A refused thread in these tests while the module's demand stays bounded: the cause would then be
  elsewhere, and #577 would reopen with that measurement.

## More Information

#577; SPEC-325; `docs/red-first/SPEC-325.md`.
