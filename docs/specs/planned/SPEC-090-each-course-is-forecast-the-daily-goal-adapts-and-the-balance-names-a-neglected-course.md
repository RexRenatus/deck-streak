# SPEC-090: each course is forecast to its next band, the daily goal adapts, and the balance names a neglected course

- **Wave:** W4. **Issue:** #86, #87, #89, #159 (epic #5). **Context(s):** `deck-streak-curriculum`
  (the velocity and the forecast, the adaptive daily goal, the cross-language balance,
  `pace_readouts`); `deck-streak-kernel` (the CPython numeric port, ADR-090); `deck-streak-analytics`
  (its compensated sum delegates to the kernel's); `deck-streak-coordination` (the pace step in
  phase 4 of the fold, the pace read model, the goal on the today view); `deck-streak-api`,
  `deck-streak-bot` and the Mini App (the pace route, `/forecast`, `/goal`, `/balance`, the ETA on
  each course, the goal ring and the language share); `scripts/` (the syllabus band tool).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-085 (charts draw on the client),
  ADR-087 (the courses and their unit bands are private configuration), ADR-090 (CPython's numeric
  semantics are ported once, into the kernel) and ADR-091 (curriculum's readouts are computed in the
  recompute's current-day step and stored as the latest readout).
- **Prerequisites:** SPEC-029 (the parity oracle), SPEC-071 (the fold, the courses file and the daily
  rollups), SPEC-077 (each course's progress and bands), SPEC-085 (the chart loader) and SPEC-086 (the
  today view the goal ring joins). **Mutation band:** `S09000-S09099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-090.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 `crates/curriculum/src/` holds only `lib.rs`. SPEC-077
  stores each course's bands, and its §5 leaves the forecast to #86; no goal, balance or syllabus
  tool exists, and the owner sees no time to the next band.
- **What is ported.** The forecast (#86): `velocity.py:mature_velocity` and `compute_velocity`. The
  adaptive goal (#87): `gamification/adaptive.py:percentile`, `percentile_p90` and
  `adaptive_daily_goal`, fed as `coaching.py:compute_all` feeds them. The balance (#89):
  `cross_language.py:compute_balance`. The syllabus band tool (#159): `tools/syllabus/export_bands.py`
  (`derive_language`, `_validate_language_units`, `build_export` and `gated_subset`), as a public tool
  that reads a syllabus directory the operator names.
- **Traps a hand port falls into.**
  - The velocity is ONE rate shared by every course: `compute_velocity` takes the median of the
    collection's daily graduations and divides each course's remaining cards by it.
  - The median is `statistics.median`: an even count averages the two middle values in float.
  - The floor of 0.1 makes a zero velocity unreachable, so the "no ETA" branch never runs; the ETA
    is 0 when nothing remains and `ceil(remaining / rate)` otherwise.
  - The next band is the first band not achieved that holds any card; a course with every band
    achieved reports C2 with 0 remaining.
  - The goal's remaining is the SUM of every course's remaining to its next band, over a fixed
    horizon of 365 days; the history p90 is floored at 30 before the goal reads it.
  - The percentile is nearest-rank, `rank = max(1, ceil(pct * n))`
    (`gamification/adaptive.py:14`); `0.9 * 70` is exactly 63.0 in float, so rank 63, and the
    golden holds that case with the empty list, one value, `0.2` and `1.0`.
  - The balance's window is the last 28 study days INCLUDING today, at the 04:00 rollover; its sort
    is by windowed reviews descending, then by course name.
- **Corrections to the issues.**
  - #86 speaks of "a zero velocity yields no ETA"; the floor makes that unreachable, and the golden
    proves the floor instead. The velocity is not per course.
  - #89's "full history" is the predecessor's read window, which is DeckStreak's too (400 days,
    SPEC-023): a course idle for longer has no review in the read and is not listed, in both
    products.
  - #159's "in-code band table" does not exist here: the unit bands live in the private courses file
    (ADR-087). The parity gate compares the courses file's bands with the tool's export.
- **What the parity oracle proves.** The median and the percentile at their float edges, the
  velocity and the forecast over synthetic courses, the goal at its floors and caps, the balance over
  synthetic reviews that straddle the window and the rollover, and the syllabus derivation and its
  five refusals over synthetic syllabi.
- **Prerequisites.** SPEC-029, SPEC-071, SPEC-077, SPEC-085 and SPEC-086, as the header names them.

## 2. Requirements

The numeric port (ADR-090)

R1. `crates/kernel/src/pynum.rs` holds CPython's float semantics these ports read: the compensated
    `sum`, `statistics.median`, `statistics.mean` (the exact mean, rounded once, which a running
    float sum is not), `round(x, n)` and the predecessor's nearest-rank percentile
    (`gamification/adaptive.py:percentile`). Each equals its golden (`goldens/pynum_basics.json`,
    `goldens/percentile.json`), and analytics' compensated sum (SPEC-071) delegates to the kernel's,
    keeping its own name.

The forecast (#86)

R2. The mature velocity is the median of the daily graduations over the stored rollups with any
    review, floored at 0.1 (`velocity.MIN_MATURE_VELOCITY`), equal to `goldens/mature_velocity.json`.
R3. Each course's forecast names its next band, the cards remaining to it and the days to it, equal
    to `goldens/forecast.json` (`velocity.py:compute_velocity`) over SPEC-077's stored course
    progress. The rate is shared by every course.

The adaptive goal (#87)

R4. The daily goal is `adaptive_daily_goal` over the summed remaining of every course, a horizon of
    365 days (`coaching._GOAL_HORIZON_DAYS`), a floor of 10 and the history p90 of the reviews on
    active days over the last 370 rollups, floored at 30, equal to `goldens/adaptive_goal.json`.
R5. The today view (SPEC-086) carries the goal beside today's reviews; before the first recompute
    stores one, the goal is pending, never 0.

The balance (#89)

R6. The balance attributes each study review in the read window to its card's course, and reports
    each course's windowed reviews, share, days idle and whether it is neglected (share under 5.0
    with any review in the read), equal to `goldens/balance.json`
    (`cross_language.py:compute_balance`, `NEGLECTED_SHARE_PCT`, a window of 28 study days).
R7. A course idle for more than 28 days but inside the read window is listed with share 0 and
    flagged neglected.

The readouts (ADR-091)

R8. The pace step registers in phase 4 of SPEC-071's fold and runs in the current study day's step
    only. It writes the forecast of each course, the goal and the balance to `pace_readouts`, one
    row per kind and scope, replacing the previous row in one write.
R9. `pace_readouts` is a `STRICT` table with `created_at`, created by
    `migrations/009001_curriculum_pace_readouts.sql` (SPEC-020 R15, R18). It is registered in the
    context map's register of DeckStreak's own tables, declared in `privacy.json` (the category
    `course-pace`), given a line in `PRIVACY.md`, and exported and erased by curriculum's data-rights
    port; the symmetry test seeds it.

The surfaces

R10. `GET /api/curriculum/pace` serves the forecast, the goal and the balance to the owner's session
     only; any other caller is answered 401 or 403 with no data. A missing readout reads as pending.
R11. The bot's `/forecast`, `/goal` and `/balance` keep their names and state what the route
     states. The Mini App shows each course's ETA on its course card (SPEC-077's progress screen),
     the goal as a ring on the home screen, and the balance as a doughnut drawn on the client through
     SPEC-085's loader, which registers the doughnut's controller and arc element only, beside a table
     of the same values.

The syllabus band tool (#159)

R12. `scripts/syllabus-bands.py` derives each course's unit bands from the syllabus directory named
     by `--syllabus-dir` or `DECKSTREAK_SYLLABUS_DIR`, never a literal path, refuses a syllabus the
     predecessor refuses (the five structural checks), and with `--check <courses file>` exits
     non-zero naming each course whose configured bands differ from the export. Its derivation and
     refusals equal `goldens/syllabus_derive.json` and `goldens/syllabus_refusals.json`.
R13. Every constant this SPEC uses (the velocity floor, the goal's floor, horizon and history floor,
     the percentile, the balance's window and threshold) equals `goldens/pace.constants.json`, held
     by a test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the kernel's sum, median, mean and round equal CPython's golden over float edge cases | `the_numeric_basics_match_cpythons_golden` |
| A2 | the nearest-rank percentile equals the golden of `adaptive.py:percentile`, the float edge included | `the_percentile_matches_the_predecessors_golden` |
| A3 | the mature velocity equals the golden of `velocity.py:mature_velocity` | `mature_velocity_matches_the_predecessors_golden` |
| A4 | each course's forecast equals the golden of `velocity.py:compute_velocity` | `the_forecast_matches_the_predecessors_golden` |
| A5 | the goal equals the golden of `adaptive.py:adaptive_daily_goal` over `compute_all`'s inputs | `the_adaptive_goal_matches_the_predecessors_golden` |
| A6 | the balance equals the golden of `cross_language.py:compute_balance` | `the_balance_matches_the_predecessors_golden` |
| A7 | a course idle for 60 days inside the read window is listed and flagged neglected | `a_course_idle_for_months_is_still_flagged` |
| A8 | every pace constant equals `goldens/pace.constants.json` | `the_pace_constants_equal_the_predecessors` |
| A9 | two recomputes store one row per kind and scope, and the second replaces the first | `the_pace_readouts_are_replaced_once_per_recompute` |
| A10 | before the first recompute every pace readout, and the goal on the today view, is pending | `the_pace_readouts_are_pending_before_the_first_recompute` |
| A11 | `pace_readouts` is exported and erased by curriculum's port | `the_pace_readouts_are_exported_and_erased` |
| A12 | the pace route answers the owner and refuses every other caller with no data | `the_pace_route_answers_only_the_owner` |
| A13 | `/forecast` states each course's next band and ETA as the route does | `forecast_states_each_course_eta_as_the_route_does` |
| A14 | `/balance` names a neglected course and its days idle | `balance_names_a_neglected_course` |
| A15 | the goal ring draws today's reviews over the goal, and a pending goal as pending | `draws todays reviews over the goal and a pending goal as pending` |
| A16 | the language share labels each course's share and marks a neglected one, beside a table | `labels each course share and marks a neglected one` |
| A17 | the syllabus derivation equals the golden of `export_bands.py:derive_language` | `test_derivation_matches_the_predecessors_golden` |
| A18 | the tool refuses each malformed synthetic syllabus as `_validate_language_units` does | `test_refusals_match_the_predecessors_golden` |
| A19 | the syllabus directory is configuration: with neither the flag nor the variable the tool refuses | `test_the_syllabus_directory_is_configuration` |
| A20 | `--check` exits non-zero and names a course whose configured bands differ from the export | `test_check_names_a_course_that_disagrees` |
| A21 | the pace step passes the stored rollups (the last 370 for the goal's p90), SPEC-077's stored course progress and the study reviews of the read window with each card's course | `the_pace_step_is_passed_its_inputs` |

```acceptance
A1: cargo test -p deck-streak-kernel --test pynum_goldens -- --exact the_numeric_basics_match_cpythons_golden
A2: cargo test -p deck-streak-kernel --test pynum_goldens -- --exact the_percentile_matches_the_predecessors_golden
A3: cargo test -p deck-streak-curriculum --test pace_goldens -- --exact mature_velocity_matches_the_predecessors_golden
A4: cargo test -p deck-streak-curriculum --test pace_goldens -- --exact the_forecast_matches_the_predecessors_golden
A5: cargo test -p deck-streak-curriculum --test pace_goldens -- --exact the_adaptive_goal_matches_the_predecessors_golden
A6: cargo test -p deck-streak-curriculum --test pace_goldens -- --exact the_balance_matches_the_predecessors_golden
A7: cargo test -p deck-streak-curriculum --test pace_balance -- --exact a_course_idle_for_months_is_still_flagged
A8: cargo test -p deck-streak-curriculum --test pace_goldens -- --exact the_pace_constants_equal_the_predecessors
A9: cargo test -p deck-streak-coordination --test pace_step -- --exact the_pace_readouts_are_replaced_once_per_recompute
A10: cargo test -p deck-streak-coordination --test pace_step -- --exact the_pace_readouts_are_pending_before_the_first_recompute
A11: cargo test -p deck-streak-curriculum --test pace_store -- --exact the_pace_readouts_are_exported_and_erased
A12: cargo test -p deck-streak-api --test pace_routes -- --exact the_pace_route_answers_only_the_owner
A13: cargo test -p deck-streak-bot --test pace_commands -- --exact forecast_states_each_course_eta_as_the_route_does
A14: cargo test -p deck-streak-bot --test pace_commands -- --exact balance_names_a_neglected_course
A15: pnpm exec vitest run web/app/src/lib/pace/GoalRing.test.ts -t "draws todays reviews over the goal and a pending goal as pending"
A16: pnpm exec vitest run web/app/src/lib/pace/LanguageShare.test.ts -t "labels each course share and marks a neglected one"
A17: python3 -m unittest discover -s scripts/tests -p test_syllabus_bands.py -k test_derivation_matches_the_predecessors_golden
A18: python3 -m unittest discover -s scripts/tests -p test_syllabus_bands.py -k test_refusals_match_the_predecessors_golden
A19: python3 -m unittest discover -s scripts/tests -p test_syllabus_bands.py -k test_the_syllabus_directory_is_configuration
A20: python3 -m unittest discover -s scripts/tests -p test_syllabus_bands.py -k test_check_names_a_course_that_disagrees
A21: cargo test -p deck-streak-coordination --test pace_step -- --exact the_pace_step_is_passed_its_inputs
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and accessibility packs stay
enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/curriculum/src/data_rights.rs`: the `course-pace` category names `pace_readouts` with its purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/+page.svelte`, `web/app/src/routes/progress/+page.svelte`, `web/app/src/routes/pace/+page.svelte` and every file under `web/app/src/lib/pace/`: the goal ring, the ETA and the language share pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/pynum.rs` | `deck-streak-kernel` | added: CPython's sum, median, mean, round and the nearest-rank percentile (ADR-090) |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the numeric module |
| `crates/kernel/tests/pynum_goldens.rs` | `deck-streak-kernel` | added: A1, A2 |
| `crates/analytics/src/metrics.rs` | `deck-streak-analytics` | changed: its compensated sum delegates to the kernel's |
| `crates/curriculum/src/pace.rs` | `deck-streak-curriculum` | added: the velocity, the forecast, the goal and the balance |
| `crates/curriculum/src/pace_store.rs` | `deck-streak-curriculum` | added: `pace_readouts`, read and replaced |
| `crates/curriculum/src/data_rights.rs` | `deck-streak-curriculum` | changed: the port exports and erases `pace_readouts` |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the pace modules |
| `crates/curriculum/tests/pace_goldens.rs` | `deck-streak-curriculum` | added: A3 to A6, A8 |
| `crates/curriculum/tests/pace_balance.rs` | `deck-streak-curriculum` | added: A7 |
| `crates/curriculum/tests/pace_store.rs` | `deck-streak-curriculum` | added: A11 |
| `crates/curriculum/Cargo.toml` | `deck-streak-curriculum` | changed: serde_json's `float_roundtrip` for the goldens' reader (a dev-dependency) |
| `migrations/009001_curriculum_pace_readouts.sql` | `deck-streak-curriculum` | added: `pace_readouts` |
| `crates/coordination/src/recompute/pace.rs` | `deck-streak-coordination` | added: the pace step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: declares the pace step's module |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: registers the pace step in phase 4 of `recompute_fold` (SPEC-071 R19); joins the goal to SPEC-086's today view (R5) |
| `crates/coordination/src/pace.rs` | `deck-streak-coordination` | added: the pace read model |
| `crates/coordination/src/today/view.rs` | `deck-streak-coordination` | changed: the goal beside today's reviews |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the pace read model |
| `crates/coordination/tests/pace_step.rs` | `deck-streak-coordination` | added: A9, A10, A21 |
| `crates/api/src/pace_routes.rs` | `deck-streak-api` | added: `GET /api/curriculum/pace` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the pace route behind the owner's session |
| `crates/api/tests/pace_routes.rs` | `deck-streak-api` | added: A12 |
| `crates/bot/src/pace_commands.rs` | `deck-streak-bot` | added: the forecast, goal and balance commands |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the three commands join the table |
| `crates/bot/tests/pace_commands.rs` | `deck-streak-bot` | added: A13, A14 |
| `web/app/src/routes/pace/+page.svelte` | miniapp | added: the balance and the goal's detail |
| `web/app/src/routes/+page.svelte` | miniapp | changed: the goal ring on the home screen |
| `web/app/src/routes/progress/+page.svelte` | miniapp | changed: each course card shows its ETA |
| `web/app/src/lib/pace/GoalRing.svelte` | miniapp | added |
| `web/app/src/lib/pace/GoalRing.test.ts` | miniapp | added: A15 |
| `web/app/src/lib/pace/LanguageShare.svelte` | miniapp | added: the doughnut and its table |
| `web/app/src/lib/pace/LanguageShare.test.ts` | miniapp | added: A16 |
| `web/app/src/lib/pace/pace.ts` | miniapp | added: the route's client and types |
| `web/app/src/lib/charts/load.ts` | miniapp | changed: registers the doughnut's controller and arc element |
| `web/app/src/lib/routes.ts` | miniapp | changed: /pace joins `ROUTES` |
| `scripts/syllabus-bands.py` | repo | added: the syllabus band tool |
| `scripts/tests/test_syllabus_bands.py` | repo | added: A17 to A20 |
| `scripts/tests/fixtures/syllabus/` | repo | added: synthetic syllabi, well-formed and malformed |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `pace_readouts` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry lists `pace_readouts` under curriculum's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `pace_readouts` |
| `privacy.json` | repo | changed: the `course-pace` category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_090.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/pynum_basics.json` | repo | added: CPython's `sum`, `statistics.median`, `statistics.mean` and `round` (adapter; float edge cases) |
| `tools/parity-oracle/goldens/percentile.json` | repo | added: the golden of `gamification/adaptive.py:percentile` (function) |
| `tools/parity-oracle/goldens/mature_velocity.json` | repo | added: the golden of `velocity.py:mature_velocity` (adapter; rollup rows) |
| `tools/parity-oracle/goldens/forecast.json` | repo | added: the golden of `velocity.py:compute_velocity` (adapter; synthetic progress) |
| `tools/parity-oracle/goldens/adaptive_goal.json` | repo | added: the golden of `gamification/adaptive.py:adaptive_daily_goal` (adapter; `compute_all`'s feed) |
| `tools/parity-oracle/goldens/balance.json` | repo | added: the golden of `cross_language.py:compute_balance` (adapter; synthetic courses and reviews) |
| `tools/parity-oracle/goldens/syllabus_derive.json` | repo | added: the golden of `tools/syllabus/export_bands.py:derive_language` (function) |
| `tools/parity-oracle/goldens/syllabus_refusals.json` | repo | added: the golden of `tools/syllabus/export_bands.py:_validate_language_units` (adapter; the refusal's kind) |
| `tools/parity-oracle/goldens/pace.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S09000-S09099.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-090-each-course-is-forecast-the-daily-goal-adapts-and-the-balance-names-a-neglected-course.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-090.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It writes no forecast, goal or balance into the daily digest or its coaching (#129, #53).
- It pays no consistency multiplier on the goal; that is SPEC-072's (#72).
- It revives no law-versus-language crowd-out statistics: inert in v9, they wait for the owner's
  decision (#179).
- It serves no forecast or balance to the agent's machine read tool (#157).
- It publishes no course pace on the public page (#156).
- It stores no history of past readouts; a trend of the pace across weeks belongs to the weekly
  report (#130).
- It imports none of the predecessor's coaching payloads; the first recompute computes them (#61).
- It ships no syllabus of the owner's, and the operator's run of the tool is private (#41).

## 6. Risks

- **The shared velocity reads as a per-course rate.** Held by A4's cases, where two courses with
  different histories receive one rate, and by the route's label, which names it the collection's.
- **A float edge moves a percentile rank or a median.** Detected by A1 and A2, whose goldens hold
  an even count and the 0.9-of-70 case.
- **A float running sum stands in for `statistics.mean`.** Detected by A1, whose golden holds lists
  where the two differ. SPEC-097's Tilt Test and SPEC-098's Divestment Day read this mean.
- **The balance's window slips a day at the rollover.** Detected by A6, whose cases put reviews at
  03:59 and 04:00 on the window's first and last study days.
- **The syllabus tool reads the owner's syllabus in public CI.** Prevented: its tests read the
  synthetic fixtures only, and the tool takes no default path (A19).

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_090.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic; no golden carries a course, deck name,
unit band or syllabus of the owner's.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `pynum_basics` | CPython's `sum`, `statistics.median`, `statistics.mean`, `round` | adapter | float lists with cancellation, even and odd counts, lists whose running sum differs from the exact mean, and halves at each digit, and `round(2.675, 2)`, whose shortest decimal is a tie but whose binary value is not (expected 2.67) |
| `percentile` | `gamification/adaptive.py:percentile` | function | none: empty, one value, `0.9` of 70 values, `0.2` and `1.0`, `0.9` of 7 distinct values (rank 6.3, which the ceiling takes to 7) and `0.0` of 70 values (rank 0, which the floor takes to 1) |
| `mature_velocity` | `velocity.py:mature_velocity` | adapter | rollup rows with and without reviews, all-zero graduations, an even count |
| `forecast` | `velocity.py:compute_velocity` | adapter | synthetic `LanguageProgress` values with gaps, empty bands, a fully achieved course and a remaining count the rate does not divide (7 over a rate of 2.0 is 3.5, which the ceiling takes to 4 and the floor to 3) |
| `adaptive_goal` | `gamification/adaptive.py:adaptive_daily_goal` | adapter | the remaining sum, the 365-day horizon and `max(percentile_p90(history), 30)` as `compute_all` feeds them, a per-day need under 10, at 10 and over it, and a history p90 under 30, at 30 and over it |
| `balance` | `cross_language.py:compute_balance` | adapter | synthetic courses patched into the module's course lookup, reviews on the window's first and last study day and on the day before it, and either side of the rollover, a course whose share is exactly 5.0 and one just under it, a long-idle course and an empty window |
| `syllabus_derive` | `tools/syllabus/export_bands.py:derive_language` | function | none: synthetic syllabi |
| `syllabus_refusals` | `tools/syllabus/export_bands.py:_validate_language_units` | adapter | one synthetic syllabus per refusal; returns which check refused, never the message's path |
| `pace.constants` | `velocity.MIN_MATURE_VELOCITY`; `coaching._GOAL_HORIZON_DAYS`; `cross_language.NEGLECTED_SHARE_PCT`; `adaptive_daily_goal`'s floor | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `pace_readouts` | `deck-streak-curriculum` | `migrations/009001_curriculum_pace_readouts.sql` | the velocity, goal and balance payloads it kept as coaching values, which the import does not carry: the first recompute recomputes them | exported and erased: no row reads as pending |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09001-VELOCITY-FLOOR` | `crates/curriculum/src/pace.rs` | the velocity floor of 0.1 | `pace_goldens::mature_velocity_matches_the_predecessors_golden` |
| `S09002-ETA-CEILING` | `crates/curriculum/src/pace.rs` | the ETA rounds up | `pace_goldens::the_forecast_matches_the_predecessors_golden` |
| `S09003-GOAL-FLOOR-TEN` | `crates/curriculum/src/pace.rs` | the goal's floor of 10 | `pace_goldens::the_adaptive_goal_matches_the_predecessors_golden` |
| `S09004-HISTORY-FLOOR-THIRTY` | `crates/curriculum/src/pace.rs` | the history p90's floor of 30 | `pace_goldens::the_adaptive_goal_matches_the_predecessors_golden` |
| `S09005-GOAL-HORIZON` | `crates/curriculum/src/pace.rs` | the goal's horizon of 365 days | `pace_goldens::the_pace_constants_equal_the_predecessors` |
| `S09006-NEAREST-RANK` | `crates/kernel/src/pynum.rs` | the rank's ceiling and its floor of 1 | `pynum_goldens::the_percentile_matches_the_predecessors_golden` |
| `S09007-NEGLECTED-UNDER-FIVE` | `crates/curriculum/src/pace.rs` | a share under 5.0 is neglected | `pace_goldens::the_balance_matches_the_predecessors_golden` |
| `S09008-BALANCE-WINDOW` | `crates/curriculum/src/pace.rs` | the window of 28 study days, today included | `pace_goldens::the_balance_matches_the_predecessors_golden` |
| `S09009-ONE-ROW-PER-KIND` | `migrations/009001_curriculum_pace_readouts.sql` | the key on `pace_readouts (kind, scope)` (a script row; the cargo killer) | `pace_step::the_pace_readouts_are_replaced_once_per_recompute` |
