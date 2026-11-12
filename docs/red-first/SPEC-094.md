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

No Rust mutant is recorded equivalent. Four Mini App mutants of `web/app/src/lib/insights/insights.ts`
are, in `scripts/mutation-equivalent.d/miniapp.json` (issue #294): each changes a branch whose value
is the same either way (`record(null)` answers its own `null`, `Number.isInteger` refuses what
`typeof value === 'number'` refused, `names(undefined)` refuses a missing `stored`, and a `null`
report is kept as `null`). The `Debug` impl of `Instruments` is killed by a whole-line assertion
(`instruments_step::the_debug_line_names_the_instruments_and_counts_their_runners`). The `Debug`
impl of `ApiState` is checked by a prefix and a suffix
(`insights_routes::the_state_debug_line_says_which_ports_it_holds`), not the whole line; a future
field the impl omits is caught by clippy's `missing_fields_in_debug` under `-D warnings` in CI's rust
job, not by this test. The refresh path's
mutants also carry rows S09413 to S09417, each proved killed by its full id. The two operator swaps
in the template token scan, once thought equivalent, are killed by the scan's split into `token_step`
and `tokens_with` (below), whose tests read a broken pair and a step.

Test edit disclosed: 29dfd0e1 extends `instruments_cycle::a_cycle_without_instruments_stores_no_report`
with a closing presence assertion (the same deployment, run with instruments, stores the alpha
report for the study day), because the tdd probe refuses a test whose only assertions are absences.
No expected value of any earlier assertion changed.

Wire walk progress guard (2026-09-29, ADR-095 amendment): mutants of `walk` that stop the position
advancing pushed fields without bound. `walk_with` now refuses a pass that does not strictly
advance, and `crates/ingest/tests/wire_progress.rs` (a reader that stays, one that steps back, one
that advances by one, one that fails, and the real walk on two fields) kills the guard's own
mutants. It adds no equivalent record; the ingest campaign row keeps its count.

Template token scan progress guard (2026-09-30, ADR-095 amendment): a mutant of the token scan that
stopped the position advancing spun a test for its whole timeout. `tokens_with` now refuses a step
that does not strictly advance with `TOKEN_NO_PROGRESS` (see the addendum below), and
`crates/insights/tests/token_progress.rs` (a reader that stays, one
that steps back, one that advances by one, the real scan, and one step's answer) kills the guard's
mutants and the two swaps previously recorded equivalent; `scripts/mutation-equivalent.d/deck-streak-insights.json`
is removed.

## Addendum, 2026-09-30: test edits after their red commits, and the token scan's refusal

Disclosure: 1fc2fc63 changed A19's test `renders a failed read as a failure line` in
`web/app/src/lib/insights/InstrumentSection.test.ts` after its green commit b6fadce. Its two checks,
`toContain('templates')` and `toContain('fields')`, became one, `toContain('templates, fields')`. The
new body, run at the red commit 9717e12, fails with `TestingLibraryElementError: Unable to find an
accessible element with the role "alert"`. The fence line for A19 is the failure of the current body.

Disclosure: `wire_progress::the_real_walk_reads_a_varint_field_and_a_length_field` (added in cc3835eb)
was edited twice after it. 03f5db02 added `println!("examined {} fields", got.len())`. 9522e7d1
replaced that line with a local `examined` helper, and `assert_eq!(got.len(), 2)` with
`assert_eq!(examined("fields", got.len()), 2)`, for the tdd examined-counts rule. The asserted count
did not change.

Token scan refusal: a stalled token scan ended with `break` and answered the tokens it had kept, so a
stalled step judged a note type on part of its templates. `tokens_with` now refuses with
`TOKEN_NO_PROGRESS`, and `config_tokens_with` marks the template failed, as a wire walk's
`NO_PROGRESS` does. In `crates/insights/tests/token_progress.rs`, the stay and step-back tests now
expect `Err(TOKEN_NO_PROGRESS)` (the step-back test expected `["a"]`), the two reading tests unwrap
the scan's `Ok`, and three tests are added for `config_tokens_with`. At 9b0ce701 they do not
compile, because `tokens_with` answered a bare list and `config_tokens_with` was absent. The class
guard `scripts/tests/test_callee_offset_loops.py` is red at 9b0ce701 by assertion:
`crates/insights/src/dark_fields.rs:206` is `'silent' != 'refuses'`, and six injected readers
(`wire_progress.rs` lines 37, 40, 50, 52 and 58, `token_progress.rs` line 47) bound no calls.

Test edit disclosed: every reader `crates/ingest/tests/wire_progress.rs` and
`crates/insights/tests/token_progress.rs` inject now counts its calls and asserts a bound, and the
empty-blob walk asserts its reader is never called. No expected value changed. Under the compare
mutant `<=` to `<`, the step-back test's first reader (`at.saturating_sub(1)` at 0, a stay) was
accepted and the walk pushed fields until the test process was killed: memory, not a named failure.

Mini App equivalents: five records on `record()` in `insights.ts` were not equivalent. A stored
envelope with no `report` key hands `parseDarkFields` an `undefined`, and each mutant let it through
to `TypeError: Cannot read properties of undefined (reading 'dark_fields')`. Two tests now kill them:
`insights.test.ts` `refuses a report the stored envelope does not hold, and every value that is no
report`, and `insights-page.test.ts` `says the server is unavailable when the stored envelope holds
no report`. Their five records are removed, and four remain.

This round's red and green lines, verbatim. `scripts/tests/test_callee_offset_loops.py` at 9b0ce701
(committed alone): 7 failures, `examined 2 loops`, `examined 9 readers`, and
`crates/insights/src/dark_fields.rs:206 'silent' != 'refuses'`. After the fix: 3 tests OK,
`examined 2 loops`, `examined 11 readers handed to ['config_tokens_with', 'tokens_with', 'walk_with']`,
`examined 3 fixture loops`, `examined 2 fixture readers`. The tests of `token_progress.rs` cannot
compile without the new `Result`, so they are committed with the source and have no red of their own.

Rows S09418 to S09423 (SPEC section 9) carry the compare and refusal mutants that cargo-mutants never
generates or leaves to a named check. Each is proved by its full id: `KILLED: its killer passed
without the mutant and failed with it`, `rows: examined 1: killed 1, survived 0, void 0`. Against the
old walk test, the compare mutant `<=` to `<` in `walk_with` ended in an aborted process and no named
failure; with the bounded readers it fails with `the walk went on past a stalled pass`, and the same
mutant in `tokens_with` fails with `the scan went on past a stalled step`. Two plants have no row: a
stalled callee handed to `token_step` (its test fails with `called Result::unwrap() on an Err value:
"template token scan made no progress"`, a fixture and not a source line), and a stalled callee in
`walk_with`, which no one-line source change expresses.

## Addendum, 2026-09-30: every post-green edit of an existing test line, named by sha (PR #403, round 3)

Rule: every non-merge commit in `dev..head` that rewrites a line of an existing test (a removed
line in a test file it modifies) is named here by its sha, with what changed and why. It was found
by generating the population: each non-merge commit of the range, each test file it modifies, each
removed line. Four were not named; the killer that lists them printed `not named by sha 4` at
692c6598, and the four are named below.

Disclosure: 5e7bf997 rewrote the two `ends_with` expectations of
`insights_routes::the_state_debug_line_says_which_ports_it_holds` in `crates/api/tests/insights_routes.rs`.
Each gained `, drills: false`, because the Debug line now names the drills port that the dev merge
4f49a97d brought into `ApiState`, and clippy's `missing_fields_in_debug` refused the impl without
it. Red at the merge with this test: `panicked at crates/api/tests/insights_routes.rs:324:5:
ApiState { readiness: …, owner: None, instruments: false, law_tiers: false }`, rc 101. Green at
692c6598: the Debug names all 5 fields. The test asserts a prefix plus a suffix of the line, so an
omitted future field is caught by clippy's `missing_fields_in_debug` in CI's rust job, not by this
test.

Disclosure: b8451ebe (`crates/ingest/tests/structure.rs`) renamed the local counter in
`the_wire_walk_matches_the_predecessors_golden` from `examined` to `cases` and added an `examined`
helper that prints the count, for the tdd examined-counts rule. No assertion changed.

Disclosure: 8bfa4c64 (`crates/ingest/tests/wire_progress.rs`, `crates/insights/tests/token_progress.rs`)
made each injected reader count its calls and assert a bound, and changed the token scan's stay and
step-back tests to expect `Err(TOKEN_NO_PROGRESS)`, because a stalled walk or scan must end with a
named failure rather than an unbounded run. The disclosures above, under the 2026-09-30 addendum,
say the same in more detail.

Disclosure: 9fd0b7a2 (`web/app/src/lib/insights/insights.test.ts`) replaced
`expect(parseListings({ instruments: [] })).toEqual([])` with a one-listing expectation
(`{ id: 'a', cadence: 'w', study_day: 0 }` parses to `{ id: 'a', cadence: 'w', studyDay: 0 }`),
because an empty list proves nothing about the parse. It is not a criterion test.
