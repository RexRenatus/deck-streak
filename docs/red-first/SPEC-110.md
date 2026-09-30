# Red-first record: SPEC-110

The order of work: the SPEC promoted, ADR-110 accepted and SPEC-042 R12 amended (4468501); the
one-router census naming each drill reply and its caller (ae11464); the goldens (1910847); the
vault's inert shapes and the drill tables (99ae1ca); the vault's tests (d1d3acf) and their
implementation in four commits (81e05e0, c493f6e, 20bfa78, c479aac); the coordination inert shapes
(5eeebf4), tests (a2e0e1a) and implementation (91115c9, 61fe55a); the daemon's test (1b8c78e) and
implementation (3f0b83a, 25eb6f9, 2edd603); the bot's inert shapes (edfe7f2), tests (d876a07) and
implementation (5449642); the api's inert routes (45e4c60), test (4739275) and implementation
(df5ceb0); the mutation rows (bb90d4d); and the pins of the stem
refusal and the token prefixes (ef4ec29).

The goldens were generated from the predecessor's own functions at `27ee2bc`, with a scratch
`--registry` and a scratch `--out`, under `PYTHONDONTWRITEBYTECODE=1`. The predecessor's checkout
was left as it was: no change and no bytecode file added.

Each criterion was run at its red commit with the SPEC's own fenced command, selecting one test, and
failed by assertion for its own criterion, not by a compile error, a missing fixture or an empty
selection. The stubs compiled and refused or returned nothing: every drill `not_active`, no list,
no queue, empty keys, zero constants, no grade parse, a job table without the post-back, an empty
memory port, a token that encoded to nothing, commands that sent no reply and routes that answered
501.

```red-first
A1: red at d1d3acf: assertion `left == right` failed: active of {"notes":{"a-note":""},"today":200}; left: Array []
A1: green at c479aac
A2: red at d1d3acf: assertion `left == right` failed: the reason "   "; left: "   ", right: ""
A2: green at c493f6e
A3: red at d1d3acf: assertion `left == right` failed: queue of the case-brief notes; left: Array []
A3: green at c493f6e
A4: red at d1d3acf: assertion `left == right` failed; left: NotActive, right: Appended { title: "Duty of care" }
A4: green at c479aac
A5: red at d1d3acf: assertion `left == right` failed; left: NotActive, right: EmptyAnswer
A5: green at c479aac
A6: red at a2e0e1a: the first answer is appended: NotActive
A6: green at 91115c9
A7: red at d1d3acf: the note write fails: Ok(NotActive)
A7: green at c479aac
A8: red at d1d3acf: assertion `left == right` failed: the graded parse of "bare-rule-later"; left: Null
A8: green at c493f6e
A9: red at a2e0e1a: assertion `left == right` failed: the clamped amount, on the study day, track law, scope once; left: []
A9: green at 91115c9
A10: red at a2e0e1a: assertion `left == right` failed: one grant; left: 0
A10: green at 91115c9
A11: red at d1d3acf: assertion `left == right` failed: the key of "law-1"; left: "law-1", right: "drill:law-1"
A11: green at c493f6e
A12: red at a2e0e1a: a missing root is the job's error: the job ran with outcome Ok
A12: green at 91115c9
A13: red at a2e0e1a: assertion `left == right` failed: the next poll completes the grade row; left: 0
A13: green at 91115c9
A14: red at a2e0e1a: the table lists the drill post-back
A14: green at 91115c9
A15: red at 1b8c78e: assertion `left == right` failed: at most ten entries: []; left: 0
A15: green at 3f0b83a
A16: red at d876a07: assertion `left == right` failed: the token of irac-one; left: String("")
A16: green at 5449642
A17: red at d876a07: the command sent no reply
A17: green at 5449642
A18: red at d876a07: the command sent no reply
A18: green at 5449642
A19: red at 4739275: assertion `left == right` failed; left: 501
A19: green at df5ceb0
A20: red at d1d3acf: drill_answers is exported and erased
A20: green at 20bfa78
A21: red at d1d3acf: assertion `left == right` failed: the constant vault_bridge.DRILL_POSTBACK_XP; left: Number(0), right: Number(15)
A21: green at c493f6e
A22: red at d1d3acf: assertion failed: has_answer(&mut tx, "irac-1").await.expect("a read")
A22: green at c479aac
A23: red at d1d3acf: xp 9 is refused by the table
A23: green at 20bfa78
A24: red at d1d3acf: assertion `left == right` failed; left: [], right: ["Free Recall", "Rule"]
A24: green at c493f6e
```

## 2026-09-29: post-green killer tests

These tests were added after the criteria were green, to kill the mutants the land bar's
mutation-verdict named missed (274 examined, 30 missed). They are disclosed here and are not
red-first criteria. Each was shown to pass with the mutant applied to the tests then in the tree,
and to fail with it once the new test was added.

- cdf44965: `drill_kills` in the vault, ten tests: the frontmatter strip and the comment cut, the
  answer headings, the deferral reason, the rollup tie, the ready label and self-check heading, the
  open refusal, the missing-folder list, the graded and active folders, and the recent grades.
- e278dc0c: `drill_paid_count::a_poll_reports_how_many_it_graded_and_how_many_it_paid` and
  `drill_answer::the_answer_heading_names_the_local_clock_of_the_rule` in coordination.
- cdd877b6: `drill_replies` in the bot, four tests: the button label, the list tail, the prompt cut
  and the view's deferral and answer button.
