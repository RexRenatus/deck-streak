# Red-first record: SPEC-501

The SPEC, ADR-519 and the interface's schematic were committed first (6087e820). The two crates' tests, the Lean entry
with its vectors and the ledger's row came next, whole, over a stub crate that keeps every shape but the behaviour
(ae64a4ed): its schema carries a planted `NoteText { text: String }` and a public checkpoint field, its codec writes
headers only and decodes every frame to one message, its handshake and decision admit everything, and its `Absent`
answers a due date. 5f886f10 turns them green: the schema, the codec, the handshake and the decision are written, the
Lean entry is re-stamped at the shipped `decide` and `negotiate`, and the map, its schematic, the inventory and the
policy carry the two crates and the device category. Each Rust red below is quoted from one run of the two crates'
every target at ae64a4ed (`cargo test --locked --offline --no-fail-fast -p deck-streak-companion -p
deck-streak-companion-slot`: 14 failed, 7 passed), each green from the same run at 5f886f10 (21 passed, 0 failed).

```red-first
C1: red at ae64a4ed: crates/companion/tests/wire_integers.rs:267: assertion `left == right` failed: fields off the allow-list (left: ["NoteText.text: String"], right: []); wire_integers.rs:292: the planted field is read: pub note_text: String, (left: 51, right: 50)
C1: green at 5f886f10
C2: red at ae64a4ed: crates/companion/tests/codec.rs:204: assertion `left == right` failed: StateHash (left: Ok(Frame { major: 1, minor: 0, epoch: 0, message: Purge }), right: Ok(Frame { major: 1, minor: 0, epoch: 859013184, message: StateHash })); codec.rs:514 (left: ["request-hello: encoded 0d000000010000000107000000 != committed 1700000001000000010700000001000000000003000000", ...], right: [])
C2: green at 5f886f10
C3: red at ae64a4ed: crates/companion/tests/codec.rs:600: assertion `left == right` failed (left: ["trailing bytes: Ok(Purge), not TrailingBytes", "an unknown kind: Ok(Purge), not UnknownKind", "a count over its bound: Ok(Purge), not CountOverBound", "a len mismatch: Ok(Purge), not LenMismatch", "a frame over 1 MiB: Ok(Purge), not OverCap"], right: [])
C3: green at 5f886f10
C4: red at ae64a4ed: crates/companion/tests/decide.rs:203: assertion `left == right` failed (left: ["malformed: Ok(NewDue { card: 42, interval_days: 1, due_day: 101 }), not Err(Malformed)", "not negotiated: Ok(NewDue { card: 42, interval_days: 1, due_day: 101 }), not Err(NotNegotiated)", ...], right: [])
C4: green at 5f886f10
C5: red at ae64a4ed: crates/companion/tests/decide.rs:328: assertion `left == right` failed (left: ["Ok 0 Trained: Ok([5, 6, 7, 8]), not Err(Mismatch)", "Ok 0 Trained: admitted wrongly", ...], right: [])
C5: green at 5f886f10
C6: red at ae64a4ed: crates/companion-slot/tests/absent.rs:106: assertion `left == right` failed (left: ["Hello { major: 1, minor: 0, release: [0, 3, 0] }: Due { status: Ok, card: 0, staged_t_ms: 0, interval_days: 1, due_day: 1, model_id: [0, 0, 0, 0] }", ...], right: [])
C6: green at 5f886f10
C7: red at ae64a4ed: crates/companion/src/wire.rs - wire::Restore (line 256) - compile fail ... FAILED: Test compiled successfully, but it's marked `compile_fail`.
C7: green at 5f886f10
C8: red at ae64a4ed: crates/companion/tests/formal_vectors_decide.rs:211: assertion `left == right` failed: {"card":42,"due":null,"due_day":103,"interval_days":3,"model":5,"outcome":"not_negotiated",...} (left: Ok(NewDue { card: 42, interval_days: 1, due_day: 101 }), right: Err("not_negotiated"))
C8: green at 5f886f10
C9: red at ae64a4ed: test_context_map_schematic.py: test_every_arrow_is_a_manifest_edge_the_fence_names: AssertionError: findings: 3; test_every_crate_directory_is_one_node_named_by_it: AssertionError: findings: 2
C9: green at 5f886f10
C10: not red: the criterion holds at the base and pins that this delivery declares no device store it does not write; 5f886f10 declared `companion-state` and the commit after 4f2a143a withdrew it
```

## What each pair disclosed

- **The planted controls are independent of the crate.** `a_field_order_swap_in_the_encoder_breaks_the_vectors`,
  `an_accept_all_decoder_passes_none_of_the_malformed_frames`, `an_accept_all_decide_is_refused_by_the_reason_table`,
  `a_minor_only_and_a_mode_blind_negotiate_are_each_refused`, `a_slot_that_answers_a_due_date_is_refused`,
  `random_bytes_never_panic_the_decoder` and the passing doctest read the committed frames, hand-written frames and
  planted functions only, so they passed at ae64a4ed as at 5f886f10: they prove their enumerations can fail, not the
  crate.
