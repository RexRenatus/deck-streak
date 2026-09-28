# Red-first record: SPEC-040

The golden came first: `tools/parity-oracle/registry/spec_040.py` registered `level_for_xp`, and the
generator wrote it from the predecessor's own `gamification/xp.py:level_for_xp` at `27ee2bc`
(afe6bc5). The registry module's sha256 read the same before and after the run, and the
predecessor's checkout was left as it was, with no bytecode written into it. Only the new golden
was written; no other golden changed.

A9's census was committed alone (3c37e4a), before any code of the ledger. It examined every crate's
sources and every migration, and found no file that names `xp_ledger`: its positive artifact, the
grant port's repository and progression's migration, did not exist yet.

The other tests were committed (b99166e) beside a progression crate whose public API was in place
and whose behaviour was stubbed: the amount was a signed newtype, the level of every total was 1
and the XP to reach any level 0, every text was accepted as a source, the grant port inserted every
request and answered `Granted`, and the data-rights port declared no table. The migration created
the table with its columns, but without its two unique indexes and without its check on the
amount. Coordination's registry, the symmetry probe's seeds, the register of DeckStreak's own
tables, `privacy.json` and `PRIVACY.md` were committed with the tests. Each criterion was run there
with the SPEC's own fenced command, selecting one test, and failed by assertion for its own
criterion, not by a compile error, a missing fixture or an empty selection. A6's record of the
compiler's refusal was generated with the finished signature, `new(amount: u32)`, and committed with
its test; at b99166e the stub's signed constructor compiled the fixture, which is A6's red.

The implementation followed (1e26a70). Between the red commits and the green one no test changed
what it asserts: at b99166e the census test took one line break inside an expected message, which
reads the same string. After green, the four hand-proved rows (9474038) were proved on the committed
tree: every row KILLED, each target restored byte for byte.

```red-first
A1: red at b99166e: assertion `left == right` failed; left: [Ok(Granted(XpAmount(40))), Ok(Granted(XpAmount(60)))], right: [Ok(Granted(XpAmount(40))), Ok(AlreadyGranted(XpAmount(40)))]
A1: green at 1e26a70
A2: red at b99166e: assertion `left == right` failed; left: [Ok(Granted(XpAmount(60))), Ok(Granted(XpAmount(75)))], right: [Ok(Granted(XpAmount(60))), Ok(AlreadyGranted(XpAmount(60)))]
A2: green at 1e26a70
A3: red at b99166e: assertion `left == right` failed: one key, one row: both tasks wrote their row, 60 and 40; left: 2, right: 1
A3: green at 1e26a70
A4: red at b99166e: assertion `left == right` failed: the level of 100 XP; left: 1, right: 2
A4: green at 1e26a70
A5: red at b99166e: assertion `left == right` failed: level 2 begins at 100 XP; left: (1, 100, 60, 40), right: (2, 100, 60, 40)
A5: green at 1e26a70
A6: red at b99166e: Expected test case to fail to compile, but it succeeded.
A6: green at 1e26a70
A7: red at b99166e: a negative amount was written: Ok(1)
A7: green at 1e26a70
A8: red at b99166e: assertion `left == right` failed: all 13 texts outside the grammar were accepted; left: ("", Ok(())), ("Reading:read:r1", Ok(())) and 11 more, right: each Err("a grant source is an opaque token matching ^[a-z0-9][a-z0-9:._-]{0,127}$")
A8: green at 1e26a70
A9: red at 3c37e4a: assertion `left == right` failed: the grant port and its migration name the table; every file that does: {}; left: {}, right: {"crates/progression/src/ledger.rs", "migrations/004001_progression_xp_ledger.sql"}
A9: green at b99166e
A10: red at b99166e: assertion `left == right` failed: the XP ledger is the owner's data: exported and erased (CHARTER 13); left: [], right: [TableRights { table: "xp_ledger", disposition: ExportAndErase }]
A10: green at 1e26a70
```

In A1 and A2 the stub wrote the replay as a second grant and answered it `Granted` with the replay's
own amount. In A3 both concurrent requests wrote a row. In A4 and A5 the stub level stayed at 1
past the threshold of level 2. In A6 the fixture's `i64` debit compiled. In A7 the table took a
negative amount. In A8 the stub accepted every text as a source. In A9 no file named the table yet.
In A10 the stub port declared no table.

SPEC-021's A2 and A8 (`every_table_of_the_schema_is_declared_by_exactly_one_port` and
`privacy_json_names_every_table_the_ports_export_or_erase`) were red at b99166e too, as SPEC-021
says they are until a new table's port declares it, and green at 1e26a70.
