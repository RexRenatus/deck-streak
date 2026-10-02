# Red-first record: SPEC-073

This record is 073a's, the first of SPEC-073's three pull requests. It records the twelve criteria
this pull request delivers (A1 to A8, A12, A13, A17 and A18); 073b and 073c add their criteria's
lines when they move them back into the acceptance fence (SPEC-073 section 3c).

The order of work: the AwardOnce model, the Lean package and the next-milestone proofs (5c7d634 to
e0053e2, then c2771d8); the SPEC promoted with ADR-303 (b67da91, 10dc92b, 6d2f9cb); the two tables
with their rights (b6d7246); the registry and the ten goldens (c47f58d); progression's tests beside
inert drafts that compiled and answered nothing (2484fbe) and their implementation (a1dbc80); the
Lean covers and vectors for `milestone.rs` (853e668, stamped at 80369e8, the vectors rewritten at
a669b79); the SPEC's progression rows, each proved KILLED by its full id (33a70da); and the split of
the SPEC into three pull requests (5e6fd0d, de69425).

Each criterion was run at its red commit, selecting its own test, and failed by assertion, not by a
compile error, a missing fixture or an empty selection. The drafts compiled and returned nothing: an
empty catalog, empty descriptions, an award that answered `AlreadyAwarded` and wrote nothing, a key
rule that accepted no key, conditions that held none, zero constants and counts, no records, no
record to chase and an empty milestone. The red was run in the worktree on the red commit's tests
before rustfmt reflowed them; the reflow changed whitespace only. A8's input is abbreviated below:
its `reviews` array is written `[...]`.

The green commit a1dbc80 also changes two test files, and no assertion changed:

- `crates/progression/tests/badges_award.rs`: the two tests that hold a connection drop it before
  `db.close()`, since the pool's close waits for every connection. At 2484fbe the assertions failed
  before the close, so the wait never showed.
- `crates/progression/tests/badges_conditions.rs`: the file allows `clippy::cast_precision_loss`
  with its reason, and one message inlines its argument (`"the count of {input}"`).

a1dbc80 also gives progression's `serde_json` dev-dependency `float_roundtrip`, so A17's
percentages are read bit for bit; it is not a test file.

```red-first
A1: red at 2484fbe: assertion `left == right` failed: the catalog's size; left: 0, right: 40
A1: green at a1dbc80
A2: red at 2484fbe: assertion `left == right` failed: the description of ink_week; left: "", right: "7-day Course Qab writing streak"
A2: green at a1dbc80
A3: red at 2484fbe: assertion `left == right` failed; left: AlreadyAwarded, right: Awarded
A3: green at a1dbc80
A4: red at 2484fbe: assertion `left == right` failed: band_qaa_A1; left: None, right: Some(Band)
A4: green at a1dbc80
A5: red at 2484fbe: no_such_badge is refused: Ok(AlreadyAwarded)
A5: green at a1dbc80
A6: red at 2484fbe: assertion `left == right` failed: first_steps over {"backlog":5,"cleared_backlog":0,"comeback_armed":false,"day_avg_seconds":0.0,"day_decks":0,"day_reviews":0,"due_today":5,"early_bird":0,"iron_will_ok":false,"leech_active":1,"lifetime":1,"mature30_answered":0,"mature30_retention":0.0,"mature_count":0,"night_owl":0,"score_total":0,"streak_current":0,"week_decks":0,"week_retention":0.0,"week_reviews":0,"week_scores":[]}; left: Some(false), right: Some(true)
A6: green at a1dbc80
A7: red at 2484fbe: assertion `left == right` failed: the constant constants.LIFETIME_REVIEWS_GRINDER; left: Some(0.0), right: Some(1000.0)
A7: green at a1dbc80
A8: red at 2484fbe: assertion `left == right` failed: the count of {"day":20000,"end_hour":7,"reviews":[...],"rollover_hour":0,"start_hour":0,"tz_offset_minutes":0}; left: 0, right: 14
A8: green at a1dbc80
A12: red at 2484fbe: assertion `left == right` failed: the records of {"prev":{"best_score":69,"most_minutes":59,"most_reviews":199},"rollups":[{"reviews":200,"score":70,"seconds":3600.0}]}; left: [], right: [("best_score", 70, "Best daily score"), ("most_reviews", 200, "Most reviews in a day"), ("most_minutes", 60, "Most minutes in a day")]
A12: green at a1dbc80
A13: red at 2484fbe: assertion `left == right` failed: the record to chase on {"board":[{"day":19997,"label":"Best daily score","previous":79,"today":70,"value":80}]}; left: None, right: Some(("Best daily score", 10))
A13: green at a1dbc80
A17: red at 2484fbe: assertion `left == right` failed: {"lifetime_reviews":0,"mature_total":0,"streak_days":0}; left: "", right: "Lifetime reviews"
A17: green at a1dbc80
A18: red at 2484fbe: assertion `left == right` failed: 0, 0, 0; left: (Reviews, 0), right: (Streak, 7)
A18: green at a1dbc80
```

