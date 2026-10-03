# Red-first record: SPEC-083

The order of work: SPEC-083 promoted and the recorder's control (3a0961c); the goldens
(f8d584a); the record's tests beside a stub store and stub pure functions, with the two migrations
(2808dca); their implementation, the rights port, the registrations and the rows (8075b52).

The goldens were generated from the predecessor's own functions at `27ee2bc`, and the predecessor's
checkout was left as it was.

The control A33 is disclosed as not red: its red was measured, not committed. With the classifier
stubbed to return nothing, the test failed by the assertion "the classifier marks an upload", on
commit 3a0961c, so every proof that rests on the recorder's zero rests on a layer shown to see a
planted upload.

Each other criterion was run at its red commit with the SPEC's own fenced command, selecting one
test, and failed by assertion for its own criterion. The stubs compiled: a store that recorded
nothing and refused nothing, a day spec of the bounds as given, a search that was not wrapped or
held, a summary of zeros, and a port that declared no skip table. A2's red is the migration
without its key.

```red-first
A33: not red: measured red at 3a0961c with the classifier stubbed to return None: the classifier marks an upload; committed green only
A1: red at 2808dca: a second take for the day is refused: Ok(SkipId(0))
A1: green at 8075b52
A2: red at 2808dca: the key refuses a second pending or applied skip for one study day
A2: green at 8075b52
A3: red at 2808dca: a skip to undo: NothingToUndo
A3: green at 8075b52
A4: red at 2808dca: assertion `left == right` failed: only an applied skip not undone covers its day (R4); left: []
A4: green at 8075b52
A7: red at 2808dca: assertion `left == right` failed; left: "2-2", right: "2"
A7: green at 8075b52
A8: red at 2808dca: assertion `left == right` failed; left: 1, right: 0
A8: green at 8075b52
A23: red at 2808dca: skip_days is the owner's data: exported and erased (CHARTER 13); left: None
A23: green at 8075b52
```

## The record's second half: the preview, the tariff, its refund and the game (#108)

A33's red was re-measured after the house merge by one named plant, beside the disclosure above,
which stays as it is. At 65b5b0c, in `crates/ingest/tests/support/recording.rs`'s `local_change`,
`carries.then(|| format!("{}: {}", request.method, body))` became
`carries.then(|| format!("{}: {}", request.method, body)).filter(|_| false)`. A33's fenced command
then failed, rc 101, by its assertion "the classifier marks an upload"
(`crates/ingest/tests/recorder_control.rs:66`). The file was restored byte-equal from a copy before
any commit or proof (its sha256 `325e3d6d80d2bd311eaecebcbb3574dca5f33807f0e9ad24df3a64b4377fe5d4`
before and after), and no commit holds the plant.

The criteria A10 to A18, A45 and A46 were written at 8dc8649 against stubs that compile: a tariff
with an empty ladder, a price of 0 and nothing paid; a preview of defaults and settlements that
answer nothing; an empty skip set; and four of the skip constants set one off (the default search
stayed as it is, since `crates/ingest/tests/settings.rs` pins its literal). 16712c8 re-asserted
A15 and A18 where the fold dates a spent freeze, on the day the walk meets the gap, so that each
fails for its own criterion. Each was then run at 16712c8 with its fenced command, selecting one
test, and failed by its own assertion; each passed at 2669bc8.

2669bc8, the green commit, also edits four test files, none of them in an assertion a criterion
here judges: A18's range `D0 - 3..=D0 - 1` is written `D0 - 3..D0`, the same days, for clippy;
`crates/coordination/tests/lapse.rs`'s two calls pass `open_lapse` its new skip set, empty;
`crates/economy/tests/wallet_census.rs` admits `crates/economy/src/tariff.rs`, which reads what a
skip paid, among the files that name the coin ledger; and `crates/coordination/tests/relight_order.rs`
counts 18 statics, the tariff's ladder among them.

```red-first
A10: red at 16712c8: skip_flow.rs:210, the preview's day spec for a golden case; left: "", right: "1-3"
A10: green at 2669bc8
A11: red at 16712c8: skip_flow.rs:277, the settlement for a golden case; left: None, right: Some(Settlement { price: 0, paid: 0, unfunded: false })
A11: green at 2669bc8
A12: red at 16712c8: skip_flow.rs:327, the skip applies and records its shortfall; left: (Pending, false), right: (Applied, true)
A12: green at 2669bc8
A13: red at 16712c8: skip_flow.rs:376, the undo refunds what the skip paid, for a golden case; left: 0, right: 50
A13: green at 2669bc8
A14: red at 16712c8: skip_tariff.rs:27, the ladder is economy.json's; left: [], right: [0, 50, 100]
A14: green at 2669bc8
A15: red at 16712c8: skip_effects.rs:283, the skip day spends no freeze; left: [(19999, -1, "consumed")], right: []
A15: green at 2669bc8
A16: red at 16712c8: skip_effects.rs:332, a missed day tiers the run down; a skip day leaves it as it was; left: [1, 1], right: [1, 2]
A16: green at 2669bc8
A17: red at 16712c8: skip_effects.rs:411, a skip inside three silent days leaves two, and no lapse; left: [Some(19997), Some(19997)], right: [Some(19997), None]
A17: green at 2669bc8
A18: red at 16712c8: skip_effects.rs:442, while the skip stands its day is bridged; left: [(19999, -1, "consumed")], right: []
A18: green at 2669bc8
A45: red at 16712c8: skip_record.rs:297, constants.SKIP_SPREAD_MIN_DAYS equals the predecessor's; left: Number(2), right: Number(1)
A45: green at 2669bc8
A46: red at 16712c8: skip_flow.rs:430, the tariff is charged once, on the skip's day; left: [], right: [(20105, -100)]
A46: green at 2669bc8
```
