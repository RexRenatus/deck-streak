# SPEC-649: a stopped test server child exits on its own before any kill

- **Context(s):** the test support of three crates, `deck-streak-ingest`
  (`crates/ingest/tests/support/mod.rs`, `crates/ingest/tests/sync.rs`), `deck-streak-engine-core`
  (`crates/engine-core/tests/support/sync_server.rs`, `crates/engine-core/tests/handshake.rs`) and
  `deck-streak-ffi` (`crates/ffi/tests/support/sync_server.rs`, `crates/ffi/tests/sync.rs`), and
  `docs` (SPEC-592 and ADR-613, each with one appended amendment).
- **Decided by:** ADR-613 (its amendment on the test helpers' stop)
- **Status:** delivered with its tests and `docs/red-first/SPEC-649.md`.

## 1. The problem, measured

The `coverage-rust` job of the coverage workflow (SPEC-592) read red on three runs whose tests all
passed:

- job 127639495405 (run 42140428459): `2252 tests run: 2252 passed`, then
  `deck-streak-32599-11034653725874368068_3.profraw: invalid instrumentation profile data (file header is corrupt)`,
  `error: no profile can be merged` and `Process completed with exit code 123`;
- job 128523171601 (run 42412384545): 2264 of 2264 tests passed, then the same refusal for
  `deck-streak-32508-5745021358488486715_0.profraw`;
- job 128513459805 (run 42409197081): 2267 of 2267 tests passed, then the same refusal for
  `deck-streak-33371-1835957162114594072_3.profraw`.

None of the three logs holds a timeout, signal, leak or failed-test line. In each, one raw profile
was cut short in its write, and the merge, in its default failure mode, refused every profile, so
the report step exited non-zero while every test passed (ADR-613 D2 keeps that refusal).

Which process wrote the cut profile is not measured: no log line names the process, and the job's
artifact holds only the run's argument list. The leading candidate is shown by source lines at the
base of this change. The three sync server test helpers stop their server child in `Drop` by
closing its standard input and then killing it at once, with no wait:

- `crates/ingest/tests/support/mod.rs` lines 286 to 293, the `Drop` of `SyncServer`, and lines 308
  and 309, the child's reader of its standard input, which reads to the end and then calls
  `std::process::exit(0)`;
- `crates/engine-core/tests/support/sync_server.rs` lines 157 to 164, and lines 183 and 184;
- `crates/ffi/tests/support/sync_server.rs` lines 175 to 183, and lines 277 and 278.

The child is the test binary itself, run again (`env::current_exe()`), so under the coverage run it
is instrumented and writes its own raw profile in its exit handlers. Closing its standard input
starts that exit; the kill that follows at once either ends the child before its write, and its
profile is lost, or lands inside the write, and its profile is cut short, the refusal above.

## 2. Requirements

R1. Each of the three support files defines `pub fn stop(child, stdin)`: it closes `stdin`, waits
up to `STOP_GRACE` for the child to exit on its own, looking again every `STOP_POLL`, kills the
child only once that bound has passed, and answers how the child ended.

R2. The `Drop` of each `SyncServer` stops its child through `stop`, and no other way; the ffi helper
still stops its statement front first.

R3. A child that exits on its own inside the grace is never killed: `stop` answers its own exit
status, and the work its exit does runs to its end.

R4. Nothing else changes for the coverage lane: no workflow, job, step, merge failure mode,
`nextest` profile or coverage tool flag moves, so a raw profile cut short for any other reason
still fails the lane in view (ADR-613 D2).

R5. No existing test, plant or assertion changes; the change removes only the three `Drop` bodies
it replaces with a call to `stop`.

## 3. Acceptance criteria of SPEC-649

| id | criterion | decided by |
|---|---|---|
| A1 | the ingest helper's `stop` lets a planted child that exits on its own finish its exit's work, and answers a successful exit, never a kill (R1, R2, R3) | `crates/ingest/tests/sync.rs` `a_stopped_server_child_exits_on_its_own_before_any_kill` |
| A2 | the engine-core helper's `stop` does the same (R1, R2, R3) | `crates/engine-core/tests/handshake.rs` `a_stopped_server_child_exits_on_its_own_before_any_kill` |
| A3 | the ffi helper's `stop` does the same (R1, R2, R3) | `crates/ffi/tests/sync.rs` `a_stopped_server_child_exits_on_its_own_before_any_kill` |

```acceptance
A1: cargo nextest run -p deck-streak-ingest --test sync -E 'test(=a_stopped_server_child_exits_on_its_own_before_any_kill)'
A2: cargo nextest run -p deck-streak-engine-core --test handshake -E 'test(=a_stopped_server_child_exits_on_its_own_before_any_kill)'
A3: cargo nextest run -p deck-streak-ffi --test sync -E 'test(=a_stopped_server_child_exits_on_its_own_before_any_kill)'
```

Each test runs its own binary again as a planted child, named by `DECKSTREAK_TEST_STOP_PLANT`. The
child reads its standard input to the end, takes `STOP_PLANT_EXIT` to exit, as an exit handler's
write does, writes a marker file and calls `std::process::exit(0)`, the server's own exit path. The
parent calls the support module's `stop` and asserts two positive results: the child's exit status
is a success, and the marker holds `exited on its own`. Each test is red at its red commit, where
`stop` keeps the base behaviour moved out of `Drop` (close standard input, kill at once, wait), by
the first assertion: the child ends by the kill's signal. R4 and R5 are held by the file manifest
below and by the pull request's diff, which changes no other path.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-649-a-stopped-test-server-child-exits-on-its-own-before-any-kill.md` | docs | added |
| `docs/specs/SPEC-592-an-advisory-coverage-report-runs-on-every-pull-request-and-dev-push-and-never-gates-a-merge.md` | docs | changed: one amendment appended, naming SPEC-649 |
| `docs/decisions/ADR-613-coverage-runs-in-its-own-workflow-as-an-advisory-report-and-never-gates-a-merge.md` | docs | changed: one amendment appended, deciding the test helpers' stop |
| `docs/red-first/SPEC-649.md` | docs | added |
| `changelog.d/fix-coverage-profraw-649.md` | docs | added |
| `crates/ingest/tests/support/mod.rs` | tests | changed: `stop`, `STOP_GRACE` and `STOP_POLL` added, and the `Drop` of `SyncServer` calls `stop` |
| `crates/engine-core/tests/support/sync_server.rs` | tests | changed: the same |
| `crates/ffi/tests/support/sync_server.rs` | tests | changed: the same, the statement front still stopped first |
| `crates/ingest/tests/sync.rs` | tests | changed: A1's test appended |
| `crates/engine-core/tests/handshake.rs` | tests | changed: A2's test appended |
| `crates/ffi/tests/sync.rs` | tests | changed: A3's test appended |

## 5. What this does NOT cover

- A planted corrupt raw profile as a standing control of the lane: not built here. The merge
  already refuses such a profile before this change and must still refuse it after, so it cannot
  be red first for this cure; the lane's own refusal stays the one SPEC-592 states.
- The writer of the cut profile in the three jobs of section 1: not measured. This change cures the
  leading candidate its source lines show; a profile cut short by another process still fails the
  lane, in view, and is read by its job id.
- Every other child a test starts in `crates/`: each already waits for its child to exit, or sends
  a stop signal and waits a bound before any kill, and is unchanged here.

## 6. Risks

- **A server that never exits on its own now costs the grace before its kill.** The kill still
  lands once `STOP_GRACE` has passed, so a hanging server ends as before, later by that bound.
- **The cut profile had another writer.** The lane then reads red again with every test passed;
  the refusal is kept, and the job id and its log lines name the next case.
- **The planted child runs the test binary again.** It runs only its own test, by exact name and on
  one thread, and the variable that makes it the plant is set only for that child, so no other
  test reads it.

## 7. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani. The change is test support and its tests in three test
targets, documents and a record; no protocol, state machine, interleaving or engine path gains or
changes, and no crate source outside `tests/` changes.