## 073b: the coordination steps, their celebrations and the daemon wiring

073b delivers A9, A10, A11, A14, A15, A16 and A19. The order of work: RED-1 `83f5d5e3` (a skeleton of
stubs that compiled and answered nothing, with the tests of A9 and A19 and the daemon's
expected-steps test red beside it), RED-2 `c8646652` (eleven more tests, each red by assertion), and
GREEN `b958e783`. Two commits then followed: `aa38575e` and `62a23487` restamp the covers of
`formal/tla/AwardOnce/` and `formal/tla/RelightOrder/`, and `dc02e83f` is a post-GREEN commit that
changes tests only.

The stubs at RED-1, each of which GREEN replaced:

- `crates/analytics/src/rollup.rs`: `recent_totals` answered an empty list.
- `crates/coordination/src/recompute/mod.rs`: `RecomputeFacts::lifetime_through` answered 0;
  `AwardOffers::offer` answered `Ok` and offered nothing; `impl Celebrate for Router` answered `Ok`
  and sent nothing.
- `crates/coordination/src/recompute/badges.rs`: `badge_key` and `badge_line` answered the empty
  string; `BadgesStep::evaluate` and `offer_badges` answered `Ok` and did nothing.
- `crates/coordination/src/recompute/records.rs`: `RECORDS_WINDOW` was 0; `record_key` and
  `record_line` answered the empty string; `plan` answered its default; `stored_records` answered
  an empty list; `RecordsStep::evaluate` and `offer_records` answered `Ok` and did nothing.
- `crates/coordination/src/progression/badge_context.rs`: `badge_context` answered the default
  context.
- `crates/coordination/src/progression/milestone_view.rs`: `milestone_view` always answered
  `Pending`.
- `crates/coordination/src/progression/records_view.rs`: `records_view` answered the default view.
- `crates/coordination/src/sync_cycle.rs`: the `FoldInput` literal carried `base_reviews: 0` and
  `offers: None`.
- `crates/daemon/src/wiring.rs`: the expected-steps test already expected `progression.badges` and
  `progression.records` in `Phase::Awards`; the registration was not yet made.

The red was run on each red commit's tests, selecting each criterion's own test, and every red below
is an assertion, not a compile error, a missing fixture or an empty selection.

The post-GREEN commit `dc02e83f` adds four tests, green at GREEN's code, and each was seen red by
assertion under a hand mutant of `recompute/badges.rs` or `recompute/records.rs` (each
mutant was applied and restored by hand, and each restore was checked by its hash):

- `badges_steps::an_owed_badge_is_raised_on_the_offers_day_and_marked_at_the_answer` kills B02, B09
  and B10;
- `records_steps::a_best_that_climbs_all_day_before_its_offer_is_not_named` kills R08;
- `records_steps::a_router_that_did_not_answer_leaves_the_record_owed` kills R10, R12 and R13;
- `records_steps::the_owed_records_are_offered_in_the_kinds_order` kills R11, which is not
  equivalent: the SQL `ORDER BY kind` is text order, and the sort gives the kinds' own order.

`dc02e83f` also changes the test-support file `crates/coordination/tests/awards_support/mod.rs`,
which the SPEC's manifest does not name: the Recorder keeps each Celebration it is handed.

The later commit `e935c55c` adds six tests that are mutation coverage, not red-first evidence: it
changes no production file, each test passes there, and each was seen red by assertion under hand
mutants of the seventeen lines the pull request's mutation job read as missed at `4441bd6b`,
applied to an archive of `e935c55c` and restored by hash:

- `rollup_store::the_recent_totals_read_the_latest_rows_on_or_before_the_day`, in analytics, kills
  `recent_totals` replaced by `Ok(vec![])`;
