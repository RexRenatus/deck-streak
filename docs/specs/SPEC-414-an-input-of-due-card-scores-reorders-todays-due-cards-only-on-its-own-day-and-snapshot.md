# SPEC-414: an input of due-card scores reorders today's due cards only on its own day and snapshot

- **Wave:** the reorder-only input of ADR-339. **Issue:** #824. **Context(s):** `engine-core`
  (`crates/engine-core`); the formal entries `lean/DueReorder` and `tla/DueReorderInput`.
- **Decided by:** ADR-428 (this SPEC's own: D1 the input's key, D2 which day and which snapshot, D3 where the
  reorder lives, D4 the score rule, D5 the scope, D6 the formal surface, D7 the readers' register) and
  ADR-339 (the reorder-only rule, whose input this SPEC gives a key; ADR-339's own text is unchanged).
- **Schematic:** `docs/schematics/due-card-scores-reorder.md`.
- **Status:** designed; built by this delivery. **Mutation band:** `S41400-S41499`.
  **Changelog:** `changelog.d/reorder-input-414.md`. **Red-first record:** `docs/red-first/SPEC-414.md`.

Refs read: `DEV` is DeckStreak `dev` `c74b6b9684bf24dc56a83c7005afd52064d2632c`. Every fact below is read at
DEV with `git show DEV:<path>` at the lines beside it, or measured by the command named. The engine's own
source is read at the workspace's pinned engine.

## 1. The problem, measured

| id | fact | where |
|---|---|---|
| M1 | ADR-339's input is "for each card due today, an identifier and a score", and its rule says "missing or stale scores leave the stock order". The input names no study day and no snapshot, so a client holding it cannot tell a stale score from a fresh one | `docs/decisions/ADR-339-every-other-experimental-model-only-reorders-cards-already-due.md:37-44` |
| M2 | No Rust item holds the reorder or its input: `git grep -n -e DueScores -e 'fn reorder' DEV -- crates` prints 0 lines | the command |
| M3 | Both web readers of the queue's head ask the engine's `GetQueuedCards` for one card (`fetch_limit: 1`) and present it in the stock order | `crates/web-engine/src/wasm.rs:263-265`, `:723-726` |
| M4 | The model `tla/DueReorderInput` with `Keyed` off, the input ADR-339 states, violates `ScoresOfAnotherDayOrSnapshotNeverApply`: a five-state behaviour (initial, the scorer's read on day 1, its scores' arrival, the switch on, a new study day) in which day 1's scores still order day 2's queue; TLC found 3777 distinct states | `formal/tla/DueReorderInput/witness/scores-apply-only-on-their-own-day-and-snapshot.cfg`, run by hand with the pinned TLC (`-workers 1 -fp 0`) |
| M5 | The same model with the scores keyed by study day and snapshot, applied only when both equal the client's own, is clean on all four properties | the formal check of `tla/DueReorderInput` at this branch's model commit |
| M6 | Clearing the input at the rollover does not close M4: a scorer run that reads on day 1 and arrives after the rollover lands in day 2's input after any clear | the same witness's model, `Snapshot`, `NewDay`, `Publish` |
| M7 | The engine's day is the core's `EngineDay::days_elapsed`, read from the engine's `SchedTimingToday` (13, 5) | `crates/engine-core/src/late.rs:21-23`, `crates/engine-core/src/dispatch.rs:43-46` |
| M8 | The engine stores a schema stamp in `col.scm`; a sync whose two copies carry different schema stamps requires a full sync; `set_schema_modified` "Forces the next sync in one direction"; the core's change fingerprint already reads `scm` | the engine's `rslib/src/storage/collection_timestamps.rs:13`, `rslib/src/sync/collection/meta.rs:74`, `rslib/src/collection/timestamps.rs:35-38`; `crates/engine-core/src/dispatch.rs:79-90` |
| M9 | The engine's sync server forces a one-way sync of a collection over its payload size by sending a fresh schema stamp in its reply without storing one | the engine's `rslib/src/sync/collection/meta.rs:178-181` |
| M10 | No cover line under `formal/` names a file this delivery edits outside `formal/`: `git grep -n '@phx covers crates/engine-core/src/lib.rs' -- formal` prints 0 lines, and no cover names `crates/engine-core/src/reorder.rs` before this delivery | the command |

## 2. Requirements

R1. `crates/engine-core/src/reorder.rs` holds `pub struct DueScores { pub day: u32, pub snapshot: i64, pub
    scores: BTreeMap<i64, f64> }`: the engine's day count and the collection's schema stamp the scores were made
    for (ADR-428 D1, D2), and one score per card id.
R2. It holds `pub fn reorder<T>(on: bool, today: u32, snapshot: i64, input: Option<&DueScores>, queue: Vec<T>,
    card: impl Fn(&T) -> i64) -> Vec<T>`, marked `#[must_use]`. It takes the queue's entries by value and gives
    them back; it has no way to build or change an entry.
R3. When `on` is false, `input` is `None`, the input's `day` is not `today`, or its `snapshot` is not
    `snapshot`, the result is `queue` exactly.
R4. Otherwise a card's score is used only when it lies in `[0, 1]`: NaN, both infinities, a value below 0 and a
    value above 1 count as no score, and `-0.0` is a score equal to `0.0`. The result is every entry whose card
    has a used score, highest score first, entries with equal scores in their stock order, then every other
    entry in its stock order.
R5. The result is a permutation of `queue`, and each entry in it is the entry the queue held, unchanged.
R6. Reordering the result again gives the result, and two inputs with the same key and the same used score for
    every queued card give the same result.
R7. The core's sources and the two client adapters' sources name `DueScores` in `src/reorder.rs` of the core
    alone, and that file names no engine crate, no other module of the core and no input or output of the
    process (ADR-428 D7).
R8. The Lean entry `lean/DueReorder` covers `reorder` and `DueScores`, and a Rust test replays every line of
    `formal/vectors/due-reorder.jsonl` against `reorder`, examining as many vectors as the header declares. The
    TLA entry `tla/DueReorderInput` covers both items beside its three `wasm.rs` spans. Every cover is stamped
    with its item's digest, and the ratchet against `dev` reports no weakening (ADR-428 D6).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | over a generated population of queues and inputs, the reorder presents the same entries with every field unchanged and moves some; a planted reorder that writes a due date is refused by the same check | `cargo test -p deck-streak-engine-core --test reorder -- --exact the_reorder_presents_the_same_entries_and_writes_no_field` |
| A2 | scored cards lead, highest first, and the stock order breaks every tie, `-0.0` and `0.0` included | `cargo test -p deck-streak-engine-core --test reorder -- --exact scored_cards_lead_highest_first_and_the_stock_order_breaks_ties` |
| A3 | NaN, both infinities, a score below 0 and a score above 1 each count as no score | `cargo test -p deck-streak-engine-core --test reorder -- --exact an_invalid_score_counts_as_no_score` |
| A4 | scores made for another study day or another snapshot leave the stock order, and the same scores on their own day and snapshot reorder it | `cargo test -p deck-streak-engine-core --test reorder -- --exact scores_made_for_another_day_or_snapshot_leave_the_stock_order` |
| A5 | the switch off, no input, an empty map and a map that scores no queued card each present the stock queue exactly, and the switch on again reorders it | `cargo test -p deck-streak-engine-core --test reorder -- --exact an_unused_input_presents_the_stock_queue_and_on_again_reorders` |
| A6 | reordering twice is reordering once, and the order reads only the queued cards' used scores | `cargo test -p deck-streak-engine-core --test reorder -- --exact reordering_twice_is_once_and_the_order_reads_only_the_queued_scores` |
| A7 | the Rust reorder gives every Lean vector's output, and the test examines as many vectors as the header declares | `cargo test -p deck-streak-engine-core --test formal_vectors_due_reorder -- --exact the_rust_reorder_gives_every_lean_vectors_output` |
| A8 | the input is named by the reorder module alone, which reaches no engine, and a planted second reader and a planted engine import are each refused by name | `cargo test -p deck-streak-engine-core --test reorder_shape -- --exact the_input_is_named_by_the_reorder_module_alone_and_it_reaches_no_engine` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test reorder -- --exact the_reorder_presents_the_same_entries_and_writes_no_field
A2: cargo test -p deck-streak-engine-core --test reorder -- --exact scored_cards_lead_highest_first_and_the_stock_order_breaks_ties
A3: cargo test -p deck-streak-engine-core --test reorder -- --exact an_invalid_score_counts_as_no_score
A4: cargo test -p deck-streak-engine-core --test reorder -- --exact scores_made_for_another_day_or_snapshot_leave_the_stock_order
A5: cargo test -p deck-streak-engine-core --test reorder -- --exact an_unused_input_presents_the_stock_queue_and_on_again_reorders
A6: cargo test -p deck-streak-engine-core --test reorder -- --exact reordering_twice_is_once_and_the_order_reads_only_the_queued_scores
A7: cargo test -p deck-streak-engine-core --test formal_vectors_due_reorder -- --exact the_rust_reorder_gives_every_lean_vectors_output
A8: cargo test -p deck-streak-engine-core --test reorder_shape -- --exact the_input_is_named_by_the_reorder_module_alone_and_it_reaches_no_engine
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-414-an-input-of-due-card-scores-reorders-todays-due-cards-only-on-its-own-day-and-snapshot.md` | docs | added |
| `docs/decisions/ADR-428-the-due-card-scores-input-carries-its-study-day-and-snapshot-and-the-core-holds-the-one-reorder.md` | docs | added |
| `docs/schematics/due-card-scores-reorder.md` | docs | added |
| `docs/LEXICON.md` | docs | changed: a glossary row for "due-card scores" in `engine-core` |
| `docs/red-first/SPEC-414.md` | docs | added |
| `changelog.d/reorder-input-414.md` | docs | added |
| `crates/engine-core/src/reorder.rs` | `engine-core` | added |
| `crates/engine-core/src/lib.rs` | `engine-core` | changed: `pub mod reorder;` and its line in the module list |
| `crates/engine-core/tests/reorder.rs` | `engine-core` | added |
| `crates/engine-core/tests/reorder_shape.rs` | `engine-core` | added |
| `crates/engine-core/tests/formal_vectors_due_reorder.rs` | `engine-core` | added |
| `formal/lean/Formal/DueReorder.lean` | formal | changed: two covers and the prose that said the entry covers nothing |
| `formal/vectors/due-reorder.jsonl` | formal | changed: the header names the covers; written by the Lean writer |
| `formal/tla/DueReorderInput/DueReorderInput.tla` | formal | changed: two covers and the prose that said the code was ahead |
| `scripts/mutation-rows.d/S41400-S41499.json` | mutation | added |

## 5. What this does NOT do

- It wires no client: no study screen calls `reorder`, and no client reads its own day or schema stamp for it.
  The web study screens own that wiring (#630), and the iPhone and iPad review screen owns theirs (#632).
- It carries no scorer, no model's code or weights, and no transport, storage or authentication of the scores;
  the repository carries the input's shape alone, as ADR-339 and SPEC-334 decided (#612).
- It adds no switch to any settings screen; the switch is the `on` argument until a client wires one (#630).
- It moves no card field and no queue: the stock scheduler's queue, its sibling bury and its fetch limit stay
  as they are (#824).
- It models one scorer run in flight, not two, and checks the model at two cards; a run at three cards did not
  finish inside the entry's budget (#824).

## 6. Risks

- The server's size-forced one-way sync (M9) replaces one copy without moving either stored schema stamp, so
  scores made before it still apply after it. The scores then order cards of the same lineage within the same
  study day, and every card they order is still one the stock scheduler made due; ADR-428 records it under
  "What would make this wrong". Detected by: the model's `FullSync` advances the snapshot on every full sync,
  so the gap is named in the model's abstractions, not hidden.
- A client wiring could pass a day or a stamp read at a different moment from the queue it reorders. Detected
  by: the wiring's own SPEC (#630, #632) and A8's register, which refuses a second file that names the input
  until the wiring adds itself.
- A later sort written with `total_cmp` would order `-0.0` below `0.0`. Detected by: A2 and the Lean vectors
  (A7), whose population puts `-0` beside `0` in every stock order.
- A field-writing step added inside `reorder` cannot compile, because the function is generic over the entry;
  a field write added elsewhere that reads the input is detected by A8's register.

## 7. Formal

FORMAL: Lean REQUIRED (the reorder is a pure ordering over every queue and input; `lean/DueReorder`, seven
theorems at `ramp=report`, each with its witness, covering `reorder` and `DueScores`, its vectors replayed by
A7). TLA REQUIRED (the scorer, the rollover, a full sync and late arrival interleave over the input the client
holds; `tla/DueReorderInput`, four properties at `ramp=report` with five witnesses, covering `reorder` and
`DueScores` beside `current_card`, `first_queued` and `rate` in `crates/web-engine/src/wasm.rs`). The
witness `scores-apply-only-on-their-own-day-and-snapshot.cfg` is ADR-339's input; the fixed model is the key
of ADR-428 D1. The verdict lines are read from the formal check per entry and from the ratchet run over the
branch, and recorded in the delivery's hand-back.

## 8. Mutation rows

The band `S41400-S41499` holds hand-proved rows over `crates/engine-core/src/reorder.rs`: the key comparison,
the score rule's two bounds, the order of the sort and the place of the unscored entries. Each row's killer is
one test of section 3, and each row is proved with the file restored.
