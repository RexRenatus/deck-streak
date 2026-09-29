# Red-first record: SPEC-132

The SPEC was moved into `docs/specs/` alone (36326aa). The tests of A1 to A15 came next (fbfc7b5) against
inert stubs: the photo and share calls exist and refuse, so every test compiles and runs, and each
red fails by assertion, never by a compile error. The implementation turned them green (caa7cca). The red
lines were read from the working tree that became the red commit.

```red-first
A1: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: Withheld { reason: PhotoUnsupported }
A1: green at caa7cca8e7dc
A2: red at fbfc7b57eb9b: assertion `left == right` failed: left: Failed right: Unsupported
A2: green at caa7cca8e7dc
A3: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: Withheld { reason: PhotoUnsupported }
A3: green at caa7cca8e7dc
A4: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: NotNow { reason: QuietHours }
A4: green at caa7cca8e7dc
A5: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: NotNow { reason: SendFailed }
A5: green at caa7cca8e7dc
A6: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: Sent { file_id: FileId("synthetic-file-id") }
A6: green at caa7cca8e7dc
A7: red at fbfc7b57eb9b: assertion failed: matches!(decision, PhotoDecision::Sent { .. })
A7: green at caa7cca8e7dc
A8: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: Withheld { reason: NudgesDisabled }
A8: green at caa7cca8e7dc
A9: red at fbfc7b57eb9b: assertion `left == right` failed: left: None right: Some(TooLarge)
A9: green at caa7cca8e7dc
A10: red at fbfc7b57eb9b: assertion `left == right` failed: left: Failed right: Ready { id: "synthetic-prepared" }
A10: green at caa7cca8e7dc
A11: red at fbfc7b57eb9b: assertion failed: Reason::ALL.contains(&Reason::PhotoUnsupported)
A11: green at caa7cca8e7dc
A12: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: Sent { file_id: FileId("size-large") }
A12: green at caa7cca8e7dc
A13: red at fbfc7b57eb9b: assertion `left == right` failed: left: Failed right: Ready { id: "prepared-1" }
A13: green at caa7cca8e7dc
A14: red at fbfc7b57eb9b: assertion `left == right` failed: left: Withheld { reason: NoNotifier } right: NotNow { reason: SendFailed }
A14: green at caa7cca8e7dc
A15: red at fbfc7b57eb9b: assertion `left == right` failed: left: [push_message, push_reveal, push_dice, push_reaction, push_pin] right: [push_message, push_reveal, push_dice, push_reaction, push_pin, push_photo, prepare_share]
A15: green at caa7cca8e7dc
```

## Test edits after the red commit

None of these changed an assertion of A1 to A15; each is disclosed here because it came after
fbfc7b5.

- caa7cca (green) grew `NAMED_SENDS` in `crates/notifications/tests/one_router.rs` from 20 rows to
  24: `OwnerChat::push_photo` and `OwnerChat::prepare_share` in the port, and
  `Transport::send_photo` and `Transport::save_prepared_inline_message` in the transport, the named
  call sites of the two new sends. `the_policy_names_every_bot_call` (A15) did not change.
- c66066d (after green) changed `the_reading_ready_kind_is_a_recorded_deviation` in
  `crates/notifications/tests/policy.rs`: the deviations it expects, still compared exactly, gained
  `withhold.reasons` for ADR-135 beside `kinds.reading_ready` for ADR-041, and it now checks that
  ADR-135 names `withhold.reasons`. The box run's `policy-deviation-has-adr` row asked for it.
- fc0e7c0 (after green) changed A12's oracle in `crates/bot/tests/support/fake_bot_api.rs`: the third
  size of the `sendPhoto` answer went from `size-medium`, 320 by 213, to `size-wide`, 4000 by 20,
  the longest by its sides and the smallest by its area, so a reader that takes the widest size
  fails. The body of `push_photo_is_one_send_photo_to_the_owner` did not change. The same commit
  added `crates/notifications/tests/photo_jpeg.rs`, nine tests of the JPEG size reader.
