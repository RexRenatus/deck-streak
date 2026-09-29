# Red-first record: SPEC-084

The goldens came first (733ef70, 0969dc8): `tools/parity-oracle/registry/spec_084.py` registered the
ladder's pure rules and its stateful adapters, and the generator wrote each golden from the
predecessor's own functions at `27ee2bc`, over synthetic inputs, with the predecessor's checkout
left as it was and no bytecode written into it. `origin/dev` was then merged twice as its own
commit (d2fb548, 7e33941), conflicts only, and the symmetry probe learned to seed the owner's latest
message (e239890).

The tests were committed (96383a4) beside inert stubs: the ladder's rules answered T0, an empty
budget, no streak break and no near miss; the router rendered every tier as a line; the bot's port
had its new calls but the stubs never used them; the owner's latest message was never recorded; the
data-rights port declared no `owner_last_message`; the feed's items carried no tier; and the Mini
App's `BEATS` showed nothing at any tier. The migration, the register of DeckStreak's own tables,
`privacy.json`, `PRIVACY.md` and the messages were committed with the tests. Each criterion was
replayed from a `git worktree add` tree with the SPEC's own fenced command, selecting its one test,
and failed by assertion for its own criterion, never by a compile error, a missing fixture or an
empty selection.

The Rust implementation followed (827e105). Between the red commit and it, tests changed in five
ways, none of them in what an acceptance criterion asserts. The golden's calls helper
(`support/ladder.rs:port_calls`) ended a reveal's run at a call of another key, since the rollup's
line is one. SPEC-041's `a_failed_recap_holds_its_rolled_celebration_again` now expects the rolled
row to keep the hold that held it, `quiet`, as the flush's golden does (SPEC-084 R11, §10). The
one-router census names the bot's new call sites and leaves out the parity oracle's tooling, which
ships nothing; the rights test lists `owner_last_message`, reset in place; SPEC-041's degraded-render
test covers the reveal, the dice and the pin; and the bot's tests gained the reaction, the dice and
the pin as Telegram receives them, the owner message's id in the gate's admission, and `sendDice` in
the fake Bot API.

The Mini App followed (5157782), with A16's test pinning the dice and the reduced-motion query by
their whole literals and T0 asserted to render nothing, so a mutant of either constant or of the
T0 guard is killed. After green, A7 and A14 were pinned the same way: the owner's intensity by the
key the predecessor's store uses, and the reveal's placeholder by its whole text. The diff's Rust
mutants then left five alive, and none of them changed what a criterion asserts. A6's test gained
the week's Monday through the router: a T5 delivered the Sunday before spends nothing, and one
delivered on the Monday spends the week's one T5, which kills the three mutants of
`ladder.rs:week_start`. The ladder tests seed the owner's latest message through
`owner_message::record`, the writer the bot records it with, which kills its `Ok(())` mutant. The
guard of `Router::send_bot` is recorded equivalent in
`scripts/mutation-equivalent.d/deck-streak-notifications.json`: a non-celebration reaches it only
at T0 or T2, where the ladder's render and the line make the same call.

```red-first
A1: red at 96383a4: assertion `left == right` failed: the requested tier of "badge" with rarity "common"; left: T0, right: T2
A1: green at 827e105
A2: red at 96383a4: assertion `left == right` failed: the weekly budget of "quiet"; left: (0, 0), right: (1, 0)
A2: green at 827e105
A3: red at 96383a4: panicked at crates/notifications/tests/ladder_tiers.rs:81:5: a band-up is exempt
A3: green at 827e105
A4: red at 96383a4: assertion `left == right` failed: StreakFacts { last_study_day: Some(StudyDay(20000)), current: 1, longest: 2 } on 20000; left: false, right: true
A4: green at 827e105
A5: red at 96383a4: assertion `left == right` failed: the cap of a day the streak broke: false; left: T0, right: T5
A5: green at 827e105
A6: red at 96383a4: assertion `left == right` failed: {"min_tier":4,...,"since_day":19996}; left: 0, right: 8
A6: green at 827e105
A7: red at 96383a4: assertion `left == right` failed: {"event_type":"quest_all",...}: the port's calls, in order; left: [("push_message", None, None)], right: [("push_reveal", None, Some(2.5s))]
A7: green at 827e105
A8: red at 96383a4: assertion `left == right` failed: a message 0 minute(s) old; left: false, right: true
A8: green at 827e105
A9: red at 96383a4: assertion `left == right` failed: a refused reaction holds its celebration; left: Sent { surface: Bot, tier: T2 }, right: Deferred { surface: Bot, hold: Send }
A9: green at 827e105
A10: red at 96383a4: assertion `left == right` failed: Some("two") {...}: the port's calls, in order; left: [("push_message", None, None), ("push_message", None, None)], right: [("push_message", None, None), ("push_dice", Some("🎰"), None), ("push_pin", None, None)]
A10: green at 827e105
A11: red at 96383a4: assertion `left == right` failed: a gap of 5 toward 100; left: false, right: true
A11: green at 827e105
A12: red at 96383a4: assertion `left == right` failed: the streak-break cap is the ladder's cap on a day the streak broke; left: String("T1"), right: String("T0")
A12: green at 827e105
A13: red at 96383a4: assertion `left == right` failed: the owner's message: its id and the instant it arrived; left: None, right: Some(LatestMessage { message_id: 901, arrived_at: UtcMillis(1736911810000) })
A13: green at 827e105
A14: red at 96383a4: assertion `left == right` failed: a record renders as a reveal; left: Sent { surface: Bot, tier: T2 }, right: Sent { surface: Bot, tier: T3 }
A14: green at 827e105
A15: red at 96383a4: assertion `left == right` failed: each item carries the tier it rendered at; left: four items, each with a Null tier, right: T2, T3, T4 and T5
A15: green at 827e105
A16: red at 96383a4: AssertionError: T1: expected [] to deeply equal [ 'message' ]
A16: green at 5157782
A17: red at 96383a4: assertion `left == right` failed: the owner's latest message is reset in place: no message; left: None, right: Some(ResetInPlace { row: {"arrived_at": Null, "message_id": Null} })
A17: green at 827e105
```

In A1, A2, A4 and A11 the stub rules answered T0, an empty budget, no break and no near miss. In A3
the stub stepped a band-up down. In A5 and A12 the stub cap and the policy file's streak-break cap
were not the golden's. In A6 the stub counted nothing. In A7, A10 and A14 the stub router rendered
every tier as a line. In A8 and A9 the stub attempted no reaction and held nothing. In A13 the bot
recorded no message. In A15 the feed's items carried no tier. In A16 the Mini App showed nothing at
any tier. In A17 the port declared no `owner_last_message`.