- c1edcd25: `drill_routes::the_list_counts_what_awaits_grading_and_what_is_deferred` in the api.
- 8a79433b: `drill_vault::a_configured_vault_opens_over_its_active_drills` in the daemon.
- 0fe0f0d1: two equivalence records for `type_of` (a date at the stem's start gives the empty prefix
  the fallback arm also gives).
- bf12e2d8 and 80e94331: mutation rows S11028 to S11040 for the pay count, the recent grade's xp, the
  graded folder, the open refusal, the daemon open, the constants and the post-back minute.

## 2026-09-29: amendment

- A1's green line read `c493f6e`. Replayed with its fenced command, A1 fails there by assertion
  (`active of {"notes":{"a-note":""},"today":200}`, left `[]`) and at `20bfa78`, and passes first at
  `c479aac`, so the line now names `c479aac`.
- `c493f6e` changed one assertion of A24's test between its red and its green: the prompt expected
  `"The prompt."` and now expects `"# A synthetic drill\n\nThe prompt."`. The predecessor's
  `_drill_prompt` keeps the note's title heading, and every prompt in `goldens/drill_meta.json`
  begins with it, so the new expectation is the predecessor's. A24's red at `d1d3acf` is on
  `sections`, which the change does not touch.
- Four green commits also edited tests that already existed: `91115c9` (the job table's slot census,
  `24 * 25` to `24 * 49`, for the post-back's 24 hourly slots), `61fe55a` (two drill seeds in
  `data_rights_symmetry`), `5449642` (the bot's command test and the help and start goldens, for the
  two drill commands) and `df5ceb0` (an unused constant dropped from `drill_routes`).
- Five later commits edited or added tests and were not named above: `ada550d3` (the job usage line
  in the daemon's `roles`), `29ce87d7` (the examined counts of `drill_commands`' note reads),
  `f840f002` (the deploy and rail-contract tests), `4b6465c7` (`drill_paid_count` and `drill_vault`,
  for clippy and the absence-only rule) and `7fe70740` (`drill_routes_composed`, the drill routes
  through the composed router). None is a red-first criterion.
- This amendment's own tests are not criteria either:
  `drill_postback::a_drill_whose_kept_key_would_overflow_the_grammar_pays_by_its_hash` (R10) and
  `drill_kills::a_link_in_the_drills_folders_is_never_read_or_written_through` (R1). Each fails by
  assertion at `cc91e8e9` and passes with the change.

## 2026-09-29: amendment, round 1 of review

The fix round added tests for two classes, each red before its change and green after it. None is a
red-first criterion. The reds are at `86028bba`, whose tree holds the tests and a stub of the bound's
constants; the greens are at `3b492c32`. Each red is by assertion, exit 101.

- The kept-id bound (R10). At `86028bba`: `drill_goldens::an_unkeyable_drill_id_is_keyed_by_its_hash`
  fails on its 122 and 123 boundary (the key of a 123-character id is kept, not hashed);
  `drill_postback::a_drill_whose_kept_key_would_overflow_the_grammar_pays_by_its_hash` fails with
  `no drill's key is refused` (the report carries the `drill_key_refused` page);
  `drill_key_population::the_kept_id_bound_is_the_grammars_own_limit_less_the_prefix` fails with
  left 128, right 122; `drill_key_population::every_key_the_post_back_forms_is_a_source_the_grammar_accepts`
  fails on the first member, `the grammar refuses the key of an id of 123 characters`. At `3b492c32`
  all four pass, and the population prints `examined 5166 ids (4513 kept, 653 hashed)`.
- The vault adapter's one gate (R1). At `86028bba`:
  `drill_kills::a_link_in_the_drills_folders_is_never_read_or_written_through` fails at its first
  listing (a linked note `linked` is listed), and
  `drill_kills::no_link_in_any_placement_is_read_listed_paid_from_or_written_through` fails on its
  first member, `File NoteInActive List: listed [...]`. At `3b492c32` both pass, and the population
  prints `examined 80 placements; unbuilt on this file system: []`.
- Manifest and rows. The manifest names `crates/coordination/tests/drill_key_population.rs`. Row
  `S11021` is re-anchored to the derived expression; the second `S11040` is now `S11041`; rows
  `S11042` to `S11045` are new.

## 2026-09-29: amendment, round 1 of review, the second CI round

`6f2522c3` came after green and changed a production line and a test file. Neither is a red-first
criterion.

- Production: `notes()` no longer binds `present` from `confined()`, and its arm that returned an
  empty list for a folder that was not present is gone; it calls `confined()` and lists. The two
  differ only for a folder that appears between its resolve and its listing.
- Test: `drill_kills::a_folder_that_cannot_be_resolved_is_an_io_error_and_only_a_missing_one_is_none`
  is a post-green killer. Not red: it pins. It has no red at the code before it: it passes at `3b492c32`, whose
  `confined()` already made a resolve failure other than `NotFound` an I/O error. Its red is the
  mutant CI's `mutation-verdict` named at `90347860`, the `NotFound` guard in `confined()` replaced
  with `true`: the test fails by assertion with `a resolve failure: []`, exit 101. At `6f2522c3` it
  passes.

## 2026-09-29: amendment, round 2 of review

- Rows `S11046` to `S11048` pin the gate's three other call sites: `notes()` before a folder is
  listed, `regular_note()` before a note's kind is read, and `read()` before a note is read.
- `drill_kills::no_link_in_any_placement_is_read_listed_paid_from_or_written_through` now requires
  `NotAFolder` from a listing of a linked `Active` or `Graded` folder, where it accepted any empty
  listing, and an empty list from every other member. A post-green test edit, not a criterion: with
  the `notes()` gate removed it fails by assertion on `File ActiveFolder List`, exit 101; at the head
  it passes.
