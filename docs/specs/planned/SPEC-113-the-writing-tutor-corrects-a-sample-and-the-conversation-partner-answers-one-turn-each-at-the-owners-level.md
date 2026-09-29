# SPEC-113: the writing tutor corrects a sample and the conversation partner answers one turn, each at the owner's level

- **Wave:** W6. **Issues:** #48 (the writing tutor) and #51 (the conversation partner) (epic #7).
  **Context(s):** `deck-streak-agent` (the two tasks, their acceptance, the table
  `conversation_turns`); `deck-streak-coordination` (one use case per duty, shared by every
  surface; the writing habit's confirmation); `deck-streak-bot`, `deck-streak-api` and the Mini App
  (the commands, the routes and the chat screen).
- **Decided by:** ADR-043 (the shell runner, its gate and caps), ADR-054 (no-AI mode is the
  default) and ADR-113 (a requested duty answers as its command's reply).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-024, SPEC-025, SPEC-026, SPEC-028, SPEC-041, SPEC-043,
  SPEC-044 and SPEC-078. **Mutation band:** `S11300-S11399`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-113.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** Both duties are DeckStreak-native (#48, #51): the predecessor had neither,
  so there is no product function to port and no golden to match.
- **The packs' contracts.** The study-duties writing-tutor template asks for a `corrections` list,
  one item per correction, a focus and a next step, about the work and never the person. The
  language-mentors pack fixes the item's shape, `- learner form → correction [category]
  explanation`, the language's category taxonomy, and at most three categories per sample (its
  `write-focus` class, which is advisory, so this SPEC makes the three a deterministic rule). The
  conversation-partner template asks for one turn only, never the learner's side, correcting as it
  goes, and left open.
- **The level.** A language persona's band is the live Road-to-C2 band when its port is wired and
  the roster's band otherwise (SPEC-044 R8).
- **The habit.** SPEC-078 R6 confirms a study day's writing per writing course, and R7 pays it once
  per course and day. A sample the owner sends is writing the owner did, whatever the tutor answers.
- **A requested run takes time.** SPEC-043's runner is a process that can run for minutes, so each
  answer is a placeholder and one edit (ADR-113).

## 2. Requirements

The writing tutor (#48)

R1. `/correct <course>` followed by text, in one message, submits a writing sample for a configured
    writing course (SPEC-078 R6); any other course, an empty sample, or a sample over 2000
    characters is refused by name with no run. `POST /api/writing/{course}/samples` takes the same
    input. Both call `coordination::writing_tutor::submit`, the one use case.
R2. The use case confirms the course's writing for the current study day through SPEC-078's
    confirmation (R6), whatever the tutor's verdict and whether or not the route is present, before
    any run. A confirmation that already exists is left as it is.
R3. The task `writing-tutor` (format `persona`, kind `language`) passes exactly: the course's
    persona (SPEC-044), `agent/duties/writing-tutor.duty.md`, the band of SPEC-044 R8 (source
    `config`), and the sample (source `learner`). It holds no tool; its caps are 300 seconds, 5 turns
    and SPEC-043 R6's default budget.
R4. Its gate is the ai-content-safety pack's required classes for a persona task (SPEC-043 R10) and
    language-mentors' `write-corrections` and `cefr-ratio`. An output is accepted only when the gate
    passes AND every correction item parses as the pack's shape AND the distinct `[category]` values
    number at most three AND a next step is present.
R5. The sample is not stored. The tutor's accepted answer is not stored beyond the run's record in
    `agent_runs` (SPEC-043 R13), which holds no text.

The conversation partner (#51)

R6. `/talk <course>` followed by text sends one turn to the partner for a configured language
    course. The Mini App's chat screen sends the same input to `POST /api/conversation/{course}/turns`.
    Both call `coordination::conversation::turn`, the one use case.
R7. `conversation_turns` (`migrations/011301_agent_conversation_turns.sql`, `STRICT`, `created_at`)
    holds the current study day's turns: course, side (`owner` or `partner`), text and study day.
    Each write deletes every row of an earlier study day in the same write, so the table never holds
    more than one study day.
R8. The task `conversation-partner` (format `persona`, kind `language`) passes exactly: the course's
    persona, `agent/duties/conversation-partner.duty.md`, the band (source `config`), and the course's
    last 12 turns of the current study day and the new turn (source `learner`). It holds no tool; its
    caps are 120 seconds, 5 turns and SPEC-043 R6's default budget.
R9. Its gate is the ai-content-safety pack's required classes for a persona task, study-duties'
    `reply-one-turn`, language-mentors' `cefr-ratio` and `pronunciation-notation`, and the course
    language's notation class (`pinyin-tones`, `furigana-ruby` and `pitch-accent`, `hangul-batchim`,
    `french-liaison` or `spanish-stress`). An output is accepted only when the gate passes; the
    owner's turn and the accepted reply are then stored together. A withheld reply stores neither.
R10. The chat screen shows the day's turns, each partner turn with its inline corrections and its
    pronunciation notes, and sends a turn with the send button. It never shows a turn of an earlier
    study day.

Both duties (ADR-113)

R11. With the route absent, each command and route answers "The writing tutor is not enabled" or
    "The conversation partner is not enabled", with no run, no alert, and (for the tutor) the
    habit still confirmed.
R12. With a route, the bot replies at once with a placeholder, runs the use case off the update
    loop, and replaces the placeholder by one edit with the accepted answer, or with SPEC-043 R12's
    refusal line for a withheld or unavailable verdict; the route answers in its response. The
    placeholder and the edit join the census's command replies.
R13. Each process runs at most one run per duty; a second request while one runs is answered at once
    that one is running, with no run.
R14. Both answer the owner only: the bot's owner gate (SPEC-026) and the owner's session (SPEC-024);
    any other caller is refused with no data and no run.

Data rights and the rules

R15. `conversation_turns` is user data: the own-tables row in the context map, coordination's
    data-rights registry and its symmetry seed, `privacy.json` (the category `conversation`, and the
    processing by route: only with an AI route configured is a sample or a turn sent to the model
    provider), `PRIVACY.md`, and the agent's data-rights port. For the v9 import (#61) it maps from
    nothing: the predecessor had no conversation partner.
R16. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no dishonest
    copy (a withheld answer is never shown) and no claim the product cannot honour (the not-enabled
    line says what is off).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with the route absent the tutor answers not enabled, launches nothing, and still confirms the habit | `with_no_ai_route_a_sample_still_counts_for_the_habit` |
| A2 | a sample confirms the day's writing once, before the run, whatever the verdict | `a_sample_confirms_the_writing_habit_once` |
| A3 | a course that is not a writing course, an empty sample and one over 2000 characters are refused with no run | `a_sample_outside_its_bounds_is_refused` |
| A4 | the tutor's task receives exactly its four inputs | `the_tutor_receives_exactly_its_inputs` |
| A5 | an answer with four distinct categories is withheld, and three are accepted | `corrections_over_three_categories_are_withheld` |
| A6 | a correction item off the pack's shape, or no next step, is withheld | `a_malformed_correction_is_withheld` |
| A7 | with the route absent the partner answers not enabled and stores nothing | `with_no_ai_route_the_partner_stores_no_turn` |
| A8 | the partner's task receives the band, at most the 12 last turns of the day, and the new turn | `the_partner_receives_at_most_twelve_turns` |
| A9 | a turn of a new study day deletes every earlier day's turns in the same write | `a_new_day_clears_the_conversation` |
| A10 | a withheld reply stores neither the owner's turn nor the reply | `a_withheld_reply_stores_no_turn` |
| A11 | the bot's placeholder is replaced by one edit with the answer | `the_placeholder_is_edited_with_the_answer` |
| A12 | a second request while a run is held is answered at once with no second run | `a_second_request_waits_for_none` |
| A13 | the bot and the route run the same use case | `both_surfaces_run_one_use_case` |
| A14 | a non-owner is refused by both routes with no data and no run | `the_writing_and_conversation_routes_are_owner_only` |
| A15 | `conversation_turns` is exported and erased | `the_conversation_turns_are_exported_and_erased` |
| A16 | the chat screen shows the day's turns with their corrections and notes | `the chat screen shows each partner turn with its corrections` |

```acceptance
A1: cargo test -p deck-streak-coordination --test writing_tutor -- --exact with_no_ai_route_a_sample_still_counts_for_the_habit
A2: cargo test -p deck-streak-coordination --test writing_tutor -- --exact a_sample_confirms_the_writing_habit_once
A3: cargo test -p deck-streak-coordination --test writing_tutor -- --exact a_sample_outside_its_bounds_is_refused
A4: cargo test -p deck-streak-coordination --test writing_tutor -- --exact the_tutor_receives_exactly_its_inputs
A5: cargo test -p deck-streak-agent --test writing_tutor -- --exact corrections_over_three_categories_are_withheld
A6: cargo test -p deck-streak-agent --test writing_tutor -- --exact a_malformed_correction_is_withheld
A7: cargo test -p deck-streak-coordination --test conversation -- --exact with_no_ai_route_the_partner_stores_no_turn
A8: cargo test -p deck-streak-coordination --test conversation -- --exact the_partner_receives_at_most_twelve_turns
A9: cargo test -p deck-streak-agent --test conversation_turns -- --exact a_new_day_clears_the_conversation
A10: cargo test -p deck-streak-coordination --test conversation -- --exact a_withheld_reply_stores_no_turn
A11: cargo test -p deck-streak-bot --test duty_commands -- --exact the_placeholder_is_edited_with_the_answer
A12: cargo test -p deck-streak-bot --test duty_commands -- --exact a_second_request_waits_for_none
A13: cargo test -p deck-streak-api --test duty_routes -- --exact both_surfaces_run_one_use_case
A14: cargo test -p deck-streak-api --test duty_routes -- --exact the_writing_and_conversation_routes_are_owner_only
A15: cargo test -p deck-streak-agent --test rights -- --exact the_conversation_turns_are_exported_and_erased
A16: pnpm exec vitest run web/app/src/lib/conversation/Chat.test.ts -t "the chat screen shows each partner turn with its corrections"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. ai-content-safety and study-duties are enforced by
their own features (#29, #32) and language-mentors is already enforced, so this delivery changes no
pack's state and the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over the golden answers under `agent/golden/writing-tutor/` (one per mentor language): every correction is on the pack's shape and in the language's taxonomy | the language-mentors pack (`write-corrections`, `cefr-ratio`) |
| B2 | over the golden turns under `agent/golden/conversation-partner/` (one per mentor language): one open turn, at the band's ratio, with the language's notation | the study-duties pack (`reply-one-turn`) and the language-mentors pack (`cefr-ratio`, `pronunciation-notation` and the five languages' notation classes) |
| B3 | over `ai-safety.json`: both tasks declare their fenced `learner` input, no tool, their golden outputs and a complete gate | the ai-content-safety pack |
| B4 | over the chat screen's source under `web/app/src/lib/conversation/`: its controls have names and its text meets contrast | the accessibility pack |
| B5 | over `privacy.json`, `PRIVACY.md` and the agent's data-rights port: `conversation` names `conversation_turns` and the processing by route | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/agent/src/writing_tutor.rs` | `deck-streak-agent` | added: the task and its acceptance |
| `crates/agent/src/conversation.rs` | `deck-streak-agent` | added: the task, its acceptance and `conversation_turns` |
| `crates/agent/src/rights.rs` | `deck-streak-agent` | changed: the port exports and erases `conversation_turns` |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the modules |
| `crates/agent/tests/writing_tutor.rs` | `deck-streak-agent` | added: A5, A6 |
| `crates/agent/tests/conversation_turns.rs` | `deck-streak-agent` | added: A9 |
| `crates/agent/tests/rights.rs` | `deck-streak-agent` | changed: A15 |
| `migrations/011301_agent_conversation_turns.sql` | `deck-streak-agent` | added |
| `agent/duties/writing-tutor.duty.md`, `agent/duties/conversation-partner.duty.md` | agent (public) | added: study-duties' templates, copied |
| `agent/prompts/writing-tutor.prompt.md`, `agent/prompts/conversation-partner.prompt.md` | agent (public) | added |
| `agent/golden/writing-tutor/`, `agent/golden/conversation-partner/` | agent (public) | added: synthetic golden answers and turns |
| `agent/redteam/` | agent (public) | changed: a case in a sample and in a turn |
| `ai-safety.json` | repo | changed: both tasks |
| `crates/coordination/src/writing_tutor.rs` | `deck-streak-coordination` | added: `submit` |
| `crates/coordination/src/conversation.rs` | `deck-streak-coordination` | added: `turn` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `conversation_turns` under the agent's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row |
| `crates/coordination/tests/writing_tutor.rs` | `deck-streak-coordination` | added: A1 to A4 |
| `crates/coordination/tests/conversation.rs` | `deck-streak-coordination` | added: A7, A8, A10 |
| `crates/bot/src/duty_commands.rs` | `deck-streak-bot` | added: `/correct` and `/talk`, the placeholder and its edit |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the commands join the table |
| `crates/bot/tests/duty_commands.rs` | `deck-streak-bot` | added: A11, A12 |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names the placeholder and its edit |
| `crates/api/src/duty_routes.rs` | `deck-streak-api` | added: the two routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes behind the owner's session |
| `crates/api/tests/duty_routes.rs` | `deck-streak-api` | added: A13, A14 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the use cases' ports |
| `web/app/src/routes/conversation/+page.svelte` | miniapp | added: the chat screen |
| `web/app/src/lib/conversation/Chat.svelte` | miniapp | added |
| `web/app/src/lib/conversation/conversation.ts` | miniapp | added: the route's client and types |
| `web/app/src/lib/conversation/Chat.test.ts` | miniapp | added: A16 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /conversation joins `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row for `conversation_turns` |
| `privacy.json`, `PRIVACY.md` | repo | changed: `conversation` and the processing by route |
| `scripts/mutation-rows.d/S11300-S11399.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-113-the-writing-tutor-corrects-a-sample-and-the-conversation-partner-answers-one-turn-each-at-the-owners-level.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w6-duty-run-and-its-degradation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-113.md` | docs | added |
| `changelog.d/` fragment | repo | added |

The agent's data-rights port is the file SPEC-043's delivery adds (named `rights.rs` in its
manifest; `privacy.json`'s globs read `data_rights.rs`, as SPEC-112's manifest note says).

## 5. What this does NOT do

- It tutors no law writing and holds no law conversation: both issues are the language mentors'
  (#48, #51).
- It keeps no sample, no draft and no conversation past its study day (#48).
- It pays no XP of its own: the writing habit pays through SPEC-078 (#94).
- It builds no settings screen for the caps or the turn window (#57).
- It imports nothing from the predecessor (#61).

## 6. Risks

- **A sample that carries instructions.** It is fenced as `learner` data, and the task holds no
  tool (SPEC-043 R9, R8); the red-team cases prove it.
- **A tutor that nitpicks every line.** Prevented by the three-category rule; detected by A5.
- **A chat that grows without bound.** One study day at most (R7) and 12 turns into the prompt (R8);
  detected by A8 and A9.
- **A run started per keystroke.** One run per duty per process (R13); detected by A12.

## 7. Parity goldens

None: both duties are DeckStreak-native (#48, #51). The golden answers and turns under
`agent/golden/` are DeckStreak's own synthetic examples for the box run.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `conversation_turns` | `deck-streak-agent` | `migrations/011301_agent_conversation_turns.sql` | nothing: the predecessor had no conversation partner | exported and erased; never older than the current study day |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11301-HABIT-FIRST` | `crates/coordination/src/writing_tutor.rs` | the habit is confirmed before, and without, the run | `writing_tutor::with_no_ai_route_a_sample_still_counts_for_the_habit` |
| `S11302-SAMPLE-MAX` | `crates/coordination/src/writing_tutor.rs` | 2000 characters kept, 2001 refused | `writing_tutor::a_sample_outside_its_bounds_is_refused` |
| `S11303-WRITING-COURSE` | `crates/coordination/src/writing_tutor.rs` | a non-writing course is refused | `writing_tutor::a_sample_outside_its_bounds_is_refused` |
| `S11304-THREE-CATEGORIES` | `crates/agent/src/writing_tutor.rs` | three kept, four withheld | `writing_tutor::corrections_over_three_categories_are_withheld` |
| `S11305-ITEM-SHAPE` | `crates/agent/src/writing_tutor.rs` | an item off the shape is withheld | `writing_tutor::a_malformed_correction_is_withheld` |
| `S11306-TURN-WINDOW` | `crates/coordination/src/conversation.rs` | 12 turns into the prompt; the test holds 13 | `conversation::the_partner_receives_at_most_twelve_turns` |
| `S11307-DAY-CLEAR` | `crates/agent/src/conversation.rs` | earlier days are deleted in the write | `conversation_turns::a_new_day_clears_the_conversation` |
| `S11308-WITHHELD-NOTHING` | `crates/coordination/src/conversation.rs` | a withheld reply stores nothing | `conversation::a_withheld_reply_stores_no_turn` |
| `S11309-ONE-RUN` | `crates/bot/src/duty_commands.rs` | a held run refuses a second; the test's second request answers within one second | `duty_commands::a_second_request_waits_for_none` |
| `S11310-ROUTE-FIRST` | `crates/coordination/src/conversation.rs` | the route check before any run or write | `conversation::with_no_ai_route_the_partner_stores_no_turn` |
