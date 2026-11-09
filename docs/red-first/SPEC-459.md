# Red-first record: SPEC-459

The order of work: SPEC-459, ADR-476 and the schematic (8aecefcd55cc890368899979a077e0454db82e16);
the two new tests A1 and A2 in `crates/ingest/tests/skip_write.rs`, written against the base helpers
and committed alone (1a54517cdd47eab7280807ae95487f9ce232f07d); R1 in
`crates/ingest/tests/support/synthetic.rs` and R2 in `crates/ingest/tests/support/mod.rs`
(d7847ea71905ba2e5236b2637e33f79e6fa5ff47); the band's rows
(ef9a9e4752a5747a026f5f7b3c69ec8bb6e8a15d); this record and the changelog fragment.

Each criterion is decided by SPEC-459's own fenced command, each selecting one test
(`running 1 test`):

```text
A1: cargo test -p deck-streak-ingest --test skip_write -- --exact a_change_on_another_client_stamps_its_copy_one_past_the_stamp_it_found
A2: cargo test -p deck-streak-ingest --test skip_write -- --exact an_other_clients_sync_that_carries_no_change_is_refused_by_name
A3: cargo test -p deck-streak-ingest --test skip_write -- --exact a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows
A3: cargo test -p deck-streak-ingest --test skip_write -- --exact a_card_that_changed_between_the_preview_and_the_converge_is_left_alone
```

At the red commit A1 and A2 each failed by its own assertion when run alone, and the whole suite of
`deck-streak-ingest` failed those two tests and no other: `skip_write` read 28 passed and 2 failed,
and every other binary read ok. A1 printed `examined 2 stamp(s)` and then failed its exact-value
assertion: the stamp written was the clock's whole second times 1000, not one past the stamp it
found. A2 failed its refusal assertion: at the base the helper returns, so `catch_unwind` answered
`Ok` and the payload read empty. A1's quote below stops before the stamp it prints, which the test
reads from the clock and from its copy, so it differs on every run.

```red-first
A1: red at 1a54517cdd47eab7280807ae95487f9ce232f07d: assertion `left == right` failed: the other client's change stamps its copy one past the stamp
A2: red at 1a54517cdd47eab7280807ae95487f9ce232f07d: a sync that left the other client's stamp at the server's is refused by name (SPEC-459 R2): ""
A3: not red: the two take tests pass at the base and after; they are the subject made deterministic, guarded byte for byte by R3
A1: green at d7847ea71905ba2e5236b2637e33f79e6fa5ff47
A2: green at d7847ea71905ba2e5236b2637e33f79e6fa5ff47
```

Test edits after the red commit, each named here:
- The green commit, d7847ea71905ba2e5236b2637e33f79e6fa5ff47, changes the two helpers the tests
  call, `synthetic::change_cards` (R1) and `support::sync_on_another_client` (R2), and no line of
  `crates/ingest/tests/skip_write.rs`: the assertions of A1 and A2, and every line of the two take
  tests, stand as the red commit holds them.

At the green commit A1, A2 and the two A3 tests each passed alone, and the whole suite read every
binary ok, `skip_write` 30 passed. A2's green is the measurement ADR-476 D1 inferred: with the other
client's stamp set to the server's after its change, the refusal fired, and the recording layer saw
that sync's one `meta` request and no `finish`.
