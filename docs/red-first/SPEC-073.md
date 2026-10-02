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
assertion under a hand mutant of `recompute/badges.rs` or `recompute/records.rs` (cargo-mutants is
not installed on the box, so the mutants were applied and restored by hand, and each restore was
checked by its hash):

- `badges_steps::an_owed_badge_is_raised_on_the_offers_day_and_marked_at_the_answer` kills B02, B09
  and B10;
- `records_steps::a_best_that_climbs_all_day_before_its_offer_is_not_named` kills R08;
- `records_steps::a_router_that_did_not_answer_leaves_the_record_owed` kills R10, R12 and R13;
- `records_steps::the_owed_records_are_offered_in_the_kinds_order` kills R11, which is not
  equivalent: the SQL `ORDER BY kind` is text order, and the sort gives the kinds' own order.

`dc02e83f` also changes the test-support file `crates/coordination/tests/awards_support/mod.rs`,
which the SPEC's manifest does not name: the Recorder keeps each Celebration it is handed.

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