- `records_steps::the_records_view_shows_each_record_against_today` kills `records_view` replaced
  by its default, `today` and `whole_minutes` each replaced by 0, 1 and -1, and the division in
  `whole_minutes` replaced by `%` and by `*`: 2,430 seconds read 40 whole minutes, where those
  two read 30 and 145,800;
- `records_steps::an_owed_record_is_raised_with_its_line` kills `record_line` replaced by
  `"xyzzy"` and by an empty string;
- `records_steps::the_records_step_is_named_progression_records` and
  `badges_steps::the_badge_step_is_named_progression_badges` kill each step's name replaced by
  `""` and by `"xyzzy"`;
- `badges_steps::the_award_offers_print_their_type_without_their_router` kills the award offers'
  `Debug` replaced by `Ok(Default::default())`.

`e935c55c` also changes `crates/analytics/tests/rollup_store.rs`, which the SPEC's manifest does not
name.

Four disclosures:

- GREEN reformats the three RED-2 test files (whitespace only) and adds
  `#[allow(clippy::too_many_lines)]` to A9's test.
- A day with no recorded card state skips the badges that read the card snapshot.
- With an empty records table, the records step passes the window's bests as `stored`, so `plan`
  seeds silently and the golden's "first" case holds.
- `formal/tla/RelightOrder/RelightOrder.tla` also covered `sync_cycle` and `run`. GREEN's hunks there
  move no RelightOrder variable (a stutter); `62a23487` restamped the covers after a re-read, with
  the stutter written into the model.

```red-first
A9: red at 83f5d5e: assertion `left == right` failed: one-day: lifetime; left: 0, right: 6
A9: green at b958e78
A10: red at c864665: assertion `left == right` failed: the new badge is sent: []; left: 0, right: 1
A10: green at b958e78
A11: red at c864665: assertion `left == right` failed; left: [], right: [first_steps, legendary_day, maturity_milestone] at 20000, unmarked
A11: green at b958e78
A14: red at c864665: assertion `left == right` failed: new: the plan; left: an empty plan, right: best_score 85 over 80 and most_minutes 61 over 60, with pr:best_score:20000 and pr:most_minutes:20000
A14: green at b958e78
A15: red at c864665: assertion `left == right` failed: pass 1: seeded at their own values; left: [], right: the three kinds stored at their own values
A15: green at b958e78
A16: red at c864665: assertion `left == right` failed: the new best is offered; left: [], right: ["pr:best_score:20000"]
A16: green at b958e78
A19: red at 83f5d5e: assertion `left == right` failed: 0 reviews, a 0-day streak and 0 mature cards; left: Pending, right: Next(Milestone { ladder: Reviews, current: 0, target: 100, pct: 0.0, remaining: 100 })
A19: green at b958e78
```

## 073b fix round 1: a beat of the seed's own day, and the mark's guard

The RED commit `a5e4cf06` adds four tests and changes no production file; the GREEN commit
`da37395e` changes the records upsert (ADR-303 Decision 2). The reds were run at the code of
`a3693b34`, selecting each test, and each is an assertion:

- `records_steps::a_record_beaten_on_the_seeds_own_day_is_offered` reads `panicked at
  crates/coordination/tests/records_steps.rs:276:5: assertion left == right failed: the beat is
  offered under its day's key, and the seed is not` with `left: []` and `right:
  ["pr:best_score:20000"]`; `test result: FAILED. 12 passed; 1 failed`.
- `record_offers`, in `crates/daemon/tests/`, drives the production fold and reads `panicked at
  crates/daemon/tests/record_offers.rs:148:5` with `left: []` and `right:
  ["pr:best_score:20000", "pr:most_reviews:20000"]`; `test result: FAILED. 0 passed; 1 failed`.
- `badges_steps::a_badge_marked_twice_keeps_its_first_mark` and
  `records_steps::a_record_marked_twice_keeps_its_first_mark` are green at the base, because the
  guard is present there. Each was seen red by assertion under a hand mutant that drops
  `AND celebrated_at IS NULL` from its statement (`panicked at
  crates/coordination/tests/badges_steps.rs:301:5: the first mark is kept`, left
  `Some(1728054000000)`, right `Some(1728050400000)`, and the same message at
  `records_steps.rs:323:5`). The mutants are the rows S07316 and S07317, and S07318 is the CASE
  before the fix.

