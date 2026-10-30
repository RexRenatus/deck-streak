# SPEC-509: One sync core: typed operations, one encoding, a hybrid clock, one id allocator and a pending queue

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #15 (the sync engine), until the pull request names its own |
| decided by | ADR-527, under ADR-522 (the sync protocol modelled first; not amended) and ADR-029 |
| measured at | `554a223e` |
| schematic | `docs/schematics/the-sync-core-and-a-pending-operations-life.md` (new); `docs/schematics/context-map.md` (changed: the new crate) |
| mutation band | S50900-S50999 (six hand rows: section 9) |

## 1. The problem, measured

DeckStreak's sync is to become one server-ordered log of operations that every device pushes to and pulls from. Its
protocol is modelled and proved (SPEC-504), and nothing in the tree runs it:

- **No sync core.** `git ls-tree -d --name-only 554a223e crates/ | grep -c -x crates/sync` prints 0.
- **The proofs cover no code.** The first line of each of `formal/vectors/{sync-clock,review-key,field-merge,push-judge}.jsonl`
  reads `"covers":null` (4 of 4), and ADR-522 decision 1 leaves each entry to be stamped by the sync core's delivery.
- **The id allocator is planned.** `formal/obligations.json` lists `lean/IdSlot` under O12 as `"state": "planned"` (1
  line), and the four Kani harnesses the sync core owns, K-sync-codec, K-key-order, K-hlc and K-next-id, as planned (4
  lines). No source of the tree names `cfg(kani)`.
- **Both clients need the same rules.** The web client runs on `wasm32` in a Worker and the iPhone and iPad client links
  one static library (`docs/CONTEXT-MAP.md`, the web engine's and the umbrella FFI crate's lines). An operation's bytes,
  its clock and its id must be computed alike on the server and on both clients, so the rules live in a crate with no
  I/O that all three can link.

## 2. Requirements

R1. **One encoding.** `crates/sync`, package `deck-streak-sync`, encodes an operation as one frame: a version byte (1),
    the head (16-byte origin, 64-bit sequence number, 64-bit clock) and the body (the kind's tag and its fields in
    declaration order), with little-endian integers, UTF-8 text behind a 32-bit length, an option as a 0 or 1 byte and
    a list behind a 16-bit count. A frame holds at most 256 KiB. `decode` is total: every input is a frame or a named
    refusal (too large, truncated, another version, an unknown kind, a value out of its range, text that is not UTF-8,
    trailing bytes). Every operation round-trips, and every strict prefix of a frame is refused as truncated.
R2. **Typed operations.** `Op` holds fourteen kinds: an answer and an undo of one; a note created, a field set and a
    note deleted; a card moved, flagged, buried and suspended; a deck created and renamed; a notetype part edited; a
    media name bound and unbound. An answer's ease is 1 to 4, its kind 0 to 3 and its duration at most an hour, and
    both ends refuse any other value. No kind updates or deletes a review row, changes a note's notetype, deletes a
    preset, resets a card or replaces a whole collection (the never-list,
    `docs/rulings/OWNER-RULING-2026-10-01-deck-writes.md:25-30`), and no kind moves a due date: a due change no answer
    made is not an operation. Tags are not here: a tag edit travels as a sealed event.
R3. **The key and its order.** A key is `(t, rank, origin, seq)`, compared in that order; `sort_keys` puts keys in key
    order whatever order they arrived in, and gives the Lean port's order on every vector of
    `formal/vectors/review-key.jsonl`.
R4. **The hybrid clock.** `tick(last, seen, pt)` is one past the larger of the device's last clock and the largest clock
    it has seen, never below the wall clock `pt`, and is refused when no larger clock fits in 64 bits. A device's clock
    (`Hlc`) keeps the largest clock it made or saw: `recv` raises it, `tick_at` makes the next one. Both give the Lean
    port's clock on every vector of `formal/vectors/sync-clock.jsonl`, byte for byte.
R5. **One id allocator beside the clock.** `next_id(floor, now, slot)` is the least id above `floor` and at or above
    `now` whose residue mod 64 is `slot`, refused when it would pass 64 bits; slot 0 is the server's and slots 1 to 63
    are devices'. The clock's `next_id_at` takes its last reading as the floor and keeps the id as its reading, so ids
    and clocks rise under one rule. It gives the Lean port's id on every vector of `formal/vectors/id-slot.jsonl`, and
    two devices on slots 1 and 2 never make the same id.
R6. **The pending queue.** A device's operation enters the queue when it is made and leaves only on its judgement: an
    acceptance or a duplicate verdict, or its appearance in a pull, acknowledges it; a terminal rejection moves it,
    bytes and reason, to the rejections the owner sees. Sending it, sending it again and a retryable refusal leave it
    pending.
