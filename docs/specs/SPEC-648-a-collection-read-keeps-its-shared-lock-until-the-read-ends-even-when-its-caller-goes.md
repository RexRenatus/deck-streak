# SPEC-648: a collection read keeps its shared lock until the read ends, even when its caller goes

- **Issue:** #957 (this delivery closes it). **Context(s):** `deck-streak-ingest` (`with_copy`);
  `formal` (the cover of `tla/OneInstrumentReader`).
- **Decided by:** ADR-671 (a collection read hands its shared lock and its read to a task that
  holds the lock to the read's end), under ADR-487 (an instrument run holds the instrument lock
  until its read ends).
- **Prerequisites:** SPEC-470 (the holder, R5), SPEC-022 (the collection lock, R7) and SPEC-023
  (the read, R1). **Mutation band:** `S64800-S64899`.
- **Status:** accepted; delivered with its tests, its row, the model's re-stamp and
  `docs/red-first/SPEC-648.md` by the pull request that closes #957.

## 1. The problem, measured

Read at dev `bd953ba7` by `git show` and `sed -n`; ADR-671 D1 cites each fact by `path:line`.

- Every read of the crate goes through `with_copy` (`crates/ingest/src/reader.rs:362-396`). It
  takes the collection lock shared (`:376`), awaits the offload's run of the whole read, which is
  the open, the work and the close (`:379-391`), and then releases the lock explicitly (`:394`).
- The held lock lives in the caller's future. The offload runs the read by
  `tokio::task::spawn_blocking` (`crates/kernel/src/offload.rs:64`), and a blocking task cannot be
  aborted. So when the caller's future is dropped, the held lock is dropped with it and unlocks
  (`crates/ingest/src/lock.rs:169-175`) while the read still runs to its end.
- A caller is dropped in the two ways ADR-487 D1 measured: the api's request limit drops the
  handler's future, and a stopping api or bot runtime drops every async task.
- An exclusive holder can then run beside the read. The sync and the skip write take the
  collection lock exclusively (`crates/ingest/src/sync.rs:205`, `crates/ingest/src/skip_write.rs:241`),
  and the shared lock is what keeps a read from seeing the copy mid-swap
  (`crates/ingest/src/lock.rs:4`, SPEC-022 R7).
- `Held::hold_to_end` (`crates/ingest/src/lock.rs:126-143`, SPEC-470 R5) already runs work in a
  task of its own that owns the lock, and ADR-487 D6 leaves the collection lock in `with_copy` to
  #957.
- The model `tla/OneInstrumentReader` covers `with_copy`
  (`formal/tla/OneInstrumentReader/OneInstrumentReader.tla:53`). The cover's digest is the sha256
  of the item's span, every line from the definition line to the first following line that is
  exactly four spaces and a closing brace, each with its newline: lines 366 to 396 at `bd953ba7`,
  whose sha256 equals the digest the cover records. The formal obligations ledger holds the entry
  as `re-stamp`, for the collection read it covers.

## 2. Requirements

R1. A collection read takes the shared collection lock before its read starts, as today, and holds
    it until the read has ended on the offload's blocking thread and the copy is closed, whether or
    not its caller still waits: `with_copy` hands the held lock and the whole read (the offload's
    run, the open, the work and the close) to `Held::hold_to_end`.