The criteria these tests decide, A16 and A10, are already recorded once in the block above, at their
first red and green; a criterion takes one entry there, so the reds quoted in this section are
prose, each with its test name, file and line.

## 073c: the routes, the bot commands and the screens

Two RED commits add the tests and only stubs of the production code: `18db650e` mounts the three
routes without the owner check over stub handlers and adds stub bot replies and views, and
`207fc032` adds stub components and readers. `18db650e` adds the Rust tests: A20's route test and
A21's and A22's bot tests, each red by assertion against those stubs: the routes answered without
the owner check and the bot answered the two commands with its help text. `207fc032` adds the web
tests: A23's gallery and A24's records screen tests, red by assertion against stubs that render
nothing. The two refusal tests in `badges.test.ts` and `records.test.ts` are not red at `207fc032`:
the stub readers return null, which every refusal expects; `80b26802`'s positive control makes each
fail against such a reader. The GREEN commit is `c1ba0334`. Each red below was run selecting its
test:

- A20, `crates/api/tests/badges_routes.rs:276:13`: the route was mounted without the owner check, so
  its stub answered 200 where the owner's session was refused.
- A21 and A22, `crates/bot/tests/badges_commands.rs:115:5` and `:146:5`: the bot answered `/badges`
  and `/records` with its help text, as it answers an unknown command.
- A23 and A24: the stubs rendered no badge and no record line.

Five tests were added after GREEN and two were extended; they change the reds above in nothing.
Each was written for a mutant that survived GREEN:

- `424d9cc9` adds refusal cases to `web/app/src/lib/badges/badges.test.ts` and
  `web/app/src/lib/records/records.test.ts` (a field of the wrong type, a study day that is not an
  ISO date, a focus badge read), and drops `badges.ts`'s `typeof family` term, which `includes()`
  already decides: StrykerJS then reads 91 of 91 and 66 of 66 mutants killed in the two parsers.
- `8760278a` adds `badges_routes::the_badge_and_record_routes_name_why_they_cannot_answer` (503
  `database_not_open`, 500 `badges_unreadable` and `records_unreadable`), the coordination view
  tests over a database, and `crates/daemon/tests/badges_route_composed.rs` (the composed role reads
  the configured courses). `distance` clamps with `max(0)`, since `>` and `>=` agree at a gap of 0,
  and row S07323 is re-anchored to it.

One more test edit came after GREEN, by the architect's ruling, and it removes no assertion.
`80b26802` gives the refusal tests in `web/app/src/lib/badges/badges.test.ts` and
`web/app/src/lib/records/records.test.ts` a positive control: each test first asserts that its
untouched fixture parses to its real values, and then runs its `toBeNull` refusals unchanged, so
each refused body, which differs from that fixture in one field, is refused for that field. It
changes no production file and none of the reds above.

```red-first
A20: red at 18db650: assertion `left == right` failed: /api/badges None; left: 200, right: 401
A20: green at c1ba033
A21: red at 18db650: assertion `left == right` failed; left: the help text "These are the commands I answer:", right: "No badges yet — study to earn your first! 👟"
A21: green at c1ba033
A22: red at 18db650: assertion `left == right` failed; left: the help text "These are the commands I answer:", right: "📈 No records yet — they mint themselves as you study."
A22: green at c1ba033
A23: red at 207fc03: AssertionError: expected '' to be '🧘 Monthly Monk 30-day streak 3 of 30'
A23: green at c1ba033
A24: red at 207fc03: AssertionError: expected [] to deeply equal [ …(3) ]
A24: green at c1ba033
```

### Round 2: the coverage written for round 1's survivors

Commits `0ec6ec93`, `263677c5` and `1a4b036b` are MUTATION COVERAGE written for round 1's
survivors: they are green at the base, not red-first, and they change no production line. The rows
commit `ecac3c8b` adds rows S07330-S07341. The mutation package at `1a4b036b` reads `examined 8
plant(s): survived 0, caught 8`, against its control at `5208f1e6`, which reads `examined 8
plant(s): survived 8`.

One survivor is recorded and not killed. `crates/coordination/src/progression/badges_view.rs:282`
(`let mut connection = db.reader().acquire().await?;`) is UNVIABLE-BY-TYPE: no compiling mutant
removes that `?`, because its Ok value is a pool connection with no default to put in its place,
and `earned_badges`' acquire at line 249 (`let mut connection = db.reader().acquire().await?;`)
runs first on the same pool.