R7. **Concurrent edits.** A register (a note field, a deck's name, a notetype part, a card's deck) takes an edit made
    on its current value as a move forward; any other edit is concurrent, the greater edit by key (then value) wins,
    and the loser is kept as a conflict with the winner's key. `merge` and `apply` give the Lean port's register on
    every vector of `formal/vectors/field-merge.jsonl`.
R8. **Equal bodies.** Two devices that make the same operation write byte-equal bodies; their frames differ only in the
    head, and `encode_body` is the body. The identity holds over each of the fourteen kinds. The design states it of a
    reschedule operation too (its test T2-6); that instance is held, not owed by this delivery, because the design
    holds reschedule out of the vocabulary until a ruling admits it. It reads not applicable here, never as a pass, and
    becomes owed with the change that admits the kind.
R9. **Kani harnesses.** `crates/sync/src/kani_proofs.rs`, compiled only under `cfg(kani)`, declares K-sync-codec
    (decode never panics on any input up to 48 bytes, unwind 50, and a note of two 4-byte fields round-trips),
    K-key-order (a strict total order over symbolic 16-byte origins), K-hlc (`hlc_tick_no_overflow`) and K-next-id
    (never panics; on success the residue and both bounds hold), each with a witness feature off by default that no
    other manifest names.
R10. **No I/O.** The crate's manifest has no dependency, build script or target table; its one dev-dependency is the
    golden reader's `serde_json` (ADR-029). No source of it reads a clock, a file, the environment, the network, a
    process, a thread or randomness. CI builds it for `wasm32-unknown-unknown` and `aarch64-apple-ios`.
R11. **The proofs cover the code.** `formal/lean/Formal/IdSlot.lean` proves `next_id_residue`, `next_id_above_floor`,
    `next_id_at_least_now` and `slots_disjoint` of its port, each refuted of a named defect by a witness, with its
    writer `Formal/IdSlotVectors.lean`. `SyncClock`, `ReviewKey`, `FieldMerge` and `IdSlot` carry `covers` lines on
    the crate's `tick`, `sort_keys` and `compare`, `merge` and `pick`, and `next_id` at their measured digests, and
    their vectors' headers name the covered item. The ledger records `lean/IdSlot` present under O12, ties the four
    entries to the crate's vectors tests, and keeps the Kani harnesses planned until a Kani run checks them.
R12. **The map and the language.** The context map gains `deck-streak-sync ... depends on: nothing` and its schematic
    the crate's node; the lexicon gains the sync context's terms.

## 3. Acceptance criteria of SPEC-509

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A01 | R1, R2: every kind round-trips over small values, exhaustively, and every truncation is refused; every refusal is named | a decoder that skips the version byte | ADR-527 D1 |
| A02 | R4: `tick` and `recv` answer every Lean clock vector byte-equal; the clock keeps what it made and saw | `tick = max(pt, last)`, no +1 | ADR-527 D2 |
| A03 | R3: `sort_keys` answers every Lean key vector byte-equal | the origin compared before the clock | ADR-527 D3 |
| A04 | R6: a pending operation leaves the queue only on its verdict or its appearance in a pull | a queue that drops on send | ADR-527 D6 |
| A05 | R7: all 6 orders of 3 concurrent edits end in one value and one loser set; `apply` answers every Lean merge vector | arrival-order last-writer-wins | ADR-527 D5 |
| A06 | R8: two devices' frames of one operation have equal bodies and differ in the head, over each of the 14 kinds (the reschedule instance is held: R8) | the clock written into the body | ADR-527 D1 |
| A07 | R5: `next_id` answers every Lean id vector byte-equal; two devices on slots 1 and 2 make disjoint ids over 10,000 random instants | the slot added without reducing mod 64 | ADR-527 D4 |
| A08 | R2: no variant of `Op` names a never-list class (examined = the variant count), each planted class refused by name | a planted `ReviewEdited` variant | ADR-527 D1 |
| A09 | R9: each Kani harness verifies and its witness fails | none: recorded as not run (section 7) | ADR-527 D7 |
| A10 | R10: the crate depends on nothing and reaches nothing outside its arguments; CI builds it for both clients' targets | a stub that reads the wall clock | ADR-527 D1 |
| A11 | R11: the ledger holds the IdSlot entry present, its rows and the four ties | the ledger test before the entry exists | ADR-527 D8 |
| A12 | R12: the fence and the schematic carry the crate, held by the census | the census names the crate's missing node and fence row | ADR-527 D8 |

