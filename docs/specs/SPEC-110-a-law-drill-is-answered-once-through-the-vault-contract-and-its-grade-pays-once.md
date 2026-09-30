# SPEC-110: a law drill is answered once through the vault contract, and its grade pays once

- **Wave:** W6. **Issue:** #136 (epic #7). **Context(s):** `deck-streak-vault` (the drill notes'
  contract: the list, the single view, the answer's append, the graded parse, the queue and the
  rollup; the tables `drill_answers` and `drill_grades`); `deck-streak-coordination` (the answer
  use case both surfaces call, the `drill_postback` job and its pay through the grant port, the
  drill-grades memory port's wiring); `deck-streak-api` and `deck-streak-bot` (the drill routes,
  `/drills` and `/drill`).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-027 (one job table), ADR-110 (a
  graded drill is recorded once and paid as a `once` grant keyed by its drill) and ADR-118 (what
  DeckStreak writes into the vault is the owner's file: its records are exported and erased, the
  files never deleted).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-026, SPEC-027, SPEC-029, SPEC-040, SPEC-041, SPEC-042
  and SPEC-044. **Mutation band:** `S11000-S11099`.
- **Status:** built: moved from `docs/specs/planned/` to `docs/specs/` by the delivery that
  carries its tests and `docs/red-first/SPEC-110.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` 04b4182 the vault crate (`crates/vault/src/`) holds the readings
  tree and the staged-run executor, and no drill code; SPEC-042 R12 says the vault context owns no
  table. SPEC-044 R7 leaves the `drill-grades` memory source unwired until this delivery (#136).
- **What is ported** (the predecessor at `27ee2bc`): the drill notes' contract
  (`vault_bridge.py:list_active_drills`, `list_unanswered_drills`, `read_active_drill`,
  `append_drill_answer`, `_read_drill_meta`, `_drill_type`, `_drill_answered`, `_safe_stem`,
  `_sanitise_defer_reason`); the graded post-back (`vault_bridge.py:_parse_graded_drill`,
  `poll_drill_postbacks`) and its once-per-drill pay (`database.py:GamifyStore.grant_drill_xp_once`);
  the queue (`nudges.py:NudgesLayer.drill_queue`) and the answer's outcomes
  (`nudges.py:NudgesLayer.submit_drill_answer`); the rollup (`vault_bridge.py:_active_drill_rollup`);
  the drill codes (`curriculum.py:LAW_DRILL_ALIASES`); and the bot's tokens and keyboard
  (`bot.py:CommandBot._encode_drill_token`, `_resolve_drill_token`, `_DRILLS_KEYBOARD_LIMIT`).
- **The numbers.** A graded drill pays 15 XP (`DRILL_POSTBACK_XP`) unless its frontmatter's `xp`
  overrides it, clamped to 10..25 (`DRILL_XP_MIN`, `DRILL_XP_MAX`); a deferral reason is cut at 120
  characters (`DEFER_REASON_MAX_LEN`); a callback token holds at most 64 bytes, its hashed form 20
  hex characters (`_MAX_CALLBACK_DATA`, `_DRILL_TOKEN_HASH_LEN`); the keyboard shows 12 drills.
- **Traps a hand port falls into.**
  - The override is read with Python's `int()`, which accepts surrounding whitespace, a sign and
    digit underscores, and refuses a decimal; a non-integer falls back to 15, never to 0. Rust's
    integer parse differs on each of these, so the golden's cases name every one. A full-width
    digit is not an integer here (an ASCII digit is), where Python reads it, as the code's own
    comment says; the value then falls back to 15.
  - `_parse_graded_drill` returns nothing, never an error, for a note with no frontmatter, a status
    other than `graded`, or an empty stem. Its outcome set is exactly {paid at the clamped XP, not
    gradeable}.
  - `_drill_answered` reads a missing marker as unanswered, the safe default; a ticked `[x]` or
    `[X]` is answered.
  - The predecessor paid on the vault-local calendar date. DeckStreak pays on the study day, which
    turns over at 04:00 (LEXICON), so a grade polled at 02:00 pays the previous study day.
  - The deferral reason drops every character of Unicode category C before it collapses spaces, so
    a newline or a tab vanishes rather than becoming a space; the cut keeps 119 characters and adds
    `…`.
  - A token of exactly 64 bytes is kept whole; 65 bytes is hashed.
  - The day base of the consistency bonus counts `drill:` XP: `database.py:GamifyStore.day_base_xp`
    leaves it in, and SPEC-072 R17 ports that rule with its golden, so nothing here excludes it.
  - The predecessor's once-guard, `drill_xp_grants`, and its ledger row were written separately. In
    DeckStreak the grant port's `once` scope is the guard (SPEC-040 R4), so there is no second guard
    to split from it.

## 2. Requirements

The drill notes

R1. The drills folder is the layout's `drill-coach` folder (`crates/vault/data/layout.json`), and
    its `Active` and `Graded` subfolders are the predecessor's (`drills.constants`). A missing
    `Active` or `Graded` folder is an empty list, never an error. A drill's id is its note's stem,
    and a stem that is empty or holds `/`, `\` or `..` is refused before any read
    (`vault_bridge.py:_safe_stem`). Amended after review: every path the adapter lists, reads or
    writes passes one gate (`DrillNotes::confined`, then the kind check of `regular_note`). It
    refuses a link to a file, a link to a directory, a link to a link, a dangling link and a link to
    a place inside the vault, whether the link is the note or the `Active` or `Graded` folder that
    holds it (the adapter's rule, `crates/vault/src/fs.rs`), where the predecessor followed links. A
    dangling link at the `Active` or `Graded` folder resolves nowhere, so it reads as a missing
    folder (an empty list, and `NotActive` for an answer) and nothing is followed; every other link
    at a folder is refused with `NotAFolder`.
    The gate stands before `list_active`, `graded`, `view` and `answer` reach the file system.
R2. The list, the unanswered list and the single view equal `goldens/drill_meta.json`
    (`vault_bridge.py:_read_drill_meta`, `list_active_drills`, `list_unanswered_drills`,
    `read_active_drill`): the id, the type (`_drill_type`), the subject, the title, the age in study
    days, answered and deferred; the single view adds the prompt and the sanitised deferral reason
    (`goldens/drill_defer_reason.json`, `_sanitise_defer_reason`), which no list shows. A note that
    cannot be read is left out and logged by its error's type, never by its path. For the drill
    workspace (#55, SPEC-121) the single view also carries `sections`, the note's `## ` headings
    after the prompt's cut in note order, leaving out `## Self-Check` and every heading after the
    Ready marker, and `self_check`, the text of each checklist line under `## Self-Check` other
    than the Ready marker. The predecessor's view carried neither, so A24 proves them, not the
    golden.
R3. The queue equals `goldens/drill_queue.json` (`nudges.py:NudgesLayer.drill_queue`): unanswered,
    awaiting grading, and deferred among the answered. The rollup equals `goldens/drill_rollup.json`
    (`vault_bridge.py:_active_drill_rollup`): active, awaiting grading, the oldest age, unmatched
    and deferred.

The answer

R4. An answer's outcome set is exactly {appended, `empty_answer`, `not_active`, `already_answered`,
    `rail_refused`} (`nudges.py:NudgesLayer.submit_drill_answer`, `vault_bridge.py:append_drill_answer`,
    SPEC-042 R3). An empty or all-space answer is `empty_answer` and writes nothing; a missing note
    is `not_active`; a ticked marker, read from this read of the note, is `already_answered`.
