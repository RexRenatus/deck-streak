# Red-first record: SPEC-023

The tests were committed first (32e96dd) beside a reader, a window, a gate, a state and an obligations
registry whose public API was in place and whose behaviour was stubbed: the scope allowed no deck,
the study-event rule refused every row, every card was on the language track and scoped by the deck
it sat in, the read returned nothing, the write door reported no change, the window's constants were
0 and its rebase kept whatever it was given, the probe read zeros, `decide` treated every anchor as
missing, the state read an empty row and wrote nothing, the run history was empty, the data-rights
port declared no `ingest_state`, and the registry collected no deadline. The migration, the goldens and
the fixtures were in place, so every test failed by assertion and none by a missing table, fixture or
golden. The implementation followed (73e7b00). Between the red commit and the green one, no test
changed what it asserts: two took lint fixes only (`clone_into` for an assignment of `to_owned`, and
`Duration::from_mins`).

Each criterion was run with the SPEC's own fenced command, and failed for its own criterion's reason.
A9's first run failed inside its fixture (two planned reviews shared one instant, which `revlog.id`
refuses); the fixture gave each review its own instant before the red commit, and A9 was run again at
32e96dd, where it failed by its assertion. Five tests beyond the fence pin R8's order of terms, R10's
deadline window at its boundaries, R8's unreadable anchor, R3's prefix, and the settings' parse.

Two tests were added after the merge commit introduced the wiring (c6dde16, which built the `sync`
job's reader, change gate and obligations in `crates/daemon/src/role_job.rs`); proved by mutation.
They could not be red first, so each was proved on the committed tree (67eec68) by hand mutants of
`role_job.rs`, each run against its one killing test with `--exact` and the file restored byte for byte
(sha256 5f4f934a1d6c before and after every mutant, the tree clean):

- `role_job::tests::a_cycle_that_cannot_finish_is_recorded_with_its_steps_reason_code` (the binary's
  unit test: `cycle_failed` gives every kind of `CycleError` its exact reason code).
  - M1, `CycleError::Obligations(_)` mapped to `recompute_failed`: killed, `Obligations(LoggingInstalled)`;
    left `Reason { code: "recompute_failed" }`, right `Reason { code: "obligations_unreadable" }`.
  - M2, `sync_record_failed` renamed `sync_failed`: killed, `History(LoggingInstalled)`; left
    `sync_failed`, right `sync_record_failed`.
  - M3, `CycleError::Window(_)` moved to the record's arm: killed, `Window(Read(WriteRefused))`; left
    `sync_record_failed`, right `recompute_failed`.
- `roles::the_sync_job_pages_on_a_malformed_scope_before_it_syncs` (`deckstreakd job sync` with a
  valid endpoint and credentials directory and a law root holding the deck separator: exit 1, one
  page `scope_settings_refused`, the fire recorded `error`, no sync run).
  - M4, the refusal's code written `sync_settings_refused`: killed, the page read
    `"reason":"sync_settings_refused"`.
  - M5, a malformed scope read as the default scope: killed, the job synced against the dead port
    (`the sync failed`, `network_unreachable`, 3 attempts) and paged
    `"reason":"network_unreachable attempts=3"`, not `scope_settings_refused`.

The probe's weights and modulus became bound parameters at b14015f (web-security's `ws.db-rules`
refuses a query assembled with `format!`); A10 held all 16 cases of `change_probe` unchanged.

```red-first
A1: red at 32e96dd: a write through the reader was not refused as read-only: INSERT INTO revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) VALUES (1, 1001, 0, 3, 1, 0, 2500, 4000, 1): Ok(0)
A1: green at 73e7b00
A2: red at 32e96dd: assertion `left == right` failed: the cards whose home deck is under Law or Language, the borrowed Law card included; left: [], right: [1001, 1002, 1003, 1004]
A2: green at 73e7b00
A3: red at 32e96dd: assertion `left == right` failed at the first case of allowed_deck_ids (include_prefixes []); left: [], right: every deck id of the case
A3: green at 73e7b00
A4: red at 32e96dd: assertion `left == right` failed: {"ease":1,"rtype":0}; left: false, right: true
A4: green at 73e7b00
A5: red at 32e96dd: assertion `left == right` failed: a card is scoped by the deck it belongs to, not the filtered deck it sits in; left: [], right: [(1001, evidence, 0, evidence), (1002, cram, torts, torts)]
A5: green at 73e7b00
A6: red at 32e96dd: assertion `left == right` failed; left: {} (the names the reader read), right: every deck's stored name, as the engine reads them with unicase registered
A6: green at 73e7b00
A7: red at 32e96dd: assertion `left == right` failed: one WARN at start: []; left: 0, right: 1
A7: green at 73e7b00
A8: red at 32e96dd: assertion `left == right` failed (INGEST_WINDOW_DAYS against ingest.constants); left: 0, right: 400
A8: green at 73e7b00
A9: red at 32e96dd: assertion `left == right` failed: the recount after the deletion is lower than the stored count; left: None, right: Some(SelfCheck { stored: 5, recounted: 3 })
A9: green at 73e7b00
A10: red at 32e96dd: assertion `left == right` failed (CARD_FIELD_WEIGHTS against ingest.constants); left: [0, 0, 0, 0, 0, 0, 0], right: [131, 137, 139, 149, 151, 157, 163]
A10: green at 73e7b00
A11: red at 32e96dd: assertion `left == right` failed (the unchanged cycle); left: Run(AnchorMissing), right: Skip
A11: green at 73e7b00
A12: red at 32e96dd: assertion `left == right` failed; left: Run(AnchorMissing), right: Run(NewestReviewChanged)
A12: green at 73e7b00
A13: red at 32e96dd: assertion `left == right` failed; left: Run(AnchorMissing), right: Run(CardFingerprintChanged)
A13: green at 73e7b00
A14: red at 32e96dd: assertion `left == right` failed; left: Run(AnchorMissing), right: Run(StudyDayChanged)
A14: green at 73e7b00
A15: red at 32e96dd: assertion `left == right` failed; left: Run(AnchorMissing), right: Run(SettingsChanged)
A15: green at 73e7b00
A16: red at 32e96dd: assertion `left == right` failed: this cycle's sync failed; left: Run(AnchorMissing), right: Run(SyncFailed)
A16: green at 73e7b00
A17: red at 32e96dd: assertion `left == right` failed; left: Run(AnchorMissing), right: Run(RescorePending)
A17: green at 73e7b00
A18: red at 32e96dd: assertion `left == right` failed: the deadline alone runs the recompute: Ran { reason: AnchorMissing, reviews: 0, cards: 0 }; left: Some(AnchorMissing), right: Some(DeadlineDue { label: "synthetic_deadline" })
A18: green at 73e7b00
A19: red at 32e96dd: assertion `left == right` failed (a deadline not yet due); left: Ran { reason: AnchorMissing, reviews: 0, cards: 0 }, right: Skipped
A19: green at 73e7b00
A20: red at 32e96dd: ingest_state is a singleton: reset in place (CHARTER 13), not None
A20: green at 73e7b00
```