```acceptance
A01: cargo test -p deck-streak-sync --test codec -- --exact every_variant_round_trips_over_small_values_and_every_truncation_is_refused
A01: cargo test -p deck-streak-sync --test codec -- --exact another_version_an_unknown_kind_and_a_trailing_byte_are_refused_by_name
A01: cargo test -p deck-streak-sync --test codec -- --exact a_value_out_of_its_range_is_refused_by_both_ends
A01: cargo test -p deck-streak-sync --test codec -- --exact a_frame_at_the_bound_is_admitted_and_one_byte_more_is_refused
A01: cargo test -p deck-streak-sync --test codec -- --exact each_refusal_reads_as_its_sentence
A02: cargo test -p deck-streak-sync --test formal_vectors_sync_clock -- --exact tick_and_recv_answer_every_lean_clock_vector_byte_equal
A02: cargo test -p deck-streak-sync --test clock -- --exact the_clock_keeps_what_it_made_and_saw_and_refuses_at_the_top_keeping_its_reading
A03: cargo test -p deck-streak-sync --test formal_vectors_review_key -- --exact sort_keys_orders_every_lean_review_key_vector_byte_equal
A04: cargo test -p deck-streak-sync --test pending -- --exact a_pending_operation_leaves_the_queue_only_on_its_verdict_or_its_appearance_in_a_pull
A04: cargo test -p deck-streak-sync --test pending -- --exact every_terminal_reason_keeps_the_bytes_and_every_pending_operation_keeps_its_frame
A05: cargo test -p deck-streak-sync --test field_merge -- --exact all_6_orders_of_3_concurrent_field_edits_end_in_one_value_and_one_loser_set
A05: cargo test -p deck-streak-sync --test field_merge -- --exact an_edit_made_on_the_current_value_moves_the_register_forward_with_no_conflict
A05: cargo test -p deck-streak-sync --test formal_vectors_field_merge -- --exact apply_merges_every_lean_field_merge_vector_byte_equal
A06: cargo test -p deck-streak-sync --test codec -- --exact two_devices_encode_one_operation_to_bodies_equal_apart_from_the_head
A07: cargo test -p deck-streak-sync --test formal_vectors_id_slot -- --exact next_id_answers_every_lean_id_slot_vector_byte_equal
A07: cargo test -p deck-streak-sync --test ids -- --exact two_devices_on_slots_1_and_2_make_disjoint_ids_over_10000_random_instants
A07: cargo test -p deck-streak-sync --test ids -- --exact a_slot_is_admitted_below_64_and_refused_from_64
A08: cargo test -p deck-streak-sync --test op_census -- --exact no_operation_variant_names_a_review_edit_a_retype_a_preset_delete_a_card_reset_or_a_replace
A10: cargo test -p deck-streak-sync --test crate_shape -- --exact the_crate_depends_on_nothing_and_reaches_nothing_outside_its_arguments
A11: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_the_rows_hold_o1_to_o17_and_the_planned_entries
A11: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_every_existing_entry_has_its_disposition_row
A12: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
A12: python3 -m unittest discover -s scripts/tests -p test_context_map_register.py
```