R5. An appended answer replaces the unticked marker once with the ticked one and appends the answer
    under its heading, byte for byte as `goldens/drill_answer_append.json`; the note's status stays
    `active`; the write lands through SPEC-042 R2's temp file and passes R3's rails.
R6. `drill_answers` (`migrations/011001_vault_drills.sql`, `STRICT`, `created_at`) holds one row per
    drill (primary key the drill id) with the study day and the surface (`bot` or `mini_app`). The
    row's insert and the note's write run in one `Db::write` (SPEC-020 R16): a second answer, from
    either surface or another process, meets the row and is `already_answered`, even when the
    note's marker reads unticked; a note write that fails rolls the row back.
R7. `coordination::drills::answer` is the one use case both surfaces call: the bot's `/drills`
    answer and `POST /api/drills/{id}/answer`. A test drives both through it.

The grade and its pay

R8. The graded parse equals `goldens/drill_graded_parse.json` (`vault_bridge.py:_parse_graded_drill`)
    for every case: no frontmatter, another status, an empty stem, no override, an override of 9,
    10, 25 and 26, a negative, whitespace, a sign, underscores and a decimal.
R9. The job `drill_postback` in `coordination::jobs::TABLE` (SPEC-027 R1), hourly at a minute the
    job table's census admits under SPEC-027 R2, and not `catch_up`, scans `Graded` and, for each
    parsed note, records the grade and asks the grant port (SPEC-040) for the clamped amount on the
    study day of the poll, source `drill:<drill id>`, track `law`, scope `once`. A missing vault
    root is the job's error (SPEC-027 R7 pages it once); a missing `Graded` folder pays nothing.
    The pay (15, between 10 and 25) is declared once, as the vault's constants `POSTBACK_XP`, `XP_MIN` and `XP_MAX`
    (ADR-047's one-home rule: the game-economy pack refuses a key its reference lacks, so `economy.json` does not carry it).
R10. A drill id is kept as-is when `drill:<id>` fits SPEC-040 R2's grammar (128 characters at most,
    so the id holds 128 less the six of `drill:`, derived from that constant and not restated)
    and the id does not begin with `h.`; every other id is keyed `drill:h.` followed by the first 32
    hex characters of the SHA-256 of the id (ADR-110), so a kept key never equals a hashed one.
