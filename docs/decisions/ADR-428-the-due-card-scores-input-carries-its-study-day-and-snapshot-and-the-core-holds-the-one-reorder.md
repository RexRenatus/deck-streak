# ADR-428: the due-card scores input carries its study day and snapshot, and the core holds the one reorder

- **Status:** proposed, with SPEC-414 (#824).
- **Context(s):** `engine-core` (`crates/engine-core`); the formal entries `lean/DueReorder` and
  `tla/DueReorderInput`.
- **Settles:** the shape of ADR-339's generic input of external due-card scores, so that "missing or stale scores
  leave the stock order" can be implemented; which day and which snapshot the input names; where the reorder
  lives; its rule for a score; the delivery's scope; the formal surface; and who may name the input.
- **Refines:** ADR-339's input ("for each card due today, an identifier and a score") with a key. ADR-339's text
  and status are left as they are; this ADR records the shape the code holds.
- **Read at:** DeckStreak `dev` `c74b6b9684bf24dc56a83c7005afd52064d2632c`. Every `path:line` below is read
  there; the engine's own source is read at the workspace's pinned engine.

## Context

ADR-339 lets a model other than FSRS-7 only reorder the cards the stock scheduler already made due, through a
generic input of external due-card scores, and rules that missing or stale scores leave the stock order. The
input it names is an identifier and a score per card. The scorer runs outside the repository and reads a
snapshot of the collection; its scores reach the client later, or never.

The model `tla/DueReorderInput` checks that input against a scorer read, the scores' arrival, the owner's switch,
a new study day and a full sync. With the input as ADR-339 states it, TLC violates
`ScoresOfAnotherDayOrSnapshotNeverApply` in five states: the scorer reads on day 1, its scores arrive, the owner
switches the input on, and the study day rolls over; day 1's scores still order day 2's queue (SPEC-414 M4). The
consequential case puts a card scored yesterday ahead of today's stock head. An input with no key cannot be told
apart from a fresh one, so the rule ADR-339 states cannot be implemented over the input it names.

No Rust item holds the reorder or its input at `dev` (SPEC-414 M2). The two web readers of the queue's head ask
the engine for one card in the stock order (`crates/web-engine/src/wasm.rs:263-265`, `:723-726`).

## Decision Drivers

- ADR-339's rule, stale scores leave the stock order, must be decidable by the client from what it holds.
- A late arrival is normal: a scorer run can finish after the rollover or after a full sync.
- Both sides must compute the key from values each one can read: the scorer from its read-only snapshot, the
  client from its own collection.
- The reorder must be one function both clients call, so the Lean vectors bind the code that runs.
- The reorder must have no way to write a card field (ADR-339; ADR-301's never-list entries 5 and 6).

## Decisions, and the alternatives each was chosen against

D1 decides the input's key, D2 which day and snapshot, D3 where the reorder lives, D4 the score rule, D5 the
scope, D6 the formal surface and D7 who may name the input. Each decision's rejected options are the bullets
under it, each with its reason on its first line.

### D1. The input carries the study day and the snapshot it was made for, and applies only when both equal the client's

```rust
pub struct DueScores {
    pub day: u32,
    pub snapshot: i64,
    pub scores: BTreeMap<i64, f64>,
}
```

The client applies the scores only when the switch is on, an input is present, `day` equals its own day and
`snapshot` equals its own snapshot. In every other case the queue is presented as the stock scheduler built it.
With this key, `tla/DueReorderInput` is clean on its four properties (SPEC-414 M5).

- ADR-339's input as written, an identifier and a score per card: rejected because it carries nothing that tells a stale score from a fresh one, and TLC finds day 1's scores ordering day 2's queue in five states.
- Clearing the held input at the rollover and at every full sync: rejected because a run that read on day 1 and arrives after the rollover lands after the clear (SPEC-414 M6), so the clear does not decide staleness.
- An expiry on the wall clock: rejected because the engine's day turns at its rollover hour and not after a fixed duration, and the scorer's clock and the device's clock need not agree.
- A key of the study day alone: rejected because a full sync within the day can replace the collection the scores were read from, and the model's `FullSync` then applies scores of the replaced snapshot.
- A key of the snapshot alone: rejected because the schema stamp does not move at the rollover, so yesterday's scores would order today's queue.

### D2. The day is the engine's day count, and the snapshot is the collection's schema stamp `col.scm`

The day is `EngineDay::days_elapsed`, the count the core already reads from the engine's `SchedTimingToday`
(`crates/engine-core/src/late.rs:21-23`, `crates/engine-core/src/dispatch.rs:43-46`): the day the stock queue is
built for. The snapshot is the collection's schema stamp, stored in `col.scm` (the engine's
`rslib/src/storage/collection_timestamps.rs:13`). The engine requires a full sync whenever two copies' schema
stamps differ (`rslib/src/sync/collection/meta.rs:74`), and `set_schema_modified` "Forces the next sync in one
direction" (`rslib/src/collection/timestamps.rs:35-36`). So two copies with the same stamp are of one lineage,
an answer and a normal sync do not move it, and both the scorer's read-only snapshot and the client's own
collection hold it. The core's change fingerprint already reads it (`crates/engine-core/src/dispatch.rs:79-90`).

- The collection's modification stamp `col.mod`: rejected because every answer moves it, so scores read before a session would stop applying at its first answer.
- The last-sync stamp `col.ls`: rejected because every normal sync moves it, so a sync that changes no card would void the day's scores.
- An id the scorer mints for each run: rejected because the client has no value of its own to compare it with, so it decides nothing.
- The server's study day from the kernel: rejected because the reorder orders the engine's queue, which is built for the engine's day, and a second day source can disagree with it around the rollover hour.

### D3. One pure function in the core, generic over the queue's entry

`crates/engine-core/src/reorder.rs` holds `DueScores` and

```rust
#[must_use]
pub fn reorder<T>(on: bool, today: u32, snapshot: i64, input: Option<&DueScores>, queue: Vec<T>,
    card: impl Fn(&T) -> i64) -> Vec<T>
```

It takes the queue by value and returns its entries in the presented order. Generic over `T`, it can read a card
id through `card` and can neither build an entry nor change one, so a field write inside it does not compile. It
names no engine type and no other module of the core. Both clients reach the core already (ADR-356), so each can
call it when its wiring lands, and no edge is added to the context map.

- A new crate for the reorder: rejected because one function does not justify a new context with its own edges on the map, and both clients already depend on the core.
- A copy in each client adapter: rejected because two copies drift, and the Lean vectors would bind only the copy a test replays.
- The server's ingest crate: rejected because no client depends on it, and the server never presents a queue.
- A function over the engine's queued-card type: rejected because it would name the engine's types and hand the function the fields it must never write; the generic form makes that impossible.

### D4. A score is used only when it lies in `[0, 1]`; the sort is stable, by `partial_cmp`, highest first

A score is used when `(0.0..=1.0).contains(&score)`: NaN, both infinities, a value below 0 and a value above 1
count as no score, and `-0.0` is a score equal to `0.0`. The used scores are sorted highest first by a stable
sort with `partial_cmp`, so equal scores, `-0.0` beside `0.0` included, keep their stock order. The unscored
entries follow in their stock order, which is ADR-339's "keeps its stock position after the scored ones" read as
the stock order among the unscored.

- `f64::total_cmp`: rejected because it orders `-0.0` below `0.0`, so a tie would break by the sign of zero instead of by the stock order.
- Clamping an out-of-range score into `[0, 1]`: rejected because a broken scorer's `1.25` would lead the queue; counting it as missing leaves that card where the stock scheduler put it.
- Refusing the whole input for one invalid score: rejected because one bad row would void a day's valid scores, and ADR-339's rule is per card.
- Unscored entries interleaved at their stock positions: rejected because ADR-339 puts a card with no score after the scored ones.

### D5. The scope: the function, its input, its tests and the covers; no wiring

This delivery adds `reorder` and `DueScores`, their tests, the Lean vectors' replay and the covers of the two
formal entries. No client calls `reorder`, and no client reads its own day or schema stamp for it.

- Wiring the web study screen now: rejected because the web study screens' campaign owns that screen (#630), no scorer publishes yet, and `current_card` and `rate` in `crates/web-engine/src/wasm.rs` are covered by `tla/DueReorderInput` and `tla/UndoOwnAnswer`, so an edit there re-opens a second model.
- Wiring the iPhone and iPad review screen now: rejected because its campaign owns that screen (#632), and the same scorer is absent.
- A fixed read of `col.scm` alone in the core's dispatcher now: rejected because only the wiring reads it, and a read with no caller is a door nothing tests.

### D6. The formal surface: Lean for the ordering, TLA for the input over time

`lean/DueReorder` proves the ordering's seven claims over every queue and input and every instance of the score
operations, and its vectors are replayed by `crates/engine-core/tests/formal_vectors_due_reorder.rs`. It covers
`reorder` and `DueScores`. `tla/DueReorderInput` checks the input against the scorer, the switch, the rollover,
a full sync and late arrival, and covers the same two items beside `current_card`, `first_queued` and `rate`.
Every property and theorem enters at `ramp=report`.

- Lean alone: rejected because stale scores are an interleaving of a scorer, a rollover, a full sync and a late arrival, which a proof about one call cannot see.
- TLA alone: rejected because ties, `-0.0` and invalid scores are a pure ordering claim that a proof over every input closes and a bounded model only samples.
- No covers on the Rust items: rejected because a later edit to the function would then leave both entries clean while they describe code that changed.

### D7. A register of the files that name the input

`crates/engine-core/tests/reorder_shape.rs` walks the core's sources and both client adapters' sources and
refuses any file other than `crates/engine-core/src/reorder.rs` that names `DueScores`, and refuses that file if
it names an engine crate, another module of the core, or the process's input and output. A wiring delivery adds
its file to the register in the change that names the input. This is ADR-339's shape test: no score input
reaches a write path of the core.

- No census, relying on the generic signature: rejected because the signature stops `reorder` from writing a field, and nothing stops another module that reads the input from writing a due date.
- A census of the core alone: rejected because a client adapter could read the input and act on it outside the core.

## Consequences

- ADR-339's rule is implementable: a stale input, on any path by which it became stale, presents the stock order.
- The scorer must publish the engine's day count and the schema stamp of the copy it read, beside its scores.
- A full sync that moves the schema stamp voids the held scores until a fresh run arrives.
- The server's size-forced one-way sync replaces a copy without moving either stored stamp (SPEC-414 M9), so
  scores read before it still apply after it, within the same day and over cards of the same lineage.
- A client that wires the reorder adds itself to D7's register.

## What would make this wrong

- A full sync that replaces a collection with one of another lineage while both stored schema stamps stay equal,
  observed in the engine's sync: the snapshot id would need a second component.
- A scorer that cannot read the engine's day count of its snapshot: the input would need another day both sides
  compute alike, with its own decision.
- A client whose stock queue is built for a day other than `EngineDay::days_elapsed`.
- A need to order by a score outside `[0, 1]`: the rule of D4 would need its own decision.
