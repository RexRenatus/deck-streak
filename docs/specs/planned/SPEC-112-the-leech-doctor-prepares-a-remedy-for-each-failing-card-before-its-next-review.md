# SPEC-112: the leech doctor prepares a remedy for each failing card before its next review

- **Wave:** W6. **Issue:** #47 (epic #7). **Context(s):** `deck-streak-agent` (the leech-doctor
  task, its candidates, its acceptance and the table `leech_remedies`); `deck-streak-ingest` (a read
  of each leech card's due day and its note's fields); `deck-streak-curriculum` (a per-subject read
  of the leech snapshot); `deck-streak-coordination` (the `leech_doctor` job and its use case);
  `deck-streak-api` and `deck-streak-bot` (the remedy's route and command).
- **Decided by:** ADR-043 (the shell runner, its gate and caps), ADR-054 (no-AI mode is the
  default), ADR-093 (a leech's remediation), ADR-112 (a remedy is prepared after the sync, capped,
  against an engine-chosen confusable card) and ADR-113 (a requested duty answers as its
  command's reply).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-024, SPEC-026, SPEC-027, SPEC-029, SPEC-043, SPEC-044, SPEC-045,
  SPEC-071 and SPEC-093. **Mutation band:** `S11200-S11299`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-112.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** SPEC-093 (planned, #133) snapshots every card at or over the leech
  threshold after each sync and gives the owner a fixed protocol per card
  (`leeches.py:remediation_protocol`). It explains nothing about the card itself. #47 asks for three
  parts per failing card, an explanation from a new angle, a mnemonic and a contrast against the
  card it is confused with, ready before the card's next review.
- **The pack's contract.** The study-duties leech-doctor template names the sections
  `explanation`, `mnemonic` and `contrast` (a law professor adds `rule` and `trap`), a frontmatter
  pair `x-leech` naming the card, its confusable card (or null) and the card's answer, a mnemonic of
  40 words at most, and "fail loud": write nothing if a part cannot be written.
- **The data it needs.** A leech row holds the card id, its language or law subject, its strand, its
  lapses, its difficulty and whether it is suspended (`leeches.py:cards_to_leech_rows`), and no due
  day and no text. The due day and the note's fields live in the collection, which only ingest reads
  (ADR-009).
- **The memory port is unwired.** SPEC-044 R7 says the `leeches` memory source is wired by the
  delivery that builds its data (#133); SPEC-093's manifest wires no memory port. This SPEC wires it.
- **No-AI is the default** (ADR-054). With the route absent there is no remedy, and the owner keeps
  SPEC-093's protocol, which is the deterministic rule the predecessor had.

## 2. Requirements

The run

R1. The job `leech_doctor` in `coordination::jobs::TABLE` runs daily at a minute after the `sync`
    slot that SPEC-027 R2's census admits, not `catch_up`. It reads the study day's sync outcome
    (SPEC-027 R1): when the day has no successful sync it prepares nothing.
R2. The queue is the snapshot's unsuspended rows (SPEC-093 R2) with no remedy, or whose remedy was
    prepared at fewer lapses than the row now holds, ordered by the card's due study day ascending,
    then lapses descending, then card id. A card with no due day (a new or a suspended card) is
    last.
R3. A run prepares at most `DECKSTREAK_LEECH_REMEDIES_PER_RUN` remedies (default 3; `.env.example`
    names it with that value). A card due on or before the next study day is therefore prepared by
    the run that follows the sync that first snapshots it, when it is inside the cap.
R4. In the same write as a run's remedies, every `leech_remedies` row whose card is no longer in the
    snapshot is deleted.

The inputs, as a set

R5. For each queued card the use case passes exactly: the card's persona (the roster's template for
    its subject, SPEC-044 R2), `agent/duties/leech-doctor.duty.md`, the subject's memory (SPEC-044
    R5), the card's note fields in field order (source `cards`), and the candidates of R6 (source
    `cards`). Nothing else is read.
R6. **The candidates.** The confusable card is chosen only from the other snapshot rows of the same
    subject, at most 8, ordered by lapses descending, then card id, each with its note's first field.
    The model names one candidate's id or none.
R7. Ingest's reader gives, for a set of card ids, each card's due study day (under the 04:00
    rollover, SPEC-020 R1) and its note's fields in field order with markup removed, from the private
    copy, in one read-only pass; an id it cannot find is absent from the answer, never an error.

The output and its acceptance

R8. The task `leech-doctor` (format `persona`, kind `law` or `language` by the subject) holds no tool
    and runs under SPEC-043 R6's default caps. Its gate is the ai-content-safety pack's required
    classes for the kind (SPEC-043 R10), the study-duties `leech-contrast` and `no-placeholder`
    classes, and for a language subject the language-mentors classes its persona's kind names.
R9. A remedy is accepted only when the gate passes AND the three sections are present (and `rule`
    and `trap` for a law subject) AND the mnemonic has at most 40 words AND the named confusable is
    a candidate of R6 or none. Otherwise it is withheld (SPEC-043 R12): nothing is stored, and the
    card is queued again on the next run.
R10. The engine writes the frontmatter (SPEC-044 R6) and the `x-leech` pair from the card's first
    field, the chosen candidate's first field or null, and the card's second field as its answer.
R11. `leech_remedies` (`migrations/011201_agent_leech_remedies.sql`, `STRICT`, `created_at`) holds one
    row per card: the card id (primary key), the subject, the lapses it was prepared at, the
    confusable card id or null, the accepted output, and the study day it was prepared.

The surfaces

R12. `/remedy` with a card id answers the card's accepted remedy as its command's reply (ADR-113),
    through the one router's command-reply census. With the route absent it answers "Leech remedies
    are not enabled" and the card's protocol from SPEC-093; a card with no remedy yet answers that it
    is queued, with the protocol.
R13. `GET /api/leeches/{card_id}/remedy` serves the same answer to the owner's session only
    (SPEC-024); any other caller is answered 401 or 403 with no data.
R14. The leech doctor raises no occasion: a remedy is read when the owner asks for it.

The memory port

R15. The `leeches` memory source (SPEC-044 R5) is wired in `crates/daemon/src/wiring.rs` to a
    per-subject read of the snapshot: at most 10 rows of the persona's own subject, by lapses
    descending, each as card id, strand and lapses, with no card text.

Data rights and the rules

R16. `leech_remedies` is user data: the own-tables row in the context map, coordination's
    data-rights registry and its symmetry seed, `privacy.json` (the category `leech-remedies`, and
    the processing by route: only with an AI route configured is a leech card's text sent to the
    model provider, as SPEC-046 R13 declares for readings), `PRIVACY.md`, and the agent's data-rights
    port. For the v9 import (#61) it maps from nothing: the predecessor had no leech doctor.
R17. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no dishonest
    copy (a withheld remedy is never shown) and no unbounded notification volume (R14).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with the route absent the task declines before any launch and records `ai_route_absent` | `the_leech_doctor_declines_before_its_runner_with_the_route_absent` |
| A2 | with the route absent `/remedy` answers not enabled with the card's protocol | `with_no_ai_route_remedy_answers_the_protocol` |
| A3 | a day with no successful sync prepares nothing | `a_day_without_a_sync_prepares_no_remedy` |
| A4 | the queue orders by due day, then lapses, then card id, and a card with no due day is last | `the_queue_orders_by_due_day` |
| A5 | a leech due on the next study day is prepared in the run after the sync that snapshots it | `a_leech_due_tomorrow_is_ready_before_its_review` |
| A6 | a run prepares at most its cap | `a_run_prepares_at_most_its_cap` |
| A7 | a remedy prepared at fewer lapses than the row holds is queued again | `a_card_that_lapsed_again_is_queued_again` |
| A8 | a remedy whose card left the snapshot is deleted in the run's write | `a_remedy_leaves_with_its_card` |
| A9 | the task receives exactly the card's persona, the duty, the subject's memory, the card's note fields in field order and the candidates | `the_task_receives_exactly_its_inputs` |
| A10 | the candidates are the same subject's other rows, at most 8, by lapses | `the_candidates_are_the_subjects_other_leeches` |
| A11 | a confusable outside the candidates is withheld and nothing is stored | `a_confusable_outside_the_candidates_is_withheld` |
| A12 | a mnemonic of 40 words is accepted and one of 41 is withheld | `a_mnemonic_over_forty_words_is_withheld` |
| A13 | a missing section, or a law remedy without `rule` or `trap`, is withheld | `a_remedy_missing_a_section_is_withheld` |
| A14 | the engine writes the `x-leech` pair from the fields | `the_engine_writes_the_leech_pair` |
| A15 | ingest's reader gives each card's due day and its fields, and skips an unknown id | `the_reader_gives_leech_cards_due_and_fields` |
| A16 | `/remedy` answers an accepted remedy through the command-reply census | `remedy_answers_the_stored_remedy` |
| A17 | the route answers the owner and refuses any other caller with no data | `the_remedy_route_is_owner_only` |
| A18 | the memory port reads only the persona's subject, at most 10 rows, with no text | `the_leech_memory_reads_only_its_subject` |
| A19 | `leech_remedies` is exported and erased | `the_remedies_are_exported_and_erased` |
| A20 | the job table holds `leech_doctor` off every reserved minute and after `sync` | `the_job_table_holds_the_leech_doctor` |

```acceptance
A1: cargo test -p deck-streak-agent --test leech_doctor -- --exact the_leech_doctor_declines_before_its_runner_with_the_route_absent
A2: cargo test -p deck-streak-bot --test remedy_command -- --exact with_no_ai_route_remedy_answers_the_protocol
A3: cargo test -p deck-streak-coordination --test leech_doctor -- --exact a_day_without_a_sync_prepares_no_remedy
A4: cargo test -p deck-streak-coordination --test leech_doctor -- --exact the_queue_orders_by_due_day
A5: cargo test -p deck-streak-coordination --test leech_doctor -- --exact a_leech_due_tomorrow_is_ready_before_its_review
A6: cargo test -p deck-streak-coordination --test leech_doctor -- --exact a_run_prepares_at_most_its_cap
A7: cargo test -p deck-streak-coordination --test leech_doctor -- --exact a_card_that_lapsed_again_is_queued_again
A8: cargo test -p deck-streak-coordination --test leech_doctor -- --exact a_remedy_leaves_with_its_card
A9: cargo test -p deck-streak-coordination --test leech_doctor -- --exact the_task_receives_exactly_its_inputs
A10: cargo test -p deck-streak-agent --test leech_doctor -- --exact the_candidates_are_the_subjects_other_leeches
A11: cargo test -p deck-streak-agent --test leech_doctor -- --exact a_confusable_outside_the_candidates_is_withheld
A12: cargo test -p deck-streak-agent --test leech_doctor -- --exact a_mnemonic_over_forty_words_is_withheld
A13: cargo test -p deck-streak-agent --test leech_doctor -- --exact a_remedy_missing_a_section_is_withheld
A14: cargo test -p deck-streak-agent --test leech_doctor -- --exact the_engine_writes_the_leech_pair
A15: cargo test -p deck-streak-ingest --test leech_cards -- --exact the_reader_gives_leech_cards_due_and_fields
A16: cargo test -p deck-streak-bot --test remedy_command -- --exact remedy_answers_the_stored_remedy
A17: cargo test -p deck-streak-api --test remedy_route -- --exact the_remedy_route_is_owner_only
A18: cargo test -p deck-streak-daemon --test leech_memory -- --exact the_leech_memory_reads_only_its_subject
A19: cargo test -p deck-streak-agent --test rights -- --exact the_remedies_are_exported_and_erased
A20: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_the_leech_doctor
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. ai-content-safety and study-duties are enforced by
their own features (#29, #32) before this delivery. This delivery changes no pack's state, so the
private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over the golden outputs under `agent/golden/leech-doctor/` (one law, one language): the contrast names both cards, and the pair is declared | the study-duties pack (`leech-contrast`, and its advisory pair, angle and mnemonic classes) |
| B2 | over `ai-safety.json`: the task `leech-doctor` declares its system files, prompt, fenced `cards` and `memory` inputs, no tool, its golden outputs and a complete gate | the ai-content-safety pack |
| B3 | over the same golden outputs: every persona's output reads only its own subject's memory | the persona-core pack (`memory-scope`) |
| B4 | over `privacy.json`, `PRIVACY.md` and the agent's data-rights port: `leech-remedies` names `leech_remedies` and the processing by route | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/agent/src/leech_doctor.rs` | `deck-streak-agent` | added: the task, the candidates, the acceptance and the pair |
| `crates/agent/src/leech_remedies.rs` | `deck-streak-agent` | added: `leech_remedies` |
| `crates/agent/src/rights.rs` | `deck-streak-agent` | changed: the port exports and erases `leech_remedies` |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the modules |
| `crates/agent/tests/leech_doctor.rs` | `deck-streak-agent` | added: A1, A10 to A14 |
| `crates/agent/tests/rights.rs` | `deck-streak-agent` | changed: A19 |
| `migrations/011201_agent_leech_remedies.sql` | `deck-streak-agent` | added |
| `agent/duties/leech-doctor.duty.md` | agent (public) | added: study-duties' template, copied |
| `agent/prompts/leech-doctor.prompt.md` | agent (public) | added: the prompt with its fenced slots |
| `agent/golden/leech-doctor/` | agent (public) | added: synthetic golden remedies, one law and one language |
| `agent/redteam/` | agent (public) | changed: a case in a card's field and in a candidate's field |
| `ai-safety.json` | repo | changed: the task |
| `crates/ingest/src/reader.rs` | `deck-streak-ingest` | changed: the leech cards' due day and fields |
| `crates/ingest/tests/leech_cards.rs` | `deck-streak-ingest` | added: A15 |
| `crates/curriculum/src/leeches.rs` | `deck-streak-curriculum` | changed: the per-subject read |
| `crates/coordination/src/leech_doctor.rs` | `deck-streak-coordination` | added: the job's use case |
| `crates/coordination/src/jobs.rs`, `crates/coordination/src/runner.rs`, `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: `leech_doctor` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `leech_remedies` under the agent's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row |
| `crates/coordination/tests/leech_doctor.rs` | `deck-streak-coordination` | added: A3 to A9 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A20 |
| `deploy/systemd/deck-streak-job@leech_doctor.timer` | deploy | added |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the job's ports and the `leeches` memory port |
| `crates/daemon/tests/leech_memory.rs` | `deck-streak-daemon` | added: A18 |
| `crates/bot/src/remedy_command.rs` | `deck-streak-bot` | added: `/remedy` |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command joins the table |
| `crates/bot/tests/remedy_command.rs` | `deck-streak-bot` | added: A2, A16 |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names `/remedy`'s reply |
| `crates/api/src/remedy_route.rs` | `deck-streak-api` | added |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the route behind the owner's session |
| `crates/api/tests/remedy_route.rs` | `deck-streak-api` | added: A17 |
| `.env.example` | repo | changed: the cap, by name, with its default |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row for `leech_remedies` |
| `privacy.json`, `PRIVACY.md` | repo | changed: `leech-remedies` and the processing by route |
| `scripts/mutation-rows.d/S11200-S11299.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-112-the-leech-doctor-prepares-a-remedy-for-each-failing-card-before-its-next-review.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w6-duty-run-and-its-degradation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-112.md` | docs | added |
| `changelog.d/` fragment | repo | added |

The agent's data-rights port is the file SPEC-043's delivery adds. SPEC-043's manifest names it
`rights.rs`, while `privacy.json`'s export and erase globs read `crates/*/src/data_rights.rs`
(SPEC-046's own correction); this SPEC changes whichever name that delivery lands.

## 5. What this does NOT do

- It changes no leech row, board, protocol or remediation, and pays no XP (#133).
- It shows no remedy in the Mini App's leech list, which shows no card content (#133).
- It builds no settings screen for the cap (#57).
- It pings nothing when a remedy is ready (#47).
- It imports nothing from the predecessor (#61).

## 6. Risks

- **A confusable card the model invents.** Prevented by R6 and R9; detected by A11.
- **Card text leaving the host with the route off.** The route check comes first (SPEC-043 R16);
  detected by A1, and the processing is declared by route (R16).
- **A queue that never reaches the soonest cards.** Prevented by R2's order; detected by A4 and A5.
- **A remedy kept for a card the owner fixed.** Removed with its card (R4); detected by A8.

## 7. Parity goldens

None: the leech doctor is DeckStreak-native (#47). The golden outputs under
`agent/golden/leech-doctor/` are DeckStreak's own synthetic examples for the box run.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `leech_remedies` | `deck-streak-agent` | `migrations/011201_agent_leech_remedies.sql` | nothing: the predecessor had no leech doctor | exported and erased |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11201-ROUTE-FIRST` | `crates/agent/src/leech_doctor.rs` | the route check before any launch | `leech_doctor::the_leech_doctor_declines_before_its_runner_with_the_route_absent` |
| `S11202-SYNC-OUTCOME` | `crates/coordination/src/leech_doctor.rs` | no remedy without the day's sync | `leech_doctor::a_day_without_a_sync_prepares_no_remedy` |
| `S11203-DUE-ORDER` | `crates/coordination/src/leech_doctor.rs` | the due day sorts first | `leech_doctor::the_queue_orders_by_due_day` |
| `S11204-RUN-CAP` | `crates/coordination/src/leech_doctor.rs` | the cap's bound; the test names the cap and one over | `leech_doctor::a_run_prepares_at_most_its_cap` |
| `S11205-LAPSES-REQUEUE` | `crates/coordination/src/leech_doctor.rs` | a fresh lapse queues the card again | `leech_doctor::a_card_that_lapsed_again_is_queued_again` |
| `S11206-STALE-DELETE` | `crates/coordination/src/leech_doctor.rs` | a gone card's remedy is deleted | `leech_doctor::a_remedy_leaves_with_its_card` |
| `S11207-CANDIDATE-SUBJECT` | `crates/agent/src/leech_doctor.rs` | candidates share the subject | `leech_doctor::the_candidates_are_the_subjects_other_leeches` |
| `S11208-CANDIDATE-CAP` | `crates/agent/src/leech_doctor.rs` | at most 8 candidates | `leech_doctor::the_candidates_are_the_subjects_other_leeches` |
| `S11209-CONFUSABLE-MEMBER` | `crates/agent/src/leech_doctor.rs` | a named confusable is a candidate | `leech_doctor::a_confusable_outside_the_candidates_is_withheld` |
| `S11210-MNEMONIC-40` | `crates/agent/src/leech_doctor.rs` | 40 words kept, 41 withheld | `leech_doctor::a_mnemonic_over_forty_words_is_withheld` |
| `S11211-LAW-SECTIONS` | `crates/agent/src/leech_doctor.rs` | a law remedy needs `rule` and `trap` | `leech_doctor::a_remedy_missing_a_section_is_withheld` |
| `S11212-MEMORY-SUBJECT` | `crates/daemon/src/wiring.rs` | the memory read is the persona's subject | `leech_memory::the_leech_memory_reads_only_its_subject` |
| `S11213-MEMORY-CAP` | `crates/daemon/src/wiring.rs` | at most 10 memory rows | `leech_memory::the_leech_memory_reads_only_its_subject` |
