# SPEC-114: a practice set is generated on request, its keys are held until it is attempted, and its completion pays once

- **Wave:** W6. **Issue:** #52 (epic #7). **Context(s):** `deck-streak-agent` (the practice-set
  task, its parse and acceptance, the table `practice_sets`); `deck-streak-coordination` (generate,
  start and submit, and the grant); `deck-streak-bot`, `deck-streak-api` and the Mini App (the
  command, the routes and the practice screen).
- **Decided by:** ADR-043 (the shell runner, its gate and caps), ADR-054 (no-AI mode is the
  default), ADR-113 (a requested duty answers as its command's reply) and ADR-114 (a practice set
  pays its completion once, never its score, and its keys stay on the server until it is
  submitted).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-024, SPEC-025, SPEC-026, SPEC-028, SPEC-040, SPEC-041,
  SPEC-043, SPEC-044 and SPEC-113. **Mutation band:** `S11400-S11499`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-114.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** Practice sets are DeckStreak-native (#52): the predecessor had no
  practice-set duty, so there is no function to port.
- **The packs' contracts.** The study-duties practice-questions template puts every item in the LSAT
  coach's format, whatever the subject: a heading whose invisible marker carries the item's id and
  key, one line per choice from `(A)`, and an explanation block per item that credits exactly one
  choice, the key, and explains every other. The lsat-coach pack adds an `x-lsat` frontmatter key
  (`set_kind`, `section`, `items`, and for a timed set `minutes` and `time_multiplier`), a `timing`
  section, a full section of ceil(35 × `time_multiplier`) minutes, five choices per LSAT item, and
  each item's provenance (`original` with its text, or `official` with a citation and no text).
- **The key is in the text.** Because the marker carries the key, a set sent to the client whole
  gives its answers away; learning-science's `answers-hidden` class forbids that.
- **The persona kinds.** An LSAT set is the LSAT coach's (kind `test-prep`); a bar set is the
  subject's law professor's (kind `law`).
- **This delivery lifts the lsat-coach pack's deferral**, which names #52.

## 2. Requirements

Generation

R1. `/practice lsat <section>` or `/practice bar <subject>` requests a set; `POST /api/practice`
    takes the same input. Both call `coordination::practice::generate`, which runs as a requested
    duty (ADR-113): a placeholder, then one edit with the set's Open button into the Mini App, a
    refusal line, or the not-enabled line. The section is one of the lsat-coach pack's sections; the
    subject is a roster law topic (SPEC-044 R2); anything else is refused by name with no run.
R2. The task `practice-set` (format `persona`, kind `test-prep` for LSAT and `law` for bar) passes
    exactly: the persona, `agent/duties/practice-questions.duty.md`, the set kind, the section or
    subject, and `DECKSTREAK_PRACTICE_TIME_MULTIPLIER` (source `config`; default 1.0) and the
    subject's memory for a bar set (SPEC-044 R5). It holds no tool and runs under SPEC-043 R6's
    default caps.
R3. Its gate is the ai-content-safety pack's required classes for the kind (SPEC-043 R10);
    study-duties' `question-key`, `choices-explained` and `no-placeholder`; learning-science's
    `answers-hidden`; for LSAT, lsat-coach's `timed-set`, `every-choice-explained`,
    `item-provenance`, `sources-cited` and `format-facts`.
R4. A set is accepted only when the gate passes AND every item's marker parses with a unique id
    and a key among its choices AND an LSAT item has exactly the five choices (A) to (E) AND every
    item's provenance is `original` AND the set holds from 1 to 26 items. The engine then splits it
    into the questions (no marker key, no explanations) and the key sheet (keys and explanations).
    Anything else is withheld (SPEC-043 R12) and nothing is stored.
R5. `practice_sets` (`migrations/011401_agent_practice_sets.sql`, `STRICT`, `created_at`) holds one
    row per accepted set: its id, set kind, section or subject, item count, minutes (or none for an
    untimed set), the questions, the key sheet, its state (`ready`, `started`, `completed`), the
    start and submission instants, the owner's answers and the count correct. An insert deletes the
    oldest rows beyond the newest 20 in the same write.

The attempt

R6. `GET /api/practice/{id}` serves the questions and the timing, and never the key sheet, while the
    set is `ready` or `started`. `POST /api/practice/{id}/start` records the start instant once.
R7. `POST /api/practice/{id}/answers` takes one answer or a skip per item, records them and the
    submission instant once, counts the correct answers against the key sheet, sets `completed`,
    and answers with the key sheet and the count. A second submission is refused with no write. The
    minutes used are reported beside the limit; a late submission is completed like any other
    (CHARTER 10: no imposed penalty).
R8. A completed set is granted once through the grant port (SPEC-040): the amount
    `xp.bonuses.practice_set` from `economy.json`, the submission's study day, source
    `practice:<set id>`, track `law`, scope `once`, in the submission's write. The count correct
    never changes the amount. `economy.json` names `practice` in `xp.day_base_excludes`.
R9. The Mini App's practice screen shows the questions, the pace clock (minutes used against the
    limit, counting up, never a date), one answer or skip per item, and after submission each
    item's key and explanations beside the owner's answer.
