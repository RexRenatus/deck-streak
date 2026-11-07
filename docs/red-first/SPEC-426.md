# Red-first record: SPEC-426

SPEC-426 (R1 to R11, A1 to A9). The SPEC, ADR-440, the schematic, the guard module and the
`LakefileWarnings` class were committed first, with no change to the lakefile
(f7538e03e3f1c63639a8b611ba82ccf418a2a95e). The stubs kept every input and answered nothing: the lakefile reader
read the file with no link walk and found no problem, and the matcher kept every comment and
string and found no hatch. So each test failed by its own assertion, on the behaviour it
names, and none on an import, a fixture or an empty selection. The lakefile's key, the reader
and the matcher were committed next (9b1fce5c2d15576f56043bde01105ddd98d8c5ec), then the mutation rows
(4520aab5ac52ee120af6ff732427015b9b83b5ee).

Every red and every green below is CI's reading of the python stage in the `hygiene` job, at
the head of the push that carried the commit. CI ran the red commit alone, and then the head
that holds the rows; it never ran the implementation commit alone. That head differs from the
implementation commit only by the rows file; the lakefile and both test modules are the same
at both:

```text
git diff --quiet 9b1fce5c2d15576f56043bde01105ddd98d8c5ec 4520aab5ac52ee120af6ff732427015b9b83b5ee -- formal/lean/lakefile.toml scripts/tests/test_formal_config.py scripts/tests/test_lean_escape_hatches.py
```

exits 0, so each green line names the commit CI ran. A red that unittest printed over several
lines is quoted by its `AssertionError` line and the message its last line ends with, joined
by `…`.

```red-first
A1: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: False is not true : {'name': 'formal', 'defaultTargets': ['Formal'], 'lean_lib': [{'name': 'Formal', 'globs': ['Formal.*']}]} != {'name': 'formal', 'defaultTargets': ['Formal'], 'lean_lib': [{'name': 'Formal', 'globs': ['Formal.*'], 'leanOptions': {'warningAsError': True}}]}
A1: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A2: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Lists differ: ["lean_lib Formal's leanOptions: the key a[571 chars] []"] != [] … : a planted fault the reader did not refuse by its arm
A2: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A3: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: AssertionError not raised : formal
A3: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A4: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Lists differ: [] != [('formal/lean/Formal.lean', 17, 'sorry')] … : the planted hatch
A4: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A5: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Lists differ: ['sorry in code: []', 'admit in code: []',[394 chars] []"] != [] … : a planted line the guard read otherwise than its arm
A5: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A6: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Lists differ: ['unterminated-comment: []', 'unterminated[73 chars] []'] != [] … : an unterminated construct not refused by its name
A6: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A7: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Lists differ: ['a sorry in code: the guard refused False[530 chars]rue'] != [] … : a plant the matcher reads otherwise than Lean
A7: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A8: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Lists differ: [] != [('formal/lean/Formal/Planted.lean', 1, 'key-off')]
A8: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
A9: red at f7538e03e3f1c63639a8b611ba82ccf418a2a95e: AssertionError: Tuples differ: ([('f[13 chars]rmal/Planted.lean', 3, 'sorry'), ('formal/lean[82 chars], []) != ([('f[13 chars]rmal/Other.lean', 1, 'axiom')], []) … : the entry admits its hits
A9: green at 4520aab5ac52ee120af6ff732427015b9b83b5ee
```

At the red commit the stage read `FAILED (failures=9)`, those nine tests and no other, and
the guard printed `examined 28 Lean files under formal/lean` before its first assertion. At the
head that holds the rows the same nine tests passed. The guard printed
`examined 28 Lean files under formal/lean`, `read 6164 lines` and `allowed 0 entries`, and A7
printed `examined 17 planted files Lean elaborated`.

## The local reading of the build (SPEC-426 section 4)