- **The other red tests were red for the stub's reason too:** `a_frame_of_another_minor_decodes_and_keeps_its_minor`,
  `an_enum_value_outside_its_type_and_another_major_are_refused`, `the_interval_bounds_and_the_due_days_arithmetic_are_exact`
  and `a_reply_that_is_not_a_due_date_refuses_by_its_status` each failed on an assertion over the stub's answer.
- **The hub's tools, at the same commits.** The ddd probe's `declared-edges-match-imports` read 2 findings at ae64a4ed
  ("workspace member deck-streak-companion (crates/companion) belongs to no context", and the same for
  `deck-streak-companion-slot`) and `context-map-parses` read 36 contexts and 100 edges; on the map of 5f886f10 both
  read OK, 38 contexts and 101 edges. The privacy pack's `inventory-valid` read OK at both. The formal check of the Lean
  kind read `FORMAL OK exit=0` at 5f886f10, its three `lean/CompanionDecide` theorems `clean: true` at `ramp=report`
  with their four witnesses renamed onto the wrong ports, and no finding.
- **Where C9's red was read.** On the tree of the test commit before a clippy-only amend of
  `crates/companion/tests/codec.rs`, which is the only file in which it differs from ae64a4ed; the census does not read
  it.
- **One fence test changed after the code.** f39e2604 makes C5's planted control name the hellos each wrong handshake
  admits (major 0 and 2 for the minor-only one, a fixture at major 1 for the mode-blind one), in place of asserting
  only that some finding exists. It reads planted functions only, so its outcome at ae64a4ed is unchanged; no criterion
  test of C1 to C10 changed after 5f886f10 except C5's planted control above, which reads planted functions only.
