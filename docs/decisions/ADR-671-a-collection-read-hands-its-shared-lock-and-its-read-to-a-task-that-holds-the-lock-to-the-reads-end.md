---
status: "accepted"
decision-makers: "the DeckStreak architect"
---

# A collection read hands its shared lock and its read to a task that holds the lock to the read's end

## Context and Problem Statement

SPEC-022 R7 gives the collection a lock: exclusive for a sync or a full download, shared for a
read, so a read never sees the copy mid-swap. SPEC-023 R1 runs every read of the copy under that
shared lock, on the kernel's offload. ADR-487 found that a lock held in a caller's future is
released when that future is dropped, while the read it guards runs on to its end on a blocking
thread nothing can stop, and cured the instrument lock by `Held::hold_to_end`. Its D6 left the same
release of the collection lock in `with_copy` to #957, which asks for a change under which the lock
taken in `with_copy` stays held until the blocking read it guards has ended, even when the caller
is dropped or its runtime stops. This record decides how.

## Decision Drivers

- The shared lock still guards the whole read, the open, the work and the close, as SPEC-023 R1
  says, whatever happens to the caller.
- One file changes, and no manifest: `crates/ingest/Cargo.toml` and `Cargo.lock` stay untouched,
  and the holder in `crates/ingest/src/lock.rs` is used as it is.
- Every read of the crate keeps going through `with_copy`, with no signature change.
- The tests decide by a gate the test opens, never by a timing race, and no red can hang.

## Decision

### D1. The mechanism, as measured at `bd953ba7`

- **The lock lives in the caller's future.** `with_copy` (`crates/ingest/src/reader.rs:366-396`)
  takes `held` shared (`:376`), awaits `self.offload.run(..)` (`:379-391`) and releases it
  (`:394`). Every other read of the crate is a call of it.
- **The read cannot be stopped.** The offload runs the read by `tokio::task::spawn_blocking`
  (`crates/kernel/src/offload.rs:64`), which no abort reaches once it runs.
- **The caller can be dropped.** As ADR-487 D1 measured, the api's request limit drops the
  handler's future when it passes, and a stopping api or bot runtime drops every async task.
- **So the lock goes early.** A dropped future drops `held`, which unlocks
  (`crates/ingest/src/lock.rs:169-175`) while the read still runs, and an exclusive take, the
  sync's (`crates/ingest/src/sync.rs:205`) or the skip write's
  (`crates/ingest/src/skip_write.rs:241`), can then run beside that read.

### D2. The cure: `with_copy` hands its lock and its read to `Held::hold_to_end`

`with_copy` still takes the shared lock before anything else, then moves the whole read, the
offload's run with its open, its work and its close, into one `async move` block, and hands the
held lock and that block to `Held::hold_to_end` (`crates/ingest/src/lock.rs:126-143`, SPEC-470 R5).
The holder runs the block in a task of its own that owns the lock and releases it explicitly when
the block ends. A caller who stops waiting only detaches that task; a task its runtime drops keeps
the lock until the process exits. The read's value or error comes back as before. One file
changes, `crates/ingest/src/reader.rs`, and no manifest.

### D3. The tests

Three tests in a `tests` module at the end of `crates/ingest/src/reader.rs` decide SPEC-648's
criteria; the reader is built there from its own fields over a zero-length copy in a temporary
directory. A1 aborts the caller while its read waits on a gate and reads the lock held until the
read ends, then free. A2 stops the runtime the read runs on while it waits on a gate, and reads the
lock held while the read runs and after it ends, with a lock of the standard library that needs no
runtime. A3 reads an ungated read's value and the lock free after it. Each gate is opened by the
test, waits a bounded time, and is opened by a guard when the guard drops, so no red hangs and no
assertion reads a time. A1 and A2 are red at the base by their assertions; A3 is not red. Row
S64801 puts the read back in the caller's future, and A1 kills it.

### D4. The model's cover, re-stamped as a stuttering step

`tla/OneInstrumentReader` covers `with_copy`. The collection lock is no variable of that model, so
the change moves none of its variables: it is a stuttering step there. The cover's digest moves to
the new span after the last edit, and two comment lines beside the read's actions name the
abstraction. No action, property, witness or configuration changes.

### D5. Not in this decision

The instrument lock, already held to its read's end by ADR-487 (#476); the exclusive takes of the
collection lock by the sync and the skip write, which stay as they are (#957).

## D2 was chosen against

- A recorded decision that no exclusive holder can run beside a released read: it would be false, since the sync's exclusive take and a full download run in the job role while an api read runs on.
- A holder inside the blocking closure: it would hold an offload permit while it waits for the lock, which ADR-487 already rejected for the instrument lock.
- Shielding only the api route: the bot's commands and every runtime stop also reach the read, so a route-only cure leaves the release open.
- A longer request limit: any limit can be passed, and a stop still drops the caller.

## D3 was chosen against

- A time limit that drops the caller in the tests: a limit races the read, while a gate the test opens decides the same order every run.
- An integration test under `crates/ingest/tests`: `with_copy` is private to the crate, so its read can be driven with a gated work only from inside it.

## Consequences

- Good, because a collection read keeps its shared lock until the read has ended, so no exclusive holder of the lock runs beside a read that still runs.
- Good, because the change is one function: no caller, no signature and no manifest changes.
- Bad, because a dropped caller's read now runs on to its end even when it was still waiting for an offload permit; it held the shared lock there already, and this is the holder's accepted rule (ADR-487 D3).
- Bad, because a read its runtime drops keeps the shared lock until the process exits (R2), and the collection lock's exclusive takes wait for it (`crates/ingest/src/sync.rs:205` and `crates/ingest/src/skip_write.rs:241` await `.exclusive()`, not a take that never waits), so a process that stopped one runtime and lived on would hold every sync and skip write until it exits.
- Neutral, because the roles' runtimes stop as their processes stop, so the kept lock ends with the process.

## Confirmation

SPEC-648 A1 to A3 decide the cure; A1 and A2 are red first at the base and green with the cure.
Row S64801 is proved by hand, its mutant killed by A1, and is read by name in CI. The formal check
reads no stale cover for `tla/OneInstrumentReader`.

## What would make this wrong

- A process that lives on after one of its runtimes stops: its kept lock would then hold every later sync and skip write until that process exits.
- A blocking read that can be stopped: the read would then end with its caller, and holding the lock to its end would be needless.
- A read that runs outside `with_copy`: it would not take the lock at all, and this cure would not reach it.

## More Information

ADR-487 (the holder, and D6, which left this to #957), SPEC-470 R3 and R5 (the holder's rules),
SPEC-022 R7 (the collection lock), SPEC-023 R1 (the read), SPEC-648 (this record's requirements
and criteria), #957 and #476.