R11. `drill_grades` (the same migration) holds one row per drill: the type, the subject, the
    accepted XP (a `CHECK` between 10 and 25), the graded study day. The row and the grant are each
    idempotent, so a poll that stops between them is completed by the next one, and a re-poll pays
    nothing more (SPEC-040 R4).
R12. The drill-grades memory port (SPEC-044 R7) is wired in `crates/daemon/src/wiring.rs`: for a
    subject, at most 10 entries, newest first, each naming the drill's type and its accepted XP,
    never the drill's text or its answer.

The surfaces

R13. `/drills` lists the unanswered drills, at most 12 on its keyboard with `…and N more` after,
    each button's data `dv:` plus the token (`bot.py:CommandBot._encode_drill_token`,
    `goldens/drill_callback_token.json`); the view's Answer button carries `da:` plus the token. A
    token resolves only when it names exactly one drill of the current list
    (`_resolve_drill_token`); any doubt is refused with a line that names no drill. After Answer,
    the owner's next message that replies to the prompt, or is not a command, is the answer; a
    command cancels it. The bot holds at most one pending drill, in memory.
R14. `/drill` alone offers the four law drill types; `/drill <code>` lists the active drills of that
    type, by the codes `i`, `u`, `o` and `c` (`curriculum.py:LAW_DRILL_ALIASES`); another code is
    refused with the list of codes. Both are registered for the owner's chat (SPEC-026 R11).