- **The privacy category is deferred to its writer.** `fix(privacy): defer the companion-state category to its writer`, the commit after 4f2a143a, restores `privacy.json` and `PRIVACY.md` to their bytes at `d3b09d5e` (at 5f886f10 the pack's `erase-complete` row read one finding the base lacks, for `devicestorage:companion-state`); at it the privacy pack's 15 rows read the base's finding counts, `defaults-private` 1, `public-scrub` 10 and each other row 0, with `inventory-valid` and `erase-complete` exit 0, and `test_privacy_policy.py` reads 8 tests, OK.
- **Not run here:** the whole workspace's tests, clippy and release profile, the documentation tests of other crates,
  and the mutation jobs, which only CI proves (SPEC-501 section 7).

## Fix 1: the edges a mutation run found untested

The verifier's 155 hand mutants of the two crates' source, run at 66b57edd against the two crates' every target, left 28
alive: each a change no test could see, because a cap, a bound or a layout length was read through the crate's own
constant, an order or an epoch was never crossed, a reply's own status was never varied, or a property (`Restore`'s
doors, the roots' `no_std`) had no test at all. 61f7aa83 adds tests that write SPEC-501's numbers out (1,048,576 bytes,
4,096 events, a 13-byte header, a 74-byte event, the status before the epoch) or read the source, beside the criterion
tests in the same files. No line of either crate's source changed, and no criterion test of C1 to C10 changed. Each red
below is the new test's failure with that mutant applied to 61f7aa83, quoted from one run per mutant of `cargo test
--locked --offline --no-fail-fast -p deck-streak-companion -p deck-streak-companion-slot` (28 runs, each exit 101); the
green is the same run unmutated at 61f7aa83, 40 passed and 0 failed.

- **R4, the epoch.** D08 (`!=` read as `<`, so a later epoch is admitted):
  `a_reply_of_a_later_epoch_is_stale_like_an_earlier_one` red: `every other epoch is stale` (left: ["epoch 8:
  Ok(NewDue { card: 42, interval_days: 3, due_day: 103 })", ...], right: []).
- **R4's order.** D17 (the epoch checked before the status): `a_stale_reply_refuses_by_its_status_first` red: `epoch 6:
  the status is named before the epoch` (left: Err(StaleEpoch), right: Err(Status(FixtureRefuses))).
- **R5, the session's epoch.** H10 (`Session::epoch` answers 0): `a_session_keeps_the_epoch_it_was_opened_with` red
  (left: (0, 0), right: (1, 1)).
- **R4, a reply's own status.** W13 and W14 (a `Hello` or a `Checkpoint` reply's status read as `Ok`):
  `every_reply_carries_its_own_status_and_one_that_is_not_a_due_refuses_by_it` red (left: ["Hello(HelloReply { status:
  Absent, ... }): status() is Ok", ...] and ["Checkpoint(CheckpointReply { status: Absent, ... }): status() is Ok",
  ...], right: []).
- **R2, a frame at the cap.** K01 (the decoder refuses 1,048,576 bytes), K33 (the encoder does) and W05 (the cap
  1,048,575): `a_frame_of_exactly_1_048_576_bytes_encodes_and_decodes_on_both_sides` red: K01 `the decoder reads a
  request frame at the cap` (left: Err(OverCap), right: Ok(true)); K33 and W05 `the encoder writes a request frame at
  the cap` (left: Err(OverCap), right: Ok(1048576)).
- **R2, a frame one byte over.** K34 (no encoder cap) and W04 (the cap 2 MiB):
  `a_frame_of_1_048_577_bytes_is_refused_by_the_encoder_and_the_decoder` red: `the encoder refuses a request frame one
  byte over the cap` (left: Ok(1048577), right: Err(OverCap)).
- **R2, a `Fold` at the bound.** K11 (the decoder refuses 4,096 events), K13 (the encoder does) and W07 (the bound
  4,095): `a_fold_of_exactly_4_096_events_encodes_and_decodes` red: K11 `the decoder reads a fold of 4,096 events`
  (left: Err(CountOverBound), right: Ok(true)); K13 and W07 `the encoder writes a fold of 4,096 events` (left:
  Err(CountOverBound), right: Ok(303121)).
- **R2, a `Fold` one over.** K14 (no encoder bound) and W06 (the bound 4,097):
  `a_fold_of_4_097_events_is_refused_by_the_encoder_and_the_decoder` red: `the encoder refuses a fold of 4,097 events`
  (left: Ok(303195), right: Err(CountOverBound)).
- **R3, `len`.** K04 (a `len` shorter than the frame admitted): `a_len_shorter_or_longer_than_the_frame_is_a_mismatch`
  red (left: ["reply, len 0: Ok(Refused { status: Absent })", "request, len 0: Ok(Purge)", ...], right: []).
- **R3, the major.** K05 (major 0 admitted): `a_major_below_or_above_1_is_refused` red (left: ["major 0: (Ok(Refused {
  status: Absent }), Ok(Purge))"], right: []).
- **R3's order.** K07 (the major checked before `len`): `the_decoder_names_a_len_mismatch_before_another_major` red:
  `reply, major 0, len 0` (left: Err(Major), right: Err(LenMismatch)).
- **R3, enum bytes.** K20 and K22 (a model or an adapter byte over 1 read as its last value):
  `every_model_and_adapter_byte_over_1_is_refused` red (left: ["hello model 2: Ok(Hello(HelloReply { ... model:
  Fixture, ... }))", ...] and ["hello adapter 2: Ok(Hello(HelloReply { ... adapter: OwnerOnly }))", "checkpoint adapter
  2: ...", ...], right: []).
- **R3, bytes after a reply's body.** K25: `the_reply_decoder_refuses_a_byte_after_the_body_of_every_kind` red (left:
  ["0x81 with a trailing byte: Ok(Hello(...))", "0x82 with a trailing byte: Ok(Ack { ... })", ...], right: []).
- **R3, an unknown reply kind.** K26 (read as `Refused { Absent }`):
  `every_kind_byte_outside_its_range_is_refused_as_unknown` red (left: ["reply kind 0x0, body []: Ok(Refused { status:
  Absent })", ...], right: []).
- **R1 and R2, the published constants.** W03 (a 12-byte header) and W09 (a 75-byte event), and W04 to W07 as well:
  `the_published_layout_constants_are_the_specs_numbers` red: W03 (left: (12, 74, 1048576, 4096, 1, 0), right: (13, 74,
  1048576, 4096, 1, 0)); W09 (left: (13, 75, 1048576, 4096, 1, 0)).
- **R7, `Restore`'s doors.** W19 (`decoded` public) and W20 (a `From<Vec<u8>>`):
  `restore_is_built_only_from_a_checkpoint_reply_or_by_the_decoder` red: `doors to Restore` (left: ["wire.rs:
  Restore::decoded is pub"] and ["wire.rs: impl From<Vec<u8>> for Restore"], right: []).
- **ADR-519 D5, `no_std`.** L01 and S03 (the interface crate's or the slot's root without `#![no_std]`):
  `the_interface_crate_is_no_std_and_a_root_without_it_is_refused` red: `the interface crate's root declares
  #![no_std]`; `the_slot_is_no_std_and_a_root_without_it_is_refused` red: `the slot's root declares #![no_std]`.

What the pairs disclosed:

- **The censuses have planted controls.** `a_planted_public_door_to_restore_is_refused` plants a public `decoded`, a
  `From<Vec<u8>>`, a public field and a second caller of the decoder's door, and the two `no_std` tests plant a root
  without the attribute, one with it in a comment and one that links `std`; each plant is refused by name, so each
  census can fail. With W19 or W20 applied the door control fails too, because its anchors are the committed lines.
- **R7's text now says what holds.** Raw bytes do build a `Restore` through `decode_request`, the companion's side of
  the codec, by design (ADR-519 D2); R7 names that door and the census that holds both.
