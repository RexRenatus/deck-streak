# SPEC-325: each multi-thread wiring test runs on two workers, whatever the host

- **Issue:** #577. **Context(s):** `deck-streak-daemon` (its library's `wiring::tests` module).
- **Decided by:** ADR-326 (a multi-thread wiring test bounds its runtime to two workers).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-325.md`. **Mutation band:** `S32500-S32599`.

## 1. The problem, measured

Measured at dev `78f46a2c` on the maintainer's machine, with the library's test binary built by
`cargo test -p deck-streak-daemon --lib --no-run`.

- #577 reports that some `wiring::tests` fail on default parallel threads with `the database
  opens: Database(Io(Os { code: 11, kind: WouldBlock ... }))` and pass with `--test-threads=1`, and
  names a shared database path or lock file as the likely cause. **The tests share neither.** Each
  test that opens the ledger makes its own temporary directory and opens `deck_streak.db` inside it
  (`crates/daemon/src/wiring.rs:768-769`, `:804-805`, `:837-838`, `:864-867`, `:985-988`), and the
  one current-thread test makes its own directory for its credential (`:1295`). No test calls
  `open_database` or takes its open lock:
  `grep -n open_database crates/daemon/src/wiring.rs` finds the doc comments and the definition at
  `:145`, and no call.
- **The error is a refused thread.** `Db::open` (`crates/kernel/src/db.rs:60-73`) wraps sqlx's
  error as `KernelError::Database`, and the one place an OS error reaches sqlx's `Error::Io` on that
  path is the SQLite driver's `ConnectionWorker::establish`, which starts one thread per connection
  with `thread::Builder::new()...spawn(..)?`. OS error 11 is `EAGAIN`: the OS refused to create a
  thread because the process's task budget was spent.
- **Why the module spends it.** Five tests in the module are `#[tokio::test(flavor =
  "multi_thread")]` (`wiring.rs:766`, `:802`, `:835`, `:862`, `:1195`). With no `worker_threads`,
  each builds a runtime of C workers, C being the host's core count, and libtest runs them at once.
  Sampling `/proc/<pid>/status` during one run of `wiring::tests` gives a peak of five runtimes of C
  workers each plus fewer than twenty other threads, so the module's demand grows with the host.
- **A deterministic reproduction.** In a shell whose `ulimit -u` is set 100 tasks above the count of
  tasks the user already runs, the module fails 5 of its 12 tests, exactly the five multi-thread
  tests, each panicking at its `expect("the database opens")` with `Database(Io(Os { code: 11, kind:
  WouldBlock, message: "Resource temporarily unavailable" }))`. The same shell with
  `--test-threads=1` passes 12 of 12. This is #577's signature, five failures at most, and its
  `--test-threads=1` escape.
- CI never meets it: `scripts/check.sh`'s `test` stage runs `cargo nextest run`, one process per
  test. A local `cargo test` runs the module in one process, which is where #577 was seen.

## 2. Requirements

R1. Every multi-thread `tokio::test` attribute in `crates/daemon/src/wiring.rs` reads exactly
    `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`, so each such test draws two
    runtime workers whatever the host's core count.
R2. A test of the module observes, through the runtime's own metrics, that a runtime built from that
    attribute runs two workers.
R3. A census over the file refuses every multi-thread `tokio::test` attribute that is not that
    spelling, naming the line; it refuses a planted unbounded attribute (its positive control), and
    prints how many multi-thread attributes it examined and refuses zero.
R4. No test is serialised, skipped, ignored or filtered, no test mutex is added, and no
    `--test-threads` setting enters any gate or command. The production open path (`Db::open`,
    `open_database`) is unchanged, because the cause is not in it.
R5. `cargo test -p deck-streak-daemon --lib wiring::tests` passes on default parallel threads in 10
    consecutive runs, and the reproduction's budgeted shell, which fails the base, passes the head.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a runtime built from the module's multi-thread attribute runs two workers, not the host's core count | `cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::a_multi_thread_wiring_test_runs_on_two_workers_whatever_the_host` |
| A2 | every multi-thread `tokio::test` attribute in `wiring.rs` is the bounded spelling, the census examines at least one, and a planted unbounded attribute is refused by its line | `cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::every_multi_thread_wiring_test_bounds_its_runtime_to_two_workers` |
| A3 | the module passes on default parallel threads, 10 consecutive runs; under the section 1 budget the base fails five multi-thread tests with the issue's message and the head passes | `cargo test -p deck-streak-daemon --lib wiring::tests` |

```acceptance
A1: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::a_multi_thread_wiring_test_runs_on_two_workers_whatever_the_host
A2: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::every_multi_thread_wiring_test_bounds_its_runtime_to_two_workers
A3: cargo test -p deck-streak-daemon --lib wiring::tests
```

A3's ten runs and its budgeted runs are measured and quoted in `docs/red-first/SPEC-325.md`. The
budget is evidence, never a gate: it counts every task the user runs, so a committed test under it
would depend on whatever else runs at that moment (ADR-326).

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | the five multi-thread attributes bounded; A1, A2 and the census helper added to `wiring::tests` |
| `docs/specs/SPEC-325-each-multi-thread-wiring-test-runs-on-two-workers-whatever-the-host.md` | docs | added |
| `docs/decisions/ADR-326-a-multi-thread-wiring-test-bounds-its-runtime-to-two-workers.md` | docs | added |
| `docs/red-first/SPEC-325.md` | docs | added |
| `scripts/mutation-rows.d/S32500-S32599.json` | scripts | added |
| `changelog.d/fix-wiring-tests-577.md` | changelog | added |

No schematic: the delivery adds no component, data flow or state machine; it changes the size of
five test runtimes.

## 5. What this does NOT do

- It does not bound the multi-thread tests of other test targets (in `crates/daemon/tests/`,
  `crates/coordination/tests/` and others). Each target is its own binary, and #577 names only the
  library's `wiring::tests`.
- It does not change how a production role sizes its runtime or opens its database. The refused
  thread was the tests' own demand, and a production open that the OS refuses a thread still fails
  loudly (#577).
- It does not retry a refused thread in `Db::open` or anywhere else (#577).

## 6. Risks

- A new multi-thread test added to the module without the bound brings the growth back: A2 refuses
  it by its line, and row S32500 proves A2 would.
- The attribute might stop meaning two workers (a macro change, or a host setting overriding it):
  A1 reads the worker count from the runtime itself, and row S32501 proves A1 fails on a runtime
  of any other size, whatever the host.
- A census can go blind: its positive control is planted beside it, and row S32502 proves a blind
  census fails that control.
- On a host of exactly two cores, A1's red at the base could not show, since the unbounded runtime
  also has two workers. The red is recorded where the host has more.

## 7. The mutation rows

`scripts/mutation-rows.d/S32500-S32599.json`, `MUTATIONS`, crate `daemon`, file `src/wiring.rs`.
cargo-mutants mutates no attribute and nothing under `#[cfg(test)]`, so the bound takes hand rows:

| row | mutant | killer |
|---|---|---|
| S32500 | `every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name` loses its `worker_threads = 2` | A2 |
| S32501 | A1's own attribute names `worker_threads = 3`, a size other than two on any host | A1 |
| S32502 | the census keeps no attribute, so it can refuse nothing | A2 (its planted control) |
