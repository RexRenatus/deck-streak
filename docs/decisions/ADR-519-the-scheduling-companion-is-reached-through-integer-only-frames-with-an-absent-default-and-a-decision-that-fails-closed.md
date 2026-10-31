# ADR-519: The scheduling companion is reached through integer-only frames, with an Absent default and a decision that fails closed

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-501 |
| builds on | ADR-002, ADR-029, ADR-512 (none amended) |
| measured at | `d3b09d5e` |

## Context

A scheduling companion is a scheduler outside DeckStreak's public code that answers due dates. The scheduling port
(ADR-512) is a typed trait in the engine core whose second implementation is to reach such a companion (#997). The
owner's builds may carry a companion as a separately built module, a WebAssembly module beside the public one in the
web client's Worker or a substituted crate in the native client's static library, and the public builds carry none.
So the interface must cross a module boundary as bytes, must carry nothing a learner wrote, must let the public build
answer every request with "absent", and must decide, in public code, whether an answer may be written. Today the answer
path writes the engine's own next state (`crates/engine-core/src/answer.rs:149-160`). This ADR decides the interface's
shape, its schema, its frames, the decision, the crates, the proof and the privacy record. Every fact below was read
at `d3b09d5e`.

## D1. A byte-framed interface with a generic name, and what it was chosen against

- **Chosen: one documented schema of integer-only frames, its codec and its decision in a crate of its own,
  `deck-streak-companion`, named for the role ("the scheduling companion"), never for any one companion.** Bytes cross
  any boundary the builds have: a function call in one binary, a call between two WebAssembly modules, a call into a
  substituted crate. One codec on both sides means one layout to document and to prove.
- **Protobuf messages, as the engine's own:** lost, because they carry strings, and a schema that can hold a string can
  hold a note.
- **JSON frames:** lost, because they carry strings too, and have no fixed layout to pin byte for byte.
- **A typed trait with no frames:** lost, because a Rust trait cannot cross between two separately built WebAssembly
  modules; the web build needs a byte boundary between them.
- **A scheduler-link crate with a typed trait over the port:** lost, because a typed trait cannot cross that boundary
  either; the port's typed trait stays inside the engine core, and its later implementation speaks these frames (#997).
- **Falling back to FSRS when the companion refuses:** lost, because a due date the companion refused must not be
  written by another scheduler; a fallback would make "absent" and "present" indistinguishable to the learner and to
  the proofs.

## D2. Integers only, held by a census

- **Chosen: every field is an integer of a fixed width, one of three closed enums, a fixed array or a `Vec` of them,
  or a type the schema file defines; byte arrays and byte vectors are admitted only as the companion's own checkpoint
  (`CheckpointReply::bytes` and `Restore`'s field).** What never crosses: note fields, tags, deck names and paths, guids,
  media, notetype names, the per-review client record, the Telegram id, sync credentials and the collection file. A
  census reads `wire.rs`'s text, refuses any other type by name, and pins the field count (49), so a field added in any
  form is read; its planted `String`, `f32`, byte `Vec` and `usize` fields must each be refused.
- **`usize` and `isize`:** refused, because their width is the target's, not the schema's.
- **`Restore`'s one field is private, and its one public constructor is `From<CheckpointReply>`**, so the app hands
  back only bytes the companion gave it; a `compile_fail` doctest holds it. The decoder builds a `Restore` from a
  frame's bytes through a crate-private door: it is the companion's side of the codec.

## D3. The frames, and the requests

- **Chosen: `len u32 | major u16 | minor u16 | kind u8 | epoch u32 | body`, little-endian, fields in declared order,
  no padding, `len` counting the whole frame; a `Vec` is a `u32` count and its items; frames at most 1 MiB and a `Fold`
  at most 4,096 events; requests take kinds 1 to 9 and replies 0x81 to 0x86, so a reply's byte never decodes as a
  request.** `epoch` is the app's session counter, raised at every `Hello`, so a reply from an earlier session is stale.
- **The decoder refuses by name**: over the cap, truncated, a `len` mismatch, another major, an unknown kind, a count
  over its bound, an enum value outside its type, trailing bytes. Another minor is admitted. Every read is
  bounds-checked, so no frame panics it. The encoder refuses a `Fold` over its bound and a frame over the cap, so every
  message it encodes decodes to itself.
- **Chosen: `Preview` and `Commit`, with no abort.** `Preview` asks the due date a press would get and changes no
  state; the app writes; `Commit` then carries the review read back from the app's log. A crash between the write and
  the commit leaves no staged state behind, and the next catch-up folds the written row.
- **A staged event with an abort:** lost, because a stage held in the companion across a write is state a crash can
  strand, and the abort exists only to clear it.
- **The layout's proof:** the committed frames, one per message kind, are written from the layout by an encoder that
  is not the crate's, so an encoder and a decoder that agree with each other but not with this document fail.

## D4. The decision, and the handshake

- **Chosen: `decide` is the one function whose `Ok` lets a due date be written.** It is `Ok(NewDue)` only when the
  session negotiated a model, the reply is a `Due` with status `Ok`, its epoch is the session's, its card and its
  previewed review's id are the press's, its model is the negotiated one, its interval is 1 to 36,500 days, and its due
  day is the press's study day plus the interval by checked `i32` arithmetic. Anything else is the first failing
  condition's named `Refusal`, twelve in all with `Malformed` from `decide_frame` and the handshake's three. A reply
  that is not a `Due` refuses by its status, `WrongStage` when its status is `Ok`.
- **The caller's rule:** a `Refusal` means nothing is written and no other scheduler is asked.
- **Chosen: `negotiate` admits only status `Ok`, major 1 and a trained model**, in that order, and a minor difference.
  `Session::new` keeps the model only when it is `Ok`, so a failed handshake leaves every later decision refusing
  `NotNegotiated`.
- **A minor-only comparison, or a handshake that never reads the model:** lost; each is planted against the 42-hello
  cross product and against the Lean claim, and refused by both.

## D5. Two crates, the Absent default, and the context map

- **Chosen: `deck-streak-companion` (the interface: schema, codec, port, decision) is `no_std` with `alloc` and depends
  on nothing; `deck-streak-companion-slot` is the one substitutable body, whose public body's `open` gives `Absent`,
  and depends on the interface alone.** A build that carries a companion replaces the slot crate whole, with the same
  name and signature; nothing else changes. `Absent` answers `Refused { status: Absent }` to every request, so a public
  build writes no due date from a companion.
- **The context map is amended in this change** with the two contexts and the slot's one edge. The engine core's,
  the umbrella FFI crate's and the web engine's edges to them are drawn by the change that first uses each (#997, #998),
  as ADR-002 requires: a declared edge with no import would fail the probe.
- **One crate holding the interface and the body:** lost, because substituting the body would then replace the decision
  too, and the decision must stay public.

## D6. The formal entry

- **Chosen: `lean/CompanionDecide`, a Lean port of `decide` and `negotiate` with three theorems at `ramp=report`**
  (admits exactly when every condition holds; names the session's and the status's refusals; the handshake admits
  exactly a present trained companion at major 1), **four witnesses** (no epoch check, no session check, a minor-only
  and a mode-blind handshake), and **vectors the shipped functions answer** in `crates/companion/tests/
  formal_vectors_decide.rs`. A model id is compared only, so the port reads it as one number; the checked `i32` sum is
  the port's `Int` sum, since a due day outside `i32` cannot equal it. The ledger records the entry present under `O6`,
  with its vectors tie.
- **Kani harnesses on the decoder and the decision:** planned, not built: the formal checker has no Kani kind yet
  (SPEC-496 section 5), and a harness no tool runs proves nothing.

## D7. The companion's state in the privacy inventory

- **Chosen: no category in this change; the change that first writes the companion's state to a device declares the
  category `companion-state` with its erase, in the same change.** This change ships the interface and the `Absent`
  slot, and nothing in it writes the checkpoint, or any companion state, to a device. When the category lands, its
  store is `devicestorage:companion-state`, the privacy pack's one store kind for a client's own device; its export is
  exempt, because the app recomputes it from the review log; and it is erased by the device erase routine, on the
  device's own erase and on `Reset` (#997).
- **The category now, with the interface that names the checkpoint:** lost, because no code writes the store and no
  erase reaches it, so the inventory would declare a store with no erase, which is worse than none; the pack's
  `erase-complete` row read one finding for it.

## Consequences

- Good, because every byte that crosses is an integer the census can read, and the decision that writes a due date is
  public, proved and tied to vectors.
- Good, because a public build has a whole, testable answer path with no companion: `Absent`, refused, nothing written.
- Bad, because a second codec for the same messages (the companion's own, in another build) must match this one; the
  committed frames are the contract it is read against.
- Bad, because the answer path still writes the engine's next state until the engine core's delivery uses the port
  (#997).
- Neutral, because the privacy inventory names no `companion-state` category yet. The category is not declared by this
  change, because nothing in it writes companion state; the change that first writes companion state to a device
  declares the category with its erase in the same change (D7), so the inventory and the erase land together, and
  every row of the privacy pack reads as it did before this change.

### Confirmation

The acceptance fence of SPEC-501: the two crates' tests, the formal check of `lean/CompanionDecide`, the ddd probe's
`context-map-parses` and `declared-edges-match-imports`, and the privacy pack's `inventory-valid` and `erase-complete`.