Each build is `lake build` with the Lean that `formal/lean/lean-toolchain` names, read against
that file first, in an export of the named commit's `formal/lean` outside the work tree. The
planted module is section 4's `Formal/PlantedWarning.lean`. Each diagnostic is quoted relative
to the package.

| step | export | exit | diagnostic and last line |
|---|---|---|---|
| 1 | the base, `b25ed04a`, with the plant | 0 | `` warning: Formal/PlantedWarning.lean:2:6: Variable name `unused` is not explicitly referenced. ``; `Build completed successfully (31 jobs).` |
| 2 | the implementation, `9b1fce5c`, with the plant | 1 | `` error: Formal/PlantedWarning.lean:2:6: Variable name `unused` is not explicitly referenced. ``; `error: build failed` |
| 3 | the implementation, `9b1fce5c`, with no plant | 0 | no line holds `warning` or `error`; `Build completed successfully (30 jobs).` |
| 4 | step 1's export, already built, given the lakefile of `9b1fce5c`, no clean | 1 | `` error: Formal/PlantedWarning.lean:2:6: Variable name `unused` is not explicitly referenced. ``; `error: build failed` |
| 5 | the implementation, `9b1fce5c`, the plant headed by `set_option warningAsError false` | 0 | `` warning: Formal/PlantedWarning.lean:3:6: Variable name `unused` is not explicitly referenced. ``; `Build completed successfully (31 jobs).` |

Step 2 is the key taking effect: the same diagnostic, at the same place, is an error and the
build fails. Step 4 shows that a build cached under the old lakefile does not keep the
warning: the changed option rebuilt every module a fresh build builds, 28 built and the
planted one failed. Step 5 is read as read: a module that sets the option false switches the
key off for itself and builds, which is why the guard refuses a module whose code names
`warningAsError` (A8).

Step 6, the oracle, ran before the red commit: each plant of section 9, written to its own file
and elaborated by `lean` alone. Every plant exited 0 and printed no `error:` line, so no plant
was rewritten. The pinned Lean prints the warning as ``declaration uses `sorry` ``, with
backticks, so each verdict below is whether its output held that line. Section 9 and R8 spell
it with single quotes, and those words were read as the warning Lean printed.

| plant | Lean warned |
|---|---|
| a sorry in code | true |
| a sorry in a line comment | false |
| a sorry in a block comment | false |
| a sorry in a nested block comment | false |
| a sorry in a doc comment | false |
| a sorry in a string | false |
| a sorry after an escaped quote in a string | false |
| a sorry after a string that holds a comment marker | true |
| a sorry after a string that holds a block opener | true |
| a sorry after a quote character literal | true |
| a sorry after an escaped character literal | true |
| a sorry in an interpolation | true |
| a sorry in an interpolated string's text | false |
| a sorry in a raw string | false |
| a sorry after a primed name | true |
| a quoted name spelled as the word | false |
| a theorem name that holds the word | false |

Seven plants warned and ten did not. `LEAN_WARNS` records these seventeen verdicts in this
order.

## The formal reading (SPEC-426 section 4 step 7, R11)

Read with the paired checker and the development branch's commit at the cut, `b25ed04a`, as
the base: first on the unchanged cut tree, the base control, and again at the head that holds
the rows. No TLC model runs, and no entry is added.

`formal check --kind lean` at the base control:

```text
FORMAL EXAMINED entries=13 properties=41 witnesses=42 configs=0 landings=20 rulings=0 deferred=0
FORMAL TOOLCHAIN compiled identity=a2518571360483e12161e99b64695e6c3e8845230129ce0ad179b5bddf9a8dc4
FORMAL SURFACES deny=0 contexts=0 rows=2653 land=0 retired=0
FORMAL OK exit=0
```

and at the head:

```text
FORMAL EXAMINED entries=13 properties=41 witnesses=42 configs=0 landings=20 rulings=0 deferred=0
FORMAL TOOLCHAIN compiled identity=a2518571360483e12161e99b64695e6c3e8845230129ce0ad179b5bddf9a8dc4
FORMAL SURFACES deny=0 contexts=0 rows=2690 land=0 retired=0
FORMAL OK exit=0
```

