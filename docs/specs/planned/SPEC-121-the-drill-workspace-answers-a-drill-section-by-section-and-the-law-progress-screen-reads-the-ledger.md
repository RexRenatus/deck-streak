# SPEC-121: the drill workspace answers a drill section by section, and the law progress screen reads the ledger

- **Wave:** W6. **Issue:** #55 (the drill workspace and progress screens) (epic #7).
  **Context(s):** the Mini App (`web/app`: the drill workspace and the law tab's progress);
  `deck-streak-coordination` (the law progress read model: the per-subject table, the drill backlog
  and the weekly history and trend, joined from the contexts that own them); `deck-streak-api`
  (`GET /api/law/progress`).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-054 (no-AI mode is the default),
  ADR-059 (public text describes DeckStreak only) and ADR-087 (the configured courses name the
  reading and the writing courses). No new decision is taken here, so this SPEC has no ADR of its
  own.
- **Prerequisites:** SPEC-020, SPEC-024, SPEC-040, SPEC-071, SPEC-077, SPEC-078, SPEC-092
  (planned, W4), SPEC-093 (planned, W4) and SPEC-110. **Mutation band:** `S12100-S12199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-121.md` (ADR-016).

## 1. The problem, measured

- **Answering a drill.** The predecessor answered a law drill in the vault note itself, or in a
  chat reply that `vault_bridge.py:append_drill_answer` appended and marked Ready for grading. The
  vault's drill templates give each drill type its answer headings (the four shapes open at
  `vault_bridge.py:_DRILL_ANSWER_HEADINGS`) and a `## Self-Check` list. SPEC-110 ports the append,
  its outcome set and the routes; #55 asks for a workspace: the prompt, a timer, the template's
  sections, the self-check and Submit for grading, which ticks Ready through the vault contract and
  never rewrites the answer.
