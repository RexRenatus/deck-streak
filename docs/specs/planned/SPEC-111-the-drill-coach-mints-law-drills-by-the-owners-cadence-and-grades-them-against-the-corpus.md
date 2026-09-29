# SPEC-111: the drill coach mints law drills by the owner's cadence and grades them against the corpus

- **Wave:** W6. **Issue:** #46 (epic #7). **Context(s):** `deck-streak-agent` (the drill coach's two
  tasks, its run plan, the grade's acceptance and its XP mapping); `deck-streak-vault` (the table
  `drill_mints`, the staged runs that create, grade and archive a drill note);
  `deck-streak-coordination` (the `drill_coach` job and its use case, the `drill_ready` occasion).
- **Decided by:** ADR-043 (the shell runner, its gate and caps), ADR-054 (no-AI mode is the
  default), ADR-110 (a graded drill is paid once by SPEC-110's post-back), ADR-111 (the drill
  coach's plan is the engine's, and a grade is accepted by the law gate and a score range) and
  ADR-113 (a prepared duty's ping is a router occasion).
- **Prerequisites:** SPEC-027, SPEC-040, SPEC-041, SPEC-042, SPEC-043, SPEC-044 and SPEC-110.
  **Mutation band:** `S11100-S11199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-111.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** SPEC-110 answers and pays a drill that is already in the vault; nothing in
  DeckStreak writes one or grades one. The study-duties pack's drill-coach template asks for one
  drill on the learner's weak points, a `drill` section that tells nothing, a law professor's `rule`
  section that is kept hidden until the learner has attempted, and a grade that cites the corpus.
- **What the predecessor did** (at `27ee2bc`): its law drills were written and graded by its agent
  outside the product code, and the product read them back (`vault_bridge.py:poll_drill_postbacks`,
  `_parse_graded_drill`). Its grading rule survives as a comment: the grader maps its score to an XP
  between 10 and 25, and the product clamps whatever it wrote. So there is no product function to
  port for the plan, and the plan is DeckStreak's own, decided by ADR-111.
- **Two rules of the vault-duties pack bind the note.** A vault-duty note's text, properties
  included, holds no calendar date (its `no-dates` class), so a drill DeckStreak mints cannot carry
  the `created` property the predecessor's notes carry; and a duty writes, moves and never deletes
  only inside its own layout folders.
- **No-AI is the default** (ADR-054): with the route absent the coach mints and grades nothing, and
  the drills already in the vault stay answerable through SPEC-110.

## 2. Requirements

The run plan (the engine's, never the model's)

R1. The cadence is configuration, `DECKSTREAK_DRILL_CADENCE`: seven entries, Monday to Sunday of the
    study day, each a law drill code of SPEC-110 R14 or `-`. Unset, it is seven `-`, so nothing is
    minted. `.env.example` names it with a neutral example.
R2. The job `drill_coach` in `coordination::jobs::TABLE` (SPEC-027 R1) runs daily at a minute the
    job table's census admits (R2), not `catch_up`. Each run, in order: archives, then grades, then
    mints.
R3. **Archive.** An unanswered drill (SPEC-110 R4's marker) whose age is at least
    `DECKSTREAK_DRILL_ARCHIVE_DAYS` study days (example 7) is moved, byte for byte, from `Active`
    to `Archive` in one staged run per job run that the engine writes (SPEC-042 R4). An answered
    drill is never archived. Nothing is deleted. The archive needs no AI route.
R4. **The zero-backlog gate.** A drill is minted only when the cadence names a code for the study
    day AND the unanswered list is empty. At most one drill is minted per study day, held by its
    `drill_mints` row's study day.
R5. **The subject.** The drill's subject is the first of the roster's law topics (SPEC-044 R2), in
    the roster's order, with no `drill_mints` row, else the one whose latest mint is oldest, ties in
    the roster's order. A roster with no law topic mints nothing.

The mint

R6. The task `drill-mint` (format `persona`, kind `law`) composes, in SPEC-043 R9's order, the law
    professor persona of the subject, `agent/duties/drill-coach.duty.md`, the subject's memory
    (`drill-grades` and `leeches`, SPEC-044 R5) and the drill code. It holds no tool (SPEC-043 R8)
    and runs under SPEC-043 R6's default caps.
R7. The engine writes the note's frontmatter (SPEC-044 R6) with `status: active`, `type` and
    `subject`, and no date. The note's body holds the `drill` section and the unticked Ready marker
    SPEC-110 reads. The persona's `rule` section is held in `drill_mints` and appears in no note
    until the drill is graded. The note is created in `Active` by a staged run whose stem is the
    code's type, the subject and a counter, never a date.
R8. The gate is the task's in `ai-safety.json`: ai-content-safety's own classes, persona-core's, and
    law-professors' `rule-cites-corpus`, `citations-resolve`, `quotes-grounded` and
    `authority-grounded` on the output; then the vault-duties classes on the staged run. A red class
    or a cap reached discards the run: no note and no `drill_mints` row, and SPEC-043 R12's verdict
    and one alert.
R9. `drill_mints` (`migrations/011101_vault_drill_mints.sql`, `STRICT`, `created_at`) holds one row
    per minted drill: the drill id (primary key), the code, the subject, the study day, the held
    `rule` section. SPEC-110's list gives a note with no `created` property the age of its mint row.
R10. A minted drill raises the occasion `drill_ready` through the router (SPEC-041 R1), whose text
    names the drill's type and subject and no date. The policy gains the kind as SPEC-041 R10 added
    `reading_ready`: class `nudge`, tiers `["T2"]`, budget `null`, dedupe `per-study-day`, setting
    `drill_ready_enabled`, and a `deviations` entry naming ADR-113. The setting withholds at `"0"`
    (SPEC-041 R4, rule 1); with the cadence unset nothing is minted, so nothing is raised.

The grade

R11. The task `drill-grade` (format `persona`, kind `law`) grades each answered drill that is still
    `active`, at most `DECKSTREAK_DRILL_GRADES_PER_RUN` per job run (example 3), oldest first. Its
    inputs are the drill's note and the owner's answer (source `vault`, fenced as SPEC-043 R9 says)
    and the held `rule` section (source `duty`).
R12. Its output is per-criterion scores, each an integer from 0 to 10 with the corpus rule it
    applies, and one next step. The grade is accepted only when the law gate (R8's classes, and
    law-professors' `grading-cites`) passes AND every score is an integer from 0 to 10 AND at least
    one criterion is scored. Any other output is withheld: the note is untouched and stays awaiting
    grading.
R13. An accepted grade's XP is the engine's: `10 + round_half_up(15 × mean / 10)` over the
    integer scores, so 0 gives 10 and 10 gives 25 (ADR-111). A staged run updates the note (status
    `graded`, `xp`, the held `rule` section and the grade appended) and moves it to `Graded`;
    SPEC-110's post-back pays it once on its next poll.
R14. With the route absent the job archives, grades nothing and mints nothing, and each skipped task
    is recorded `ai_route_absent` (SPEC-043 R16), with no alert.

Data rights and the rules

R15. `drill_mints` is user data: the own-tables row in the context map, coordination's data-rights
    registry and its symmetry seed, `privacy.json` and `PRIVACY.md` (the `law-drills` category), and
    the vault's data-rights port. The notes stay the owner's files (ADR-118). For the v9 import
    (#61) it maps from nothing: the predecessor kept no mint record, and an imported note keeps its
    `created` property.
R16. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no dishonest
    copy (a drill that did not pass its gate is not shown) and no unbounded notification volume
    (one `drill_ready` per study day at most).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with the route absent the drill coach's tasks decline before any launch | `the_drill_coach_declines_before_its_runner_with_the_route_absent` |
| A2 | with the route absent the job still archives and writes no drill | `with_no_ai_route_the_job_still_archives_and_writes_no_drill` |
| A3 | an unset cadence mints nothing, and a day whose entry is `-` mints nothing | `an_unset_cadence_mints_nothing` |
| A4 | one unanswered drill in `Active` stops the mint | `one_unanswered_drill_stops_the_mint` |
| A5 | a drill aged one day under the limit stays, and one at the limit is moved, byte for byte | `an_unanswered_drill_is_archived_at_the_limit` |
| A6 | an answered drill is never archived, and nothing is deleted | `an_answered_drill_is_never_archived` |
| A7 | the subject is the roster's first without a mint, else the oldest mint | `the_subject_rotates_through_the_roster` |
| A8 | a minted note holds the `drill` section and the marker, no date, and no `rule` section; the row holds it | `a_minted_note_hides_its_rule_section` |
| A9 | a red gate from the scripted runner leaves no note and no row, and raises one alert | `a_red_mint_leaves_the_vault_untouched` |
| A10 | a grade with a score of 11, a decimal, or no criterion is withheld and the note is untouched | `a_grade_outside_its_range_is_withheld` |
| A11 | the XP mapping gives 10 for 0, 25 for 10, and rounds a half up | `the_grade_maps_its_mean_to_xp` |
| A12 | an accepted grade updates and moves the note, and the post-back pays it once | `an_accepted_grade_is_moved_and_paid_once` |
| A13 | the job grades at most the configured number, oldest first | `the_job_grades_at_most_its_cap` |
| A14 | a minted drill raises `drill_ready` through the router, and its setting at `"0"` withholds it | `a_minted_drill_raises_drill_ready` |
| A15 | a mint's persona reads only its own subject's memory | `the_mint_reads_only_its_subjects_memory` |
| A16 | `drill_mints` is exported and erased | `the_mint_table_is_exported_and_erased` |
| A17 | the job table holds `drill_coach` off every reserved minute | `the_job_table_holds_the_drill_coach` |
| A18 | an imported note with `created` keeps its age, and a minted one takes its mint row's | `a_minted_drill_takes_its_age_from_its_row` |
| A19 | the mint's task receives exactly the subject's law professor persona, `drill-coach.duty.md`, the subject's `drill-grades` and `leeches` memory and the drill code, in SPEC-043 R9's order | `the_mint_task_receives_exactly_its_inputs` |
| A20 | the grade's task receives exactly the drill's note and the owner's answer, fenced as `vault`, and the held `rule` section as `duty` | `the_grade_task_receives_exactly_its_inputs` |

```acceptance
A1: cargo test -p deck-streak-agent --test drill_coach -- --exact the_drill_coach_declines_before_its_runner_with_the_route_absent
A2: cargo test -p deck-streak-coordination --test drill_coach -- --exact with_no_ai_route_the_job_still_archives_and_writes_no_drill
A3: cargo test -p deck-streak-agent --test drill_coach -- --exact an_unset_cadence_mints_nothing
A4: cargo test -p deck-streak-agent --test drill_coach -- --exact one_unanswered_drill_stops_the_mint
A5: cargo test -p deck-streak-coordination --test drill_coach -- --exact an_unanswered_drill_is_archived_at_the_limit
A6: cargo test -p deck-streak-coordination --test drill_coach -- --exact an_answered_drill_is_never_archived
A7: cargo test -p deck-streak-agent --test drill_coach -- --exact the_subject_rotates_through_the_roster
A8: cargo test -p deck-streak-coordination --test drill_coach -- --exact a_minted_note_hides_its_rule_section
A9: cargo test -p deck-streak-coordination --test drill_coach -- --exact a_red_mint_leaves_the_vault_untouched
A10: cargo test -p deck-streak-agent --test drill_coach -- --exact a_grade_outside_its_range_is_withheld
A11: cargo test -p deck-streak-agent --test drill_coach -- --exact the_grade_maps_its_mean_to_xp
A12: cargo test -p deck-streak-coordination --test drill_coach -- --exact an_accepted_grade_is_moved_and_paid_once
A13: cargo test -p deck-streak-coordination --test drill_coach -- --exact the_job_grades_at_most_its_cap
A14: cargo test -p deck-streak-coordination --test drill_coach -- --exact a_minted_drill_raises_drill_ready
A15: cargo test -p deck-streak-agent --test drill_coach -- --exact the_mint_reads_only_its_subjects_memory
A16: cargo test -p deck-streak-vault --test drill_store -- --exact the_mint_table_is_exported_and_erased
A17: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_the_drill_coach
A18: cargo test -p deck-streak-vault --test drill_goldens -- --exact a_minted_drill_takes_its_age_from_its_row
A19: cargo test -p deck-streak-agent --test drill_coach -- --exact the_mint_task_receives_exactly_its_inputs
A20: cargo test -p deck-streak-agent --test drill_coach -- --exact the_grade_task_receives_exactly_its_inputs
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. ai-content-safety and study-duties are enforced by
their own features (#29, #32) before this delivery; vault-duties stays pending until SPEC-116 (#49)
lifts it, and B4 is judged from then. This delivery changes no pack's state, so the private wiring
does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `ai-safety.json`: the tasks `drill-mint` and `drill-grade` are declared with their system files, prompts, fenced inputs and sources, no tool, their golden outputs and a gate naming every class R8 and R12 name | the ai-content-safety pack |
| B2 | over the golden outputs under `agent/golden/drill-coach/`: every rule cites the corpus and every grade cites what it applies | the law-professors pack (`rule-cites-corpus`, `grading-cites`) |
| B3 | over `agent/duties/drill-coach.duty.md` and the golden mints: the `drill` section tells nothing and comes first | the study-duties pack |
| B4 | over the golden staged runs under `agent/golden/drill-coach/runs/`: every blocking vault-duties class, `no-dates` and `never-deletes` included | the vault-duties pack |
| B5 | over `notifications-policy.json`: `drill_ready` is a nudge with a setting and a dedupe | the notifications-policy pack |
| B6 | over `privacy.json`, `PRIVACY.md` and `crates/vault/src/data_rights.rs`: the `law-drills` category names `drill_mints` | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/agent/src/drill_coach.rs` | `deck-streak-agent` | added: the plan, the two tasks, the grade's acceptance and its XP mapping |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the module |
| `crates/agent/tests/drill_coach.rs` | `deck-streak-agent` | added: A1, A3, A4, A7, A10, A11, A15, A19, A20 |
| `agent/duties/drill-coach.duty.md` | agent (public) | added: study-duties' template, copied |
| `agent/prompts/drill-mint.prompt.md`, `agent/prompts/drill-grade.prompt.md` | agent (public) | added: the task prompts with their fenced slots |
| `agent/golden/drill-coach/` | agent (public) | added: golden mints, grades and staged runs, synthetic |
| `agent/redteam/` | agent (public) | changed: a case per untrusted source the tasks read |
| `ai-safety.json` | repo | changed: the two tasks |
| `crates/vault/src/drills.rs` | `deck-streak-vault` | changed: a note's age from its mint row |
| `crates/vault/src/drill_store.rs` | `deck-streak-vault` | changed: `drill_mints` |
| `crates/vault/src/data_rights.rs` | `deck-streak-vault` | changed: `drill_mints` |
| `crates/vault/data/layout.json` | `deck-streak-vault` | changed: the drill coach moves inside its own folder |
| `crates/vault/tests/drill_store.rs` | `deck-streak-vault` | changed: A16 |
| `crates/vault/tests/drill_goldens.rs` | `deck-streak-vault` | changed: A18 |
| `migrations/011101_vault_drill_mints.sql` | `deck-streak-vault` | added |
| `crates/coordination/src/drill_coach.rs` | `deck-streak-coordination` | added: the job's use case |
| `crates/coordination/src/jobs.rs`, `crates/coordination/src/runner.rs`, `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: `drill_coach` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `drill_mints` |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row |
| `crates/coordination/tests/drill_coach.rs` | `deck-streak-coordination` | added: A2, A5, A6, A8, A9, A12 to A14 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A17 |
| `deploy/systemd/deck-streak-job@drill_coach.timer` | deploy | added |
| `notifications-policy.json` | repo | changed: the kind `drill_ready` |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the job's ports |
| `.env.example` | repo | changed: the three settings, by name, with neutral examples |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row for `drill_mints` |
| `privacy.json`, `PRIVACY.md` | repo | changed: `drill_mints` in `law-drills` |
| `scripts/mutation-rows.d/S11100-S11199.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-111-the-drill-coach-mints-law-drills-by-the-owners-cadence-and-grades-them-against-the-corpus.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/law-drill-answer-grade-and-pay.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/schematics/w6-duty-run-and-its-degradation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-111.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It pays nothing itself; SPEC-110's post-back pays every graded drill once (#136).
- It shows no drill on a screen; the drill workspace does (#55).
- It builds no settings screen for the cadence, the archive age or the grade cap (#57).
- It lets no model choose a drill's XP, type or day (#46).
- It mints no LSAT or bar practice set; practice questions do (#52).
- It imports no predecessor note or record (#61).

## 6. Risks

- **A mint that answers itself**, putting the rule in the note before the attempt. Prevented by R7's
  held section, and detected by A8.
- **A grade outside its range paying.** Prevented by R12's acceptance and R13's engine mapping, then
  SPEC-110's clamp; detected by A10 and A11.
- **A run that deletes a note.** The executor has no delete verb (SPEC-042 R4), and the archive is a
  move; detected by A6.
- **A backlog that grows while the owner is away.** Prevented by the zero-backlog gate and the
  archive; detected by A4 and A5.

## 7. Parity goldens

None: the plan, the acceptance and the mapping are DeckStreak's own (ADR-111), because the
predecessor had no product function for them. SPEC-110's goldens prove the notes' contract this
duty writes to, and the golden outputs under `agent/golden/drill-coach/` are DeckStreak's own
examples for the box run, never the predecessor's.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `drill_mints` | `deck-streak-vault` | `migrations/011101_vault_drill_mints.sql` | nothing: the predecessor kept no mint record | exported and erased; the note stays in the owner's vault |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11101-ROUTE-FIRST` | `crates/agent/src/drill_coach.rs` | the route check before any launch | `drill_coach::the_drill_coach_declines_before_its_runner_with_the_route_absent` |
| `S11102-CADENCE-DASH` | `crates/agent/src/drill_coach.rs` | a `-` entry mints nothing | `drill_coach::an_unset_cadence_mints_nothing` |
| `S11103-ZERO-BACKLOG` | `crates/agent/src/drill_coach.rs` | one unanswered drill stops the mint | `drill_coach::one_unanswered_drill_stops_the_mint` |
| `S11104-ARCHIVE-AGE` | `crates/coordination/src/drill_coach.rs` | the age limit's `>=`; the test names the limit and one under | `drill_coach::an_unanswered_drill_is_archived_at_the_limit` |
| `S11105-ARCHIVE-UNANSWERED-ONLY` | `crates/coordination/src/drill_coach.rs` | an answered drill is kept | `drill_coach::an_answered_drill_is_never_archived` |
| `S11106-RULE-HELD` | `crates/agent/src/drill_coach.rs` | the `rule` section never reaches the minted note | `drill_coach::a_minted_note_hides_its_rule_section` |
| `S11107-RED-DISCARDS` | `crates/coordination/src/drill_coach.rs` | a withheld verdict writes no row | `drill_coach::a_red_mint_leaves_the_vault_untouched` |
| `S11108-SCORE-MAX` | `crates/agent/src/drill_coach.rs` | a score over 10 is withheld; the test names 10 and 11 | `drill_coach::a_grade_outside_its_range_is_withheld` |
| `S11109-SCORE-INTEGER` | `crates/agent/src/drill_coach.rs` | a decimal score is withheld | `drill_coach::a_grade_outside_its_range_is_withheld` |
| `S11110-XP-MAPPING` | `crates/agent/src/drill_coach.rs` | 10 plus 15 times the mean over 10; the test names a mean whose product ends in .5 | `drill_coach::the_grade_maps_its_mean_to_xp` |
| `S11111-GRADE-CAP` | `crates/coordination/src/drill_coach.rs` | the per-run cap | `drill_coach::the_job_grades_at_most_its_cap` |
| `S11112-SUBJECT-ROTATION` | `crates/agent/src/drill_coach.rs` | the oldest mint's subject is next | `drill_coach::the_subject_rotates_through_the_roster` |
| `S11113-ONE-MINT-A-DAY` | `crates/agent/src/drill_coach.rs` | a second mint on one study day is refused | `drill_coach::one_unanswered_drill_stops_the_mint` |