Both reports read `findings` 0. The `rows` count grows by this delivery's 37 rows and nothing
else moves. Every theorem row reads the same at both:

| entry | theorem | clean at the base | clean at the head | ramp | flip |
|---|---|---|---|---|---|
| `lean/Chest` | `a_guaranteed_chest_is_its_rarity_whatever_the_draw` | true | true | report | unmoved |
| `lean/Chest` | `each_payout_keeps_its_cap_or_its_fixed_amount` | true | true | report | unmoved |
| `lean/Chest` | `the_epic_odds_lie_between_the_base_and_the_ceiling` | true | true | report | unmoved |
| `lean/Exchange` | `a_day_counts_once_per_bucket` | true | true | report | unmoved |
| `lean/Exchange` | `best_day_is_the_latest_maximum` | true | true | report | unmoved |
| `lean/Exchange` | `bucket_ends_at_the_first_colon` | true | true | report | unmoved |
| `lean/Exchange` | `rate_defined_iff_a_graduation` | true | true | report | unmoved |
| `lean/Exchange` | `totals_keep_every_row` | true | true | report | unmoved |
| `lean/Exchange` | `window_spans_the_capped_days` | true | true | report | unmoved |
| `lean/HabitWriting` | `no_day_is_all_confirmed_over_no_writing_course` | true | true | report | unmoved |
| `lean/HabitWriting` | `the_writing_day_pays_each_confirmed_course_and_the_bonus_only_over_a_full_set` | true | true | report | unmoved |
| `lean/NextMilestone` | `all_complete_is_the_top_review_rung` | true | true | report | unmoved |
| `lean/NextMilestone` | `pick_is_least` | true | true | report | unmoved |
| `lean/NextMilestone` | `ties_go_to_the_earlier_ladder` | true | true | report | unmoved |
| `lean/OpenLapse` | `answers_the_rule` | true | true | report | unmoved |
| `lean/OpenLapse` | `at_most_one_step_per_day` | true | true | report | unmoved |
| `lean/OpenLapse` | `never_overflows` | true | true | report | unmoved |
| `lean/ReplayHistory` | `the_selection_keeps_the_rated_reviews_after_the_last_reset` | true | true | report | unmoved |
| `lean/RoadToC2` | `a_band_up_is_only_a_later_band_and_a_first_sighting_is_silent` | true | true | report | unmoved |
| `lean/RoadToC2` | `an_achieved_band_after_a_gap_never_counts` | true | true | report | unmoved |
| `lean/RoadToC2` | `the_current_band_is_a1_or_the_end_of_the_run_from_a1` | true | true | report | unmoved |
| `lean/RoadToC2` | `the_law_pillar_lies_in_its_range_and_never_rises_with_leeches` | true | true | report | unmoved |
| `lean/SensitiveDeck` | `a_marked_deck_or_an_ancestor_refuses_the_card` | true | true | report | unmoved |
| `lean/SensitiveDeck` | `an_unresolved_deck_or_an_unreadable_set_is_refused` | true | true | report | unmoved |
| `lean/SensitiveDeck` | `marking_more_decks_never_admits_a_refused_card` | true | true | report | unmoved |
| `lean/SkipTariff` | `the_amount_paid_stays_within_the_price_and_the_wallet` | true | true | report | unmoved |
| `lean/SkipTariff` | `the_last_price_repeats` | true | true | report | unmoved |
| `lean/SkipTariff` | `the_price_is_a_ladder_entry` | true | true | report | unmoved |
| `lean/SkipTariff` | `the_price_never_falls_as_the_count_grows` | true | true | report | unmoved |
| `lean/SkipTariff` | `the_refund_is_what_the_skip_paid` | true | true | report | unmoved |
| `lean/TableCensusReach` | `refused_within_dev_and_a_lone_character_spared` | true | true | report | unmoved |
| `lean/TokenBonus` | `a_window_with_no_study_review_pays_nothing` | true | true | report | unmoved |
| `lean/TokenBonus` | `the_bonus_is_the_capped_sum_of_the_windows_study_reviews` | true | true | report | unmoved |
| `lean/TokenBonus` | `the_bonus_never_passes_its_cap` | true | true | report | unmoved |
| `lean/Wallet` | `clip_debit_pays_within_its_bounds` | true | true | report | unmoved |
| `lean/Wallet` | `the_mint_is_monotone_in_the_base` | true | true | report | unmoved |
| `lean/Wallet` | `the_mint_lies_between_zero_and_the_cap` | true | true | report | unmoved |
| `lean/XpReconciliation` | `a_closed_row_is_never_lowered_by_a_recompute` | true | true | report | unmoved |
| `lean/XpReconciliation` | `a_days_review_xp_never_falls_as_answers_arrive` | true | true | report | unmoved |
| `lean/XpReconciliation` | `confirmed_xp_is_never_below_the_xp_shown` | true | true | report | unmoved |
| `lean/XpReconciliation` | `the_close_keeps_the_last_provisional_amount` | true | true | report | unmoved |