- **The progress.** The predecessor's law dashboards were filled from a stats file
  (`vault_bridge.py:build_vault_stats`): the per-subject table (`_law_subject_rows`, with the
  subject fold of `_fold_subject_key` and `_folded_subject_keys`), the drill backlog
  (`_drills_block` over `_active_drill_rollup`), and the four-week history and trend
  (`_weeks_block`, `_weekly_history_rows`, `_weekly_trend`). #55 asks for native screens that read
  the database directly, never the vault's dashboards. The stats file itself is W8's (#153).
- **What already exists.** SPEC-110 serves the drills (`GET /api/drills`, `GET /api/drills/{id}`,
  `POST /api/drills/{id}/answer`, R15), its single view carries the note's `sections` and
  `self_check` (R2), and its rollup is `_active_drill_rollup`'s (R3). SPEC-077 adds the law tab
  (`/law`) and `GET /api/law` (R15).

## 2. Requirements

The drill workspace (Mini App)

R1. `/drills` joins `ROUTES`. It lists SPEC-110's drills, and opens a drill's workspace at
    `/drills?drill=<id>` from the single view: the title, the type, the prompt, one answer field per
    entry of `sections` in that order (one untitled field when `sections` is empty), the
    `self_check` lines as checkboxes, and a count-up timer from the workspace's opening.
R2. Submit for grading sends `POST /api/drills/{id}/answer` with one answer: for each section whose
    field holds more than spaces, the line `### <heading>` (the heading without its `## `), a blank
    line and the field's text exactly as typed, the sections joined by a blank line; with no
    sections, the one field's text exactly as typed. The workspace never trims, reorders or rewrites
    a field's text, and it sends nothing else; the append and the Ready marker are SPEC-110 R5's.
R3. Each outcome of SPEC-110 R4 is shown by name: `appended` shows Ready for grading and returns to
    the list; `empty_answer`, `not_active`, `already_answered` and `rail_refused` each show their own
    line. After any outcome but `appended`, and after a failed request, every field keeps its text.
R4. The timer and the self-check ticks live only in the open screen: neither is stored, sent or
    counted, and neither gates Submit.
R5. The workspace calls no model, so it behaves the same with the AI route absent (ADR-054).

The law progress read model (coordination)

R6. `coordination::law::progress(now)` answers three blocks, `subjects`, `drills` and `weeks`, each
    carrying `measured`. It reads the ledger and SPEC-110's drill views, and no vault dashboard or
    stats file.
R7. **The subject table.** Its subjects are the union of the law subjects of the deck names
    (SPEC-092's law subject, over the reader's name read) and the subjects of the law rows of
    SPEC-093's leech snapshot. Each row equals `goldens/law_subject_rows.json`
    (`vault_bridge.py:_law_subject_rows`): the subject, its deck count, its leeches by SPEC-093's
    status (total, active, holding, regressed) and its active drills (the drills of SPEC-110's list
    whose subject folds to the row's key). The fold equals `goldens/subject_fold.json`
    (`_fold_subject_key`: `&` to `and`, every run of whitespace removed, then full case folding as
    Python's `str.casefold`, which is not a lower-casing). Two subjects of the union that fold to
    one key are refused before any row is built, never merged (`_folded_subject_keys`). The rows
    are sorted by subject, at most 64 (`MAX_LAW_SUBJECT_ROWS`), with the count of rows past the
    64th.
R8. **The drill backlog** is SPEC-110 R3's rollup over every subject of the table, the rows past
    the 64th included (active, awaiting grading, the oldest age and creation, unmatched and
    deferred), and the count of `drill_grades` rows (`_drills_block`).
R9. **The weeks** are the four Monday-to-Sunday weeks ending with the week of the current study day
    (SPEC-020 R1, the 04:00 rollover), in order, and each week carries (`_weeks_block`):
    - its law XP and its language XP: the sums of the `xp_ledger` amounts of its study days on the
      `law` and the `language` track;
    - its drill grants: the count of its `xp_ledger` rows whose source begins `drill:` (SPEC-110 R9);
    - its language reviews: the sum of the `daily_lang_stats` reviews of its study days;
    - its habit days: the sum, over each configured course, of the distinct study days that course
      has in `minutes_log`, plus the sum, over each writing course, of the distinct study days that
      course has in `writing_log` (SPEC-078 R6, ADR-087); a code of no configured course counts
      nothing;
    - `observed`: whether any of its seven study days has a `daily_rollup` row.
    Each of the five tables is read by one query per request, bounded to the first week's start.
R10. **The history and the trend.** The history equals `goldens/weekly_history_rows.json`
     (`_weekly_history_rows`): the start and end, the days elapsed (0 to 7), `complete` only at 7,
     `observed`, and the six metrics. The trend equals `goldens/weekly_trend.json`
     (`_weekly_trend`): the basis, the latest week that is both complete and observed, the earlier
     such weeks it is compared with, the partial week's start, and per metric the value, the
     baseline (the mean rounded half to even, as Python's `round`), the delta and the direction
     (`up`, `down` or `flat`). With no such week, or none earlier, the metrics are empty.
R11. **Degradation.** A subject table that cannot be built (a read error or a refused fold) answers
     `measured` false, and so does the backlog, because unmatched drills cannot be counted without
     the table (`build_vault_stats`). A weeks block any of whose five reads fails answers `measured`
     false and leaves the other two blocks as they are. The predecessor degraded its weeks and its
     history separately; the screen shows only the history and the trend, so here they are one
     block. An unmeasured block carries no number: no rows, and the trend's empty shape.
R12. The constants (`WEEKLY_HISTORY_WEEKS` 4, `WEEKLY_TREND_METRICS`, `_WEEKLY_TREND_BASIS`,
     `MAX_LAW_SUBJECT_ROWS` 64) equal `goldens/law_progress.constants.json`.

The route and the screen

R13. `GET /api/law/progress` answers the three blocks to the owner only (SPEC-024); any other caller
     gets 401 or 403 and no data.
R14. The law tab (`/law`) shows the subject table, the backlog and the trend. An unmeasured block
     reads "couldn't measure", never a zero; the in-progress week is labelled partial.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the subject rows equal the golden, a drill subject spelled with `&` and spaces among the cases | `the_subject_rows_match_the_predecessors_golden` |
| A2 | the fold equals the golden, a subject whose case folding differs from its lower case and a collision among the cases | `the_subject_fold_matches_the_predecessors_golden` |
| A3 | a refused fold leaves the subject table and the backlog unmeasured and the weeks measured; a failed read of any one of the weeks' five tables leaves only the weeks unmeasured | `each_block_degrades_and_the_backlog_follows_the_subjects` |
| A4 | 64 subjects answer 64 rows and 0 past; 65 answer 64 rows, sorted, and 1 past | `the_subject_table_holds_64_rows_and_counts_the_rest` |
| A5 | the backlog is SPEC-110's rollup over every subject, the 65th included, and the `drill_grades` count | `the_drill_backlog_is_the_rollup_and_the_graded_count` |
| A6 | over a seeded ledger, each week's law XP, language XP, drill grants, language reviews, habit days and `observed` come from `xp_ledger`, `daily_lang_stats`, `minutes_log`, `writing_log` and `daily_rollup`, and a code of no configured course counts no habit day | `the_weeks_read_each_of_their_five_tables` |
| A7 | a review at 03:59 on a Monday belongs to the previous week and one at 04:00 to the new week | `a_week_starts_at_the_mondays_rollover` |
| A8 | the history equals the golden, the current week's Sunday (6 days elapsed, not complete) and a week with one observed day among the cases | `the_weekly_history_matches_the_predecessors_golden` |
| A9 | the trend equals the golden: no history, no earlier week, a partial and an unobserved week left out, means of 2.5 and 3.5, and deltas of -1, 0 and +1 | `the_weekly_trend_matches_the_predecessors_golden` |
| A10 | every constant equals the golden | `the_progress_constants_equal_the_predecessors` |
| A11 | with a dashboard note and a stats file planted in the vault, the answer is unchanged | `the_progress_reads_the_ledger_not_the_vault_dashboards` |
| A12 | the route answers the owner and refuses every other caller with no data | `the_law_progress_route_answers_only_the_owner` |
| A13 | the workspace shows the prompt, one field per section in order, the self-check lines and the timer | `the workspace shows the prompt, a field per section and the self-check` |
| A14 | Submit sends each non-blank section under its heading, each text exactly as typed, and nothing else | `submit sends each section's text unchanged` |
| A15 | a note with no answer heading offers one field, sent exactly as typed | `a note with no answer heading offers one field` |
| A16 | each of the five outcomes shows its line, and after each but `appended`, and after a failed request, every field keeps its text | `a refused or failed submit keeps every field` |
| A17 | the timer counts up, and neither it nor a self-check tick is stored, sent or gates Submit | `the timer and the self-check are never stored or sent` |
| A18 | the law tab shows the table, the backlog and the trend, and an unmeasured block reads "couldn't measure", never 0 | `an unmeasured block reads as not measured, never zero` |

```acceptance
A1: cargo test -p deck-streak-coordination --test law_progress -- --exact the_subject_rows_match_the_predecessors_golden
A2: cargo test -p deck-streak-coordination --test law_progress -- --exact the_subject_fold_matches_the_predecessors_golden
A3: cargo test -p deck-streak-coordination --test law_progress -- --exact each_block_degrades_and_the_backlog_follows_the_subjects
A4: cargo test -p deck-streak-coordination --test law_progress -- --exact the_subject_table_holds_64_rows_and_counts_the_rest
A5: cargo test -p deck-streak-coordination --test law_progress -- --exact the_drill_backlog_is_the_rollup_and_the_graded_count
A6: cargo test -p deck-streak-coordination --test law_progress -- --exact the_weeks_read_each_of_their_five_tables
A7: cargo test -p deck-streak-coordination --test law_progress -- --exact a_week_starts_at_the_mondays_rollover
A8: cargo test -p deck-streak-coordination --test law_progress -- --exact the_weekly_history_matches_the_predecessors_golden
A9: cargo test -p deck-streak-coordination --test law_progress -- --exact the_weekly_trend_matches_the_predecessors_golden
A10: cargo test -p deck-streak-coordination --test law_progress -- --exact the_progress_constants_equal_the_predecessors
A11: cargo test -p deck-streak-coordination --test law_progress -- --exact the_progress_reads_the_ledger_not_the_vault_dashboards
A12: cargo test -p deck-streak-api --test law_progress_route -- --exact the_law_progress_route_answers_only_the_owner
A13: pnpm exec vitest run web/app/src/lib/drills/Workspace.test.ts -t "the workspace shows the prompt, a field per section and the self-check"
A14: pnpm exec vitest run web/app/src/lib/drills/Workspace.test.ts -t "submit sends each section's text unchanged"
A15: pnpm exec vitest run web/app/src/lib/drills/Workspace.test.ts -t "a note with no answer heading offers one field"
A16: pnpm exec vitest run web/app/src/lib/drills/Workspace.test.ts -t "a refused or failed submit keeps every field"
A17: pnpm exec vitest run web/app/src/lib/drills/Workspace.test.ts -t "the timer and the self-check are never stored or sent"
A18: pnpm exec vitest run web/app/src/lib/law/LawProgress.test.ts -t "an unmeasured block reads as not measured, never zero"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The accessibility pack is already enforced, and this
delivery changes no pack's state, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `web/app/src/routes/drills/+page.svelte`, `web/app/src/routes/law/+page.svelte`, `web/app/src/lib/drills/` and `web/app/src/lib/law/`: both screens pass the accessibility audit in both Telegram colour schemes, the timer announced politely and every field labelled by its section | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/law/progress.rs` | `deck-streak-coordination` | added: the three blocks, the fold, the weeks, the history and the trend |
| `crates/coordination/src/law/mod.rs` | `deck-streak-coordination` | changed: the progress module |
| `crates/coordination/tests/law_progress.rs` | `deck-streak-coordination` | added: A1 to A11 |
| `crates/api/src/law_routes.rs` | `deck-streak-api` | changed: `GET /api/law/progress` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the route |
| `crates/api/tests/law_progress_route.rs` | `deck-streak-api` | added: A12 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the read model's ports in the `api` role |
| `web/app/src/routes/drills/+page.svelte` | miniapp | added: the list and the workspace |
| `web/app/src/lib/drills/Workspace.svelte` | miniapp | added |
| `web/app/src/lib/drills/drills.ts` | miniapp | added: SPEC-110's routes' client and types, and the answer's composition |
| `web/app/src/lib/drills/Workspace.test.ts` | miniapp | added: A13 to A17 |
| `web/app/src/routes/law/+page.svelte` | miniapp | changed: the progress blocks in the law tab |
| `web/app/src/lib/law/LawProgress.svelte` | miniapp | added |
| `web/app/src/lib/law/lawProgress.ts` | miniapp | added: the route's client and types |
| `web/app/src/lib/law/LawProgress.test.ts` | miniapp | added: A18 |
| `web/app/src/lib/routes.ts` | miniapp | changed: `/drills` joins `ROUTES` |
| `tools/parity-oracle/registry/spec_121.py` | repo | added: the goldens of §7 |
| `tools/parity-oracle/goldens/law_subject_rows.json`, `subject_fold.json`, `weekly_history_rows.json`, `weekly_trend.json`, `law_progress.constants.json` | repo | added: generated by §7 |
| `scripts/mutation-rows.d/S12100-S12199.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-121-the-drill-workspace-answers-a-drill-section-by-section-and-the-law-progress-screen-reads-the-ledger.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-121.md` | docs | added |
| `docs/schematics/law-drill-answer-grade-and-pay.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It shows no unit progress. The units and their can-do lines live in the vault's subject maps,
  which the owner curates by hand, and no table in the ledger holds them; where a unit view reads
  from is the owner's question (#55).
- It sets no time target for a drill type; a target is a runtime setting, and settings get their
  screen in W7 (#57).
- It stores no draft of an answer, no timer and no self-check tick; the answer's only record is
  SPEC-110's (#136).
- It writes no stats file into the vault and fills no vault dashboard; the stats bridge is W8's
  (#153).
- It adds no chart; the charts are SPEC-085's (#152).

## 6. Risks

- **A rewritten answer.** R2 sends each field exactly as typed; A14 compares the request body with
  the typed text byte for byte.
- **A merged subject.** R7 refuses a fold collision; detected by A2 and A3.
- **A lower-cased fold.** A lower-casing differs from Python's case folding outside the law
  vocabulary; A2's golden names such a subject.
- **A fabricated zero.** R11 and R14 answer an unmeasured block without numbers; detected by A3 and
  A18.
- **Rounding.** The trend's baseline rounds half to even, as the predecessor's `round`; A9 names
  2.5 and 3.5.
- **A week cut at midnight.** R9 uses the study day; detected by A7.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_121.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `law_subject_rows` | `vault_bridge.py:_law_subject_rows` | adapter | stubs the store's deck names and the law board with invented subjects: a subject with decks and no leech, one with leeches of each status and no deck, and drill counts keyed by a spelling with `&` and spaces |
| `subject_fold` | `vault_bridge.py:_fold_subject_key`, `_folded_subject_keys` | function | invented subjects with `&`, spaces, tabs and mixed case, one whose case folding differs from its lower case (`ß`), and one population with a collision |
| `weekly_history_rows` | `vault_bridge.py:_weekly_history_rows` | function | four synthetic weeks, with today on the current week's Monday and on its Sunday (0 and 6 elapsed days, a past week 7), and observed sets holding none, one and every day of a week |
| `weekly_trend` | `vault_bridge.py:_weekly_trend` | function | an empty history, one complete week, a partial last week, an unobserved middle week, baselines whose means are 2.5 and 3.5, and deltas of -1, 0 and +1 |
| `law_progress.constants` | `vault_bridge.WEEKLY_HISTORY_WEEKS`, `WEEKLY_TREND_METRICS`, `_WEEKLY_TREND_BASIS`, `MAX_LAW_SUBJECT_ROWS` | constants | none |

`_weeks_block` reads the predecessor's own tables and has no golden: A6 proves its sources as a set
over DeckStreak's tables, and A8 and A9 prove the arithmetic over its output.

## 8. Tables and the v9 import

No table. The screens read the tables SPEC-040, SPEC-071, SPEC-078, SPEC-093 and SPEC-110 own, and
W8's import (#61) fills those.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S12101-COMPLETE-AT-SEVEN` | `crates/coordination/src/law/progress.rs` | a week is complete only at 7 elapsed days; the golden names 6 and 7 | `law_progress::the_weekly_history_matches_the_predecessors_golden` |
| `S12102-OBSERVED-ANY-DAY` | `crates/coordination/src/law/progress.rs` | one observed day makes the week observed; the golden names one | `law_progress::the_weekly_history_matches_the_predecessors_golden` |
| `S12103-TREND-FILTER` | `crates/coordination/src/law/progress.rs` | only complete and observed weeks count; the golden names a partial and an unobserved week | `law_progress::the_weekly_trend_matches_the_predecessors_golden` |
| `S12104-HALF-EVEN` | `crates/coordination/src/law/progress.rs` | the baseline rounds half to even; the golden names 2.5 and 3.5 | `law_progress::the_weekly_trend_matches_the_predecessors_golden` |
| `S12105-FLAT-AT-ZERO` | `crates/coordination/src/law/progress.rs` | a zero delta is `flat`; the golden names -1, 0 and +1 | `law_progress::the_weekly_trend_matches_the_predecessors_golden` |
| `S12106-FOLD-REFUSES` | `crates/coordination/src/law/progress.rs` | a collision is refused, never merged | `law_progress::the_subject_fold_matches_the_predecessors_golden` |
| `S12107-CASEFOLD` | `crates/coordination/src/law/progress.rs` | the fold is a case folding, not a lower-casing; the golden names `ß` | `law_progress::the_subject_fold_matches_the_predecessors_golden` |
| `S12108-SUBJECT-CAP` | `crates/coordination/src/law/progress.rs` | 64 rows; the test names 64 and 65 | `law_progress::the_subject_table_holds_64_rows_and_counts_the_rest` |
| `S12109-BACKLOG-DEPENDS` | `crates/coordination/src/law/progress.rs` | an unmeasured table leaves the backlog unmeasured | `law_progress::each_block_degrades_and_the_backlog_follows_the_subjects` |
| `S12110-ROLLOVER-WEEK` | `crates/coordination/src/law/progress.rs` | weeks are cut at the study day; the test names 03:59 and 04:00 | `law_progress::a_week_starts_at_the_mondays_rollover` |
| `S12111-CONFIGURED-CODES` | `crates/coordination/src/law/progress.rs` | habit days count configured courses' codes only | `law_progress::the_weeks_read_each_of_their_five_tables` |
| `S12112-OWNER-ONLY` | `crates/api/src/law_routes.rs` | the route refuses any caller but the owner | `law_progress_route::the_law_progress_route_answers_only_the_owner` |