R2. A read whose runtime drops its task while the read runs keeps the lock until the process exits
    (the holder's own rule, SPEC-470 R3 and R5).
R3. A read that ends returns its value or its error exactly as before, and the lock is released
    explicitly after it; a failed unlock is released by the close that follows it (SPEC-022 R7).
R4. Every read of the crate still goes through `with_copy`; no caller, no signature, no other file
    of the crate and no manifest changes.
R5. The cover of `with_copy` in `tla/OneInstrumentReader` is re-stamped after the last edit: the
    change moves none of the model's variables (the collection lock is not one), so it is a
    stuttering step there, and the model names it so beside the read's actions.

## 3. Acceptance criteria of SPEC-648

| id | criterion | decided by |
|---|---|---|
| A1 | a read whose caller is dropped keeps the collection lock while the read runs, and the lock is free once the read has ended | `a_read_whose_caller_is_dropped_keeps_the_collection_lock_until_it_ends` |
| A2 | a read its runtime drops keeps the collection lock while the read runs and after it has ended, until the process ends | `a_read_its_runtime_drops_keeps_the_collection_lock_until_the_process_ends` |
| A3 | a read that ends returns its work's value and leaves the collection lock free | `a_read_that_ends_releases_the_collection_lock_and_returns_its_value` |

```acceptance
A1: cargo test -p deck-streak-ingest --lib -- --exact reader::tests::a_read_whose_caller_is_dropped_keeps_the_collection_lock_until_it_ends
A2: cargo test -p deck-streak-ingest --lib -- --exact reader::tests::a_read_its_runtime_drops_keeps_the_collection_lock_until_the_process_ends
A3: cargo test -p deck-streak-ingest --lib -- --exact reader::tests::a_read_that_ends_releases_the_collection_lock_and_returns_its_value
```

A1 and A2 are red first: each compiles at the base and fails there by its assertion, since the
lock goes with the caller. A3 is not red: a read that ends released the lock and returned its value
before the cure too, and it guards that the cure keeps both. The three tests sit in a `tests`
module at the end of `crates/ingest/src/reader.rs`, which builds the reader from its own fields
over a zero-length copy in a temporary directory. A1 and A2 each drive a gate the test opens
itself, with a bounded wait and a guard that opens the gate when it drops, so no red can hang; no
assertion reads a time.

The span check: after the last edit to `with_copy`, the sha256 of its span, read by the rule of
section 1, equals the digest the cover line of `tla/OneInstrumentReader` records, so the
repository's formal check reports no stale cover for the entry.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/ingest/src/reader.rs` | `deck-streak-ingest` | changed: `with_copy` hands its held lock and its read to `Held::hold_to_end`, with one doc sentence on it; a `tests` module at the end holds A1 to A3 |
| `formal/tla/OneInstrumentReader/OneInstrumentReader.tla` | `formal` | changed: the `with_copy` cover re-stamped, and two comment lines naming the stuttering step |
| `scripts/mutation-rows.d/S64800-S64899.json` | `mutation rows` | added: S64801 |
| `docs/specs/SPEC-648-a-collection-read-keeps-its-shared-lock-until-the-read-ends-even-when-its-caller-goes.md` | `docs` | added: this SPEC |
| `docs/decisions/ADR-671-a-collection-read-hands-its-shared-lock-and-its-read-to-a-task-that-holds-the-lock-to-the-reads-end.md` | `docs` | added: its decision |
| `docs/schematics/collection-read-lock.md` | `docs` | added: the read's lock across the caller, the holder's task and the blocking read |
| `docs/red-first/SPEC-648.md` | `docs` | added: the red-first record |
| `changelog.d/collection-read-lock-648.md` | `docs` | added: the changelog fragment |

No Cargo manifest or lockfile, no other source file, no formal configuration, witness or ledger
row, and no workflow changes. `crates/ingest/src/lock.rs` is used as it is.

## 5. What this does NOT cover

- The instrument lock: SPEC-470 already holds it to its read's end (#476).
- The collection lock's exclusive takes by the sync and the skip write, which still wait for every
  shared holder, a read a stopped runtime left holding included (#957).
- A model of the collection lock: it is no variable of `tla/OneInstrumentReader`, whose property is
  who reads the instruments when (#476).
- The api's request limit and the role runtimes' stop, which still drop their callers (#476).

## 6. Mutation rows, and the risks

| row | target | what it guards | killer |
|---|---|---|---|
| `S64801-A-DROPPED-READ-KEEPS-THE-COLLECTION-LOCK` | `crates/ingest/src/reader.rs` | a read whose caller is dropped keeps the shared collection lock until the read ends (R1, A1) | `lib::reader::tests::a_read_whose_caller_is_dropped_keeps_the_collection_lock_until_it_ends` |

Its mutant runs the read and the release in the caller's own future again, as the base did, so a
dropped caller unlocks while the read runs, and A1 reads the lock free too soon.

The risks, and what detects each:

- **A red that hangs instead of failing.** Each gated read waits a bounded time, and the gate's
  guard opens it when the guard drops, so a failed assertion still ends the read.
- **A test that passes while examining nothing.** Each fence line selects one test by `--exact`,
  and its run reads `running 1 test`.
- **A lock kept by a process that lives on after one of its runtimes stops.** The forgotten lock is
  held until the process exits, and the sync and the skip write wait for it; ADR-671 names it
  among the consequences.
- **A read that still waited for an offload permit when its caller went.** It now runs to its end
  all the same, as the holder's rule says (ADR-487 D3); ADR-671 names it among the consequences.

## 7. FORMAL

Required by the surface, a lock held across a task boundary, and met by the existing model:
`tla/OneInstrumentReader` already models the holder's two halves, `HoldToEnd` and `KeepOnStop`,
each shown needed by a witness of its own, and this change applies that holder to the collection
read the model covers. The collection lock is no variable of the model, so the change is a
stuttering step there: the cover of `with_copy` is re-stamped after the last edit, and two comment
lines beside the read's actions name the abstraction. No entry, property, witness or configuration
changes, and the ledger row stays `re-stamp`. No workflow runs the model checker, so no checker run
is owed by this change.
