# ADR-527: The sync core is one crate with no I/O: typed operations, one encoding, a hybrid clock with the id allocator beside it, and a pending queue

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-509 |
| builds on | ADR-522 (the sync protocol modelled and proved first; its decision 1 is discharged here, not amended), ADR-029 (the golden reader) |
| measured at | `554a223e` |

## Context

DeckStreak's sync is to become one log of operations that the server orders and every device pushes to and pulls from.
ADR-522 put the protocol's model and four Lean proofs in the tree first, at report, with no code to cover, and left the
sync core's delivery to stamp each proof and add the Rust tests that read its vectors. The server and both clients
must compute an operation's bytes, its clock, its key and a new id alike, and the web client runs on `wasm32` while the
iPhone and iPad client links one static library, so the rules need a home all three can link.

## D1. One log of typed operations in one encoding, and what it was chosen against

- **Chosen: the server-authoritative operation log.** Each device keeps a base (the fold of the server's operations up
  to its cursor) and a queue of its own operations the server has not judged; the server is the one writer, judges
  each pushed operation, gives the accepted ones their place, applies them and answers after it commits. Devices
  converge because the base is a deterministic fold of a log every device reads in one order.
- **Chosen against:** a clean-room port of Anki's own sync protocol (no public specification, and the owner dropped the
  Anki apps); automerge 0.12.0 (an HTML field could merge into markup neither side wrote, and a proof would trust the
  library); cr-sqlite (last released 2024-01-17, and it loses a column's concurrent value silently); SQLite session
  changesets as the transport (row-level, not intent-level: a changeset can carry an update or delete of a review row,
  which the never-list forbids, sqlx's SQLite driver has no session feature, and patchsets miss data conflicts); and
  four further candidates the design's stack review ruled out by measurement.
- **The vocabulary is typed and closed.** `Op` has fourteen kinds. No kind updates or deletes a review row, changes a
  reviewed note's notetype, deletes a preset, resets a card or replaces a whole collection
  (`docs/rulings/OWNER-RULING-2026-10-01-deck-writes.md:25-30`), and a census over the enum holds it. No kind moves a
  due date: a due change no answer made is not an operation until a ruling admits one. Tags travel as sealed events.
- **One canonical byte encoding, chosen against JSON.** A version byte, the head, then the body: the kind's tag and the
  fields in declaration order, little-endian integers, length-prefixed UTF-8. Bytes, because the server knows a
  duplicate by comparing bytes, because the codec is small enough for Kani to prove total, and because it adds no
  crate. The decoder is total: every input is a frame or a named refusal. Two devices that make the same operation write
  equal bodies, and only the head differs.

## D2. A hybrid clock, and what it was chosen against

- **Chosen: `hlc = max(pt, max(last, seen) + 1)`.** An operation's clock is past every clock its device made or pulled
  and never below the device's wall clock, so an operation made after seeing another sorts after it, and the clock
  stays the wall clock wherever that is not behind something seen. It is refused, never wrapped, at the top of the
  64-bit range.
- **Chosen against:** arrival order (an edit made offline a week ago would overwrite a newer one because it arrives
  later); a wall clock alone (under skew an edit made after seeing another can sort before it); Lamport clocks (they
  lose physical time, which a review's instant needs); vector clocks (they grow with the device count, and the server
  detects concurrency from an edit's base, not from the clock).

## D3. The key and its order, and what it was chosen against

- **Chosen: the key `(t, rank, origin, seq)`, compared in that order, and every fold in key order.** A card's reviews
  are folded in key order, never in the order the server's log delivered them, so two devices that hold the same reviews
  fold them alike. Two operations of one device differ in `seq` and two devices differ in `origin`, so two distinct
  operations never share a key.
- **Chosen against:** folding in server order (a late review that sorts before a folded one would change a card's
  counters by when it arrived); comparing the origin before the clock (a device's whole history would sort before
  another's).

## D4. One id allocator beside the clock, and what it was chosen against

- **Chosen: `next_id(floor, now, slot)`, the least id above the floor and at or above the instant whose residue mod 64
  is the slot's, with the clock's last reading as the floor.** Slot 0 is the server's and 1 to 63 are given once each
  to an enrolled device. An id is its creation instant within 63 ms, so a note's added date stays true; a device's next
  id is above every id it has seen; two slots never make the same id; and ids and clocks rise under one rule: the
  clock's `next_id_at` takes its reading as the floor and keeps the id as its reading.
- **Chosen against:** uniform draws from [2^52, 2^53) (an id would no longer be an instant, so the added date is lost,
  and a draw can collide, which costs a retry); a separate counter per device (a second clock to keep in step). The
  server keeps refusing an id that already exists, as a guard.
- **Cost:** 63 device slots, each given once.

## D5. Concurrent edits keep every loser, and what it was chosen against

- **Chosen: an edit made on a register's current value moves it forward; any other is concurrent, the greater edit
  by key (then by value) wins, and the loser is kept as a conflict with the winner's key.** A set of concurrent edits
  sharing one base ends at the edit with the greatest key, with every other edit kept, whatever order they arrive in.
  The same rule serves a note field, a deck's name, a notetype part and a card's deck.
- **Chosen against:** merging the text (an HTML field could become markup neither side wrote); refusing the second
  edit (an offline device's work would be lost to whoever synced first); last writer wins by arrival with no record (a
  stale offline edit would silently overwrite a newer one).

## D6. A pending operation leaves only on its judgement, and what it was chosen against

- **Chosen: an operation leaves the device's queue only on the server's verdict on it or its appearance in a pull.**
  An acceptance, a duplicate and a pull acknowledge it; a terminal rejection moves it, bytes and reason, to the
  rejections the owner sees; a retryable refusal sends it back to queued. Sending it changes its stage and nothing else.
- **Chosen against:** dropping it on send (a lost answer or a crash between the send and the verdict loses the
  operation, the model's `a-pending-op-dropped-on-send` witness).

## D7. Kani harnesses in the shipped crate, and what they were chosen against

- **Chosen: four harnesses in `crates/sync/src/kani_proofs.rs`, compiled only under `cfg(kani)`, each with a witness
  feature that switches on one recorded defect in the shipped function and is off by default.** The formal check's
  Kani kind reads their header lines, runs each harness and runs it again with its witness on, where it must fail. The
  workspace's lints declare `cfg(kani)` so every other build stays warning-free, and each witness branch is allowed by
  name at its site, since it selects code by the build configuration.
- **Chosen against:** harnesses with no witness (a harness that cannot fail proves nothing); a copy of the functions
  inside the harness module (the harness would check the copy, not the shipped code).
- **Recorded as not run.** No Kani install was on the machine this was built on, so the harnesses have not run; the
  ledger keeps them planned until a Kani run checks each and its witness.

## D8. The map, the language and the ledger

- **The context map gains `deck-streak-sync`, which depends on nothing,** in its bottom layer beside the other crates
  both clients may link. The edges from the server's log, the umbrella FFI crate and the web engine are drawn by the
  changes that first use the crate.
- **The lexicon gains the sync context's terms:** operation, server seq, pending, conflict record and sealed event.
- **The proofs cover the code.** The clock, key, merge and id entries carry covers on the crate's code at measured
  digests, and their vectors' headers name the covered item; the ledger records the id entry present under O12 and
  ties each of the four to the crate's test that reads its vectors byte for byte. The push judge's entry and the op
  log model stay as ADR-522 left them: the judge lands with the server's log, and both are stamped there.

## Consequences

- The server and both clients can link one encoding, one clock and one id rule, with nothing else.
- A change to a covered function stales its proof's cover, and the formal check reads it.
- The vocabulary cannot grow a never-list class without its census refusing it by name.
- The judge, the log, the routes and the store are not here; until they land, nothing runs the core end to end.

## Confirmation

SPEC-509's fence: the crate's tests (A01 to A08, A10), the ledger's tests (A11) and the context map's census (A12);
the formal check over the Lean entries; the red-first record.

## What would make this wrong

- A client that needs a clock or an id rule the server does not share.
- A due change that must replicate without an answer: it needs a ruling and a new kind, not a change to this core.
- More than 63 devices over the collection's life, which the slot count does not allow.
