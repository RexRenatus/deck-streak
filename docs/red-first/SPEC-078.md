# Red-first record: SPEC-078 (part 078a, the reading minutes log)

The SPEC, ADR-078 and the HabitXpFollowsItsLog model with its five witnesses were committed first
(7554f4ba, 21267b61). The goldens, the migration and the habits crate's stubs followed at 69eaf55d,
and the coordination use cases, the habit step, the bot's `/read` and `/undo` and the settle's
derived-source check followed at 8fdc46a9, each a stub that compiles against the tests. Every
fenced criterion ran red at 8fdc46a9, by its exact name, in one call over the eight crates
(`--no-fail-fast -- --test-threads=1`). A3, A4, A4b, A5, A5b and A30 fail at the stub's refusal
(`Err(NoCourse)`, which the stub answers for every entry), read through the test's `expect` or
`assert`; the rest fail by assertion. A31's first command is red because no port declares the new
table, which the data-rights registry's fourteenth port answers at green.

```red-first
A1: red at 8fdc46a9: assertion `left == right` failed: Object {"minutes_today": Number(1)}; left: 0
A2: red at 8fdc46a9: assertion `left == right` failed: "qaa"; left: None
A4c: red at 8fdc46a9: assertion `left == right` failed: day 0; left: StudyDay(0)
A3: red at 8fdc46a9: logged for qab: Err(NoCourse) (case {"entries":[["qab",20101,40],["qaa",20102,30]],"today":20103})
A4: red at 8fdc46a9: the entry is logged: Err(NoCourse)
A4b: red at 8fdc46a9: Err(NoCourse)
A5: red at 8fdc46a9: the entry is logged: Err(NoCourse)
A5b: red at 8fdc46a9: the entry is logged: Err(NoCourse)
A6: red at 8fdc46a9: assertion `left == right` failed: the day's XP and its week's bonus are settled from the log; left: []
A6b: red at 8fdc46a9: assertion `left == right` failed: every entry was logged; left: 0
A30: red at 8fdc46a9: Err(NoCourse)
A14: red at 8fdc46a9: assertion `left == right` failed: constants.READING_XP_PER_MIN; left: Number(0)
A22: red at 8fdc46a9: assertion `left == right` failed: one table; left: 0
A23: red at 8fdc46a9: read:qaa is derived
A24: red at 8fdc46a9: assertion `left == right` failed: the habit step is phase 4's, after the streaks and before the derived bonuses (SPEC-078 R5)
A25: red at 8fdc46a9: assertion `left == right` failed: no course yet
A26: red at 8fdc46a9: assertion `left == right` failed: /read qaa 0
A27: red at 8fdc46a9: assertion `left == right` failed: an empty log
A28: red at 8fdc46a9: assertion `left == right` failed: hb:c:qaaaaaaa; left: None
A29: red at 8fdc46a9: assertion `left == right` failed: /read is in the menu; left: None
A31: red at 8fdc46a9: assertion `left == right` failed; left: ["minutes_log is declared by no port"]
```

Amended tests are not red-first evidence and are recorded only so a reader knows why they were red
at 8fdc46a9: `commands.rs`'s `every_golden_message_is_what_the_bot_sends` and
`the_menu_is_registered_for_the_owners_chat_only` (the menu and the help and start goldens move at
green), and `data_rights_symmetry.rs`'s two seeded-schema probes (SEEDS gains the new table's row
at green). `kernel/tests/schema.rs`'s `every_migration_names_the_context_that_owns_its_tables` is
red until `docs/CONTEXT-MAP.md`'s ownership register names the new table (a data-carry red).

Correction: A31 is decided by two commands, and only its first was red at 8fdc46a9.
`every_table_of_the_schema_is_declared_by_exactly_one_port` was red as recorded above.
`privacy_json_names_every_table_the_ports_export_or_erase` was GREEN at 8fdc46a9: it reads only the
tables a port exports or erases, and no port named `minutes_log` until the habits port joined the
registry at green, so it had nothing new to judge at red. It is a guard that holds at green, not
red-first evidence (ruling 107, item 4).

Green: every fenced criterion passed by its exact name at 52c607a7, the green commit, in one call
over the eight crates (`--no-fail-fast -- --test-threads=1`), and no crate, migration or query
cache changed after it. A31's two commands both passed there.

```red-first
A1: green at 52c607a7
A2: green at 52c607a7
A4c: green at 52c607a7
A3: green at 52c607a7
A4: green at 52c607a7
A4b: green at 52c607a7
A5: green at 52c607a7
A5b: green at 52c607a7
A6: green at 52c607a7
A6b: green at 52c607a7
A30: green at 52c607a7
A14: green at 52c607a7
A22: green at 52c607a7
A23: green at 52c607a7
A24: green at 52c607a7
A25: green at 52c607a7
A26: green at 52c607a7
A27: green at 52c607a7
A28: green at 52c607a7
A29: green at 52c607a7
A31: green at 52c607a7
```