`formal check --ratchet-only` at the base control:

```text
FORMAL EXAMINED entries=40 properties=132 witnesses=167 configs=34 landings=20 rulings=0 deferred=0
FORMAL SURFACES deny=0 contexts=0 rows=2653 land=0 retired=0
FORMAL OK exit=0
```

and at the head:

```text
FORMAL EXAMINED entries=40 properties=132 witnesses=167 configs=34 landings=20 rulings=0 deferred=0
FORMAL SURFACES deny=0 contexts=0 rows=2690 land=0 retired=0
FORMAL OK exit=0
```

with no weakening. `formal ramp`, for one theorem of each of the 13 Lean entries, read the same
at both, each `FORMAL OK exit=0`:

| entry | theorem | clean landings | eligible | flip | ramp |
|---|---|---|---|---|---|
| `lean/Chest` | `a_guaranteed_chest_is_its_rarity_whatever_the_draw` | 20 | true | unmoved | report |
| `lean/Exchange` | `a_day_counts_once_per_bucket` | 20 | true | unmoved | report |
| `lean/HabitWriting` | `no_day_is_all_confirmed_over_no_writing_course` | 20 | true | unmoved | report |
| `lean/NextMilestone` | `all_complete_is_the_top_review_rung` | 20 | true | unmoved | report |
| `lean/OpenLapse` | `answers_the_rule` | 20 | true | unmoved | report |
| `lean/ReplayHistory` | `the_selection_keeps_the_rated_reviews_after_the_last_reset` | 20 | true | unmoved | report |
| `lean/RoadToC2` | `a_band_up_is_only_a_later_band_and_a_first_sighting_is_silent` | 20 | true | unmoved | report |
| `lean/SensitiveDeck` | `a_marked_deck_or_an_ancestor_refuses_the_card` | 20 | true | unmoved | report |
| `lean/SkipTariff` | `the_amount_paid_stays_within_the_price_and_the_wallet` | 20 | true | unmoved | report |
| `lean/TableCensusReach` | `refused_within_dev_and_a_lone_character_spared` | 20 | true | unmoved | report |
| `lean/TokenBonus` | `a_window_with_no_study_review_pays_nothing` | 20 | true | unmoved | report |
| `lean/Wallet` | `clip_debit_pays_within_its_bounds` | 20 | true | unmoved | report |
| `lean/XpReconciliation` | `a_closed_row_is_never_lowered_by_a_recompute` | 20 | true | unmoved | report |

The head is not yet landed, so its clean-landing count is the base's. The lakefile is a root
file of the package and an input of every Lean entry, so its landing restarts each entry's
count, as ADR-440 D8 states.

