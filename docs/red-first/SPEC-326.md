# Red-first record: SPEC-326

The SPEC, its schematic, ADR-327 and the SPEC-084 amendment were committed first. The seven tests
were then committed alone at d1d11f5d, against the base's coordination, notifications and daemon
code, and each ran by its exact name before any implementation. Every test is written against
the base's public API (no `Router::db`, no reader of the stored streak), so the base builds with
them and each red is by assertion. Each run reads `running 1 test` and `test result: FAILED.
0 passed; 1 failed`.

A1 to A3 read the held row's `tier_pending`, the tier the router decided for a held celebration
(a holding router records the deferred decision itself at T0). A4 to A6 compare the bot port's
calls with those of a twin database flushed by hand with the break's facts; at the base the
production flush passes no facts, so the held line renders as a message rather than as the T1
reaction. A7's first assertion is the route's: at the base the award is routed with no read of
the stored streak, so `celebrate` answers `Ok(())`.

```red-first
A1: red at d1d11f5d: assertion `left == right` failed: a break on the award's day caps it at the policy's cap; left: ["T2"], right: ["T1"]
A2: red at d1d11f5d: assertion `left == right` failed: a break on the day caps the level-up at the policy's cap; left: ["T2"], right: ["T1"]
A3: red at d1d11f5d: assertion `left == right` failed: a break on the relight's day caps it at the policy's cap; left: ["T2"], right: ["T1"]
A4: red at d1d11f5d: assertion `left == right` failed: the job re-caps with the stored streak, as a flush handed the break's facts does; left: ["message: synthetic held celebration"], right: ["reaction to 4242: 🎉"]
A5: red at d1d11f5d: assertion `left == right` failed: the bot's flush re-caps with the stored streak, as a flush handed the break's facts does; left: ["message: synthetic held celebration"], right: ["reaction to 4242: 🎉"]
A6: red at d1d11f5d: assertion `left == right` failed: the cycle's flush re-caps with the stored streak, as a flush handed the break's facts does; left: ["message: synthetic held celebration"], right: ["reaction to 4242: 🎉"]
A7: red at d1d11f5d: a streak that cannot be read leaves the award unanswered, so it stays owed: Ok(())
A1: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
A2: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
A3: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
A4: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
A5: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
A6: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
A7: green at 24a391dd: running 1 test; test result: ok. 1 passed; 0 failed
```

The existing test in `level_up_cycle.rs`, `a_sync_cycle_announces_a_level_reached_once`, passes at
d1d11f5d beside the new one.

Each criterion then ran by its exact name at 24a391dd, the implementation's commit, and passed.
At the same commit `cargo clippy -p deck-streak-coordination -p deck-streak-notifications -p
deck-streak-daemon --all-targets -- -D warnings` and `cargo fmt --check` exit 0, and the DDD
probe reads `DDD lexicon-locks OK: examined 3107 declaration(s) in 120 file(s)`. No test file
changed between the red commit and the green one.

## The mutation rows

Proved at 43edbcb5 by `python3 scripts/mutation_rows.py prove --band S32600-S32699`, which read
`rows: examined 14: killed 14, survived 0, void 0` and exited 0. Each killer ran without the
mutant, selecting one test that passed, then with it, selecting one test that failed, and each
target's bytes were restored and checked by sha256.

```
S32600-THE-FACTS-ARE-THE-LANGUAGE-STREAKS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32601-THE-FACTS-CARRY-THE-CURRENT-LENGTH: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32602-THE-FACTS-CARRY-THE-LONGEST-LENGTH: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32603-THE-FACTS-CARRY-THE-LAST-STUDY-DAY: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32604-THE-OCCASION-CARRIES-THE-FACTS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32605-A-FAILED-READ-ROUTES-NOTHING: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32606-A-FAILED-READ-FLUSHES-NOTHING: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32607-THE-FLUSH-CARRIES-THE-FACTS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32608-AN-AWARD-IS-ROUTED-WITH-THE-FACTS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32609-THE-LEVEL-UP-IS-ROUTED-WITH-THE-FACTS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32610-THE-RELIGHT-IS-ROUTED-WITH-THE-FACTS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32611-THE-HELD-FLUSH-JOB-RE-CAPS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32612-THE-CYCLES-FLUSH-RE-CAPS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
S32613-THE-BOTS-FLUSH-RE-CAPS: KILLED at 43edbcb5: control selected 1 and passed; mutant selected 1 and failed
```