`crates/habits/src/store.rs`, 19 of 19 killed. This and the next three paragraphs answer the 46
mutants of this part's own code that CI's mutation verdict found missed at 84ef91e3 (ruling 152): 45
are killed by new tests in the crate that owns the mutated file, the 46th is equivalent, and no
criterion is added or edited, so these tests are mutation coverage, not red-first evidence. Each
kill was proved by plant: the exact mutant of its MISSED line was applied to a scratch copy of the
tree, the one killer ran by its exact name (`running 1 test`) and failed as quoted, and the file was
restored byte for byte (sha256 equal); every killer passes on the unmutated tree. No test of the
habits package called the store, and a mutant runs only its own package's tests, so the coordination
tests that reach it never ran against it. `crates/habits/tests/minutes_store.rs` checks each answer
against the table as plain SQL reads it. `the_store_writes_an_entry_whole_and_answers_its_id` kills
`insert` replaced by `Ok(0)`, `Ok(1)` and `Ok(-1)` (each read `left: []` against the two rows
written). `the_newest_entry_is_read_back_whole` kills `newest` replaced by `Ok(None)` (`left: None`,
right the entry of id 2). `a_removal_takes_only_the_entry_it_names` kills `remove` replaced by
`Ok(())` (both rows were left). `minutes_between_sums_one_course_over_both_bounds` kills
`minutes_between` replaced by `Ok(0)` and `Ok(1)` and `minutes` replaced by `0` and `1` (`left: [0,
0, 0, 0]` or `[1, 1, 1, 1]`, `right: [45, 15, 20, 0]`).
`minutes_by_code_sums_each_course_of_a_span_in_code_order` kills its five replacements (`left: []`,
`[("", 0)]`, `[("", 1)]`, `[("xyzzy", 0)]` and `[("xyzzy", 1)]` against `[("qaa", 45), ("qab",
20)]`), and `all_time_by_code_sums_each_course_in_the_order_first_logged` kills its five the same
way against `[("qab", 30), ("qaa", 30)]`.

`crates/habits/src/minutes.rs`, 16 of 17 killed, one equivalent. The habits package tested only the
entry, the reading XP and the week; the choices below were reached only from coordination.
`crates/habits/tests/minutes_rules.rs` spells every expected value literally.
`the_weekly_bonus_is_earned_from_the_goal_up` kills `goal_bonus` replaced by `0` and by `1` and line
60's `>=` replaced by `<` (`left` 0 at 210 minutes, 1 at every count, and 150 below 210 with 0 from
it, against 150 from 210 up). `the_most_used_course_is_the_first_holding_the_most_minutes` kills
`most_used` and `first_max` each replaced by `None`, `Some(String::new())` and
`Some("xyzzy".into())`, and line 149's `>` replaced by `<` and by `==` (`left: None`, `Some("")` or
`Some("xyzzy")`, `right: Some("qab")`, the week's most minutes), and by `>=` (`left: Some("qab")`,
`right: Some("qaa")`, the first of a tie). `each_courses_xp_sources_are_named_for_it` kills
`read_source` and `goal_source` each replaced by `String::new()` and `"xyzzy".into()` (`left: ""` or
`"xyzzy"` against `"read:qaa"` and `"readgoal:qaa"`). Line 50's `>` replaced by `>=` in `reading_xp`
survives every test: the two arms differ only at a day's XP of exactly 240, where both answer 240,
so no input tells them apart. Planted, it passed the habits package whole and coordination's
`habits_minutes` whole, and it is left to the seat, with no record and no exclusion.

`crates/coordination/src/habits/minutes.rs`, 7 of 7 killed. The coordination tests ran against these
mutants, but `habit_rows` keeps a settled row's day, source and amount and drops its closed flag,
and an owner's correction writes the amount whatever that flag says. `closed_rows` reads the flag.
`an_entry_settles_its_own_day_open_and_an_earlier_week_start_closed` kills line 242's `<` replaced
by `<=` and by `==` (Tuesday's `read:qaa` read closed `true`, expected `false`) and line 245's `<`
replaced by `>` (Monday's `readgoal:qaa` read `false`, expected `true`).
`an_entry_on_its_weeks_first_day_settles_that_day_open` kills line 245's `<=` and `==` (Monday's
`readgoal:qab` read `true`, expected `false`).
`an_undo_on_a_later_day_settles_the_entrys_day_closed` kills line 242's `>` (after Thursday's undo,
Tuesday's `read:qaa` read `false`, expected `true`).
`a_habit_writers_debug_shows_its_rule_and_instant_only` kills `HabitWriter`'s `fmt` replaced by
`Ok(Default::default())` (`left: ""` against the whole rendering).

`crates/coordination/src/recompute/habits.rs`, 3 of 3 killed. The step's name was read only by the
daemon's wiring test, in another package, and its closed flag only by the amount-only helper.
`the_habit_step_is_named_for_the_folds_report` kills `name` replaced by `""` and by `"xyzzy"`
(`left: [(DaySteps, "")]` and `[(DaySteps, "xyzzy")]`, `right: [(DaySteps, "habits.reading_xp")]`).
`the_recompute_settles_a_past_day_closed_and_today_open` kills line 46's deleted `!` (Sunday's
`read:qaa` read closed `false` and the current Monday's `true`, expected `true` and `false`).

Ruling 156 closes the last missed mutant of the habits read tree, `crates/habits/src/minutes.rs` line 50, `>` replaced by `>=` in `reading_xp`. The clamp answered the cap on both sides of `xp == cap`, so no test could tell the two operators apart, and an equivalent record is not admitted. `reading_xp` is now `minutes.saturating_mul(READING_XP_PER_MIN).min(READING_XP_DAILY_CAP_PER_LANG)`, a minimum with no comparison to mutate, and it is no longer `const` because `Ord::min` is not callable there; every caller is a runtime call. Readings: the old and new bodies agree on 200002 of 200002 inputs (`minutes` 0 to 200000 and `u32::MAX`); `the_minutes_xp_matches_the_predecessors_golden` examined 36 cases before and after; the habits and coordination packages pass whole; clippy with `-D warnings` and `cargo fmt --check` are clean; the two body replacements cargo-mutants can still generate, `0` and `1`, each fail that golden test (`running 1 test`, 1 failed) in a scratch copy restored byte for byte.