## The mutation rows (SPEC-426 section 8)

Each find is written from the line the implementation commit landed and occurs exactly once in
its file; none sits inside an f-string. Every row is in `scripts/mutation-rows.d/S42600-S42699.json`,
table `SCRIPT_MUTATIONS`.

| id | target | find | replacement | killer |
|---|---|---|---|---|
| `S42600-THE-FORMAL-LIBRARY-MAKES-EVERY-WARNING-AN-ERROR` | `formal/lean/lakefile.toml` | `leanOptions = { warningAsError = true }` | `leanOptions = { warningAsError = false }` | A1 |
| `S42601-THE-DECLARED-DOCUMENT-HOLDS-THE-KEY-ON` | `scripts/tests/test_formal_config.py` | `"leanOptions": {"warningAsError": True}` | `"leanOptions": {"warningAsError": False}` | A1 |
| `S42602-AN-ABSENT-KEY-IS-REFUSED` | `scripts/tests/test_formal_config.py` | `problems.append(("key-absent",` | `(("key-absent",` | A2 |
| `S42603-ONLY-THE-BOOLEAN-TRUE-TURNS-THE-KEY-ON` | `scripts/tests/test_formal_config.py` | `elif options[WARNING_KEY] is not True:` | `elif not options[WARNING_KEY]:` | A2 |
| `S42604-THE-KEY-IS-READ-ON-THE-FORMAL-LIBRARY-ALONE` | `scripts/tests/test_formal_config.py` | `lib.get("name") == LIBRARY` | `True` | A2 |
| `S42605-A-SECOND-FORMAL-LIBRARY-IS-REFUSED` | `scripts/tests/test_formal_config.py` | `if len(libraries) > 1:` | `if len(libraries) > 2:` | A2 |
| `S42606-AN-ARGUMENT-NAMING-THE-KEY-IS-REFUSED` | `scripts/tests/test_formal_config.py` | `if WARNING_KEY in str(argument):` | `if False:` | A2 |
| `S42607-THE-ARGUMENT-ARM-READS-THE-PACKAGE-TOO` | `scripts/tests/test_formal_config.py` | `holders = [("the package", doc)] + ` | `holders = ` | A2 |
| `S42608-A-LINK-ON-THE-LAKEFILE-PATH-IS-REFUSED` | `scripts/tests/test_formal_config.py` | `refuse_link_components(root / LAKEFILE_PATH, root)` | `None` | A3 |
| `S42609-SORRY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"sorry",  # a hole: the declaration rests on sorryAx` | `"-",  # a hole: the declaration rests on sorryAx` | A5 |
| `S42610-THE-ADMIT-TACTIC-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"admit",  # the tactic that closes a goal with sorry` | `"-",  # the tactic that closes a goal with sorry` | A5 |
| `S42611-AXIOM-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"axiom",  # a new axiom, which no proof discharges` | `"-",  # a new axiom, which no proof discharges` | A5 |
| `S42612-NATIVE-DECIDE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"native_decide",  # the compiler's evaluation, trusted by an axiom` | `"-",  # the compiler's evaluation, trusted by an axiom` | A5 |
| `S42613-IMPLEMENTED-BY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"implemented_by",  # runs other code than the definition the kernel checked` | `"-",  # runs other code than the definition the kernel checked` | A5 |
| `S42614-THE-EXTERN-ATTRIBUTE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"extern",  # runs foreign code in the definition's place` | `"-",  # runs foreign code in the definition's place` | A5 |
| `S42615-UNSAFE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"unsafe",  # a declaration the kernel does not check` | `"-",  # a declaration the kernel does not check` | A5 |
| `S42616-THE-NOLINT-ATTRIBUTE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `"nolint",  # a linter switched off for one declaration` | `"-",  # a linter switched off for one declaration` | A5 |
| `S42617-A-LINTER-SWITCHED-OFF-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `LINTER_PREFIX = "linter."` | `LINTER_PREFIX = "linter.unusedVariables"` | A5 |
| `S42618-ONLY-FALSE-SWITCHES-A-LINTER-OFF` | `scripts/tests/test_lean_escape_hatches.py` | `value == "false"` | `value in ("false", "true")` | A5 |
| `S42619-A-ZERO-HEARTBEAT-LIMIT-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `numeral_value(value) == 0` | `numeral_value(value) == 1` | A5 |
| `S42620-THE-HEARTBEAT-ARM-READS-A-DOTTED-NAME` | `scripts/tests/test_lean_escape_hatches.py` | `name.endswith("." + HEARTBEATS)` | `False` | A5 |
| `S42621-A-ZERO-IN-ANOTHER-BASE-IS-ZERO` | `scripts/tests/test_lean_escape_hatches.py` | `base = NUMERAL_BASES.get(token[:2].lower(), 10)` | `base = 10` | A5 |
| `S42622-A-LINE-COMMENT-IS-PROSE` | `scripts/tests/test_lean_escape_hatches.py` | `elif text.startswith("--", index):` | `elif text.startswith("---", index):` | A5 |
| `S42623-BLOCK-COMMENTS-NEST` | `scripts/tests/test_lean_escape_hatches.py` | `nesting += 1` | `nesting = 1` | A7 |
| `S42624-AN-ESCAPED-CHARACTER-STAYS-IN-ITS-STRING` | `scripts/tests/test_lean_escape_hatches.py` | `index += 2  # a backslash takes the next character with it` | `index += 1  # a backslash takes the next character with it` | A7 |
| `S42625-AN-INTERPOLATION-IS-CODE` | `scripts/tests/test_lean_escape_hatches.py` | `interpolated = start > 0 and text[start - 1] == "!"` | `interpolated = False` | A7 |
| `S42626-A-CHARACTER-LITERAL-IS-NOT-A-STRING` | `scripts/tests/test_lean_escape_hatches.py` | `elif (end := char_literal_end(text, index)) is not None:` | `elif False:` | A7 |
| `S42627-A-RAW-STRING-ENDS-AT-ITS-OWN-HASHES` | `scripts/tests/test_lean_escape_hatches.py` | `close = '"' + "#" * hashes` | `close = '"'` | A7 |
| `S42628-AN-UNTERMINATED-BLOCK-COMMENT-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `return refuse(path, text, start, out, problems, "unterminated-comment")` | `return None` | A6 |
| `S42629-AN-UNTERMINATED-STRING-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `return refuse(path, text, start, out, problems, "unterminated-string")` | `return None` | A6 |
| `S42630-AN-UNTERMINATED-RAW-STRING-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `return refuse(path, text, start, out, problems, "unterminated-raw-string")` | `return None` | A6 |
| `S42631-AN-UNTERMINATED-INTERPOLATION-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `return refuse(path, text, opened, out, problems, "unterminated-interpolation")` | `return None` | A6 |
| `S42632-THE-POPULATION-IS-EVERY-LEAN-FILE-UNDER-FORMAL-LEAN` | `scripts/tests/test_lean_escape_hatches.py` | `ROOT_PREFIX = "formal/lean/"` | `ROOT_PREFIX = "formal/lean/Formal/"` | A4 |
| `S42633-A-LINK-AT-A-LEAN-FILE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `refuse_link_components(Path(root) / path, root)` | `None` | A4 |
| `S42634-AN-ALLOW-LIST-ENTRY-ADMITS-ONLY-ITS-OWN-COUNT` | `scripts/tests/test_lean_escape_hatches.py` | `if found and found == count:` | `if found:` | A9 |
| `S42635-AN-UNUSED-ALLOW-LIST-ENTRY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `stale.append(entry)` | `pass` | A9 |
| `S42636-A-MODULE-THAT-NAMES-THE-KEY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `if token == KEY:` | `if False:` | A8 |
