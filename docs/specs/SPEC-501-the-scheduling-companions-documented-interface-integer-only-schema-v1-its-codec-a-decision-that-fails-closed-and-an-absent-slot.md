# SPEC-501: The scheduling companion's documented interface: integer-only schema v1, its codec, a decision that fails closed, and an Absent slot

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #997 (the second implementation behind the scheduling port, which this interface serves) |
| decided by | ADR-519, under ADR-002, ADR-029 and ADR-512 |
| measured at | `d3b09d5e` (every file this SPEC cites is byte-equal there and at the delivery's base) |
| schematic | `docs/schematics/the-scheduling-companions-documented-interface.md` (new); `docs/schematics/context-map.md` (changed: two nodes and one arrow) |
| mutation band | S50100-S50199 (no hand row: section 9) |

## 1. The problem, measured

A scheduling companion is a scheduler that lives beside DeckStreak, outside its public code, and answers due dates.
Before any client can ask one, the two sides need one documented interface: what crosses, how it is framed, who decides
whether an answer may be written, and what a build that carries no companion does. What was read at `d3b09d5e`:

- **No interface exists.** The workspace has 32 crates (`git ls-tree -d --name-only d3b09d5e crates/ | wc -l`), and
  none frames a request to a scheduler outside the engine. The context map's fence names 36 contexts and 100 edges
  (the ddd probe's `context-map-parses`, run at `d3b09d5e`).
- **The answer path writes the engine's own next state.** `crates/engine-core/src/answer.rs:149-160`:
  `answer_request` takes `next: Option<SchedulingState>` and writes it as `new_state: next`. Nothing today could stand a
  companion's due date in its place, and nothing today could refuse to write one.
- **The scheduling port is a typed trait inside the engine core** (SPEC-494, ADR-512): its first implementation is
  FSRS-7, and a later one is to sit behind it with a switch whose default the owner sets (#997). A typed trait cannot
  cross between two separately built modules, such as two WebAssembly modules in one Worker; bytes can.
- **The formal ledger plans this delivery's proofs.** `formal/obligations.json:120` names `lean/CompanionDecide` as a
  planned artifact of `O6` ("Fail closed where the scheduler is absent"), and `:122-123` the Kani harnesses `K-decide`
  and `K-companion-codec`; the Lake package holds 16 entries and no companion module
  (`git ls-tree --name-only d3b09d5e formal/lean/Formal/`).
- **The privacy inventory has no device category.** `privacy.json` names 40 categories, each a set of tables in the
  service's database (`grep -c '"id"' privacy.json`); none names state a client keeps on the device for a scheduler.

## 2. Requirements

R1. **Schema v1, integers only.** `crates/companion/src/wire.rs` defines every type that crosses: the three closed enums
`Status` (7 values), `Model` (2) and `Adapter` (2); `HelloReply`; `ReviewEvent` (11 fields, 74 bytes); `Preview`;
`Request` (`Hello`, `Fold`, `Preview`, `Commit`, `Retract`, `Checkpoint`, `Restore`, `Purge`, `StateHash`); `Reply`
(`Hello`, `Ack`, `Due`, `Checkpoint`, `Hash`, `Refused`); `CheckpointReply`; and `Restore`. Every field is an integer
of a fixed width, a closed enum, a fixed array or a `Vec` of them, or a type the file defines; a byte array or a byte
`Vec` is admitted only as the companion's own checkpoint, `CheckpointReply::bytes` and `Restore`'s field. `MAJOR` is 1
and `MINOR` is 0. `Preview` asks a due date and changes no state; `Commit` carries the review the app wrote, read back
from its log; there is no abort request.

R2. **The codec.** `crates/companion/src/codec.rs` frames a message as `len u32 | major u16 | minor u16 | kind u8 |
epoch u32 | body`, little-endian, fields in declared order, no padding, `len` counting the whole frame; a `Vec` is a
`u32` count then its items. Requests take kinds 1 to 9 and replies 0x81 to 0x86. A frame is at most 1,048,576 bytes and
a `Fold` at most 4,096 events. `decode(encode(x)) == x` for every request and reply, and the frames are byte-equal to
`formal/vectors/companion-frames.jsonl`, which holds one frame per message kind, written from the layout above by an
encoder independent of the crate's.

R3. **The decoder refuses by name.** It refuses a frame over the cap (`OverCap`), shorter than its header or its body
(`Truncated`), whose `len` is not its length (`LenMismatch`), of another major (`Major`), of an unknown kind
(`UnknownKind`), with a count over its bound (`CountOverBound`), with an enum value outside its type (`UnknownEnum`),
or with bytes after its body (`TrailingBytes`), and admits another minor. It never panics.

R4. **The decision fails closed.** `crates/companion/src/decide.rs`: `decide(session, asked, replied_epoch, reply)` is
`Ok(NewDue)` only when the session negotiated a model, the reply is a `Due` with status `Ok`, its epoch is the
session's, its card and its previewed review's id are the press's, its model is the negotiated one, its interval is 1 to
36,500 days, and its due day is the press's study day plus the interval by checked `i32` arithmetic. Anything else is
one named `Refusal`, the first that fails in that order; a reply that is not a `Due` refuses by its status (`Absent`,
`Status`, or `WrongStage` when its status is `Ok`). `decide_frame` decodes, refusing `Malformed`, then decides. A
`Refusal` means nothing is written and no other scheduler is asked.

R5. **The handshake.** `negotiate(hello)` is `Ok(model_id)` only for status `Ok`, major 1 and a trained model, and
refuses `Absent`, `Status`, `Mismatch` and `Fixture`, in that order; a minor difference is admitted. `Session::new`
keeps the model only when `negotiate` is `Ok`, so after a failed handshake every decision refuses `NotNegotiated`.

R6. **The Absent default and the slot.** `crates/companion/src/port.rs` defines the `Companion` trait (`fn call(&mut
self, epoch: u32, request: &Request) -> Reply`) and `Absent`, which answers `Refused { status: Absent }` to every
request. `crates/companion-slot` is the one substitutable body: its `open(weights: &[u8]) -> Box<dyn Companion + Send>`
opens `Absent`, and it depends on the interface crate alone.

R7. **`Restore` holds only the companion's bytes.** `Restore`'s one field is private, its one public constructor is
`From<CheckpointReply>`, and no public function of its own and no `From` of raw bytes builds one. Its one other door is
`decode_request`, by design (ADR-519 D2): the decoder reads a `Restore` frame's bytes through a crate-private function,
because it is the companion's side of the codec. So the app builds a `Restore` only from a `CheckpointReply`, and raw
bytes become one only as a frame the decoder reads; the census
`restore_is_built_only_from_a_checkpoint_reply_or_by_the_decoder` holds both doors.

R8. **The formal entry.** `formal/lean/Formal/CompanionDecide.lean` ports `decide` and `negotiate` and proves, at
`ramp=report`: `decide` is `ok d` exactly when every condition of R4 holds; it names the session's and the status's
refusals; `negotiate` is `ok m` exactly for an ok status, major 1 and a trained model. Each theorem has a witness that
refutes it over a wrong port. Its vectors, `formal/vectors/companion-decide.jsonl`, are byte-equal to the writer's
output, and `crates/companion/tests/formal_vectors_decide.rs` answers every one with the shipped functions. The formal
ledger records the entry present under `O6`, with its vectors tie.

R9. **The map carries the two contexts.** The fence gains `deck-streak-companion ... depends on: nothing` and
`deck-streak-companion-slot ... depends on: companion`; the schematic draws both crates and `companion_slot -->
companion`, pointing down; the ddd probe's `context-map-parses` and `declared-edges-match-imports` read green.

R10. **The privacy inventory declares no companion state yet.** The category `companion-state` is not declared by this
delivery, because nothing in it writes the companion's checkpoint, or any of its state, to a device: it ships the
interface and the `Absent` slot only, and a declared store that no code writes and no erase reaches would be worse
than none. The change that first writes companion state to a device declares the category, with its erase, in the same
change. So `privacy.json` and `PRIVACY.md` are unchanged, and every row of the privacy pack reads as it did at
`d3b09d5e`, its `inventory-valid` and `erase-complete` rows green.

R11. **Unchanged.** Every other crate, the engine core's answer path, every client, every other formal entry and vector,
every migration, every mutation row, the privacy inventory and the policy. The only lines changed outside the two crates
and the new formal files are the workspace's two dependency entries, the lock's two packages, `Formal/Vectors.lean`'s
import and arm, the ledger's `O6` artifact, tie and row and its test's constants, the map and the schematic.

## 3. Acceptance criteria of SPEC-501

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| C1 | R1: every field of every type in `wire.rs` is on the allow-list, 49 examined, and a planted `String`, `f32`, byte `Vec` and `usize` field are each refused by name | the stub schema carries a planted `String` field | ADR-519 D2 |
| C2 | R2: 6,000 generated messages over all 15 kinds decode to themselves, the 15 committed frames are byte-equal, and an encoder with two fields swapped breaks exactly the frame it touches | the stub codec writes headers only and decodes every frame to one message | ADR-519 D3 |
| C3 | R3: the five malformed frames (trailing bytes, an unknown kind, a count over its bound, a `len` mismatch, a frame over 1 MiB) are each refused by name, 5 of 5, and an accept-all decoder passes none | the stub decoder admits every frame | ADR-519 D3 |
| C4 | R4: `decide_frame` refuses each of its ten reasons and admits the one valid case, 11 of 11, and an accept-all decision is refused by the same table | the stub decision admits every reply | ADR-519 D4 |
| C5 | R5: `negotiate` answers all 42 hellos (7 statuses, majors 0 to 2, both models) as R5 says, and a minor-only and a mode-blind handshake are each refused | the stub handshake admits every hello | ADR-519 D4 |
| C6 | R6: the public slot answers `Refused { Absent }` to all 9 request kinds, and a slot that answers a due date is refused | the stub default answers a due date | ADR-519 D5 |
| C7 | R7: a `compile_fail` doctest that builds a `Restore` from raw bytes does not compile, and one that builds it from a `CheckpointReply` does | the stub's field is public, so the raw build compiles | ADR-519 D2 |
| C8 | R8: the Lean entry's three theorems are clean at `ramp=report`, its four witnesses refute them, its vectors are byte-equal, and the shipped functions answer all 275 vectors | the stub decision and handshake answer the vectors otherwise | ADR-519 D6 |
| C9 | R9: the ddd probe's `context-map-parses` and `declared-edges-match-imports` are green with the two new contexts, and the schematic census holds the drawing | the crates exist and the fence and the drawing do not name them | ADR-519 D5 |
| C10 | R10: the inventory and the policy declare no `companion-state` category, the pack's `inventory-valid` and `erase-complete` rows are green with every row at the base's count, and the policy test is green | not red: the criterion holds at the base; it pins that this delivery declares no device store it does not write | ADR-519 D7 |

```acceptance
C1: cargo test -p deck-streak-companion --test wire_integers -- --exact every_field_of_every_wire_type_is_an_integer_a_closed_enum_or_a_named_checkpoint_byte_field
C1: cargo test -p deck-streak-companion --test wire_integers -- --exact a_planted_string_field_and_a_planted_float_field_are_each_refused
C2: cargo test -p deck-streak-companion --test codec -- --exact every_generated_request_and_reply_decodes_to_itself
C2: cargo test -p deck-streak-companion --test codec -- --exact every_frame_is_byte_equal_to_the_committed_vectors
C2: cargo test -p deck-streak-companion --test codec -- --exact a_field_order_swap_in_the_encoder_breaks_the_vectors
C3: cargo test -p deck-streak-companion --test codec -- --exact the_decoder_refuses_the_five_malformed_frames_by_name
C3: cargo test -p deck-streak-companion --test codec -- --exact an_accept_all_decoder_passes_none_of_the_malformed_frames
C4: cargo test -p deck-streak-companion --test decide -- --exact decide_frame_refuses_each_of_its_ten_reasons_and_admits_the_one_valid_case
C4: cargo test -p deck-streak-companion --test decide -- --exact an_accept_all_decide_is_refused_by_the_reason_table
C5: cargo test -p deck-streak-companion --test decide -- --exact negotiate_admits_only_a_present_companion_at_major_one_with_a_trained_model
C5: cargo test -p deck-streak-companion --test decide -- --exact a_minor_only_and_a_mode_blind_negotiate_are_each_refused
C6: cargo test -p deck-streak-companion-slot --test absent -- --exact the_public_slot_refuses_every_request_kind_as_absent
C6: cargo test -p deck-streak-companion-slot --test absent -- --exact a_slot_that_answers_a_due_date_is_refused
C7: cargo test -p deck-streak-companion --doc
C8: cargo test -p deck-streak-companion --test formal_vectors_decide -- --exact decide_and_negotiate_answer_every_lean_vector
C9: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
C10: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py
```

The rest of C8, C9 and C10 is decided by the hub's tools, run from its checkout with `--root` at this tree, as ADR-002's
confirmation line runs the ddd probe: `phxd formal check --root . --base <the delivery's base> --kind lean --gate
--format json` (C8; section 8 says what it must read), `python3 scripts/ddd-probe.py --root . check
context-map-parses` and `check declared-edges-match-imports` (C9), and `python3 scripts/privacy-gdpr-probe.py --root .
check inventory-valid` and `check erase-complete` (C10). The crates' other tests, `a_frame_of_another_minor_decodes_and_keeps_its_minor`,
`random_bytes_never_panic_the_decoder`, `an_enum_value_outside_its_type_and_another_major_are_refused`,
`the_interval_bounds_and_the_due_days_arithmetic_are_exact` and `a_reply_that_is_not_a_due_date_refuses_by_its_status`,
pin the edges of R3 and R4 beside the criteria. `python3 -m unittest discover -s scripts/tests -p
test_formal_obligations.py` holds the ledger's change (R8).

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/companion/Cargo.toml` | companion | new: no dependency; `serde_json` as the vectors reader's dev-dependency (ADR-029) |
| `crates/companion/src/lib.rs` | companion | new: the crate's doc and its four modules, `no_std` with `alloc` |
| `crates/companion/src/wire.rs` | companion | new: schema v1 (R1, R7) |
| `crates/companion/src/codec.rs` | companion | new: the frames and their codec (R2, R3) |
| `crates/companion/src/port.rs` | companion | new: `Companion` and `Absent` (R6) |
| `crates/companion/src/decide.rs` | companion | new: `negotiate`, `Session`, `decide`, `decide_frame` (R4, R5) |
| `crates/companion/tests/wire_integers.rs` | companion tests | new: C1 |
| `crates/companion/tests/codec.rs` | companion tests | new: C2, C3 |
| `crates/companion/tests/decide.rs` | companion tests | new: C4, C5 |
| `crates/companion/tests/formal_vectors_decide.rs` | companion tests | new: C8's tie |
| `crates/companion-slot/Cargo.toml` | slot | new: the interface crate, its one dependency |
| `crates/companion-slot/src/lib.rs` | slot | new: `open`, the public body (R6) |
| `crates/companion-slot/tests/absent.rs` | slot tests | new: C6 |
| `formal/lean/Formal/CompanionDecide.lean` | formal | new: the port, three theorems, four witnesses (R8) |
| `formal/lean/Formal/CompanionDecideVectors.lean` | formal | new: the vector writer |
| `formal/lean/Formal/Vectors.lean` | formal | changed: the writer's import and arm |
| `formal/vectors/companion-decide.jsonl` | formal | new: the writer's output, 275 vectors |
| `formal/vectors/companion-frames.jsonl` | formal | new: the 15 committed frames (R2) |
| `formal/obligations.json` | formal | changed: `O6`'s Lean artifact present with its three theorems, its vectors tie, and the entry's row |
| `scripts/tests/test_formal_obligations.py` | formal tests | changed: the ledger's new entry, tie and present artifacts |
| `Cargo.toml` | workspace | changed: the two crates' workspace dependency entries |
| `Cargo.lock` | workspace | changed: the two crates' packages |
| `docs/CONTEXT-MAP.md` | docs | changed: the two fence rows and their prose |
| `docs/schematics/context-map.md` | docs | changed: the two nodes, their layer and the arrow |
| `docs/schematics/the-scheduling-companions-documented-interface.md` | docs | new: component and data flow |
| `docs/specs/SPEC-501-the-scheduling-companions-documented-interface-integer-only-schema-v1-its-codec-a-decision-that-fails-closed-and-an-absent-slot.md` | docs | new: this file |
| `docs/decisions/ADR-519-the-scheduling-companion-is-reached-through-integer-only-frames-with-an-absent-default-and-a-decision-that-fails-closed.md` | docs | new: the decisions |
| `docs/red-first/SPEC-501.md` | docs | new: the red-first record |
| `changelog.d/scheduling-companion-interface-501.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- The engine core asking the port at the answer: the preview, write and commit steps, the fold of the log, the catch-up
  after a crash, the device's checkpoint file, its load and the device erase routine that deletes it, the privacy
  category `companion-state` declared with that erase, and the answer path writing a due date built from `NewDue` in
  place of the engine's next state (#997).
- The umbrella FFI crate composing the slot, and the web engine carrying frames in the Worker beside a separately built
  module (#998).
- Any body of the slot but `Absent`, and any weights (#997).
- Building the two crates for the clients' targets in CI, which the change that first links them adds (#998).

## 6. Risks

- **A field that can carry text.** Detected by C1's census, which reads every field of `wire.rs` and refuses anything off
  the allow-list by name; its planted fields include a byte `Vec` outside the checkpoint.
- **An encoder and a decoder that agree with each other and not with the documented layout.** Detected by C2's committed
  frames, written from the layout by an encoder that is not the crate's; a swapped pair of fields breaks them.
- **A decision that admits what it should refuse.** Detected by C4's table, C5's cross product, and C8's vectors from an
  independent port whose theorems state the conditions; each enumeration also runs over a planted accept-all or wrong
  function and must refuse it.
- **A census that reads nothing.** Each enumerating test prints its examined count and pins it.
- **A forged checkpoint.** `Restore`'s field is private (C7). The decoder builds one from a frame's bytes: it is the
  companion's side of the codec, and a body reads its own frames.
- **A device store the inventory declares before any code writes it.** A category for the checkpoint would name the
  store `devicestorage:companion-state`, and with no device erase to reach it the privacy pack's `erase-complete` row
  would read a finding for it. Held by R10: no file of this delivery writes companion state to a device, so the
  category waits for the change that writes it and lands with its erase (#997), and every row of the pack reads as it
  did at `d3b09d5e`.

## 7. What only CI proves

The whole workspace's clippy and tests (`rust`), the release profile's tests (`release`), the documentation tests
(`doctest`), and the mutation jobs over the two crates (`mutation-rust`), each read by name at the head.

## 8. Formal model

- **Lean: `lean/CompanionDecide`, new.** The formal check of the entry, and the ratchet over the whole tree, are run by
  the builder at its head, re-run by the verifier, and re-run at land. The check must read no `HOLE`, `MODEL_ERROR`,
  `DERIVED_DRIFT`, `STALE` or `WITNESS_SURVIVED`, `clean` true on each of the three theorems, and each witness built. A
  theorem moves from `ramp=report` to `ramp=block` only by the streak of clean landings the checker counts.
- **TLA+: NOT APPLICABLE here.** Nothing interleaves in this delivery; the answer protocol's model is the engine core's
  delivery's (#997).
- **Kani: planned, not built.** The harnesses `K-decide` (the decision is sound over all integer inputs) and
  `K-companion-codec` (the decoder never panics up to the harness bound, and a decoded frame encodes back to itself)
  wait for the formal checker's Kani kind (SPEC-496 section 5); the ledger keeps both planned under `O6`, and no
  harness is committed that no tool runs.

## 9. Mutation rows

None. The two crates are new code with no hand-held property beyond their tests: cargo-mutants generates their mutants,
and C1 to C8 are their killers. The band S50100-S50199 holds no row.
