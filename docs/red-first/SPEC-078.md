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