R10. With the route absent, `/practice` and `POST /api/practice` answer "Practice sets are not
    enabled" with no run and no alert; a stored set can still be attempted and completed, and pays.
R11. Every route and the command answer the owner only (SPEC-024, SPEC-026).

Data rights and the rules

R12. `practice_sets` is user data: the own-tables row in the context map, coordination's
    data-rights registry and its symmetry seed, `privacy.json` (the category `practice-sets`, and
    the processing by route), `PRIVACY.md`, and the agent's data-rights port. For the v9 import
    (#61) it maps from nothing: the predecessor had no practice sets.
R13. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no imposed
    penalty (R7), no forward date (R9) and no dishonest copy (a withheld set is never shown).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with the route absent `/practice` answers not enabled and launches nothing | `with_no_ai_route_practice_answers_not_enabled` |
| A2 | with the route absent a stored set is still completed and paid | `with_no_ai_route_a_stored_set_still_pays` |
| A3 | an unknown section or subject is refused with no run | `an_unknown_section_is_refused` |
| A4 | the task receives exactly its inputs, memory only for a bar set | `the_practice_task_receives_exactly_its_inputs` |
| A5 | an item with a key outside its choices, a repeated id, or four LSAT choices is withheld | `a_set_with_a_bad_item_is_withheld` |
| A6 | an `official` item is withheld | `an_official_item_is_withheld` |
| A7 | a set of 26 items is accepted and one of 27, or of none, is withheld | `a_set_outside_its_item_bounds_is_withheld` |
| A8 | the stored questions hold no key and no explanation | `the_questions_carry_no_key` |
| A9 | the set route serves no key sheet before submission | `the_set_route_hides_the_keys_until_submitted` |
| A10 | a submission counts the correct answers and returns the key sheet | `a_submission_is_scored_and_explained` |
| A11 | a second submission is refused with no write | `a_second_submission_is_refused` |
| A12 | a completed set pays its amount once, on the submission's study day, whatever the count | `a_completed_set_pays_once_whatever_its_score` |
| A13 | a late submission is completed and paid | `a_late_submission_still_completes` |
| A14 | the table keeps the newest 20 sets | `the_table_keeps_the_newest_twenty_sets` |
| A15 | a non-owner is refused by every route with no data and no write | `the_practice_routes_are_owner_only` |
| A16 | `practice_sets` is exported and erased | `the_practice_sets_are_exported_and_erased` |
| A17 | the screen hides every key until submission and then shows each explanation | `the practice screen shows no key before submission` |

```acceptance
A1: cargo test -p deck-streak-coordination --test practice -- --exact with_no_ai_route_practice_answers_not_enabled
A2: cargo test -p deck-streak-coordination --test practice -- --exact with_no_ai_route_a_stored_set_still_pays
A3: cargo test -p deck-streak-coordination --test practice -- --exact an_unknown_section_is_refused
A4: cargo test -p deck-streak-coordination --test practice -- --exact the_practice_task_receives_exactly_its_inputs
A5: cargo test -p deck-streak-agent --test practice_set -- --exact a_set_with_a_bad_item_is_withheld
A6: cargo test -p deck-streak-agent --test practice_set -- --exact an_official_item_is_withheld
A7: cargo test -p deck-streak-agent --test practice_set -- --exact a_set_outside_its_item_bounds_is_withheld
A8: cargo test -p deck-streak-agent --test practice_set -- --exact the_questions_carry_no_key
A9: cargo test -p deck-streak-api --test practice_routes -- --exact the_set_route_hides_the_keys_until_submitted
A10: cargo test -p deck-streak-coordination --test practice -- --exact a_submission_is_scored_and_explained
A11: cargo test -p deck-streak-coordination --test practice -- --exact a_second_submission_is_refused
A12: cargo test -p deck-streak-coordination --test practice -- --exact a_completed_set_pays_once_whatever_its_score
A13: cargo test -p deck-streak-coordination --test practice -- --exact a_late_submission_still_completes
A14: cargo test -p deck-streak-agent --test practice_set -- --exact the_table_keeps_the_newest_twenty_sets
A15: cargo test -p deck-streak-api --test practice_routes -- --exact the_practice_routes_are_owner_only
A16: cargo test -p deck-streak-agent --test rights -- --exact the_practice_sets_are_exported_and_erased
A17: pnpm exec vitest run web/app/src/lib/practice/PracticeSet.test.ts -t "the practice screen shows no key before submission"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. This delivery lifts the lsat-coach pack's deferral:
the private wiring changes it from pending to enforced when the delivery merges, and the builder
hands back that JSON diff. ai-content-safety, study-duties and learning-science are enforced by
their own features (#29, #32) before it.

| id | criterion | decided by |
|---|---|---|
| B1 | over the golden sets under `agent/golden/practice-set/` (one timed LSAT section, one bar set): timing, five explained choices, provenance and sources | the lsat-coach pack (`timed-set`, `every-choice-explained`, `item-provenance`, `sources-cited`, `format-facts`) |
| B2 | over the same golden sets: every item's key is one of its choices and explained | the study-duties pack (`question-key`, `choices-explained`) |
| B3 | over the same golden sets' questions: no answer shown before the attempt | the learning-science pack (`answers-hidden`) |
| B4 | over `agent/personas/` and the LSAT coach's template: the template's contract and notice | the lsat-coach pack (`template-contract`, `lsac-marks`) |
| B5 | over `ai-safety.json`: the task declares its inputs, no tool, its golden outputs and a complete gate | the ai-content-safety pack |
| B6 | over `economy.json`: `practice_set` is declared once and `practice` is left out of the day base | the game-economy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/agent/src/practice.rs` | `deck-streak-agent` | added: the task, the parse, the split and `practice_sets` |
| `crates/agent/src/rights.rs` | `deck-streak-agent` | changed: the port exports and erases `practice_sets` |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the module |
| `crates/agent/tests/practice_set.rs` | `deck-streak-agent` | added: A5 to A8, A14 |
| `crates/agent/tests/rights.rs` | `deck-streak-agent` | changed: A16 |
| `migrations/011401_agent_practice_sets.sql` | `deck-streak-agent` | added |
| `agent/personas/lsat-coach.persona.md` | agent (public) | added: the lsat-coach pack's template, copied with its slots unfilled |
| `agent/duties/practice-questions.duty.md` | agent (public) | added: study-duties' template, copied |
| `agent/prompts/practice-set.prompt.md` | agent (public) | added |
| `agent/golden/practice-set/` | agent (public) | added: synthetic golden sets |
| `agent/roster.example.json` | agent (public) | changed: a neutral test-prep topic bound to the LSAT coach |
| `ai-safety.json` | repo | changed: the task |
| `crates/coordination/src/practice.rs` | `deck-streak-coordination` | added: generate, start and submit, and the grant |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `practice_sets` under the agent's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row |
| `crates/coordination/tests/practice.rs` | `deck-streak-coordination` | added: A1 to A4, A10 to A13 |
| `crates/bot/src/duty_commands.rs` | `deck-streak-bot` | changed: `/practice` |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command joins the table |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names `/practice`'s replies |
| `crates/api/src/practice_routes.rs` | `deck-streak-api` | added: the four routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes behind the owner's session |
| `crates/api/tests/practice_routes.rs` | `deck-streak-api` | added: A9, A15 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the use cases' ports |
| `web/app/src/routes/practice/+page.svelte` | miniapp | added: the practice screen |
| `web/app/src/lib/practice/PracticeSet.svelte` | miniapp | added |
| `web/app/src/lib/practice/practice.ts` | miniapp | added: the routes' client and types |
| `web/app/src/lib/practice/PracticeSet.test.ts` | miniapp | added: A17 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /practice joins `ROUTES` |
| `economy.json` | repo | changed: `xp.bonuses.practice_set` and `practice` in `xp.day_base_excludes` |
| `.env.example` | repo | changed: the time multiplier, by name, with its default |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row for `practice_sets` |
| `privacy.json`, `PRIVACY.md` | repo | changed: `practice-sets` and the processing by route |
| `scripts/mutation-rows.d/S11400-S11499.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-114-a-practice-set-is-generated-on-request-its-keys-are-held-until-it-is-attempted-and-its-completion-pays-once.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w6-duty-run-and-its-degradation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-114.md` | docs | added |
| `changelog.d/` fragment | repo | added |

The agent's data-rights port is the file SPEC-043's delivery adds, as SPEC-112's manifest note says.

## 5. What this does NOT do

- It shows no official test item: every item is original (#52).
- It pays nothing for a score, and it keeps no score history or chart (#52).
- It builds no settings screen for the time multiplier or the XP (#57).
- It re-prices no other XP source (#281).
- It imports nothing from the predecessor (#61).

## 6. Risks

- **Keys reaching the client early.** The split is in the engine (R4) and the route serves only the
  questions (R6); detected by A8, A9 and A17.
- **A wrong key the model wrote.** It changes only the count shown, never the pay (R8); detected by
  A12.
- **Pay farmed by requesting sets.** A set pays only when completed, once per set; each request is
  one run at a time per process (ADR-113), under SPEC-043's caps.

## 7. Parity goldens

None: practice sets are DeckStreak-native (#52). The golden sets under `agent/golden/practice-set/`
are DeckStreak's own synthetic examples for the box run.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `practice_sets` | `deck-streak-agent` | `migrations/011401_agent_practice_sets.sql` | nothing: the predecessor had no practice sets | exported and erased; the newest 20 kept |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11401-ROUTE-FIRST` | `crates/coordination/src/practice.rs` | the route check before any run | `practice::with_no_ai_route_practice_answers_not_enabled` |
| `S11402-KEY-IN-CHOICES` | `crates/agent/src/practice.rs` | a key outside the choices is withheld | `practice_set::a_set_with_a_bad_item_is_withheld` |
| `S11403-FIVE-CHOICES` | `crates/agent/src/practice.rs` | an LSAT item has five choices | `practice_set::a_set_with_a_bad_item_is_withheld` |
| `S11404-ORIGINAL-ONLY` | `crates/agent/src/practice.rs` | an official item is withheld | `practice_set::an_official_item_is_withheld` |
| `S11405-ITEM-MAX` | `crates/agent/src/practice.rs` | 26 items kept, 27 withheld | `practice_set::a_set_outside_its_item_bounds_is_withheld` |
| `S11406-SPLIT-KEYS` | `crates/agent/src/practice.rs` | the questions lose their keys | `practice_set::the_questions_carry_no_key` |
| `S11407-HIDE-UNTIL-SUBMITTED` | `crates/api/src/practice_routes.rs` | no key sheet before `completed` | `practice_routes::the_set_route_hides_the_keys_until_submitted` |
| `S11408-SUBMIT-ONCE` | `crates/coordination/src/practice.rs` | a second submission writes nothing | `practice::a_second_submission_is_refused` |
| `S11409-PAY-ONCE` | `crates/coordination/src/practice.rs` | scope `once` and the fixed amount | `practice::a_completed_set_pays_once_whatever_its_score` |
| `S11410-NO-LATE-PENALTY` | `crates/coordination/src/practice.rs` | a late submission still completes | `practice::a_late_submission_still_completes` |
| `S11411-KEEP-TWENTY` | `crates/agent/src/practice.rs` | the newest 20 kept; the test inserts 21 | `practice_set::the_table_keeps_the_newest_twenty_sets` |
