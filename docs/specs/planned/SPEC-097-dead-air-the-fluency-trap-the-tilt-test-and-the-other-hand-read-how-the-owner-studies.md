# SPEC-097: Dead Air, the Fluency Trap, the Tilt Test and the Other Hand read how the owner studies

- **Wave:** W4. **Issues:** #143, #146, #148, #151 (epic #5). **Context(s):** `deck-streak-kernel`
  (`pynum` gains CPython's `shuffle` and `math.erfc`); `deck-streak-ingest` (the
  tilt pairs and the provenance counts); `deck-streak-insights` (the four instruments and their
  registry rows); `deck-streak-coordination` (the session spans passed to Dead Air, the reads'
  floors, the on-demand runs); `deck-streak-bot` (`/fluency`, `/tilt`, `/otherhand`); the Mini App
  (four sections of the insights screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-090 (CPython's numeric semantics
  live once, in the kernel), ADR-094 (the frame: weekly and on-demand runs, one latest report per
  instrument), ADR-097 (a skip day's rows are attributed by its card snapshot and window) and ADR-095 (the reads keep SPEC-023's scope, and only a read of answers keeps its
  window).
- **Prerequisites:** SPEC-023 (the read), SPEC-029, SPEC-081 (the sessions), SPEC-083 (the skip record and its card snapshot; the skip day writes one type-4 review-log row per moved card, SPEC-083 R18 under ADR-089), SPEC-090 (`pynum`), SPEC-094 (the frame) and SPEC-095
  (the Mersenne Twister and the review reads). **Mutation band:** `S09700-S09799`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-097.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** The insights crate holds no code at `dev` dd98601; SPEC-094 and SPEC-095
  (planned) build the frame and the first weekly instruments.
- **What is ported.** Dead Air (`deadair.py:build_deadair_report` and `gap_ms`, over the spans of
  `gamification/chests.py:session_bounds_from_reviews`), the Fluency Trap (`fluency.py:audit`),
  the Tilt Test (`tilt.py:read_tilt_rows` and `build_tilt_report`) and the Other Hand
  (`provenance.py:compute_provenance` over `anki_reader.py:read_due_date_provenance`).
- **Cadence.** Dead Air is weekly and depends on the weekly report (#130), so W4 stores it
  (ADR-094). The other three are on demand: the predecessor serves them from a command and a read
  tool (`pipeline_layers/read_api.py:ReadApiLayer`, `bot.py:CommandBot`).
- **Traps a hand port falls into.**
  - Dead Air's gap is `(id - time) - previous id` inside one session only, never across two, and a
    negative gap is counted in its own bucket, never clamped. The buckets split at 5, 30, 120 and
    600 seconds; the post-gap pass rates compare gaps of 120 seconds or more with gaps under 5
    seconds, and need 30 observations in each arm. Its window is 30 days, and answer time is
    capped as effort caps it.
  - Dead Air's sessions are SPEC-081's: its spans split at a gap of 10 minutes and are inclusive at
    both ends.
  - The Fluency Trap's snap threshold is the smaller of 1500 ms and the whole part of 0.35 times the
    median answer time (`statistics.median`, so an even count averages the middle two). A passing
    type-1 answer at or under it is snap-Good; its outcome is the same card's first type-1 answer
    on a LATER study day, and a same-day re-show is never one. The verdict's p is a pooled
    two-proportion test through `math.erfc`; the window is 180 days, and the per-deck rows need 25
    reviews and show 5.
  - The Tilt Test's estimate is the mean over cards of each card's own after-a-miss miss rate minus
    its after-a-pass miss rate, never a pooled rate. Its intervals resample cards, its permutation
    test shuffles outcomes within each card's interval band (young under 21 days, mature at 21 or
    more), and each generator is seeded `20260806` and consumed in the predecessor's order. The
    resamples shrink to fit the two work caps, never below 200. `statistics.mean` rounds the exact
    mean once, which a running float sum does not; SPEC-090 ports it.
  - The tilt pairs lag each study event against the same card's previous one over the whole scoped
    log, and count only answers after the window's floor (ADR-095).
  - The Other Hand counts manual and reschedule rows (types 4 and 5) against all rows, fetching at
    most 100,000 manual rows and naming a truncation; a bulk act is a run of such rows with gaps
    under 60 seconds.
- **Corrections to the issues.**
  - #151 attributes rows to the service's skip days by their moved cards, and so does this census: DeckStreak's skip day writes one type-4 review-log row per card it moves (SPEC-083 R18, ADR-089), so a manual row belongs to a skip day when its card is in that skip's snapshot and its id falls between that skip's creation and the next skip's (`provenance.py:build_skip_evidence`, ADR-097), and an undone skip's rows are named undone; every other manual row is the owner's edits or an earlier tool's.
  - #151 says the census is whole-collection. Here it covers the owner's scope (ADR-095), which is
    the whole collection when `DECKSTREAK_INCLUDE_DECKS` is unset; the report states which, and
    over which window.
- **What the parity oracle proves.** The numeric additions over edge cases; Dead Air's gap and
  report; the fluency audit over each verdict; the tilt report over each verdict and the sweep; the
  provenance census with and without truncation; and the constants.
- **Prerequisites.** SPEC-023, SPEC-029, SPEC-081, SPEC-083, SPEC-090, SPEC-094 and SPEC-095, as the
  header names them.

## 2. Requirements

The numbers (ADR-090)

R1. `pynum` gains the Mersenne Twister's `shuffle` (each index drawn by
    rejection over `getrandbits`, as CPython's `_randbelow` draws it) and `math.erfc`, equal to
    `goldens/pynum_shuffle_erfc.json`; the Tilt Test's means are SPEC-090's `statistics.mean`.

Dead Air (#143)

R2. Coordination passes Dead Air the study events of the last 30 days, read through SPEC-023's read
    with that floor, and the spans of SPEC-081's sessions over them, which equal
    `goldens/session_bounds.json` (`chests.py:session_bounds_from_reviews`).
R3. The gap equals `goldens/dead_air_gap.json` (`deadair.py:gap_ms`), and the report equals
    `goldens/dead_air_report.json` (`deadair.py:build_deadair_report`). Dead Air is weekly, and its
    constants (the bucket edges, 120,000 and 5,000 ms, 30 observations, pass at ease 2 and 30 days)
    equal `goldens/dead_air.constants.json`.

The Fluency Trap (#146)

R4. The audit equals `goldens/fluency_audit.json` (`fluency.py:audit`) over the scoped reviews and
    cards of the last 180 days, read through SPEC-023's read with that floor. Its verdict is
    `insufficient_data`, `inflates`, `no_effect` or `protects`, and it flags the conflict with the
    speed badge when it inflates. Its constants (1500 ms, 0.35, 30, 180, 5 and 25) equal
    `goldens/fluency.constants.json`.

The Tilt Test (#148)

R5. `crates/ingest/src/review_reads.rs` reads the tilt pairs: each study event after the window's
    floor with its card, ease, last interval, home deck and the same card's previous ease over the
    whole scoped log, equal to `goldens/tilt_rows.json` (`tilt.py:read_tilt_rows` over a synthetic
    collection).
R6. The report equals `goldens/tilt_report.json` (`tilt.py:build_tilt_report`), with the verdict
    `null`, `no_effect` or `effect_detected` and the sign read from the paired delta. Its constants
    (20 cards, the sweep 1, 2, 3, 5 and 10, 2000 and 2000 resamples, the seeds, 0.05 and the two
    work caps) equal `goldens/tilt.constants.json`.

The Other Hand (#151)

R7. `crates/ingest/src/review_reads.rs` reads the provenance counts: all rows and manual rows since
    the window's floor, the manual rows' ids and cards up to 100,000, and the cards whose first row
    is manual, keeping the scope, equal to `goldens/provenance_counts.json`
    (`anki_reader.py:read_due_date_provenance` over a synthetic collection).
R8. The census equals `goldens/provenance_report.json` (`provenance.py:compute_provenance`) with an empty skip ledger, and equals `goldens/provenance_report_skips.json` over a synthetic ledger of two skips, one undone, read from ingest's `skip_days` and its card snapshot (SPEC-083 R1, ADR-097). The report states the rows it attributes to DeckStreak's skip days, whether
    it covers the whole collection or the owner's scope, and its window; its constants (100,000 and
    60,000 ms) equal `goldens/provenance.constants.json`.

The surfaces

R9. The Fluency Trap, the Tilt Test and the Other Hand are on-demand instruments (SPEC-094 R8).
    `/fluency`, `/tilt` and `/otherhand` run their instrument and answer with its headline, or that
    a run is in progress; the insights screen offers the same run on each of their sections.
R10. The insights screen shows each report as a section: Dead Air's attention share, gap buckets
     and post-gap rates, or each refusal with its reason; the Fluency Trap's verdict and deck rows;
     the Tilt Test's verdict with its sign, the sweep and the raw gap labelled as confounded, and
     its statement that the association is not a cause; and the Other Hand's counts, bulk acts and
     truncation with its coverage statement. A report with a failed read renders its failure.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `shuffle` and `erfc` equal CPython's for every golden case | `shuffle_and_erfc_match_cpythons_golden` |
| A2 | the session spans equal the golden | `the_session_spans_match_the_predecessors_golden` |
| A3 | the gap equals the golden, never crossing a session | `the_dead_air_gap_matches_the_predecessors_golden` |
| A4 | Dead Air's report equals the golden | `the_dead_air_report_matches_the_predecessors_golden` |
| A5 | Dead Air's constants equal the predecessor's | `the_dead_air_constants_equal_the_predecessors` |
| A6 | the fluency audit equals the golden | `the_fluency_audit_matches_the_predecessors_golden` |
| A7 | the audit reads inflates, protects and no effect on synthetic data | `the_audit_reads_inflates_protects_and_no_effect` |
| A8 | the fluency constants equal the predecessor's | `the_fluency_constants_equal_the_predecessors` |
| A9 | the tilt pairs over a synthetic collection equal the predecessor's reader | `the_tilt_pairs_match_the_predecessors_reader` |
| A10 | the tilt report equals the golden | `the_tilt_report_matches_the_predecessors_golden` |
| A11 | each tilt verdict appears on synthetic data | `each_tilt_verdict_appears_on_synthetic_data` |
| A12 | the tilt constants equal the predecessor's | `the_tilt_constants_equal_the_predecessors` |
| A13 | the provenance counts over a synthetic collection equal the predecessor's reader | `the_provenance_counts_match_the_predecessors_reader` |
| A14 | the census equals the golden | `the_provenance_census_matches_the_predecessors_golden` |
| A15 | the provenance constants equal the predecessor's | `the_provenance_constants_equal_the_predecessors` |
| A16 | Dead Air runs weekly over the sessions' spans | `dead_air_runs_weekly_over_the_sessions_spans` |
| A17 | each on-demand command runs its instrument or answers that a run is in progress | `each_on_demand_command_runs_its_instrument` |
| A18 | the Other Hand states its coverage and the rows it attributes to DeckStreak's skip days | `states its coverage and the rows deckstreak's skip days wrote` |
| A19 | the Tilt Test reads its verdict with its sign and never as a cause | `reads the verdict with its sign and never as a cause` |
| A20 | a Dead Air refusal renders its reason | `renders each refusal with its reason` |
| A21 | the Fluency Trap renders its deck rows and verdict | `renders the verdict and the deck rows` |
| A22 | the census over a ledger of two skips, one undone, equals the golden | `the_provenance_census_with_skips_matches_the_predecessors_golden` |

```acceptance
A1: cargo test -p deck-streak-kernel --test pynum_statistics -- --exact shuffle_and_erfc_match_cpythons_golden
A2: cargo test -p deck-streak-coordination --test dead_air_step -- --exact the_session_spans_match_the_predecessors_golden
A3: cargo test -p deck-streak-insights --test dead_air -- --exact the_dead_air_gap_matches_the_predecessors_golden
A4: cargo test -p deck-streak-insights --test dead_air -- --exact the_dead_air_report_matches_the_predecessors_golden
A5: cargo test -p deck-streak-insights --test dead_air -- --exact the_dead_air_constants_equal_the_predecessors
A6: cargo test -p deck-streak-insights --test fluency -- --exact the_fluency_audit_matches_the_predecessors_golden
A7: cargo test -p deck-streak-insights --test fluency -- --exact the_audit_reads_inflates_protects_and_no_effect
A8: cargo test -p deck-streak-insights --test fluency -- --exact the_fluency_constants_equal_the_predecessors
A9: cargo test -p deck-streak-ingest --test study_reads -- --exact the_tilt_pairs_match_the_predecessors_reader
A10: cargo test -p deck-streak-insights --test tilt -- --exact the_tilt_report_matches_the_predecessors_golden
A11: cargo test -p deck-streak-insights --test tilt -- --exact each_tilt_verdict_appears_on_synthetic_data
A12: cargo test -p deck-streak-insights --test tilt -- --exact the_tilt_constants_equal_the_predecessors
A13: cargo test -p deck-streak-ingest --test study_reads -- --exact the_provenance_counts_match_the_predecessors_reader
A14: cargo test -p deck-streak-insights --test other_hand -- --exact the_provenance_census_matches_the_predecessors_golden
A15: cargo test -p deck-streak-insights --test other_hand -- --exact the_provenance_constants_equal_the_predecessors
A16: cargo test -p deck-streak-coordination --test dead_air_step -- --exact dead_air_runs_weekly_over_the_sessions_spans
A17: cargo test -p deck-streak-bot --test instrument_commands -- --exact each_on_demand_command_runs_its_instrument
A18: pnpm exec vitest run web/app/src/lib/insights/OtherHandSection.test.ts -t "states its coverage and the rows deckstreak's skip days wrote"
A19: pnpm exec vitest run web/app/src/lib/insights/TiltSection.test.ts -t "reads the verdict with its sign and never as a cause"
A20: pnpm exec vitest run web/app/src/lib/insights/DeadAirSection.test.ts -t "renders each refusal with its reason"
A21: pnpm exec vitest run web/app/src/lib/insights/FluencySection.test.ts -t "renders the verdict and the deck rows"
A22: cargo test -p deck-streak-insights --test other_hand -- --exact the_provenance_census_with_skips_matches_the_predecessors_golden
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges this over the committed tree and posts its
verdict on the pull request as the `box/packs` status. It has no line in the acceptance fence,
because no public test can run a pack's row. This SPEC adds no table: the reports stay in SPEC-094's
`instrument_reports`. The accessibility pack stays enforced; no check is deferred or lifted for this
delivery, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over every file under `web/app/src/lib/insights/`: the four new sections and their run buttons pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/pynum.rs` | `deck-streak-kernel` | changed: `shuffle` and `erfc` (ADR-090) |
| `crates/kernel/tests/pynum_statistics.rs` | `deck-streak-kernel` | added: A1 |
| `crates/ingest/src/review_reads.rs` | `deck-streak-ingest` | changed: the tilt pairs and the provenance counts |
| `crates/ingest/tests/study_reads.rs` | `deck-streak-ingest` | added: A9, A13 |
| `crates/insights/src/dead_air.rs` | `deck-streak-insights` | added: Dead Air |
| `crates/insights/src/fluency.rs` | `deck-streak-insights` | added: the Fluency Trap |
| `crates/insights/src/tilt.rs` | `deck-streak-insights` | added: the Tilt Test |
| `crates/insights/src/other_hand.rs` | `deck-streak-insights` | added: the Other Hand |
| `crates/insights/src/registry.rs` | `deck-streak-insights` | changed: one weekly and three on-demand rows |
| `crates/insights/src/lib.rs` | `deck-streak-insights` | changed: the modules |
| `crates/insights/tests/dead_air.rs` | `deck-streak-insights` | added: A3 to A5 |
| `crates/insights/tests/fluency.rs` | `deck-streak-insights` | added: A6 to A8 |
| `crates/insights/tests/tilt.rs` | `deck-streak-insights` | added: A10 to A12 |
| `crates/insights/tests/other_hand.rs` | `deck-streak-insights` | added: A14, A15, A22 |
| `crates/coordination/src/instruments.rs` | `deck-streak-coordination` | changed: the floors, the sessions' spans and the on-demand runs |
| `crates/coordination/tests/dead_air_step.rs` | `deck-streak-coordination` | added: A2, A16 |
| `crates/bot/src/instrument_commands.rs` | `deck-streak-bot` | added: the three on-demand commands |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the commands join the table |
| `crates/bot/tests/instrument_commands.rs` | `deck-streak-bot` | added: A17 |
| `web/app/src/lib/insights/DeadAirSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/DeadAirSection.test.ts` | miniapp | added: A20 |
| `web/app/src/lib/insights/FluencySection.svelte` | miniapp | added |
| `web/app/src/lib/insights/FluencySection.test.ts` | miniapp | added: A21 |
| `web/app/src/lib/insights/TiltSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/TiltSection.test.ts` | miniapp | added: A19 |
| `web/app/src/lib/insights/OtherHandSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/OtherHandSection.test.ts` | miniapp | added: A18 |
| `web/app/src/lib/insights/insights.ts` | miniapp | changed: the four reports' types and the run call |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `tools/parity-oracle/registry/spec_097.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/pynum_shuffle_erfc.json` | repo | added: CPython's `random.Random.shuffle` and `math.erfc` (adapter) |
| `tools/parity-oracle/goldens/session_bounds.json` | repo | added: the golden of `chests.py:session_bounds_from_reviews` (adapter; synthetic reviews) |
| `tools/parity-oracle/goldens/dead_air_gap.json` | repo | added: the golden of `deadair.py:gap_ms` (adapter) |
| `tools/parity-oracle/goldens/dead_air_report.json` | repo | added: the golden of `deadair.py:build_deadair_report` (adapter; synthetic reviews and sessions) |
| `tools/parity-oracle/goldens/dead_air.constants.json` | repo | added: Dead Air's constants (constants) |
| `tools/parity-oracle/goldens/fluency_audit.json` | repo | added: the golden of `fluency.py:audit` (adapter; synthetic reviews, cards, decks and calendar) |
| `tools/parity-oracle/goldens/fluency.constants.json` | repo | added: the Fluency Trap's constants (constants) |
| `tools/parity-oracle/goldens/tilt_rows.json` | repo | added: the golden of `tilt.py:read_tilt_rows` (adapter; a temporary synthetic collection) |
| `tools/parity-oracle/goldens/tilt_report.json` | repo | added: the golden of `tilt.py:build_tilt_report` (adapter; synthetic rows for each verdict) |
| `tools/parity-oracle/goldens/tilt.constants.json` | repo | added: the Tilt Test's constants (constants) |
| `tools/parity-oracle/goldens/provenance_counts.json` | repo | added: the golden of `anki_reader.py:read_due_date_provenance` (adapter; a temporary synthetic collection) |
| `tools/parity-oracle/goldens/provenance_report.json` | repo | added: the golden of `provenance.py:compute_provenance` (adapter; an empty skip ledger) |
| `tools/parity-oracle/goldens/provenance_report_skips.json` | repo | added: the golden of `provenance.py:compute_provenance` with `build_skip_evidence` (adapter; a synthetic ledger of two skips, one undone, and their card snapshots) |
| `tools/parity-oracle/goldens/provenance.constants.json` | repo | added: the Other Hand's constants (constants) |
| `scripts/mutation-rows.d/S09700-S09799.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-097-dead-air-the-fluency-trap-the-tilt-test-and-the-other-hand-read-how-the-owner-studies.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-097.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no weekly report; Dead Air's stored report waits for it (#130).
- It revives neither the Churn Tax (#173), the Latency Debt audit (#174), the Two-Button Grading
  audit (#175) nor the Shuffle Test (#176), each inert in v9 and waiting for the owner's decision.
- It serves no instrument to the agent's machine read tool (#157).
- It imports none of the predecessor's reports (#61).

## 6. Risks

- **A gap straddles two sessions.** Detected by A3, whose golden holds two sessions an overnight
  apart.
- **A pooled tilt rate replaces the paired estimate** and flips the sign. Detected by A10.
- **A float running sum stands in for `statistics.mean`.** Detected by SPEC-090's A1 and by A10.
- **The Other Hand claims a row as DeckStreak's.** Prevented by the snapshot-and-window rule (ADR-097) and detected
  by A22 and A18.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_097.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `pynum_shuffle_erfc` | CPython's `random.Random.shuffle`, `math.erfc` | adapter | shuffles of lists of 1 to 40 items under the tilt seed, and `erfc` across its range |
| `session_bounds` | `chests.py:session_bounds_from_reviews` | adapter | synthetic reviews with gaps at and around 10 minutes |
| `dead_air_gap` | `deadair.py:gap_ms` | adapter | review pairs with positive, zero and negative gaps |
| `dead_air_report` | `deadair.py:build_deadair_report` | adapter | synthetic reviews and sessions, each refusal and each bucket |
| `fluency_audit` | `fluency.py:audit` | adapter | synthetic reviews, cards, deck names and a calendar for each verdict, an even median and a same-day re-show |
| `tilt_rows` | `tilt.py:read_tilt_rows` | adapter | a temporary collection file with study events and manual rows between them |
| `tilt_report` | `tilt.py:build_tilt_report` | adapter | synthetic rows for each verdict, a failed read and both work caps |
| `provenance_counts` | `anki_reader.py:read_due_date_provenance` | adapter | a temporary collection file with manual, reschedule and study rows, and a small cap |
| `provenance_report` | `provenance.py:compute_provenance` | adapter | synthetic rows, deck names and an empty skip ledger, truncated and not |
| `provenance_report_skips` | `provenance.py:compute_provenance`, `build_skip_evidence` | adapter | synthetic rows and a ledger of two skips, one undone, with card snapshots, rows inside and outside each window |
| `dead_air.constants`, `fluency.constants`, `tilt.constants`, `provenance.constants` | the modules' constants | constants | none |

## 8. Tables and the v9 import

This SPEC adds no table. Every report is stored in SPEC-094's `instrument_reports`, and none is
imported (#61).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09701-GAP-SUBTRACTS-TIME` | `crates/insights/src/dead_air.rs` | the gap subtracts the answer's time | `dead_air::the_dead_air_gap_matches_the_predecessors_golden` |
| `S09702-NEGATIVE-GAP-KEPT` | `crates/insights/src/dead_air.rs` | a negative gap is counted, never clamped | `dead_air::the_dead_air_report_matches_the_predecessors_golden` |
| `S09703-SNAP-THRESHOLD` | `crates/insights/src/fluency.rs` | the smaller of 1500 ms and 0.35 of the median | `fluency::the_fluency_audit_matches_the_predecessors_golden` |
| `S09704-LATER-DAY-FOLLOW-UP` | `crates/insights/src/fluency.rs` | a follow-up is on a later study day | `fluency::the_audit_reads_inflates_protects_and_no_effect` |
| `S09705-PAIRED-DELTA` | `crates/insights/src/tilt.rs` | the per-card delta is averaged, never pooled | `tilt::the_tilt_report_matches_the_predecessors_golden` |
| `S09706-MIN-RESAMPLES` | `crates/insights/src/tilt.rs` | the resamples never fall below 200 | `tilt::the_tilt_constants_equal_the_predecessors` |
| `S09707-ACT-GAP` | `crates/insights/src/other_hand.rs` | a bulk act splits at 60 seconds | `other_hand::the_provenance_census_matches_the_predecessors_golden` |
| `S09708-TILT-LAG-BEFORE-FLOOR` | `crates/ingest/src/review_reads.rs` | the previous ease is lagged before the window's floor applies | `study_reads::the_tilt_pairs_match_the_predecessors_reader` |
