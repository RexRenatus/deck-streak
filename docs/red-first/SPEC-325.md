# Red-first record: SPEC-325

The SPEC and ADR-326 were committed first (3685698a). The two tests and the census helper were
then committed alone at 21d366c5, with every multi-thread attribute in `crates/daemon/src/wiring.rs`
still unbounded, A1's own included, and the module ran before the cure. A1 and A2 fail by
assertion, and they are the only two of the module's 14 tests that fail on an unloaded run:
`test result: FAILED. 12 passed; 2 failed`.

A3 is the module command itself, its fence spelling the filter `wiring::tests::`, which selects
the same 14 tests as #577's `wiring::tests`. Its red is the issue's own failure, reproduced
deterministically in a shell whose `ulimit -u` is set 100 tasks above the count of tasks the user
already runs (SPEC-325 section 1). The budget is evidence, never a gate (ADR-326).

```red-first
A1: red at 21d366c5: assertion `left == right` failed: the runtime runs the attribute's two workers, not one per core of the host; left: the host's core count, right: 2
A2: red at 21d366c5: assertion `left == right` failed: each multi-thread test names worker_threads = 2; left: six `#[tokio::test(flavor = "multi_thread")]` attributes, right: []
A3: red at 21d366c5: under the section 1 budget, 7 of 14 failed: A1, A2 and the five ledger-opening multi-thread tests, each panicking at `the database opens: Database(Io(Os { code: 11, kind: WouldBlock, message: "Resource temporarily unavailable" }))`
A1: green at 6234909b
A2: green at 6234909b
A3: green at 6234909b
```

## What changed between red and green

6234909b changes one line shape, six times: `#[tokio::test(flavor = "multi_thread")]` becomes
`#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` on the five ledger-opening tests and
on A1. A1's own attribute is a test line edited between its red and its green, and that is
deliberate: at 21d366c5 it carried the unbounded attribute so that A1 observed the runtime the
base gives every multi-thread test, and failed by assertion on it. Its body, its assertion and A2
are unchanged between the two commits. Row S32501 holds A1 to the runtime rather than the text: a
runtime of three workers fails it on any host.

## The budgeted runs

At the base, 78f46a2c, without the tests (12 tests in the module), under the budget:

```text
test result: FAILED. 7 passed; 5 failed; 0 ignored; 0 measured; 1 filtered out
    wiring::tests::every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name
    wiring::tests::the_owners_cycle_that_cannot_read_its_run_record_is_refused_by_the_sync_code
    wiring::tests::the_owners_sync_marks_the_rescore_before_it_reads_its_settings
    wiring::tests::the_owners_sync_that_cannot_mark_the_rescore_is_refused_by_its_own_code
    wiring::tests::the_owners_sync_without_a_credentials_directory_is_refused_by_its_own_code
the same budget with --test-threads=1: test result: ok. 12 passed; 0 failed
```

At the red commit, 21d366c5, three runs under the budget:

```text
run 1: test result: FAILED. 11 passed; 3 failed (A1, A2, and one WouldBlock panic)
run 2: test result: FAILED. 7 passed; 7 failed (A1, A2, and five WouldBlock panics)
run 3: test result: FAILED. 7 passed; 7 failed (A1, A2, and five WouldBlock panics)
```

At the green commit, 6234909b, ten runs under the same budget, each exit 0 with no WouldBlock line:

```text
runs 1 to 10: test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
```

## The acceptance runs

At 6234909b, `cargo test -j 1 -p deck-streak-daemon --lib wiring::tests`, #577's own command, on
default parallel threads, ten consecutive runs, then the daemon library whole once:

```text
acceptance run 1 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 2 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 3 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 4 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 5 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 6 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 7 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 8 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 9 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
acceptance run 10 rc=0 test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
whole lib rc=0 test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Mutation rows, proved at 2448aa43 (`rows: examined 3: killed 3, survived 0, void 0`), each
killer selecting one test, the file restored byte for byte:

- S32500: KILLED
- S32501: KILLED
- S32502: KILLED
