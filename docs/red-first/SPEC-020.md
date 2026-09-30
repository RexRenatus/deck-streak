# Red-first record: SPEC-020

The kernel's goldens were registered and generated first (be87f76), then the tests were committed
(8034114) beside a kernel whose public API was in place and whose behaviour was stubbed: the study
day was always day 0 and rendered as nothing, the settings parser read nothing, the loader returned
an empty secret, the redactor replaced nothing, the log lines had no priority prefix, the offload
had no bound, the database opened with sqlx's defaults and applied no migration, every data-rights
declaration was accepted, and the constants were placeholders. Each criterion was run there, with
the SPEC's own fenced command, and failed by assertion for its own criterion, not by a compile error,
a missing fixture or an empty selection. The implementation followed (fd002b9). Between the two
commits no test changed what it asserts: three test files took lint fixes only (`Duration::from_hours`,
a method path for a closure, a doc comment's wording).

Both commits were replayed from a `git archive` export with a target directory of their own: 8034114
ran 26 of 26 red, and fd002b9 ran 26 of 26 green, offline against the committed `.sqlx/` cache. At
8034114 the `compile_fail` doc test on `Verdict` also failed in the doctest stage, because a dropped
verdict compiled.

```red-first
A1: red at 8034114: assertion `left == right` failed: the study day of {"instant_ms":856180799999,"rollover_hour":0,"utc_offset_minutes":-720}; left: Some(0), right: Some(9908)
A1: green at fd002b9
A2: red at 8034114: assertion `left == right` failed: epoch day 0 rendered as "", not "1970-01-01"
A2: green at fd002b9
A3: red at 8034114: assertion `left == right` failed: an explicit digest hour of 13 before a rollover of 17 started as Ok(KernelSettings { .. }), not Err(DigestBeforeRollover { digest: "DECKSTREAK_DIGEST_HOUR", rollover: "DECKSTREAK_ROLLOVER_HOUR" })
A3: green at fd002b9
A4: red at 8034114: assertion `left == right` failed: the digest hour of {"raw":null,"rollover_hour":0}; left: Ok(0), right: Ok(9)
A4: green at fd002b9
A5: red at 8034114: assertion `left == right` failed: an unset CREDENTIALS_DIRECTORY was refused as Err(Malformed { .. }), not Err(Missing { setting: "CREDENTIALS_DIRECTORY" })
A5: green at fd002b9
A6: red at 8034114: assertion `left == right` failed: a malformed DECKSTREAK_ROLLOVER_HOUR started as Ok(KernelSettings { .. }), not Err(Malformed { setting: "DECKSTREAK_ROLLOVER_HOUR", expected: "a whole hour from 0 to 23" })
A6: green at fd002b9
A7: red at 8034114: assertion `left == right` failed: DEFAULT_ROLLOVER_HOUR; left: 0, right: 4 (the policy's rollover hour)
A7: green at fd002b9
A8: red at 8034114: assertion `left == right` failed: constants.DEFAULT_ROLLOVER_HOUR; left: Number(0), right: Number(4)
A8: green at fd002b9
A9: red at 8034114: assertion `left == right` failed: 04:00 local turns the study day; left: 0, right: 1
A9: green at fd002b9
A10: red at 8034114: Verdict is not declared #[must_use]: ["#[derive(Clone, Debug, PartialEq, Eq)]"]
A10: green at fd002b9
A11: red at 8034114: assertion `left == right` failed: the loaded secret; left: Some(""), right: Some("tidal-orchid-velvet")
A11: green at fd002b9
A12: red at 8034114: a credential missing from the directory loaded as Ok(Secret(..)), not Err(Missing { id: "telegram-bot-token" })
A12: green at fd002b9
A13: red at 8034114: assertion `left == right` failed: the child's value field; left: String("velvet-orchid-lantern"), right: "***REDACTED***"
A13: green at fd002b9
A14: red at 8034114: assertion `left == right` failed: {"secrets":["kitten-stapler"],"text":"sync as kitten-stapler failed"}; left: Some("sync as kitten-stapler failed"), right: Some("sync as ***REDACTED*** failed")
A14: green at fd002b9
A15: red at 8034114: assertion `left == right` failed: the child's ERROR line; left: None, right: Some("<3>")
A15: green at fd002b9
A16: red at 8034114: assertion `left == right` failed: blocking tasks running at once; left: 5, right: 2
A16: green at fd002b9
A17: red at 8034114: assertion `left == right` failed: journal_mode; left: "delete", right: "wal"
A17: green at fd002b9
A18: red at 8034114: assertion `left == right` failed: the first writer; left: Err("update 0: error returned from database: (code: 5) database is locked"), right: Ok(60)
A18: green at fd002b9
A19: red at 8034114: a write through the read-only opener was not refused as read-only: Ok(SqliteQueryResult { changes: 1, last_insert_rowid: 2 })
A19: green at fd002b9
A20: red at 8034114: assertion `left == right` failed: the first delivery's tables; left: [], right: ["late_first", "late_third"]
A20: green at fd002b9
A21: red at 8034114: examined 0 table(s) of the migrated schema: the population is empty, because Db::open applied no migration
A21: green at fd002b9
A22: red at 8034114: examined 0 table(s) of the planted schema: the planted migration was not applied, so the census refused nothing
A22: green at fd002b9
A23: red at 8034114: assertion `left == right` failed: migration 2001 creates settings_generation, which the ownership register must give to kernel; left: None, right: Some("kernel")
A23: green at fd002b9
A24: red at 8034114: assertion `left == right` failed: a declaration naming minutes_log twice; left: Ok(Declaration { .. }), right: Err(TableDeclaredTwice { context: "habits", table: "minutes_log" })
A24: green at fd002b9
A25: red at 8034114: assertion `left == right` failed: an exemption with the reason ""; left: Ok(Declaration { .. }), right: Err(ExemptWithoutReason { context: "coordination", table: "cron_fires" })
A25: green at fd002b9
A26: red at 8034114: assertion `left == right` failed: the kernel's declaration; left: 0 tables, right: 2
A26: green at fd002b9
```

No test sleeps: A9 drives a `ManualClock`, A16 holds its blocking tasks on channels the test releases
one at a time, and A18 runs two writers against one file in a temporary directory.

The 2026-09-30 amendment (#420) adds A27. The test was committed against the file as it stood
(589bf0bd9fedf6692a6464edfde277cfe5a41b3c) and failed by assertion, then the two rows were removed
(d5bdc47255dce7645382d3c3390d5892af9085d4).

```red-first
A27: red at 589bf0bd9fedf6692a6464edfde277cfe5a41b3c: AssertionError: Lists differ: ["register 'v9 table' repeats buffs", "register 'v9 table' has 65 unique names, prose says 64"] != []
A27: green at d5bdc47255dce7645382d3c3390d5892af9085d4
```

The arm that holds the predecessor's register to the predecessor's 64 tables came after, and its
red is a plant, because the file it guards was already right. A copy of the file with
`xp_settlement` added to the predecessor's register and its count moved to 65 passed the check
without the arm and failed it with the arm, by assertion: `AssertionError: Lists differ:
["register 'v9 table' names xp_settlement, which is not one of the predecessor's"] != []`. The
copy was read through `CONTEXT_MAP_PATH`, and the file itself was not edited.

A later plant reads the section as it renders. Issue #420's two rows, `buffs` and `xp_settlement`, appended to the predecessor's register as a continuation row in each of three spellings (no leading pipe, a one-space indent, a bare name), passed the check before (`Ran 1 test ... OK`, rc 0) and failed it after, by assertion (rc 1): `AssertionError: Lists differ: ["register 'v9 table' repeats buffs", ...] != []`. Each copy was read through `CONTEXT_MAP_PATH`.

A further plant reads the section by CommonMark's block rules. Issue #420's two rows, `buffs` and `xp_settlement`, appended to the predecessor's register in the row starts `` #`x` ``, `` ####### `x` `` and ```` ```x``` ```` (none opens a block on GitHub, so each is a table row) passed the check before (`Ran 1 test ... OK`, rc 0) and failed it after, by assertion (rc 1): `AssertionError: Lists differ: ["register 'v9 table' repeats buffs", ...] != []`. Each copy was read through `CONTEXT_MAP_PATH`.
