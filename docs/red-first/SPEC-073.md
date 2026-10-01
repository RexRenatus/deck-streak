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
