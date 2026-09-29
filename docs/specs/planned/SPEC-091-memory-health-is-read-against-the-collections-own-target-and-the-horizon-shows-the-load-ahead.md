# SPEC-091: memory health is read against the collection's own target, and the horizon shows the load ahead

- **Wave:** W4. **Issue:** #90, #91 (epic #5). **Context(s):** `deck-streak-curriculum` (memory
  health, calibration and its advice, input readiness, stability depth, the mature trend, the
  horizon, `memory_readouts`); `deck-streak-coordination` (the memory step in phase 4 of the fold,
  which passes analytics' rollups and day number in; the read models); `deck-streak-api`,
  `deck-streak-bot` and the Mini App (the routes, `/memory`, the Memory tab and the horizon chart).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-085 (charts render on the
  client), ADR-090 (CPython's numeric semantics are ported once, into the kernel) and ADR-091
  (curriculum's readouts are computed in the current study day's step and stored as the latest
  readout).
- **Prerequisites:** SPEC-029, SPEC-071 (the rollups, their mature split and the collection's day
  number), SPEC-077 (each card's memory state and retrievability), SPEC-085 (the chart loader),
  SPEC-090 (the kernel's numeric port) and SPEC-092 (the strand parse input readiness reads).
  **Mutation band:** `S09100-S09199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-091.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 `crates/curriculum/src/` holds only `lib.rs`. SPEC-077
  (planned) publishes each card's memory state and retrievability, and SPEC-071 (planned) the daily
  rollups with their young and mature split; nothing reads either for memory health.
- **What is ported.** Memory health (#90): `coaching.py:_memory_health`, `_calibration`,
  `_collection_desired_pct`, `_retention_advice` and `_mature_trend`, and
  `comprehension.py:compute_input_readiness` and `compute_stability_depth`. The horizon (#91):
  `horizon.py:compute_horizon` and `build_readout`, as `coaching.py:horizon_readout` calls them.
- **Traps a hand port falls into.**
  - Memory health skips cards with no memory state and suspended cards (queue −1); a card is
    retrievable at R 0.9 or more, and at risk below 0.8 only when it is a review card (type 2) with
    an interval of 21 days or more.
  - Calibration reads the 7 NEWEST rollups (rollups arrive newest first) with at least 5 answers,
    averages their true retention with `sum/len` over floats, compares it with the collection's
    desired retention within 5 points, and rounds the mean to one place; with none it is no-data.
  - The desired retention is the MEDIAN of each card's desired retention times 100 (an even count
    averages the two middle values), rounded to one place, and 90.0 when no card carries one.
  - The mature trend reads the 30 newest rollups, skips a day whose mature split is missing or zero
    (a missing split is never 0 %), needs 5 mature answers over the window, and reports the
    newer half's answer-weighted retention minus the older half's only from 4 data days.
  - Input readiness counts only strands whose name starts with "vocab" in any case, through
    SPEC-092's parse; it skips suspended and never-studied new cards, calls a card known at R 0.9 or
    more, and reads "not enough data yet" below 20 introduced cards.
  - Stability depth is the median stability in days and the share at 100 days or more.
  - The horizon classifies by queue FIRST: queues −1 to −3 are excluded, a new card (type 0 or
    queue 0) is counted as new, queues 1 and 4 hold epoch seconds and are owed now, a borrowed card
    with a negative due holds a filtered position, and only queues 2 and 3 are bucketed by
    `due − today`, arrears clamped to day 0, over 365 days. `today` is the collection's day number
    (SPEC-071), never a date.
  - Memory health, readiness, depth and the horizon read the clock (retrievability decays and the
    day number moves), so they are computed at the current study day's recompute, never stored per
    day (ADR-091).
- **What the parity oracle proves.** Every readout over synthetic cards, memory states, rollups and
  courses: the retrievable and at-risk edges, each calibration status, each readiness band, the
  trend's missing splits and half split, and the horizon's every queue branch.
- **Prerequisites.** SPEC-029, SPEC-071, SPEC-077, SPEC-085, SPEC-090 and SPEC-092, as the header
  names them.

## 2. Requirements

The readouts (#90)

R1. Memory health (retrievable, at risk and the count with a memory state) equals
    `goldens/memory_health.json` (`coaching.py:_memory_health`) at the recompute's clock.
R2. The collection's desired retention equals `goldens/desired_retention.json`
    (`coaching.py:_collection_desired_pct`), through the kernel's median and rounding (SPEC-090).
R3. Calibration equals `goldens/calibration.json` (`coaching.py:_calibration`), and its advice
    equals `goldens/retention_advice.json` (`coaching.py:_retention_advice`).
R4. The mature trend equals `goldens/mature_trend.json` (`coaching.py:_mature_trend`), fed the
    rollups newest first, as the predecessor's store returns them.
R5. Input readiness per course equals `goldens/input_readiness.json`
    (`comprehension.py:compute_input_readiness`), and stability depth per course equals
    `goldens/stability_depth.json` (`comprehension.py:compute_stability_depth`).

The horizon (#91)

R6. The horizon's 365-day histogram, its new and owed-now counts and its 30-day obligation equal
    `goldens/horizon.json` (`horizon.py:compute_horizon`), and its readout, naming the obligation
    and both levers, equals `goldens/horizon_readout.json` (`horizon.py:build_readout`, with R2's
    desired retention).

The step and the table (ADR-091)

R7. The memory step registers in phase 4 of SPEC-071's fold and runs in the current study day's
    step only. Coordination passes it analytics' rollups (newest first) and the collection's day
    number; curriculum reads no analytics table. It writes each readout to `memory_readouts`, one row
    per kind and scope, replacing the previous row in one write.
R8. `memory_readouts` is a `STRICT` table with `created_at`, created by
    `migrations/009101_curriculum_memory_readouts.sql` (SPEC-020 R15, R18). It is registered in the
    context map's register of DeckStreak's own tables, declared in `privacy.json` (the category
    `memory-health`), given a line in `PRIVACY.md`, and exported and erased by curriculum's
    data-rights port; the symmetry test seeds it.

The surfaces

R9. `GET /api/curriculum/memory` and `GET /api/curriculum/horizon` serve the readouts to the
    owner's session only; any other caller is answered 401 or 403 with no data, and a missing
    readout reads as pending.
R10. `/memory` keeps its name and states memory health, calibration with its advice, readiness per
     course, depth per course and the mature trend, as the memory route states them.
R11. The Mini App's Memory tab shows the readouts as gauges, each drawn as a meter with its value
     as text, with the retention advice beside calibration; the horizon is a bar chart of the 365
     days drawn on the client through SPEC-085's loader, beside its readout's obligation and levers
     as text.
R12. Every constant this SPEC uses (the retrievable and at-risk edges, the mature interval, the
     calibration window, minimum and tolerance, the fallback retention, the trend's window, minimum
     and data days, the readiness bands, the known edge, the deep stability and the minimum
     introduced, the horizon's 365 days, 30-day window, flat peak of 25 and queue sets) equals
     `goldens/memory.constants.json`, held by a test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | memory health equals the golden of `coaching.py:_memory_health` | `memory_health_matches_the_predecessors_golden` |
| A2 | the desired retention equals the golden of `coaching.py:_collection_desired_pct` | `the_desired_retention_matches_the_predecessors_golden` |
| A3 | calibration equals the golden of `coaching.py:_calibration` | `calibration_matches_the_predecessors_golden` |
| A4 | the advice equals the golden of `coaching.py:_retention_advice` | `retention_advice_matches_the_predecessors_golden` |
| A5 | the mature trend equals the golden of `coaching.py:_mature_trend` | `the_mature_trend_matches_the_predecessors_golden` |
| A6 | input readiness equals the golden of `comprehension.py:compute_input_readiness` | `input_readiness_matches_the_predecessors_golden` |
| A7 | stability depth equals the golden of `comprehension.py:compute_stability_depth` | `stability_depth_matches_the_predecessors_golden` |
| A8 | the horizon equals the golden of `horizon.py:compute_horizon` | `the_horizon_matches_the_predecessors_golden` |
| A9 | the horizon's readout equals the golden of `horizon.py:build_readout` | `the_horizon_readout_matches_the_predecessors_golden` |
| A10 | every memory constant equals `goldens/memory.constants.json` | `the_memory_constants_equal_the_predecessors` |
| A11 | the step passes the rollups newest first, so calibration reads the latest seven | `the_memory_step_reads_the_newest_rollups` |
| A12 | two recomputes store one row per kind and scope, and the second replaces the first | `the_memory_readouts_are_replaced_once_per_recompute` |
| A13 | `memory_readouts` is exported and erased by curriculum's port | `the_memory_readouts_are_exported_and_erased` |
| A14 | both routes answer the owner and refuse every other caller with no data | `the_memory_routes_answer_only_the_owner` |
| A15 | `/memory` states each readout the route states | `memory_states_each_readout` |
| A16 | each gauge states its value as text beside its meter | `states each gauge value as text` |
| A17 | the horizon chart states the thirty-day obligation and both levers as text | `states the obligation and both levers as text` |

```acceptance
A1: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact memory_health_matches_the_predecessors_golden
A2: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact the_desired_retention_matches_the_predecessors_golden
A3: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact calibration_matches_the_predecessors_golden
A4: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact retention_advice_matches_the_predecessors_golden
A5: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact the_mature_trend_matches_the_predecessors_golden
A6: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact input_readiness_matches_the_predecessors_golden
A7: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact stability_depth_matches_the_predecessors_golden
A8: cargo test -p deck-streak-curriculum --test horizon_goldens -- --exact the_horizon_matches_the_predecessors_golden
A9: cargo test -p deck-streak-curriculum --test horizon_goldens -- --exact the_horizon_readout_matches_the_predecessors_golden
A10: cargo test -p deck-streak-curriculum --test memory_goldens -- --exact the_memory_constants_equal_the_predecessors
A11: cargo test -p deck-streak-coordination --test memory_step -- --exact the_memory_step_reads_the_newest_rollups
A12: cargo test -p deck-streak-coordination --test memory_step -- --exact the_memory_readouts_are_replaced_once_per_recompute
A13: cargo test -p deck-streak-curriculum --test memory_store -- --exact the_memory_readouts_are_exported_and_erased
A14: cargo test -p deck-streak-api --test memory_routes -- --exact the_memory_routes_answer_only_the_owner
A15: cargo test -p deck-streak-bot --test memory_commands -- --exact memory_states_each_readout
A16: pnpm exec vitest run web/app/src/lib/memory/MemoryGauges.test.ts -t "states each gauge value as text"
A17: pnpm exec vitest run web/app/src/lib/memory/HorizonChart.test.ts -t "states the obligation and both levers as text"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and accessibility packs stay
enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/curriculum/src/data_rights.rs`: the `memory-health` category names `memory_readouts` with its purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/memory/+page.svelte` and every file under `web/app/src/lib/memory/`: the gauges and the horizon chart pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/curriculum/src/memory.rs` | `deck-streak-curriculum` | added: memory health, the desired retention, calibration and its advice, the mature trend |
| `crates/curriculum/src/comprehension.rs` | `deck-streak-curriculum` | added: input readiness and stability depth |
| `crates/curriculum/src/horizon.rs` | `deck-streak-curriculum` | added: the horizon and its readout |
| `crates/curriculum/src/memory_store.rs` | `deck-streak-curriculum` | added: `memory_readouts`, read and replaced |
| `crates/curriculum/src/data_rights.rs` | `deck-streak-curriculum` | changed: the port exports and erases `memory_readouts` |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the modules |
| `crates/curriculum/tests/memory_goldens.rs` | `deck-streak-curriculum` | added: A1 to A7, A10 |
| `crates/curriculum/tests/horizon_goldens.rs` | `deck-streak-curriculum` | added: A8, A9 |
| `crates/curriculum/tests/memory_store.rs` | `deck-streak-curriculum` | added: A13 |
| `migrations/009101_curriculum_memory_readouts.sql` | `deck-streak-curriculum` | added: `memory_readouts` |
| `crates/coordination/src/recompute/memory.rs` | `deck-streak-coordination` | added: the memory step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the memory step in phase 4 |
| `crates/coordination/src/memory.rs` | `deck-streak-coordination` | added: the memory and horizon read models |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the read models |
| `crates/coordination/tests/memory_step.rs` | `deck-streak-coordination` | added: A11, A12 |
| `crates/api/src/memory_routes.rs` | `deck-streak-api` | added: the two routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes behind the owner's session |
| `crates/api/tests/memory_routes.rs` | `deck-streak-api` | added: A14 |
| `crates/bot/src/memory_commands.rs` | `deck-streak-bot` | added: the memory command |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command joins the table |
| `crates/bot/tests/memory_commands.rs` | `deck-streak-bot` | added: A15 |
| `web/app/src/routes/memory/+page.svelte` | miniapp | added: the Memory tab |
| `web/app/src/lib/memory/memory.ts` | miniapp | added: the routes' client and types |
| `web/app/src/lib/memory/MemoryGauges.svelte` | miniapp | added |
| `web/app/src/lib/memory/MemoryGauges.test.ts` | miniapp | added: A16 |
| `web/app/src/lib/memory/HorizonChart.svelte` | miniapp | added: the bar chart through the loader |
| `web/app/src/lib/memory/HorizonChart.test.ts` | miniapp | added: A17 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /memory joins `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `memory_readouts` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry lists `memory_readouts` under curriculum's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `memory_readouts` |
| `privacy.json` | repo | changed: the `memory-health` category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_091.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/memory_health.json` | repo | added: the golden of `coaching.py:_memory_health` (adapter; synthetic cards at a fixed clock) |
| `tools/parity-oracle/goldens/desired_retention.json` | repo | added: the golden of `coaching.py:_collection_desired_pct` (adapter; odd, even and empty counts) |
| `tools/parity-oracle/goldens/calibration.json` | repo | added: the golden of `coaching.py:_calibration` (function; synthetic rollups) |
| `tools/parity-oracle/goldens/retention_advice.json` | repo | added: the golden of `coaching.py:_retention_advice` (function) |
| `tools/parity-oracle/goldens/mature_trend.json` | repo | added: the golden of `coaching.py:_mature_trend` (function; synthetic rollups) |
| `tools/parity-oracle/goldens/input_readiness.json` | repo | added: the golden of `comprehension.py:compute_input_readiness` (adapter; synthetic courses) |
| `tools/parity-oracle/goldens/stability_depth.json` | repo | added: the golden of `comprehension.py:compute_stability_depth` (adapter; synthetic courses) |
| `tools/parity-oracle/goldens/horizon.json` | repo | added: the golden of `horizon.py:compute_horizon` (adapter; synthetic cards in every queue) |
| `tools/parity-oracle/goldens/horizon_readout.json` | repo | added: the golden of `horizon.py:build_readout` (adapter) |
| `tools/parity-oracle/goldens/memory.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S09100-S09199.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-091-memory-health-is-read-against-the-collections-own-target-and-the-horizon-shows-the-load-ahead.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/curriculum-readouts-and-the-can-do-pass.md` | docs | added by the W4 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-091.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It computes no retention by hour and no best study hour; those belong to the digest's coaching
  (#53).
- It writes nothing into the daily digest (#129).
- It serves no readout, season, milestone or horizon to the agent's machine read tool (#157).
- It changes no desired retention in the collection; the advice is a line for the owner to act on
  in Anki, and no setting is edited in the Mini App (#57).
- It imports none of the predecessor's coaching payloads; the first recompute computes them (#61).

## 6. Risks

- **Calibration reads the oldest rollups** when the order is reversed. Detected by A11.
- **A missing mature split reads as 0 %.** Detected by A5, whose cases include missing splits.
- **An intraday card lands 1.78 billion days out.** Detected by A8's queue 1 and 4 cases.
- **The horizon's chart ships a second copy of the chart library.** Prevented by SPEC-085's single
  loader, and detected by its test that only the loader imports it.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_091.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `memory_health` | `coaching.py:_memory_health` | adapter | synthetic cards with and without memory states, suspended, at the edges 0.8 and 0.9, at a fixed `now_sec` |
| `desired_retention` | `coaching.py:_collection_desired_pct` | adapter | synthetic cards whose memory states carry odd and even counts of desired retentions, and none |
| `calibration` | `coaching.py:_calibration` | function | none: rollups newest first, each status and the tolerance's edges |
| `retention_advice` | `coaching.py:_retention_advice` | function | none: each status |
| `mature_trend` | `coaching.py:_mature_trend` | function | none: rollups with missing and zero splits, below the minimum, 3 and 4 data days |
| `input_readiness` | `comprehension.py:compute_input_readiness` | adapter | synthetic courses patched where the module reads them, vocab and other strands, each band and 19 and 20 introduced |
| `stability_depth` | `comprehension.py:compute_stability_depth` | adapter | synthetic courses and stabilities around 100 days |
| `horizon` | `horizon.py:compute_horizon` | adapter | synthetic cards in every queue, a borrowed card with a negative due, arrears and a due past 365 days |
| `horizon_readout` | `horizon.py:build_readout` | adapter | the same cards with a desired retention |
| `memory.constants` | `coaching._CALIBRATION_TOL_PCT`, `_MATURE_TREND_DAYS`, `_MATURE_TREND_MIN_ANSWERED`; the `comprehension` and `horizon` module constants | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `memory_readouts` | `deck-streak-curriculum` | `migrations/009101_curriculum_memory_readouts.sql` | the coaching values it kept per key, which the import does not carry: the first recompute recomputes them | exported and erased: no row reads as pending |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09101-RETRIEVABLE-EDGE` | `crates/curriculum/src/memory.rs` | retrievable at R 0.9 | `memory_goldens::memory_health_matches_the_predecessors_golden` |
| `S09102-AT-RISK-INTERVAL` | `crates/curriculum/src/memory.rs` | at risk needs an interval of 21 days | `memory_goldens::memory_health_matches_the_predecessors_golden` |
| `S09103-CALIBRATION-TOLERANCE` | `crates/curriculum/src/memory.rs` | the 5-point tolerance | `memory_goldens::calibration_matches_the_predecessors_golden` |
| `S09104-FALLBACK-NINETY` | `crates/curriculum/src/memory.rs` | the desired retention's fallback of 90.0 | `memory_goldens::the_desired_retention_matches_the_predecessors_golden` |
| `S09105-TREND-DATA-DAYS` | `crates/curriculum/src/memory.rs` | the delta needs 4 data days | `memory_goldens::the_mature_trend_matches_the_predecessors_golden` |
| `S09106-MIN-INTRODUCED` | `crates/curriculum/src/comprehension.rs` | 20 introduced cards | `memory_goldens::input_readiness_matches_the_predecessors_golden` |
| `S09107-DAY-NUMBER-QUEUES` | `crates/curriculum/src/horizon.rs` | only queues 2 and 3 are bucketed | `horizon_goldens::the_horizon_matches_the_predecessors_golden` |
| `S09108-HORIZON-DAYS` | `crates/curriculum/src/horizon.rs` | the 365-day horizon | `memory_goldens::the_memory_constants_equal_the_predecessors` |
| `S09109-ONE-ROW-PER-KIND` | `migrations/009101_curriculum_memory_readouts.sql` | the key on `memory_readouts (kind, scope)` (a script row; the cargo killer) | `memory_step::the_memory_readouts_are_replaced_once_per_recompute` |
