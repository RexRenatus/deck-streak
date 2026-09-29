# SPEC-098: Divestment Day, the hand, the sabbatical clock and the runway price the owner's time

- **Wave:** W4. **Issues:** #147, #149, #150, #144 (epic #5). **Context(s):** `deck-streak-ingest`
  (the runway's budget reads); `deck-streak-insights` (the four instruments, the elapsed time to a
  retrievability floor and the cold floor, their registry rows); `deck-streak-coordination` (the
  inputs each instrument receives from analytics, curriculum and ingest, the on-demand runs, the
  hand's cleared marks); `deck-streak-bot` (`/divest`, `/hand`, `/park`); the Mini App (four
  sections of the insights screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-085 (charts render on the
  client), ADR-090 (CPython's numeric semantics live once, in the kernel), ADR-094 (the frame:
  weekly and on-demand runs, one latest report per instrument), ADR-095 (the reads keep SPEC-023's
  scope, and only a read of answers keeps its window) and ADR-098 (the clock ports two primitives of
  an inert module and nothing else of it).
- **Prerequisites:** SPEC-023 (the read and its scope), SPEC-029, SPEC-071 (the courses, the daily
  rollup, the leech threshold), SPEC-077 (each card's memory state), SPEC-085 (the chart loader),
  SPEC-090 (`pynum`: the median, the mean and the percentile), SPEC-091 (the collection's desired
  retention), SPEC-092 (the law subject), SPEC-094 (the frame and the wire walk) and SPEC-095 (the
  presets and the deck kinds). **Mutation band:** `S09800-S09899`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-098.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 the insights crate holds no code; SPEC-094 and SPEC-095
  (planned) build the frame, the wire walk and the preset and deck-kind reads. No read groups the
  cards by home deck, queue and type, and none reads the collection's parent-limits flag.
- **What is ported.** Divestment Day (`divest.py:report` with `_classify`, `_price_card` and
  `_dormant_decks`), the hand (`hand.py:build_hand`, `deal_hand`, `hand_size`,
  `render_search_string`, `measured_seconds_per_card` and `estimated_minutes`), the sabbatical
  clock (`sabbatical.py:compute_sabbatical` over `peak.py:elapsed_at_floor` and
  `fsrs.py:clamped_decay_exponent`) and the runway (`runway.py:read_runway_rows`, `limit_for_deck`
  and `build_runways`).
- **Cadence.** Divestment Day, the hand and the clock are on demand: the predecessor serves them from
  a command and a read tool (`pipeline_layers/read_api.py:ReadApiLayer.divest_report`,
  `deal_hand_report` and `park_report`). The runway is a weekly section and depends on the weekly
  report (#130), so W4 computes and stores it (ADR-094).
- **Traps a hand port falls into.**
  - Divestment Day's classification is a chain, and a card lands in one section at most. A suspended
    card (queue −1) is never classified; then a lapse count at or above the leech threshold is a
    leech sink; then 12 or more repetitions with a difficulty of 8.0 or more is churn; then an
    interval of 365 days or more is overkill.
  - A card's price is its own mean capped answer time when it has 3 or more study events in the
    window, else the window's median capped answer time; its repetitions a year come from the window
    when it has 2 or more, else from 365.25 over its interval, else 12 for a card with no interval.
    The mean and the median are CPython's (SPEC-090), and each answer is capped as effort caps it.
  - A dormant deck is a top-level deck with 200 or more new, unsuspended cards and no study event in
    the window. It is priced over its new cards that no other section claimed, times 8 lifetime
    reviews, times the window's median answer time; a deck priced at zero hours is dropped.
  - Each section sorts by hours descending, then by card id (by deck name for dormant decks), and
    keeps 8 rows; its hours sum every row, not only the 8 kept. The renderer hides a section and a
    row priced at zero hours.
  - The hand's size is the nearest-rank 20th percentile of the answered counts of the last 60
    stored rollups that have any, needs 14 samples (else 15), and is clamped to 5..191, where 191 is
    `(4096 − 256 − 4) // 20`, so the `cid:` search fits one message.
  - The hand excludes queues −1, −2 and −3 only, so a new card stays eligible. It orders warmest
    first by the distance between a card's stability and the days since its last review; a card
    with no memory state or no last review comes last, and a tie keeps the read's order (a stable
    sort).
  - The hand's minutes are `ceil(count × pace / 60)`, at least 1 for a hand that is not empty. The
    pace is the stored rollups' seconds over their answered counts; with none it is 10 seconds and
    the minutes are labelled estimated.
  - The clock partitions each course's cards in this order: suspended, new (type 0 or queue 0),
    unmodelled (no positive stability), undated (no last review), modelled. Only the modelled cards
    are projected: each card's days to a floor minus the days since its last review, bucketed by
    `floor(days + 1e-9)` into 366 days or beyond. The loss floor is 0.5; the due floor is the
    collection's desired retention.
  - The days since a card's last review run from that review's UTC calendar date to the run's study
    day, as the predecessor counts them.
  - A park day is the first day the running count of lost cards reaches 10, 25 or 50 percent of the
    modelled cards. The maintenance rate is the largest `ceil(running count / day)` over days 1 to
    365. The debt at 30, 91, 182 and 365 days sums the buckets from day 0 to that day inclusive; its
    hours exist only when the pace was measured.
  - The runway's unseen cards are type 0 or queue 0 outside queues −1, −2 and −3, counted per home
    deck. A deck's track is its course, or its law subject when that is not the law root itself.
  - The runway's rate belongs to each track's ROOT deck, found by climbing the deck tree by name:
    the root's own limit (the normal deck kind's override, field 7, else its preset's new cards per
    day, field 9, where an absent field is 0), shared among the tracks under that root by each
    track's share of the root's unseen cards. Summing each child's limit overstates the rate.
  - An unknown limit makes the rate unknown (`limits_unknown`), and a rate of zero or less is
    `zero_limit`; neither is ever a number of days. Otherwise the runway is
    `ceil(unseen / rate)` days and that over 365.25 years. Tracks sort by unseen cards descending,
    then by track, and 8 are shown.
  - The collection's parent-limits flag is read and disclosed, and never enters the rate: a root's
    own limit governs its subtree either way.
  - Anki collates the deck name column `unicase`; as SPEC-094 R4 has it, no read registers a
    collation or orders, groups or seeks on it.
- **Corrections to the issues.**
  - #144's runway read the whole collection. Here the card counts keep SPEC-023's scope (ADR-095),
    while the deck tree, the deck kinds and the presets are read whole, because a scoped deck's
    budget may be set on a root outside the scope.
  - #149's Mini App tracks cleared cards "from the next sync". The predecessor had no such mark; here
    the next sync marks a dealt card cleared when it holds a study event of that card after the
    hand was dealt, and nothing else clears it.
- **What the parity oracle proves.** The runway's reader over a synthetic collection; Divestment
  Day's report over each reason and a dormant deck; the hand's size, order, search and minutes; the
  elapsed time to a floor across decays and floors; the clock over every partition; the runways over
  shared roots, overrides and unknown limits; and the constants.
- **Prerequisites.** SPEC-023, SPEC-029, SPEC-071, SPEC-077, SPEC-085, SPEC-090, SPEC-091, SPEC-092,
  SPEC-094 and SPEC-095, as the header names them.

## 2. Requirements

The reads (ADR-095)

R1. `crates/ingest/src/budget_reads.rs` reads, from the private copy and read-only: the count of
    cards per home deck, queue and type, keeping SPEC-023's scope; the id, name and kind of every
    deck; and the collection's `applyAllParentLimits` value, parsed as a JSON boolean or as `true`,
    `1`, `false` or `0`, and unknown otherwise. With the scope set to every deck the rows equal
    `goldens/runway_rows.json` (`runway.py:read_runway_rows` over a synthetic collection). The deck
    read follows SPEC-094 R4's rule for `unicase` name columns, and a failed read is named in the
    report, never read as zero.
R2. Coordination passes Divestment Day the scoped cards and the study events of the last 90 days
    (an instant 90 × 86,400,000 ms before the run), read through SPEC-023's read with that floor,
    and the leech threshold (`DECKSTREAK_LEECH_THRESHOLD`, SPEC-071); the hand, the scoped cards
    with their memory state (SPEC-077), the run's instant, and the answered counts and seconds of
    analytics' last 60 stored rollups (SPEC-071), newest first; the clock, the scoped cards with
    their memory state and course (SPEC-071 R2), the run's study day, the collection's desired
    retention (curriculum's port of `coaching.py:_collection_desired_pct`, SPEC-091 R2) over the
    same cards, and the rollups' pace when it was measured; and the runway, R1's rows, SPEC-095's
    presets and each deck's track (its course, or its law subject from SPEC-092 when that is not
    the law root).

Divestment Day (#147)

R3. The report equals `goldens/divest_report.json` (`divest.py:report`), with its measured hours, its
    projected hours for dormant decks and at most 8 rows in each section.
R4. A dormant deck is priced over its new, unsuspended cards that no other section claimed, and
    never over a card another section priced.
R5. The constants (90 days, 8.0, 12, 365, 200, 8, 12, 8 rows and 365.25 days) and the answer cap
    equal `goldens/divest.constants.json`.

The hand (#149)

R6. The hand's size equals `goldens/hand_size.json` (`hand.py:hand_size`, over SPEC-090's
    percentile), and the dealt hand, its `cid:` search and its minutes equal
    `goldens/hand.json` (`hand.py:build_hand` with `measured_seconds_per_card`).
R7. A hand dealt without a measured pace carries the 10-second fallback and says so, and its minutes
    are labelled estimated wherever they are shown.
R8. The constants (0.20, 14 samples, 15, 5, 4096, 256, 19 digits, the ceiling of 191, 10 seconds and
    60 rollups) equal `goldens/hand.constants.json`.
R9. The hand is stored as the instrument's latest report with the instant it was dealt. After each
    sync's recompute, coordination marks each of its cards cleared when that sync's study events
    hold an answer of the card after that instant; a card answered before it, or never, stays
    unchecked.

The sabbatical clock (#150, ADR-098)

R10. `crates/insights/src/sabbatical.rs` ports the elapsed time to a retrievability floor with its
     decay clamp, equal to `goldens/elapsed_at_floor.json` (`peak.py:elapsed_at_floor` through
     `fsrs.py:clamped_decay_exponent`, decays missing, in range and out of range, floors at and
     above 1), and the cold floor 0.5. Nothing else of the predecessor's peak module is ported.
R11. The clock equals `goldens/sabbatical.json` (`sabbatical.py:compute_sabbatical`): each course's
     partition counts, its loss and due curves over 366 days with the counts beyond them, its park
     days, its two maintenance rates and its debt at each horizon, and the count of cards with no
     course.
R12. Every course in the courses file has a clock, with zero counts when it has no card.
R13. The constants (366 days, the horizons 30, 91, 182 and 365, the quantiles 0.10, 0.25 and 0.50,
     the cold floor 0.5 and the tie epsilon 1e-9) equal `goldens/sabbatical.constants.json`.
     A card with no decay, or a zero one, reads 0.2 (`fsrs.DEFAULT_DECAY`; `sabbatical.py:314`,
     `fsrs.py:18` at `27ee2bc`), and `goldens/sabbatical.json` holds a card of that case.

The runway (#144)

R14. The runways equal `goldens/runways.json` (`runway.py:build_runways` over `limit_for_deck`):
     each track's unseen cards, its root decks, its shared rate or its reason, its days and years,
     the count of unseen cards with no track, and the flag as read.
R15. The runway is a weekly instrument (SPEC-094 R7). A zero or unknown rate is stored and shown
     as its reason, `zero_limit` or `limits_unknown`, and never as a number of days.
R16. The constants (8 rows, 365.25 days and the two reasons) equal `goldens/runway.constants.json`.

The surfaces

R17. Divestment Day, the hand and the clock are on-demand instruments (SPEC-094 R8). `/divest` and
     `/hand` run theirs; `/park` takes a course's code or alias, runs the clock and answers with
     that course's clock; with no argument, or one that names no course, it answers with its usage
     and the courses' aliases, read from the courses file. Each answers that a run is in progress
     when one is.
R18. The insights screen shows each report as a section:
     - Divestment Day's measured and projected hours and its sections, with every zero-hour section
       and row hidden;
     - the hand as a checklist of its cards, a button that copies the `cid:` search, the minutes
       with their label, and each cleared card checked;
     - the chosen course's clock, its loss and due curves drawn through SPEC-085's chart with a
       table of values, each park day or "beyond a year", and the maintenance rates as lower bounds;
     - the runway's tracks with their days and years, or their reason, and the flag as read.
     A report with a failed read renders its failure.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the runway rows equal the golden of the predecessor's reader over a synthetic collection | `the_runway_rows_match_the_predecessors_reader` |
| A2 | a deck outside the scope adds no unseen count, and its root's limit is still read | `a_deck_outside_the_scope_adds_no_unseen_count` |
| A3 | Divestment Day's report equals the golden | `the_divest_report_matches_the_predecessors_golden` |
| A4 | a dormant deck is priced over its unclaimed new cards only | `a_dormant_deck_is_priced_over_its_startable_cards_only` |
| A5 | Divestment Day's constants equal the golden | `the_divest_constants_equal_the_predecessors` |
| A6 | the hand's size equals the golden | `the_hand_size_matches_the_predecessors_golden` |
| A7 | the dealt hand, its search and its minutes equal the golden | `the_dealt_hand_matches_the_predecessors_golden` |
| A8 | a hand without a measured pace carries the fallback and says so | `a_hand_without_a_measured_pace_says_so` |
| A9 | the hand's constants equal the golden | `the_hand_constants_equal_the_predecessors` |
| A10 | the elapsed time to a floor equals the golden | `elapsed_at_floor_matches_the_predecessors_golden` |
| A11 | the clock equals the golden | `the_sabbatical_clock_matches_the_predecessors_golden` |
| A12 | every course has a clock, even with no card | `every_course_has_a_clock_even_with_no_card` |
| A13 | the clock's constants equal the golden | `the_sabbatical_constants_equal_the_predecessors` |
| A14 | the runways equal the golden | `the_runways_match_the_predecessors_golden` |
| A15 | a zero or unknown rate is its reason and never a number | `a_zero_or_unknown_rate_is_a_reason_never_a_number` |
| A16 | the runway's constants equal the golden | `the_runway_constants_equal_the_predecessors` |
| A17 | the next sync clears only the cards answered after the deal | `the_next_sync_clears_only_cards_answered_after_the_deal` |
| A18 | the clock reads the collection's desired retention and the measured pace | `the_clock_reads_the_desired_retention_and_the_measured_pace` |
| A19 | each of the three commands runs its instrument or answers that a run is in progress | `each_time_command_runs_its_instrument` |
| A20 | `/park` with no course, or an unknown one, answers with the courses' aliases | `park_without_a_known_course_names_the_aliases` |
| A21 | Divestment Day hides every zero-hour section and row | `hides every zero-hour section and row` |
| A22 | the hand checks each cleared card and copies its search | `checks each cleared card and copies the search` |
| A23 | the hand labels fallback minutes as estimated | `labels fallback minutes as estimated` |
| A24 | the clock draws both curves and states each park day or beyond a year | `draws both curves and states each park day` |
| A25 | the runway shows a reason, never a number, for a zero or unknown rate | `shows a reason and never a number for an unknown rate` |

```acceptance
A1: cargo test -p deck-streak-ingest --test budget_reads -- --exact the_runway_rows_match_the_predecessors_reader
A2: cargo test -p deck-streak-ingest --test budget_reads -- --exact a_deck_outside_the_scope_adds_no_unseen_count
A3: cargo test -p deck-streak-insights --test divest -- --exact the_divest_report_matches_the_predecessors_golden
A4: cargo test -p deck-streak-insights --test divest -- --exact a_dormant_deck_is_priced_over_its_startable_cards_only
A5: cargo test -p deck-streak-insights --test divest -- --exact the_divest_constants_equal_the_predecessors
A6: cargo test -p deck-streak-insights --test hand -- --exact the_hand_size_matches_the_predecessors_golden
A7: cargo test -p deck-streak-insights --test hand -- --exact the_dealt_hand_matches_the_predecessors_golden
A8: cargo test -p deck-streak-insights --test hand -- --exact a_hand_without_a_measured_pace_says_so
A9: cargo test -p deck-streak-insights --test hand -- --exact the_hand_constants_equal_the_predecessors
A10: cargo test -p deck-streak-insights --test sabbatical -- --exact elapsed_at_floor_matches_the_predecessors_golden
A11: cargo test -p deck-streak-insights --test sabbatical -- --exact the_sabbatical_clock_matches_the_predecessors_golden
A12: cargo test -p deck-streak-insights --test sabbatical -- --exact every_course_has_a_clock_even_with_no_card
A13: cargo test -p deck-streak-insights --test sabbatical -- --exact the_sabbatical_constants_equal_the_predecessors
A14: cargo test -p deck-streak-insights --test runway -- --exact the_runways_match_the_predecessors_golden
A15: cargo test -p deck-streak-insights --test runway -- --exact a_zero_or_unknown_rate_is_a_reason_never_a_number
A16: cargo test -p deck-streak-insights --test runway -- --exact the_runway_constants_equal_the_predecessors
A17: cargo test -p deck-streak-coordination --test hand_step -- --exact the_next_sync_clears_only_cards_answered_after_the_deal
A18: cargo test -p deck-streak-coordination --test hand_step -- --exact the_clock_reads_the_desired_retention_and_the_measured_pace
A19: cargo test -p deck-streak-bot --test time_commands -- --exact each_time_command_runs_its_instrument
A20: cargo test -p deck-streak-bot --test time_commands -- --exact park_without_a_known_course_names_the_aliases
A21: pnpm exec vitest run web/app/src/lib/insights/DivestSection.test.ts -t "hides every zero-hour section and row"
A22: pnpm exec vitest run web/app/src/lib/insights/HandSection.test.ts -t "checks each cleared card and copies the search"
A23: pnpm exec vitest run web/app/src/lib/insights/HandSection.test.ts -t "labels fallback minutes as estimated"
A24: pnpm exec vitest run web/app/src/lib/insights/ParkSection.test.ts -t "draws both curves and states each park day"
A25: pnpm exec vitest run web/app/src/lib/insights/RunwaySection.test.ts -t "shows a reason and never a number for an unknown rate"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges this over the committed tree and posts its
verdict on the pull request as the `box/packs` status. It has no line in the acceptance fence,
because no public test can run a pack's row. This SPEC adds no table: the reports stay in SPEC-094's
`instrument_reports`, whose category now names the card ids the hand and Divestment Day hold. The
privacy-gdpr and accessibility packs stay enforced; no check is deferred or lifted for this
delivery, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/coordination/src/data_rights.rs`: the `research-instruments` category names the card ids its reports hold, and export and erase still cover `instrument_reports` | the privacy-gdpr pack |
| B2 | over every file under `web/app/src/lib/insights/`: the four new sections, the hand's checklist and copy button, and the clock's chart and its table pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/budget_reads.rs` | `deck-streak-ingest` | added: the card counts, the deck tree and the parent-limits flag |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/budget_reads.rs` | `deck-streak-ingest` | added: A1, A2 |
| `crates/insights/src/divest.rs` | `deck-streak-insights` | added: Divestment Day |
| `crates/insights/src/hand.rs` | `deck-streak-insights` | added: the hand |
| `crates/insights/src/sabbatical.rs` | `deck-streak-insights` | added: the clock, the elapsed time to a floor and the cold floor (ADR-098) |
| `crates/insights/src/runway.rs` | `deck-streak-insights` | added: the runway |
| `crates/insights/src/registry.rs` | `deck-streak-insights` | changed: one weekly and three on-demand rows |
| `crates/insights/src/lib.rs` | `deck-streak-insights` | changed: the modules |
| `crates/insights/tests/divest.rs` | `deck-streak-insights` | added: A3 to A5 |
| `crates/insights/tests/hand.rs` | `deck-streak-insights` | added: A6 to A9 |
| `crates/insights/tests/sabbatical.rs` | `deck-streak-insights` | added: A10 to A13 |
| `crates/insights/tests/runway.rs` | `deck-streak-insights` | added: A14 to A16 |
| `crates/coordination/src/time_instruments.rs` | `deck-streak-coordination` | added: the four instruments' inputs and the hand's cleared marks |
| `crates/coordination/src/instruments.rs` | `deck-streak-coordination` | changed: the step calls the cleared marks after each sync |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/hand_step.rs` | `deck-streak-coordination` | added: A17, A18 |
| `crates/bot/src/time_commands.rs` | `deck-streak-bot` | added: the divest, hand and park commands |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the commands join the table |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot's commands receive the owner's courses loaded at start, for /park's aliases (R17) |
| `crates/bot/tests/time_commands.rs` | `deck-streak-bot` | added: A19, A20 |
| `web/app/src/lib/insights/DivestSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/DivestSection.test.ts` | miniapp | added: A21 |
| `web/app/src/lib/insights/HandSection.svelte` | miniapp | added: the checklist and the copy button |
| `web/app/src/lib/insights/HandSection.test.ts` | miniapp | added: A22, A23 |
| `web/app/src/lib/insights/ParkSection.svelte` | miniapp | added: the course picker and the two curves |
| `web/app/src/lib/insights/ParkSection.test.ts` | miniapp | added: A24 |
| `web/app/src/lib/insights/RunwaySection.svelte` | miniapp | added |
| `web/app/src/lib/insights/RunwaySection.test.ts` | miniapp | added: A25 |
| `web/app/src/lib/insights/insights.ts` | miniapp | changed: the four reports' types and the park run's course |
| `privacy.json` | repo | changed: the `research-instruments` category names card ids |
| `PRIVACY.md` | repo | changed: the category's line names card ids |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `tools/parity-oracle/registry/spec_098.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/runway_rows.json` | repo | added: the golden of `runway.py:read_runway_rows` (adapter; a temporary synthetic collection) |
| `tools/parity-oracle/goldens/divest_report.json` | repo | added: the golden of `divest.py:report` (adapter; synthetic cards, reviews and decks) |
| `tools/parity-oracle/goldens/divest.constants.json` | repo | added: Divestment Day's constants (constants) |
| `tools/parity-oracle/goldens/hand_size.json` | repo | added: the golden of `hand.py:hand_size` (function) |
| `tools/parity-oracle/goldens/hand.json` | repo | added: the golden of `hand.py:build_hand` (adapter; synthetic cards and rollups) |
| `tools/parity-oracle/goldens/hand.constants.json` | repo | added: the hand's constants (constants) |
| `tools/parity-oracle/goldens/elapsed_at_floor.json` | repo | added: the golden of `peak.py:elapsed_at_floor` (function) |
| `tools/parity-oracle/goldens/sabbatical.json` | repo | added: the golden of `sabbatical.py:compute_sabbatical` (adapter; synthetic courses and cards) |
| `tools/parity-oracle/goldens/sabbatical.constants.json` | repo | added: the clock's constants (constants) |
| `tools/parity-oracle/goldens/runways.json` | repo | added: the golden of `runway.py:build_runways` (adapter; synthetic decks, blobs and tracks) |
| `tools/parity-oracle/goldens/runway.constants.json` | repo | added: the runway's constants (constants) |
| `scripts/mutation-rows.d/S09800-S09899.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-098-divestment-day-the-hand-the-sabbatical-clock-and-the-runway-price-the-owners-time.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-098.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no weekly report; the runway's stored report waits for it (#130).
- It offers no hand from the lapse digest's button, which arrives with the comeback protocol (#123).
- It revives neither the Birth Cohort (#177) nor the Peak Day taper (#182), each inert in v9 and waiting for the
  owner's decision; the clock ports only the two primitives it reads.
- It builds no settings screen for the leech threshold or the scope (#57).
- It serves no instrument to the agent's machine read tool (#157).
- It imports none of the predecessor's reports (#61).

## 6. Risks

- **A child's limit is summed beside its root's** and the runway reads short. Detected by A14, whose
  golden holds a root with two tracks and children that carry their own presets.
- **An unknown limit reads as zero or as a large number.** Prevented by the reason and detected by
  A15 and A25.
- **The scoped read loses the root that sets a scoped deck's budget.** Detected by A2, whose root
  sits outside the scope.
- **A suspended leech is priced as churn.** Detected by A3, whose golden holds a suspended card with
  a leech's lapses, repetitions and difficulty.
- **The clock counts days since a review in local time.** Detected by A11, whose golden holds a
  review between local and UTC midnight.
- **A decay out of range overflows the curve.** Detected by A10, whose golden holds decays of zero,
  below 0.001 and above 10.
- **SPEC-097 builds at the same time** and adds rows to the same registry, command table and
  insights types. The second to merge resolves adjacent lines only; each keeps its own files.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_098.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic: each adapter patches synthetic courses
and law subjects in where the module reads them, so no name of the owner's enters a golden.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `runway_rows` | `runway.py:read_runway_rows` | adapter | a temporary collection file with filtered and home decks, kinds, presets and each form of the flag |
| `divest_report` | `divest.py:report` | adapter | synthetic cards for each reason, a suspended leech, a dormant deck with claimed cards, and window reviews |
| `hand_size` | `hand.py:hand_size` | function | answered counts below, at and above 14 samples, and at both clamps |
| `hand` | `hand.py:build_hand` | adapter | synthetic cards in every queue with and without a memory state, ties, and rollups with and without a pace |
| `elapsed_at_floor` | `peak.py:elapsed_at_floor` | function | stabilities, decays in and out of range, and floors from 0.1 to above 1 |
| `sabbatical` | `sabbatical.py:compute_sabbatical` | adapter | synthetic courses and cards in every partition, a card of no course, a card with a missing decay (read as 0.2), and a review between local and UTC midnight |
| `runways` | `runway.py:build_runways` | adapter | synthetic deck trees, a root shared by two tracks, overrides, a zero and an unknown limit, and more than 8 tracks |
| `divest.constants`, `hand.constants`, `sabbatical.constants`, `runway.constants` | the modules' constants | constants | none |

## 8. Tables and the v9 import

This SPEC adds no table. Every report is stored in SPEC-094's `instrument_reports`, and none is
imported (#61).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09801-SUSPENDED-NEVER-CLASSIFIED` | `crates/insights/src/divest.rs` | a suspended card is never classified | `divest::the_divest_report_matches_the_predecessors_golden` |
| `S09802-DORMANT-UNCLAIMED` | `crates/insights/src/divest.rs` | a dormant deck is priced over unclaimed cards only | `divest::a_dormant_deck_is_priced_over_its_startable_cards_only` |
| `S09803-HAND-CLAMP` | `crates/insights/src/hand.rs` | the size is clamped to 5..191 | `hand::the_hand_size_matches_the_predecessors_golden` |
| `S09804-WARMTH-DISTANCE` | `crates/insights/src/hand.rs` | the warmth is an absolute distance, and an unmodelled card comes last | `hand::the_dealt_hand_matches_the_predecessors_golden` |
| `S09805-FLOOR-NOT-NEGATIVE` | `crates/insights/src/sabbatical.rs` | the elapsed time to a floor is never negative | `sabbatical::elapsed_at_floor_matches_the_predecessors_golden` |
| `S09806-PARTITION-ORDER` | `crates/insights/src/sabbatical.rs` | a suspended card is counted before a new one | `sabbatical::the_sabbatical_clock_matches_the_predecessors_golden` |
| `S09807-ROOT-SHARE` | `crates/insights/src/runway.rs` | a root's limit is shared by unseen share, never summed per child | `runway::the_runways_match_the_predecessors_golden` |
| `S09808-UNKNOWN-NOT-ZERO` | `crates/insights/src/runway.rs` | an unknown limit is a reason, never zero | `runway::a_zero_or_unknown_rate_is_a_reason_never_a_number` |
| `S09809-SCOPED-COUNTS` | `crates/ingest/src/budget_reads.rs` | the counts keep the scope while the tree is read whole | `budget_reads::a_deck_outside_the_scope_adds_no_unseen_count` |
| `S09810-CLEARED-AFTER-DEAL` | `crates/coordination/src/time_instruments.rs` | only an answer after the deal clears a card | `hand_step::the_next_sync_clears_only_cards_answered_after_the_deal` |
