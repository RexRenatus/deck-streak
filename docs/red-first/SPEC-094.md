# Red-first record: SPEC-094

Each criterion's tests were committed against inert stubs (or absent routes, or absent screens)
and run there, so each failed by assertion, before the implementation that turns it green. A
criterion's red and green are two commits. The criteria's extra tests, which the SPEC's list does
not name, went red and green in the same two commits as the criterion they sit beside:
`a_report_shows_at_most_the_cap_and_counts_the_rest` (red: 0 vs 45) and
`names_made_safe_at_the_read_are_compared_made_safe` (red: `left: 0 right: 1`) with A9 to A12;
`an_inert_row_never_runs` (red: `left: [] right: ["alpha"]`) with A12 to A17;
`a_run_in_progress_is_answered_and_starts_nothing` (red: 404 vs 409) and
`a_cross_site_run_is_refused` (red: 404 vs 403) with A18.

```red-first
A1: red at 37990f8: examined 0 pair rules, so nothing was judged
A1: green at b8ea3f0
A2: red at 37990f8: not JSON: Conventions {..}
A2: green at b8ea3f0
A3: red at 37990f8: template rule: Conventions {..}
A3: green at b8ea3f0
A4: red at f170628: left: [] right: [(1, 0, "150")]
A4: green at 924f863
A5: red at f170628: no template rows were read
A5: green at 924f863
A6: red at f170628: left: [] right: [400, 400, 200]
A6: green at 924f863
A7: red at f170628: a note type used only outside the scope was read
A7: green at 924f863
A8: red at f170628: left: "<b>Example</b> & .." right: "&lt;b&gt;Example&lt;/b&gt; &amp; .."
A8: green at 924f863
A9: red at b826570: left: [] right: [("Type A", "Dark", 5)]
A9: green at b68f556
A10: red at b826570: left: [] right: ["Back", "Front"]
A10: green at b68f556
A11: red at b826570: left: 0 right: 3
A11: green at b68f556
A12: red at b826570: left: ["dark_fields","sleeper"] right: ["dark_fields"]
A12: green at b68f556
A13: red at ec7a942: left: [] right: ["alpha"]
A13: green at 685c6c8
A14: red at ec7a942: left: [] right: ["alpha"]
A14: green at 685c6c8
A15: red at ec7a942: Err(Unknown)
A15: green at 685c6c8
A16: red at ec7a942: panicked "a report": the store returned none
A16: green at 685c6c8
A17: red at ec7a942: panicked "the table is declared"
A17: green at 685c6c8
A18: red at 17a4c4b: every route refuses a caller with no session: the routes were not mounted, so the answer was 404 and not 401
A18: green at 6b5a0c4
A19: red at 9717e12: Unable to find an accessible element with the role "alert"
A19: green at b6fadce
A20: red at 9717e12: Unable to find an element with the text: /checked 4 note types/i
A20: green at b6fadce
A21: red at ec7a942: the run: "not built"
A21: green at 685c6c8
```

The parity goldens (59f4f52) were generated from the predecessor's own functions before the tests
that read them, so A9 to A11 compare against values no test of this delivery wrote.

In `crates/ingest/tests/structure.rs` the `unhex` and `hex` helpers changed between the red commit
and the green one, to satisfy clippy's pedantic lints; no assertion changed. The existing SPEC-027
test `crates/coordination/tests/data_rights.rs` was edited in the green commit of A13 to A17,
because a second table is now declared for export and erasure.

Disclosure: after its red commit, `the_wire_walk_matches_the_predecessors_golden` in
`crates/ingest/tests/structure.rs` gained a local `examined` helper call that prints how many golden cases it examined
(the tdd examined-counts rule). No assertion changed.

Between the reds and greens, three commits edited a test file, disclosed here by their shas:
- 685c6c8 changed the assertion in `crates/coordination/tests/data_rights.rs` and its symmetry test,
  because a second table is now declared for export and erasure (the count of named tables grew).
- b6fadce updated `web/app/src/lib/startapp.test.ts`, because the new `insights` start token and the
  `/insights` route entry join the lists that test compares.
- 924f863 changed the `unhex` helper in `crates/ingest/tests/structure.rs` (a clippy pedantic
  rewrite of the same hex decoding); no expected value changed.

## Addendum, 2026-09-29: the missed mutants on the diff get killers (PR #403)

The diff's mutants were listed and run per file with cargo-mutants after the criteria were green,
and the survivors each got a whole-value test in the crate that holds the mutant. These are not
new criteria, so the block above keeps its one red line and one green line per criterion. Each
killer was run against its mutant: it survived before the test and was caught after it.

| commit | killer tests | mutants they kill |
|---|---|---|
| e246a7c5 | `lock::a_try_take_answers_the_lock_when_free_and_nothing_while_held`; `structure::each_failed_structure_read_is_named_once_in_the_order_it_failed`, `structure::a_presence_read_that_fails_in_every_batch_is_named_once`; `dark_fields::a_token_needs_two_opening_braces_in_a_row`, `dark_fields::the_instrument_names_itself_versions_its_report_and_hands_back_its_failed_reads` | `try_exclusive`, `StructureReads::fail`, `tokens_in` (174), `DarkFields::id`, `schema_version`, `failed_reads` |
| 090c47b0 | `instruments_step::a_frame_answers_the_id_and_version_of_its_instrument`; `instruments_cycle::a_cycle_with_instruments_runs_the_step_after_its_sync`, `instruments_cycle::a_cycle_without_instruments_stores_no_report` | `Frame::id`, `Frame::schema_version`, `run_instruments` |
| eb415974 | `insights_routes::an_unreadable_store_answers_500_with_a_reason_code_alone`; `commands::the_handlers_hold_the_instruments_only_once_they_are_handed_them` | `unreadable`, `Commands::instruments` |
| 4bb144d9 | `instruments_wiring::a_role_with_valid_settings_gets_the_instruments`, `instruments_wiring::the_late_holder_answers_not_ready_until_it_is_filled`; `roles::only_the_sync_job_loads_the_owners_conventions` | `instruments_for_role`, `LateInstruments::fill`, `role_job` gate |

Four mutants no test can tell apart are recorded equivalent, each with its reason, in
`scripts/mutation-equivalent.d/`: the two `Debug` impls of `Instruments` and `ApiState`, whose text
no caller reads, and the two operator swaps in `tokens_in` (`>` to `>=` at line 180 and `+` to `-`
at line 182), which change only which bytes are rescanned, never a token. The refresh path's
mutants also carry rows S09413 to S09417, each proved killed by its full id.
