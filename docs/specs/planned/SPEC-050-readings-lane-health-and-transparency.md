# SPEC-050: the readings' health is judged from fires and outputs, every could-not-tell reason has one class, correct refusals never page, and the owner sees why

- **Wave:** W1. **Issue:** #36 (epic #2). **Context(s):** `deck-streak-readings` (the verdict, the triage, the census); `deck-streak-coordination` (the check and its pages); `deck-streak-api` and the Mini App (the status panel).
- **Decided by:** ADR-012 (the parity oracle, and `diverges` for an intended difference), ADR-019
  (the last sync, not the file's age, decides; an absent owner is a pause), and ADR-050 (pages through
  the router, once per state and study day, and the status panel).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-050.md` (ADR-016).

## 1. The problem, measured

- **The predecessor reported an absent owner as a failure.** Every night its lane refused on the
  stale-snapshot gate, and by code path it sent a coalesced "topics failed tonight" message that
  ignored quiet hours, plus a health page, although the refusal was correct and the owner was simply
  not studying (the second-brain inventory, private). Its health verdict itself was sound
  (`preread_health.py:verdict`: not installed, dark, quiet, healthy, stale, armed and refusing, one
  page per state per day), and so was its triage of every could-not-tell reason
  (`undetermined_triage.py:classify_undetermined`) and its census of deck roots
  (`preread_census.py:classify`); its telemetry was visible only on a metrics endpoint the owner
  never opens.
- **What changes.** The owner's staleness decision removed the file-age gate (ADR-019), so the
  predecessor's `stale_snapshot` reason and its `owner_absent` class have no counterpart here: an
  absent owner is `paused` (SPEC-045), which is never a failure.
- **What the parity oracle proves.** `preread_health.py:verdict`, `undetermined_triage.py:classify_undetermined`
  for every reason both systems have (the `stale_snapshot` cases carry `diverges` with ADR-019), and
  `preread_census.py:classify`.
- **Prerequisites.** SPEC-045 and SPEC-046 (states, runs and attempts), SPEC-041 (the router's alert
  kind), SPEC-043 (`agent_runs`), SPEC-027 (the cron-fire ledger and the hourly dead-man watch), and
  SPEC-051 (the Mini App's readings client, which gains the panel).

## 2. Requirements

R1. The health verdict is a pure function of the evidence it is given: whether the readings tables
    are installed, whether the generation job has fired, the number of runs, the study day of the
    latest run and of the latest ready reading, and the number of refusing topics in the latest run.
    It returns `not_installed`, `dark` (fired, no run ever), `quiet` (a recent run with nothing ready
    and nothing refusing, or no fire yet), `healthy` (a ready reading inside the window, which takes
    precedence over refusals), `stale` (the latest run older than the window) or
    `armed_and_refusing` (no recent reading and a refusing topic). The window is 1 study day
    (`preread_health.py:FRESHNESS_WINDOW_DAYS`). It equals the golden of `preread_health.py:verdict`,
    with the predecessor's schema version mapped to "installed".
R2. `could_not_tell` and `failed` topics refuse; `no_new_cards` and `paused` never do.
R3. Every could-not-tell reason has exactly one class, `rail_broken` or `config_fault` (SPEC-045's
    closed map), and a test over the reason enum proves the partition total. The mapping equals the
    golden of `undetermined_triage.py:classify_undetermined` for every reason both systems have.
R4. `dark`, `stale`, `armed_and_refusing`, and each could-not-tell class present, raise an `alert`
    occasion through the router (SPEC-041), deduplicated per state and study day, so each pages at most
    once a study day; alerts are exempt from quiet hours by the policy. `no_new_cards`, `paused`,
    `quiet` and `healthy` never page. A page names the state, the class and the counts, and never a
    topic key, a deck name or a path.
R5. The check runs after every generation run and at every hourly dead-man watch (SPEC-027), so a
    generation job that never fires is caught as `dark` or `stale`.
R6. `GET /api/readings/status` answers the authenticated owner only, with the last run (start, end,
    trigger, outcome), each topic's state with its class and reason, the health verdict, the census,
    and 30 days of attempts, tokens and estimated cost summed per study day from `reading_attempts`
    and `agent_runs`. The Mini App's status screen shows them, the cost labelled as the CLI's estimate.
R7. The census classes each top-level deck root `live`, `dormant` (never reviewed), `dead` (no new
    card available) or `unmeasured` (its read failed), with whether a topic binds it; the live window
    is 30 days (`preread_census.py:LIVE_WINDOW_DAYS`), and the classes equal the golden of
    `preread_census.py:classify`. A read failure is `unmeasured`, never a zero.
R8. There is no spend cap; the panel's cost is information, and the agent's per-run caps remain the
    safety bound.
R9. No identifier this delivery declares in the readings context says `lane` (the lexicon's lock for
    `topic`); the verdict is `ReadingsHealth`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the verdict distinguishes not installed, dark, quiet, healthy, stale and armed and refusing, equal to the golden of `preread_health.py:verdict` | `the_health_verdict_matches_the_parity_golden` |
| A2 | a run whose topics are all `no_new_cards` or `paused` raises no alert | `no_new_cards_and_a_pause_never_page` |
| A3 | a broken rail or a config fault pages once per study day however many runs and watches see it, and again on the next study day | `a_broken_rail_or_a_config_fault_pages_once_per_study_day` |
| A4 | every could-not-tell reason has exactly one class (examined count equals the reasons) | `every_could_not_tell_reason_has_exactly_one_class` |
| A5 | the triage equals the golden of `undetermined_triage.py:classify_undetermined` where the reasons exist in both, and the diverging cases cite ADR-019 | `the_triage_matches_the_parity_golden_where_both_have_the_reason` |
| A6 | a generation job that never fires is caught as `dark` or `stale` by the hourly watch (injected clock) | `a_generation_that_never_fired_is_caught_by_the_hourly_watch` |
| A7 | the status route serves the last run, each topic's state and 30 days of attempts, tokens and cost to the owner only | `the_status_route_shows_the_last_run_states_and_thirty_days_of_cost` |
| A8 | the status screen shows the last run, each topic's state and thirty days of attempts, tokens and estimated cost | `shows the last run, the state of every topic and thirty days of cost` |
| A9 | the census equals the golden of `preread_census.py:classify`, and a failed read is `unmeasured` | `the_root_census_matches_the_parity_golden` |
| A10 | a page's payload carries no topic key, deck name or path (planted names refused) | `a_page_names_no_topic_deck_or_path` |

```acceptance
A1: cargo test -p deck-streak-readings --test health -- --exact the_health_verdict_matches_the_parity_golden
A2: cargo test -p deck-streak-coordination --test readings_health -- --exact no_new_cards_and_a_pause_never_page
A3: cargo test -p deck-streak-coordination --test readings_health -- --exact a_broken_rail_or_a_config_fault_pages_once_per_study_day
A4: cargo test -p deck-streak-readings --test triage -- --exact every_could_not_tell_reason_has_exactly_one_class
A5: cargo test -p deck-streak-readings --test triage -- --exact the_triage_matches_the_parity_golden_where_both_have_the_reason
A6: cargo test -p deck-streak-coordination --test readings_health -- --exact a_generation_that_never_fired_is_caught_by_the_hourly_watch
A7: cargo test -p deck-streak-api --test readings_status -- --exact the_status_route_shows_the_last_run_states_and_thirty_days_of_cost
A8: pnpm exec vitest run web/app/src/routes/readings/status/status.test.ts -t "shows the last run, the state of every topic and thirty days of cost"
A9: cargo test -p deck-streak-readings --test census -- --exact the_root_census_matches_the_parity_golden
A10: cargo test -p deck-streak-coordination --test readings_health -- --exact a_page_names_no_topic_deck_or_path
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/readings/src/health.rs` | `deck-streak-readings` | added: `ReadingsHealth` |
| `crates/readings/src/triage.rs` | `deck-streak-readings` | added |
| `crates/readings/src/census.rs` | `deck-streak-readings` | added |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed |
| `crates/readings/tests/health.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/triage.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/census.rs` | `deck-streak-readings` | added |
| `crates/coordination/src/readings/health.rs` | `deck-streak-coordination` | added: the evidence, the check, the pages |
| `crates/coordination/src/readings/status.rs` | `deck-streak-coordination` | added: the status view |
| `crates/coordination/src/liveness.rs` | `deck-streak-coordination` | changed: the hourly watch runs the readings check |
| `crates/coordination/tests/readings_health.rs` | `deck-streak-coordination` | added |
| `crates/api/src/routes/readings.rs` | `deck-streak-api` | changed: the status route |
| `crates/api/tests/readings_status.rs` | `deck-streak-api` | added |
| `web/app/src/routes/readings/status/+page.svelte` | miniapp | added |
| `web/app/src/routes/readings/status/status.test.ts` | miniapp | added |
| `web/app/src/lib/readings/api.ts` | miniapp | changed: the status call |
| `tools/parity-oracle/generate.py` | repo | changed: registers the three functions |
| `tools/parity-oracle/goldens/verdict.json` | repo | added: `preread_health.py:verdict` |
| `tools/parity-oracle/goldens/classify_undetermined.json` | repo | added |
| `tools/parity-oracle/goldens/classify.json` | repo | added: `preread_census.py:classify` |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-050-readings-lane-health-and-transparency.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-050-readings-health-pages-and-status-panel.md` | docs | added |
| `docs/red-first/SPEC-050.md` | docs | added |

## 5. What this does NOT do

- It exports no metrics series; the panel and the structured logs carry the same facts, and a metrics
  endpoint is the observability delivery's (#24).
- It adds no readings line to the daily digest (#129).
- It alerts on no correct refusal, and it offers no comeback reading (#35).
- It schedules no generation; it only notices when one did not happen (#39).

## 6. Risks

- **A page leaks a topic or a deck name** into Telegram. Prevented by R4 and tested by A10.
- **The hourly watch and the job both page one state.** Deduplicated per state and study day (A3).
- **The cost figure is read as money spent.** It is the CLI's estimate through the owner's
  subscription, labelled so on the panel.
- **Golden file names collide.** The oracle names a golden by its function's last segment
  (`verdict.json`, `classify.json`); a later wave porting a function of the same name must rename
  one, which the golden reader's provenance check would expose.