A09 has no fence line: its harnesses run through the formal check's Kani kind, which reads them from
`crates/sync/src/kani_proofs.rs` (section 7). The Lean entries are judged by the formal check, a local check outside
the tree: `phxd formal check --root . --base <base> --kind lean --gate --format json` reads every theorem built, every
witness built, the vectors byte-equal and every cover fresh.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/sync/Cargo.toml` | sync | new: no dependency; the golden reader's dev-dependency; four Kani witness features |
| `crates/sync/src/lib.rs` | sync | new: the crate's doc, its modules and re-exports |
| `crates/sync/src/op.rs` | sync | new: R2 |
| `crates/sync/src/codec.rs` | sync | new: R1, R8 |
| `crates/sync/src/key.rs` | sync | new: R3 |
| `crates/sync/src/clock.rs` | sync | new: R4 |
| `crates/sync/src/ids.rs` | sync | new: R5 |
| `crates/sync/src/pending.rs` | sync | new: R6 |
| `crates/sync/src/merge.rs` | sync | new: R7 |
| `crates/sync/src/kani_proofs.rs` | sync | new: R9 |
| `crates/sync/tests/codec.rs`, `clock.rs`, `ids.rs`, `pending.rs`, `field_merge.rs`, `op_census.rs`, `crate_shape.rs` | sync tests | new: A01, A02, A04 to A08, A10 |
| `crates/sync/tests/formal_vectors_{sync_clock,review_key,field_merge,id_slot}.rs` | sync tests | new: A02, A03, A05, A07 |
| `formal/lean/Formal/IdSlot.lean`, `formal/lean/Formal/IdSlotVectors.lean` | formal | new: R11 |
| `formal/lean/Formal/Vectors.lean` | formal | changed: one import and one arm |
| `formal/lean/Formal/SyncVectors.lean` | formal | changed: the header names the module's first covered item |
| `formal/lean/Formal/SyncClock.lean`, `ReviewKey.lean`, `FieldMerge.lean` | formal | changed: `covers` lines and their prose |
| `formal/vectors/id-slot.jsonl` | formal | new: written by the writer |
| `formal/vectors/sync-clock.jsonl`, `review-key.jsonl`, `field-merge.jsonl` | formal | changed: the header line, written by the writer |
| `formal/obligations.json`, `scripts/tests/test_formal_obligations.py` | formal | changed: R11 |
| `Cargo.toml` | workspace | changed: the crate's workspace dependency entry; `cfg(kani)` declared to the lints |
| `Cargo.lock` | workspace | changed: the crate's package entry |
| `.github/workflows/ci.yml` | CI | changed: the `web-engine` job builds the crate for both clients' targets |
| `scripts/mutation-rows.d/S50900-S50999.json` | mutation rows | new: six hand rows (section 9) |
| `docs/CONTEXT-MAP.md`, `docs/schematics/context-map.md`, `docs/LEXICON.md` | docs | changed: R12 |
| `docs/schematics/the-sync-core-and-a-pending-operations-life.md` | docs | new |
| `docs/specs/SPEC-509-...md`, `docs/decisions/ADR-527-...md`, `docs/red-first/SPEC-509.md` | docs | new |
| `changelog.d/sync-core-509.md` | changelog | new |

## 5. What this does NOT do

- No server judge, log, push, pull or snapshot route, and no store: the server's operation log owns them (#15). The
  push judge's Lean entry and the op log model gain their covers there, where the judge, the writer's commit and the
  device's pull and verdict handlers run together.
- No reschedule operation: a due change no answer made stays out of the vocabulary until a ruling admits it. The
  identity of equal bodies (R8) is held over every kind the vocabulary has; its reschedule instance (the design's
  T2-6) is held with the kind, not applicable to this delivery, and owed by the change that admits it.
- No clock skew refusal: the bound on how far an operation's clock may run past the server's is the judge's (#15).
- No client links the crate yet: the umbrella FFI crate's and the web engine's edges are drawn by the change that
  first uses it (#15).
- No Kani run: the harnesses are written and their witness features declared; they run through the formal check's
  Kani kind where it is installed (section 7).

## 6. Risks

- **A frame that decodes to something else.** Detected by A01's exhaustive round trip and its truncations, and A06.
- **A clock, a key or an id that drifts from its proof.** Each is read against the Lean port's vectors byte for byte
  (A02, A03, A05, A07), and each cover goes stale on any change to its span.
- **A never-list class entering the vocabulary.** Detected by A08's census, whose planted variants must each be refused
  by name.
- **The crate gaining a dependency or a reader of the outside world.** Detected by A10's census and its planted
  manifest and sources.
- **A census that reads nothing.** Each census prints its examined count and refuses zero.

## 7. What only CI and the Kani kind prove

The crate's builds for `wasm32-unknown-unknown` and `aarch64-apple-ios` (`web-engine`, A10), the whole workspace's
clippy and tests (`rust`), the release profile's tests (`release`), and the mutation jobs over the new files
(`mutation-rust`), each read by name at the head. A09 is recorded as not run: the four harnesses verify, and each
witness fails, only where the formal check's Kani kind is installed.

## 8. Formal model

- **Lean: IdSlot (new) and SyncClock, ReviewKey, FieldMerge (stamped).** Each theorem's witness refutes it of its named
  defect; each theorem's planted control (the claim stated of the defect, given the real proof) is refused at its own
  line, and `#print axioms` lists at most `propext`, `Classical.choice` and `Quot.sound` (the red-first record).
- **TLA+: no change.** The op log model gains its covers with the server's log (section 5).
- **Kani: four harnesses, planned in the ledger until they run** (A09).

## 9. Mutation rows

Six hand rows in `scripts/mutation-rows.d/S50900-S50999.json`, for the bounds cargo-mutants never rewrites, a constant
or a literal: the frame bound (S50900), the hour (S50901), the wire version (S50902), the ease's top (S50903), the
review kind's top (S50904) and the slot count (S50905). Each names the one test that kills it. Everything else in the
crate is cargo-mutants' to generate and the crate's own tests' to kill; each witness branch is behind a feature no
test build enables and holds no operator the tool rewrites.
