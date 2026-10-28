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
  test of C1 to C10 changed after 5f886f10.
- **The privacy category is deferred to its writer.** `fix(privacy): defer the companion-state category to its writer`, the commit after 4f2a143a, restores `privacy.json` and `PRIVACY.md` to their bytes at `d3b09d5e` (at 5f886f10 the pack's `erase-complete` row read one finding the base lacks, for `devicestorage:companion-state`); at it the privacy pack's 15 rows read the base's finding counts, `defaults-private` 1, `public-scrub` 10 and each other row 0, with `inventory-valid` and `erase-complete` exit 0, and `test_privacy_policy.py` reads 8 tests, OK.
- **Not run here:** the whole workspace's tests, clippy and release profile, the documentation tests of other crates,
  and the mutation jobs, which only CI proves (SPEC-501 section 7).
