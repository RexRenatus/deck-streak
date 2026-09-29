# Red-first record: SPEC-072

The order of work: the SPEC promoted and ADR-072 accepted (6b6b216); the goldens (40b6981); the
census of `xp_settlement` writers and `settle` callers (3672453); ingest's tier tests beside an
inert `parse_tier` (805ac4d) and their implementation (2965fe5); progression's tests beside inert
stubs (a25cc88) and their implementation (de00943); the fold's tests beside inert steps (1703ca1)
and the steps (4b63c0c); the level routes' test beside routes that answered 404 (6d36fc5) and the
routes (3413ecc); the bot's `/level` test beside a menu that answered with the unknown-command help
(9703b06) and the command (8a70fb2); the level screen's tests beside a stub screen (b324a6f) and the
screen (48df2cb); the census fix that reads the settled rows through the crate root (22f2390); the
whole-value pins of the derived registry, the table names and the step names (23d6f7e); and the
SPEC's mutation rows, each proved KILLED by its full id (dfb73e0, then 5ea3f7b).

The goldens were generated from the predecessor's own functions at `27ee2bc`, with a scratch
`--registry` and a scratch `--out`, under `PYTHONDONTWRITEBYTECODE=1`; the predecessor's checkout
was left as it was: no change and no bytecode file added.

Each criterion was run at its red commit, selecting its own test, and failed by assertion, not by a
compile error, a missing fixture or an empty selection. The stubs compiled and returned nothing:
zero XP, no rows, an empty title, a parse that found no tier, steps that settled no day, and a
screen that rendered no list and no region. The web red was run in the worktree; every other red
was replayed from a scratch `git worktree add` tree.

A12's census failed at a25cc88 with the fold's step not yet calling `settle`; its green is 22f2390,
after the crate-root re-export left the census needle to the recompute steps alone.

A12 gained an arm in fix round 1: the census now also reads `SettleRequest`, so a grouped import
of the operation and its request outside the recompute steps is found. Its red is the changed test
beside a planted grouped import in `level_up.rs` (5dfd74a); its green removes the plant (168c835).
The base criterion's own red and green above stay as they were.

A30 (R14) and A31 (R24) were added in fix round 1. A30's test covers code already in the head, so
its red is the test beside a planted swap of the level before and after in `sync_cycle.rs`
(1f4b006) and its green removes the plant (578a351). A31's red is the composed router answering 503
`law_tiers_unavailable` (8467b95) and its green wires the law tiers' source (6d82561).

The pins added after the implementation (`the_derived_registry_and_the_tables_are_pinned_whole`,
`the_step_names_and_the_level_up_kind_are_pinned_whole`, `a_day_settles_the_bonus_sources`) pin
values the implementation already held, so they are not red: their evidence is the mutation row
each one kills (S07211 to S07214, S07218 to S07221).

```red-first
A1: red at a25cc88: assertion `left == right` failed: the XP of {"ease":1,"ivl":0,"rtype":0,"tier":null}; left: 0, right: 4
A1: green at de00943
A2: red at a25cc88: the constant constants.XP_BASE: 0.0 against 10
A2: green at de00943
A3: red at 805ac4d: assertion `left == right` failed: the tier of " T1 Subject::Topic "; left: None, right: Some("T1")
A3: green at 2965fe5
A4: red at 805ac4d: assertion `left == right` failed; left: [(1, None), (2, None), (3, None), (4, None)], right: [(1, Some("T3")), (2, None), (3, None), (4, Some("T4"))]
A4: green at 2965fe5
A5: red at 1703ca1: assertion `left == right` failed: the language source holds the language card's reviews and no law review; left: None, right: Some(34)
A5: green at 4b63c0c
A6: red at 1703ca1: assertion `left == right` failed; left: None, right: Some(24)
A6: green at 4b63c0c
A7: red at a25cc88: assertion `left == right` failed: a recompute over less leaves the closed day's amount; left: 60, right: 100
A7: green at de00943
A8: red at a25cc88: assertion `left == right` failed; left: [], right: [("reviews", 40)]
A8: green at de00943
A9: red at a25cc88: assertion `left == right` failed; left: [], right: [("reviews", 90)]
A9: green at de00943
A10: red at a25cc88: assertion `left == right` failed: one row per day, source and track, however often it settles; left: 0, right: 3
A10: green at de00943
A11: red at a25cc88: a grant's source is not a derived one: 10
A11: green at de00943
A12: red at a25cc88: the fold's XP step calls settle; every file that does: {}
A12: green at 22f2390
A12: red at 5dfd74a: assertion `left == right` failed; left: ["crates/coordination/src/level_up.rs calls settle outside the recompute steps, and only the owner's correction may"], right: []
A12: green at 168c835
A13: red at a25cc88: assertion `left == right` failed: both tables; left: 60, right: 100
A13: green at de00943
A14: red at a25cc88: assertion `left == right` failed: the bonuses of {"backlog_zero":true,"graduations":3,"score_total":95,"streak_days":7,"studied":true}; left: []
A14: green at de00943
A15: red at 1703ca1: assertion `left == right` failed: the backfilled day has no snapshot, the closing day and the current day have; left: [(20000, None), (20001, None), (20002, None)]
A15: green at 4b63c0c
A16: red at a25cc88: assertion `left == right` failed; left: "", right: "Sprout"
A16: green at de00943
A17: red at 1703ca1: the recompute's XP crosses a level
A17: green at 4b63c0c
A18: red at 1703ca1: the days earned XP
A18: green at 4b63c0c
A19: red at a25cc88: assertion `left == right` failed: the run of {"day_results":[[10,false],[10,false],[70,false]]}; left: 0, right: 1
A19: green at de00943
A20: red at a25cc88: assertion `left == right` failed: the base of the two-table day; left: 0, right: 149
A20: green at de00943
A21: red at a25cc88: assertion `left == right` failed; left: 0, right: 550
A21: green at de00943
A22: red at a25cc88: assertion `left == right` failed: a reading's grant is not review XP; left: 0, right: 300
A22: green at de00943
A23: red at a25cc88: assertion `left == right` failed: the consistency bonus of {"base":1000,"buff":false,"reviews":800,"reviews_law":0,"run":1}; left: 0, right: 149
A23: green at de00943
A24: red at a25cc88: assertion `left == right` failed: the arming of {"backlog_zero":100,"buff":false,"rollup_reviews":40,"skip":false}; left: false, right: true
A24: green at de00943
A25: red at a25cc88: the same day, not a skip, is armed
A25: green at de00943
A26: red at 6d36fc5: assertion `left == right` failed: /api/level None; left: 404, right: 401
A26: green at 3413ecc
A27: red at 9703b06: Level 1: Sprout in These are the commands I answer (the unknown-command help)
A27: green at 8a70fb2
A28: red at b324a6f: TestingLibraryElementError: Unable to find an accessible element with the role "list" and name "Today's XP by source"
A28: green at 48df2cb
A29: red at b324a6f: TestingLibraryElementError: Unable to find an accessible element with the role "region" and name "Consistency run"
A29: green at 48df2cb
A30: red at 1f4b006: assertion `left == right` failed: one line, for the level reached; left: [], right: ["🐣 Level 3: Sprout"]
A30: green at 578a351
A31: red at 8467b95: assertion `left == right` failed: the composed router answers the owner; left: 503, right: 200
A31: green at 6d82561
```