R15. `GET /api/drills` (the list and the queue), `GET /api/drills/{id}` (the single view) and
    `POST /api/drills/{id}/answer` answer the authenticated owner only (SPEC-024's session), and
    every reply of the bot and body of the API is the use case's, word for word.
R16. The bot's new replies are named in SPEC-041's census (`crates/notifications/tests/one_router.rs`)
    as command replies, so no drill reply goes around the router's rules.

Data rights and the rules

R17. `drill_answers` and `drill_grades` are user data: registered in the context map's own tables,
    in coordination's data-rights registry and its symmetry test, declared in `privacy.json` and
    `PRIVACY.md` (category `law-drills`), and exported and erased by the vault's new data-rights
    port `crates/vault/src/data_rights.rs`. The drill notes are the owner's files in the owner's
    vault: an erase deletes these rows, and never a note (ADR-118). This amends SPEC-042 R12: the vault context owns these two tables and still depends on the kernel only. The amendment is owed: the delivery that builds SPEC-110 adds SPEC-042's dated amendment line, and SPEC-042 itself is not edited by the plan.
R18. For the v9 import (#61): `drill_grades` maps from the predecessor's `drill_xp_grants`, row for
    row, its XP read from the matching `drill:` row of the predecessor's XP ledger; `drill_answers`
    maps from nothing, because the predecessor kept the answer only in the note, whose marker R4
    reads.
R19. CHARTER 10's eleven anti-goals bind this SPEC as one block; the one it touches is no unbounded
    faucet: a drill pays at most 25, once.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the list, the unanswered list and the single view equal the golden, and a refused stem reads nothing | `the_drill_views_match_the_predecessors_golden` |
| A2 | the deferral reason equals the golden, 119 characters and `…` over the cut | `the_defer_reason_matches_the_predecessors_golden` |
| A3 | the queue and the rollup equal their goldens | `the_queue_and_rollup_match_the_predecessors_golden` |
| A4 | an append equals the golden byte for byte and the status stays active | `an_answer_is_appended_as_the_predecessor_appends_it` |
| A5 | each of the five outcomes is returned for its case, and only `appended` writes | `each_answer_outcome_is_returned_for_its_case` |
| A6 | a second answer through the other surface is `already_answered` and the note holds one answer | `a_second_answer_from_either_surface_is_refused` |
| A7 | a note write that fails leaves no `drill_answers` row | `a_failed_note_write_leaves_no_answer_row` |
| A8 | the graded parse equals the golden for every case | `the_graded_parse_matches_the_predecessors_golden` |
| A9 | a poll pays the clamped XP once on the poll's study day, track `law`, scope `once`; a re-poll pays nothing | `a_graded_drill_pays_once_on_the_study_day` |
| A10 | a poll at 02:00 pays the previous study day | `a_poll_before_the_rollover_pays_the_previous_study_day` |
| A11 | an id outside the grammar, or beginning `h.`, is keyed `drill:h.` and 32 hex, and one that fits is kept | `an_unkeyable_drill_id_is_keyed_by_its_hash` |
| A12 | a missing root is the job's error and a missing `Graded` pays nothing | `a_missing_root_is_an_error_and_a_missing_graded_folder_is_zero` |
| A13 | a poll stopped after the grant and before the grade row is completed by the next poll, which pays nothing | `an_interrupted_poll_is_completed_without_a_second_pay` |
| A14 | the job table holds `drill_postback`, hourly, off every reserved minute | `the_job_table_holds_the_drill_postback` |
| A15 | the memory port gives at most 10 entries, newest first, with no drill text | `the_drill_grades_port_gives_types_and_xp_only` |
| A16 | a token equals the golden at 64 and 65 bytes, and a token naming no drill or two is refused | `the_drill_token_matches_the_predecessors_golden` |
| A17 | `/drills` shows 12 buttons and `…and N more`, and the next message after Answer is the answer | `drills_lists_twelve_and_takes_the_next_message` |
| A18 | `/drill` offers four types and `/drill x` lists the codes | `drill_filters_by_the_four_codes` |
| A19 | the three routes answer the owner and refuse any other session | `the_drill_routes_answer_only_the_owner` |
| A20 | both tables are exported and erased, and an erase deletes no note | `the_drill_tables_are_exported_and_erased_and_no_note` |
| A21 | every drill constant equals `goldens/drills.constants.json` | `the_drill_constants_equal_the_predecessors` |
| A22 | a drill that has an answer row is `already_answered` even when its note's marker is unticked | `an_answer_row_refuses_an_unticked_note` |
| A23 | the grade row refuses an XP of 9 and of 26, and takes 10 and 25 | `the_grade_row_refuses_xp_outside_its_band` |
| A24 | the single view's `sections` and `self_check` are the note's, in order, for a note of each of the four shapes and one with no answer heading (both empty) | `the_single_view_carries_the_answer_sections_and_the_self_check` |

```acceptance
A1: cargo test -p deck-streak-vault --test drill_goldens -- --exact the_drill_views_match_the_predecessors_golden
A2: cargo test -p deck-streak-vault --test drill_goldens -- --exact the_defer_reason_matches_the_predecessors_golden
A3: cargo test -p deck-streak-vault --test drill_goldens -- --exact the_queue_and_rollup_match_the_predecessors_golden
A4: cargo test -p deck-streak-vault --test drill_answer -- --exact an_answer_is_appended_as_the_predecessor_appends_it
A5: cargo test -p deck-streak-vault --test drill_answer -- --exact each_answer_outcome_is_returned_for_its_case
A6: cargo test -p deck-streak-coordination --test drill_answer -- --exact a_second_answer_from_either_surface_is_refused
A7: cargo test -p deck-streak-vault --test drill_answer -- --exact a_failed_note_write_leaves_no_answer_row
A8: cargo test -p deck-streak-vault --test drill_goldens -- --exact the_graded_parse_matches_the_predecessors_golden
A9: cargo test -p deck-streak-coordination --test drill_postback -- --exact a_graded_drill_pays_once_on_the_study_day
A10: cargo test -p deck-streak-coordination --test drill_postback -- --exact a_poll_before_the_rollover_pays_the_previous_study_day
A11: cargo test -p deck-streak-vault --test drill_goldens -- --exact an_unkeyable_drill_id_is_keyed_by_its_hash
A12: cargo test -p deck-streak-coordination --test drill_postback -- --exact a_missing_root_is_an_error_and_a_missing_graded_folder_is_zero
A13: cargo test -p deck-streak-coordination --test drill_postback -- --exact an_interrupted_poll_is_completed_without_a_second_pay
A14: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_the_drill_postback
A15: cargo test -p deck-streak-daemon --test drill_memory -- --exact the_drill_grades_port_gives_types_and_xp_only
A16: cargo test -p deck-streak-bot --test drill_commands -- --exact the_drill_token_matches_the_predecessors_golden
A17: cargo test -p deck-streak-bot --test drill_commands -- --exact drills_lists_twelve_and_takes_the_next_message
A18: cargo test -p deck-streak-bot --test drill_commands -- --exact drill_filters_by_the_four_codes
A19: cargo test -p deck-streak-api --test drill_routes -- --exact the_drill_routes_answer_only_the_owner
A20: cargo test -p deck-streak-vault --test drill_store -- --exact the_drill_tables_are_exported_and_erased_and_no_note
A21: cargo test -p deck-streak-vault --test drill_goldens -- --exact the_drill_constants_equal_the_predecessors
A22: cargo test -p deck-streak-vault --test drill_answer -- --exact an_answer_row_refuses_an_unticked_note
A23: cargo test -p deck-streak-vault --test drill_store -- --exact the_grade_row_refuses_xp_outside_its_band
A24: cargo test -p deck-streak-vault --test drill_goldens -- --exact the_single_view_carries_the_answer_sections_and_the_self_check
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, telegram-platform and game-economy packs
stay enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/vault/src/data_rights.rs`: the `law-drills` category names `drill_answers` and `drill_grades` with purpose, basis and retention, and export and erase cover both | the privacy-gdpr pack |
| B2 | over `crates/bot/src/drill_commands.rs` and `crates/bot/src/commands.rs`: every callback datum the drill keyboards build is 1 to 64 bytes, and every drill reply stays within the message length | the telegram-platform pack |
| B3 | over `economy.json`: its `xp` section is unchanged by this delivery and the pack still passes; the drill pay lives in the vault's constants only | the game-economy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/vault/src/drills.rs` | `deck-streak-vault` | added: the list, the single view, the queue, the rollup, the append, the graded parse, the key rule |
| `crates/vault/src/drill_store.rs` | `deck-streak-vault` | added: `drill_answers` and `drill_grades` |
| `crates/vault/src/data_rights.rs` | `deck-streak-vault` | added: the vault's data-rights port |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the modules |
| `crates/vault/Cargo.toml` | `deck-streak-vault` | changed: the workspace dependencies the store uses |
| `crates/vault/tests/drill_goldens.rs` | `deck-streak-vault` | added: A1 to A3, A8, A11, A21, A24 |
| `crates/vault/tests/drill_answer.rs` | `deck-streak-vault` | added: A4, A5, A7, A22 |
| `crates/vault/tests/drill_store.rs` | `deck-streak-vault` | added: A20, A23 |
| `crates/vault/tests/fixtures/drills/` | `deck-streak-vault` | added: synthetic drill notes |
| `migrations/011001_vault_drills.sql` | `deck-streak-vault` | added: `drill_answers` and `drill_grades` |
| `crates/coordination/src/drills.rs` | `deck-streak-coordination` | added: the answer use case and the post-back step |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: `drill_postback` |
| `crates/coordination/src/runner.rs` | `deck-streak-coordination` | unchanged (amendment): the job's entrance is the daemon's job role, which dispatches the drill post-back |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the vault's port and its two tables |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row in each table |
| `crates/coordination/tests/drill_answer.rs` | `deck-streak-coordination` | added: A6 |
| `crates/coordination/tests/drill_postback.rs` | `deck-streak-coordination` | added: A9, A10, A12, A13 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A14 |
| `deploy/systemd/deck-streak-job@drill_postback.timer` | deploy | added: the job's timer, equal to the table |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the drill-grades memory port and the vault's data-rights port |
| `crates/daemon/tests/drill_memory.rs` | `deck-streak-daemon` | added: A15 |
| `crates/bot/src/drill_commands.rs` | `deck-streak-bot` | added: the tokens, the keyboards and the replies' text |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: /drills, /drill, the `dv:` and `da:` callbacks and the pending answer; `Commands` gains the port |
| `crates/bot/tests/drill_commands.rs` | `deck-streak-bot` | added: A16 to A18 |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names the drill replies and their callers |
| `crates/api/src/drill_routes.rs` | `deck-streak-api` | added: the three routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes are mounted; `ApiState` gains the port |
| `crates/api/tests/drill_routes.rs` | `deck-streak-api` | added: A19 |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables rows for `drill_answers` and `drill_grades` |
| `privacy.json` | repo | changed: the `law-drills` category |
| `PRIVACY.md` | docs | changed: one line for the category |
| `tools/parity-oracle/registry/spec_110.py` | tools | added: the goldens of §7 |
| `tools/parity-oracle/goldens/drill_meta.json`, `drill_defer_reason.json`, `drill_queue.json`, `drill_rollup.json`, `drill_answer_append.json`, `drill_graded_parse.json`, `drill_callback_token.json`, `drills.constants.json` | tools | added |
| `scripts/mutation-rows.d/S11000-S11099.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-110-a-law-drill-is-answered-once-through-the-vault-contract-and-its-grade-pays-once.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/law-drill-answer-grade-and-pay.md` | docs | unchanged (amendment): added by the W6 architect turn; the code proved no correction necessary |
| `docs/red-first/SPEC-110.md` | docs | added |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the `job` role dispatches `drill_postback` |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role hands the drill notes' reader and the answer's writer to its commands at start (R13) |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role hands the drill notes' reader and the answer's writer to `ApiState` at start |
| `changelog.d/` fragment | repo | added |
| `crates/vault/src/unicode_other.rs` | `deck-streak-vault` | changed (amendment): the drill Unicode table, a lookup compared at each boundary |
| `crates/daemon/src/drill_vault.rs` | `deck-streak-daemon` | added (amendment): the roles open the drill notes from the environment; an unset vault is not a start refusal |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed (amendment): the job usage line names the drill post-back |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed (amendment): the re-exports the bot and the api open the drills with |
| `crates/api/src/session_routes.rs`, `crates/api/src/lib.rs` | `deck-streak-api` | changed (amendment): the state-change guard is crate-visible and the module is declared |
| `crates/bot/tests/messages/help.msg.json`, `crates/bot/tests/messages/start.msg.json` | `deck-streak-bot` | changed (amendment): the menu goldens list the two drill commands |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed (amendment): the menu entries |
| `economy.json` | repo | dropped (amendment): ADR-047 keeps an amount v9's template lacks as a constant of its own context, and the game-economy pack refuses the key; R9's pay is the vault's constants, pinned by A21's golden and rows S11004-S11006 |
| `crates/bot/src/lib.rs` | deck-streak-bot | changed (amendment): the drill modules are declared |
| `crates/daemon/src/lib.rs` | deck-streak-daemon | changed (amendment): the `drill_vault` module is declared |
| `crates/vault/src/drill_notes.rs` | deck-streak-vault | added (amendment): the drill notes' reader and answer writer |
| `crates/vault/src/unicode_other_tests.rs` | deck-streak-vault | added (amendment): the literal boundary tests of the Unicode table |
| `deploy/rail-contract.json` | deploy | changed (amendment): the `drill_postback` timer joins the rail census |
| `deploy/README.md` | deploy | changed (amendment): the job list and table name `drill_postback` |
| `deploy/scripts/effective-check.py` | deploy | changed (amendment): the instance pattern admits an underscore |
| `scripts/tests/test_deploy_templates.py` | repo | changed (amendment): two waivers for the `drill_postback` timer |
| `scripts/tests/test_rail_contract.py` | repo | changed (amendment): the `run_job` arms test excludes the drill post-back, which the job role dispatches |
| `docs/decisions/ADR-110-a-graded-drill-is-recorded-once-and-paid-as-a-once-grant-keyed-by-its-drill.md` | docs | changed (amendment): accepted, with the Unicode amendment |
| `docs/specs/SPEC-042-vault-adapter-core-and-readings-date-tree.md` | docs | changed (amendment): the dated amendment line R12 is owed |
| `crates/vault/tests/drill_kills.rs` | deck-streak-vault | added (amendment): the post-green killers of the vault's drill mutants |
| `crates/coordination/tests/drill_paid_count.rs` | deck-streak-coordination | added (amendment): the post-green killer of the post-back's paid count |
| `crates/bot/tests/drill_replies.rs` | deck-streak-bot | added (amendment): the post-green killers of the drill label, list and view |
| `crates/daemon/tests/drill_vault.rs` | deck-streak-daemon | added (amendment): the post-green killer of the drill vault's open |
| `crates/daemon/tests/drill_routes_composed.rs` | deck-streak-daemon | added (amendment): the drill routes served through the composed router |
| `scripts/mutation-equivalent.d/deck-streak-vault.json` | repo | changed (amendment): the two `type_of` equivalence records |
| `crates/coordination/tests/drill_key_population.rs` | deck-streak-coordination | added (amendment): the population of ids of every length 1 to the grammar's limit plus 8, each key accepted by the grammar and the bound equal to the grammar's less the prefix |
| `docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | docs | changed (amendment): §7's vault row counts the two equivalents |

## 5. What this does NOT do

- It mints no drill and grades none; the drill coach does (#46).
- It builds no drill workspace or progress screen in the Mini App (#55).
- It serves no drill to the agent's machine tools; `list_drills` and `get_drill` are the MCP
  server's, behind its `drills` scope (#157, #158).
- It writes no nightly stats file into the vault; the vault stats bridge does, in W8 (#153).
- It adds no drill line to the morning brief (#122).
- It imports no predecessor row; the v9 import does (#61).
- It builds no settings screen for a drill setting (#57).
- It cannot tell a hard link from a regular file: the adapter sees both as a file, so a note hard
  linked to a file outside the vault is read (#442).
- It closes no swap between the gate and the open: a link put in place after the gate has passed
  and before the file is opened is followed (#442).

## 6. Risks

- **A double pay** from a re-poll, a poll racing another, or a poll stopped midway. Prevented by the
  grant port's `once` scope inside its `BEGIN IMMEDIATE` write (SPEC-040 R4, R5), and detected by
  A9 and A13.
- **A lost answer** when the bot and the Mini App answer one drill at once. Prevented by R6's row
  inside the write, and detected by A6.
- **An over-credit** from a model's `xp: 250`. Prevented by the clamp, and detected by A8 and rows
  S11004 and S11005.
- **A parse that differs from Python's `int()`**, paying 15 where the predecessor paid 20. Detected
  by A8's whitespace, sign and underscore cases.
- **A category read that differs by Unicode version** in the deferral reason. The golden's cases
  name controls, format characters and a zero-width joiner, and no code point whose category
  changed between the predecessor's Unicode tables and the crate's.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_110.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic: notes written into a temporary
folder, with invented subjects and prompts.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `drill_meta` | `vault_bridge.py:list_active_drills`, `list_unanswered_drills`, `read_active_drill` | adapter | a temporary vault with answered, unanswered, deferred, dated and undated notes, a stem with `..`, and a folder named `x.md` |
| `drill_defer_reason` | `vault_bridge.py:_sanitise_defer_reason` | function | none: controls, a right-to-left override, a zero-width joiner, a block scalar indicator, 119, 120 and 121 characters |
| `drill_queue` | `nudges.py:NudgesLayer.drill_queue` | adapter | stubs the layer's vault read with the `drill_meta` notes |
| `drill_rollup` | `vault_bridge.py:_active_drill_rollup` | adapter | the same notes, with a subject set that leaves one unmatched |
| `drill_answer_append` | `vault_bridge.py:append_drill_answer` | adapter | a temporary note per case: unticked, ticked `[x]` and `[X]`, no marker, a missing note, an empty answer |
| `drill_graded_parse` | `vault_bridge.py:_parse_graded_drill` | adapter | a temporary note per case of R8 |
| `drill_callback_token` | `bot.py:CommandBot._encode_drill_token`, `_resolve_drill_token` | function | none: ids giving 63, 64 and 65 bytes with each prefix, a multibyte id, and a list holding two ids with one hash |
| `drills.constants` | `vault_bridge.DRILL_POSTBACK_XP`, `DRILL_XP_MIN`, `DRILL_XP_MAX`, `DEFER_REASON_MAX_LEN`, the `Active` and `Graded` folders; `bot._MAX_CALLBACK_DATA`, `_DRILL_TOKEN_HASH_LEN`, `_DRILLS_KEYBOARD_LIMIT`; `curriculum.LAW_DRILL_ALIASES` | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `drill_answers` | `deck-streak-vault` | `migrations/011001_vault_drills.sql` | nothing: the predecessor kept the answer only in the note | exported and erased; the note stays in the owner's vault |
| `drill_grades` | `deck-streak-vault` | `migrations/011001_vault_drills.sql` | `drill_xp_grants`, row for row, with the XP of its `drill:` ledger row (#61) | exported and erased; an erase takes back no XP, which the ledger's own erase covers |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11001-READY-MARKER` | `crates/vault/src/drills.rs` | a ticked marker reads as answered | `drill_answer::each_answer_outcome_is_returned_for_its_case` |
| `S11002-ALREADY-ANSWERED` | `crates/vault/src/drills.rs` | a ticked note is refused, never appended | `drill_answer::each_answer_outcome_is_returned_for_its_case` |
| `S11003-ANSWER-ROW-GUARD` | `migrations/011001_vault_drills.sql` | the primary key on the drill id (a script-mutation row) | `drill_answer::an_answer_row_refuses_an_unticked_note` |
| `S11004-CLAMP-MIN` | `crates/vault/src/drills.rs` | the 10 floor; the golden names 9 and 10 | `drill_goldens::the_graded_parse_matches_the_predecessors_golden` |
| `S11005-CLAMP-MAX` | `crates/vault/src/drills.rs` | the 25 ceiling; the golden names 25 and 26 | `drill_goldens::the_graded_parse_matches_the_predecessors_golden` |
| `S11006-DEFAULT-FIFTEEN` | `crates/vault/src/drills.rs` | the default when no override parses | `drill_goldens::the_drill_constants_equal_the_predecessors` |
| `S11007-GRADED-ONLY` | `crates/vault/src/drills.rs` | only `status: graded` pays | `drill_goldens::the_graded_parse_matches_the_predecessors_golden` |
| `S11008-ONCE-SCOPE` | `crates/coordination/src/drills.rs` | the grant's scope is `once` | `drill_postback::a_graded_drill_pays_once_on_the_study_day` |
| `S11009-TRACK-LAW` | `crates/coordination/src/drills.rs` | the grant's track is `law` | `drill_postback::a_graded_drill_pays_once_on_the_study_day` |
| `S11010-STUDY-DAY` | `crates/coordination/src/drills.rs` | the pay's day is the study day, not the calendar date | `drill_postback::a_poll_before_the_rollover_pays_the_previous_study_day` |
| `S11011-DEFER-CUT` | `crates/vault/src/drills.rs` | the 120-character cut; the golden names 120 and 121 | `drill_goldens::the_defer_reason_matches_the_predecessors_golden` |
| `S11012-CALLBACK-64` | `crates/bot/src/drill_commands.rs` | a 64-byte token is kept whole; the golden names 64 and 65 | `drill_commands::the_drill_token_matches_the_predecessors_golden` |
| `S11013-HASH-20` | `crates/bot/src/drill_commands.rs` | the hashed token's 20 characters | `drill_commands::the_drill_token_matches_the_predecessors_golden` |
| `S11014-KEYBOARD-12` | `crates/bot/src/drill_commands.rs` | the keyboard's 12 | `drill_commands::drills_lists_twelve_and_takes_the_next_message` |
| `S11015-SAFE-STEM` | `crates/vault/src/drills.rs` | a stem holding `..` is refused | `drill_goldens::the_drill_views_match_the_predecessors_golden` |
| `S11016-UNKEYABLE-ID` | `crates/vault/src/drills.rs` | an id outside the grammar is hashed, one inside is kept | `drill_goldens::an_unkeyable_drill_id_is_keyed_by_its_hash` |
| `S11017-EMPTY-ANSWER` | `crates/vault/src/drills.rs` | an all-space answer writes nothing | `drill_answer::each_answer_outcome_is_returned_for_its_case` |
| `S11018-XP-CHECK` | `migrations/011001_vault_drills.sql` | the `CHECK` between 10 and 25 (a script-mutation row) | `drill_store::the_grade_row_refuses_xp_outside_its_band` |
