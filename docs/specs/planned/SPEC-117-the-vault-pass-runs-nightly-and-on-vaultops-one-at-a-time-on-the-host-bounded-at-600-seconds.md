# SPEC-117: the vault pass runs nightly and on `/vaultops`, one at a time on the host, bounded at 600 seconds

- **Wave:** W6. **Issues:** #54 (the vault duties as a nightly pass and an on-demand run) and #155
  (`/vaultops`) (epic #7). **Context(s):** `deck-streak-coordination` (the pass, its claim, its
  bound, the job and the table `vault_passes`); `deck-streak-bot` (`/vaultops`); `deck-streak-api`
  (the latest pass).
- **Decided by:** ADR-027 (the one job table), ADR-054 (no-AI mode is the default), ADR-113 (a
  requested duty answers as its command's reply) and ADR-117 (the pass runs in-process, one at a
  time on the host, under the predecessor's 600-second bound).
- **Prerequisites:** SPEC-020, SPEC-024, SPEC-026, SPEC-027, SPEC-041, SPEC-043 and SPEC-116.
  **Mutation band:** `S11700-S11799`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-117.md` (ADR-016).

## 1. The problem, measured

- **The duties exist; nothing runs them.** SPEC-116 builds the curator, tomorrow's daily note and the
  Sunday synthesis as use cases. #54 asks for them nightly and on the owner's trigger; #155 names
  the trigger `/vaultops`.
- **The predecessor's trigger** (`bot.py:CommandBot._run_vaultops`, at `27ee2bc`) posts a
  placeholder, runs the pass in the background under a 600-second bound
  (`bot.py:_VAULT_OPS_TIMEOUT_SECS`), and edits the placeholder to one of four lines: completed,
  timed out after 600 s, an exit status with a pointer to its service log, or failed unexpectedly.
  `bot.py:CommandBot._deliver_vaultops` sends instead of editing when the placeholder has no
  message id. Its pass was an external program; DeckStreak's pass is its own code (ADR-117).
- **Two processes can reach a pass.** The nightly job runs in the `job` role and `/vaultops` in the
  `bot` role (SPEC-027 R11). Two passes at once could both create tomorrow's daily note, and
  SPEC-042 R2's rename would let the second replace the first. ADR-113's one run per process does
  not reach across processes, so the pass claims a row in the one ledger both processes share.

## 2. Requirements

The pass

R1. `coordination::vault_pass::run(trigger, study_day)` runs, in order, SPEC-116's curator, the
    daily note, and on a Sunday study day the synthesis. A duty's discard, withheld verdict or
    absent route does not stop the next (SPEC-116 R13, R14).
R2. **The claim.** A pass first inserts its `vault_passes` row with outcome `running`, in one
    `BEGIN IMMEDIATE` transaction that also marks `abandoned` every `running` row started more than
    660 seconds earlier (the bound plus a minute) and deletes every row that finished more than 90
    days earlier. The migration holds a partial unique index over the rows whose outcome is
    `running`, so while one pass runs on the host, a second, from either role, is refused with
    `already_running` and runs nothing.
R3. **The bound.** A pass, whatever its trigger, is bounded at 600 seconds, the predecessor's
    `bot.py:_VAULT_OPS_TIMEOUT_SECS` (golden `vaultops.constants`). At the bound its row records
    `timed_out` and the pass stops at its next cancellation point; an apply of a staged run that has
    begun is never cancelled, so SPEC-042 R4's all-or-nothing holds. SPEC-116's caps (240 and 300
    seconds) keep a pass that meets no fault under the bound.
R4. **The outcome.** `completed` when the pass reached its end, whatever each duty's verdict (SPEC-043
    R12 records and alerts each one); `failed` when the pass itself errored (the vault unreadable, a
    ledger write refused). The error goes to the service log through SPEC-020's redaction, and never
    into a reply, a route's answer or an occasion.

The nightly job

R5. The job `vault_pass` in `coordination::jobs::TABLE` (SPEC-027 R1) runs daily at 21 hours local
    time, at a minute the census admits (SPEC-027 R2), not `catch_up`, with trigger `nightly` for
    the study day it fires in. It reads the study day's sync outcome as every job does and runs
    whatever it is, because no vault duty reads the collection. Its unit is
    `deploy/systemd/deck-streak-job@vault_pass.timer`.

`/vaultops`

R6. `/vaultops` keeps its name. It is owner-only through SPEC-026 R4's gate. Any text after the
    command is ignored and never reaches the pass, whose only inputs are its trigger `on_demand` and
    the study day.
R7. The bot replies at once with the placeholder "⏳ Running vault-ops…", runs the pass off the update
    loop, and edits the placeholder once to exactly one line: "✅ vault-ops completed.", "⚠️ vault-ops
    timed out after 600s." or "⚠️ vault-ops failed unexpectedly." (the predecessor's lines, golden
    `vaultops_outcomes`), or DeckStreak's "⏳ vault-ops is already running." for `already_running`.
    A placeholder that has no message id is followed by a send instead of an edit. The placeholder
    and the edit join the census's command replies (ADR-113), so quiet hours and a lapse never hold
    them (#155).
R8. The predecessor's fourth line, an exit status with a pointer to its service log, is not ported:
    an in-process pass has no exit status, and the pointer names the predecessor's operations
    (ADR-059). Every failure reads the failed line.
R9. With the route absent, `/vaultops` runs the pass: the daily note needs no model, and the
    curator and the synthesis record `ai_route_absent` (ADR-113).

The latest pass

R10. `GET /api/vault/passes/latest`, behind SPEC-024 R7's `OwnerSession`, answers the newest
    `vault_passes` row by start instant: its trigger, outcome, study day, and start and finish
    instants (finish `null` while running), or 204 when there is none. This is the last status #54
    asks for; the Mini App's admin action that shows it is W7's (#57).

The table

R11. The coordination context owns `vault_passes` (`migrations/011701_coordination_vault_passes.sql`,
    `STRICT`, `created_at`, following SPEC-020 R15 and R18): id, trigger (`nightly` or `on_demand`),
    study day, outcome (`running`, `completed`, `timed_out`, `failed` or `abandoned`), start and
    finish instants, and the partial unique index of R2. Its six files are listed in §4. The rows
    are the owner's activity: an export lists them, and an erase deletes them. The predecessor kept
    no table for its passes, so W8's import maps nothing into it.
R12. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no
    dishonest copy (R7's lines say what happened, and R8 drops a line that pointed elsewhere) and
    no unbounded work (R2, R3).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a pass runs the curator, the daily note and (on a Sunday) the synthesis in order, and past a discarded duty | `the_pass_runs_its_duties_in_order_and_past_a_discard` |
| A2 | a second pass while one runs is refused with `already_running` and runs nothing | `a_second_pass_is_refused_while_one_runs` |
| A3 | a `running` row started 661 seconds earlier is abandoned at the next claim, and one started 660 seconds earlier is not | `a_stale_running_pass_is_abandoned_at_the_next_claim` |
| A4 | on a paused clock, a pass whose scripted runner never answers records `timed_out` at 600 seconds | `a_pass_past_its_bound_records_timed_out` |
| A5 | an apply of a staged run in progress at the bound finishes, and the vault holds the whole run | `an_apply_in_progress_finishes_past_the_bound` |
| A6 | a pass whose vault is unreadable records `failed`, and its error reaches the log only | `a_failing_pass_records_failed_and_logs_its_error` |
| A7 | the claim deletes a pass that finished 91 days earlier and keeps one that finished 90 days earlier | `the_claim_drops_passes_older_than_ninety_days` |
| A8 | the pass's bound equals the predecessor's golden | `the_vault_pass_bound_is_the_predecessors` |
| A9 | the job table holds `vault_pass` at 21 hours, off every reserved minute, not `catch_up` | `the_job_table_holds_the_vault_pass` |
| A10 | with the route absent a pass completes and writes the daily note | `a_pass_with_no_ai_route_completes` |
| A11 | `/vaultops` replies with the placeholder and edits it once to each outcome's line, equal to the golden's | `vaultops_edits_its_placeholder_to_the_outcome` |
| A12 | `/vaultops` from anyone but the owner runs nothing | `vaultops_from_anyone_but_the_owner_runs_nothing` |
| A13 | text after `/vaultops` never reaches the pass | `vaultops_passes_no_text_to_the_pass` |
| A14 | `/vaultops` while a pass runs answers that one is running | `vaultops_while_a_pass_runs_says_so` |
| A15 | a failed pass's edit is exactly the failed line, with no text of the error | `a_failed_vaultops_reply_carries_no_diagnostic` |
| A16 | `/vaultops` inside quiet hours is answered | `vaultops_answers_inside_quiet_hours` |
| A17 | a placeholder with no message id is followed by one send of the outcome | `vaultops_sends_when_the_placeholder_has_no_id` |
| A18 | the latest pass is answered, newest by start, with its finish null while running | `the_latest_pass_is_answered` |
| A19 | with no pass the route answers 204, and without the owner's session 401 | `the_latest_pass_route_is_owner_only_and_empty_is_204` |
| A20 | every `vault_passes` row is exported, and an erase leaves none | `vault_passes_export_and_erase_are_symmetric` |

```acceptance
A1: cargo test -p deck-streak-coordination --test vault_pass -- --exact the_pass_runs_its_duties_in_order_and_past_a_discard
A2: cargo test -p deck-streak-coordination --test vault_pass -- --exact a_second_pass_is_refused_while_one_runs
A3: cargo test -p deck-streak-coordination --test vault_pass -- --exact a_stale_running_pass_is_abandoned_at_the_next_claim
A4: cargo test -p deck-streak-coordination --test vault_pass -- --exact a_pass_past_its_bound_records_timed_out
A5: cargo test -p deck-streak-coordination --test vault_pass -- --exact an_apply_in_progress_finishes_past_the_bound
A6: cargo test -p deck-streak-coordination --test vault_pass -- --exact a_failing_pass_records_failed_and_logs_its_error
A7: cargo test -p deck-streak-coordination --test vault_pass -- --exact the_claim_drops_passes_older_than_ninety_days
A8: cargo test -p deck-streak-coordination --test vault_pass -- --exact the_vault_pass_bound_is_the_predecessors
A9: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_the_vault_pass
A10: cargo test -p deck-streak-coordination --test vault_pass -- --exact a_pass_with_no_ai_route_completes
A11: cargo test -p deck-streak-bot --test vaultops_command -- --exact vaultops_edits_its_placeholder_to_the_outcome
A12: cargo test -p deck-streak-bot --test vaultops_command -- --exact vaultops_from_anyone_but_the_owner_runs_nothing
A13: cargo test -p deck-streak-bot --test vaultops_command -- --exact vaultops_passes_no_text_to_the_pass
A14: cargo test -p deck-streak-bot --test vaultops_command -- --exact vaultops_while_a_pass_runs_says_so
A15: cargo test -p deck-streak-bot --test vaultops_command -- --exact a_failed_vaultops_reply_carries_no_diagnostic
A16: cargo test -p deck-streak-bot --test vaultops_command -- --exact vaultops_answers_inside_quiet_hours
A17: cargo test -p deck-streak-bot --test vaultops_command -- --exact vaultops_sends_when_the_placeholder_has_no_id
A18: cargo test -p deck-streak-api --test vault_pass_route -- --exact the_latest_pass_is_answered
A19: cargo test -p deck-streak-api --test vault_pass_route -- --exact the_latest_pass_route_is_owner_only_and_empty_is_204
A20: cargo test -p deck-streak-coordination --test vault_pass -- --exact vault_passes_export_and_erase_are_symmetric
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. This delivery changes no pack's state: the vault-duties
pack is enforced by SPEC-116, and the rows below judge the files this SPEC adds.

| id | criterion | decided by |
|---|---|---|
| B1 | over `crates/bot/src/vaultops_command.rs`: every reply is escaped for the parse mode and stays within the message length | the telegram-platform pack |
| B2 | over the migrations under `migrations/`, this one among them: every table has `created_at` | the ledger-sqlite pack |
| B3 | over `privacy.json`, `PRIVACY.md` and `crates/coordination/src/data_rights.rs`: `vault_passes` is declared with purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/vault_pass.rs` | `deck-streak-coordination` | added: `run`, the claim, the bound, the outcome |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job `vault_pass` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/vault_pass.rs` | `deck-streak-coordination` | added: A1 to A8, A10, A20 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A9 |
| `migrations/011701_coordination_vault_passes.sql` | `deck-streak-coordination` | added: the table and its partial unique index |
| `crates/coordination/src/data_rights.rs` | `deck-streak-coordination` | changed: `vault_passes` exported and erased |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `vault_passes` registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `vault_passes` row |
| `docs/CONTEXT-MAP.md` | docs | changed: `vault_passes` in coordination's own tables |
| `privacy.json`, `PRIVACY.md` | repo | changed: `vault_passes` |
| `crates/bot/src/vaultops_command.rs` | `deck-streak-bot` | added: `/vaultops` |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command registered |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the module |
| `crates/bot/tests/vaultops_command.rs` | `deck-streak-bot` | added: A11 to A17 |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names the placeholder and its edit |
| `crates/api/src/vault_pass_route.rs` | `deck-streak-api` | added: the latest pass |
| `crates/api/src/router.rs`, `crates/api/src/lib.rs` | `deck-streak-api` | changed: the route and the module |
| `crates/api/tests/vault_pass_route.rs` | `deck-streak-api` | added: A18, A19 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the pass's ports in the `job` and `bot` roles |
| `deploy/systemd/deck-streak-job@vault_pass.timer` | deploy | added |
| `tools/parity-oracle/registry/spec_117.py` | repo | added: the two goldens of §7 |
| `tools/parity-oracle/goldens/vaultops.constants.json`, `tools/parity-oracle/goldens/vaultops_outcomes.json` | repo | added: generated by §7 |
| `scripts/mutation-rows.d/S11700-S11799.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-117-the-vault-pass-runs-nightly-and-on-vaultops-one-at-a-time-on-the-host-bounded-at-600-seconds.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w6-duty-run-and-its-degradation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-117.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It runs no external program and needs no privilege rule; the issue's exact-command shape is
  replaced by the in-process pass (ADR-117, #155).
- It builds no Mini App admin action; the settings screen that shows the last status is W7's (#57).
- It changes nothing in the predecessor's own nightly, which keeps running until the cutover's first
  verified week (#62).
- It builds none of the duties themselves (#49, #50).

## 6. Risks

- **Two passes at once.** Refused by R2's index on the host, not only in a process; detected by A2
  and A14.
- **A crashed pass that blocks every later one.** R2 abandons a `running` row after 660 seconds;
  detected by A3.
- **A diagnostic reaching the chat.** R4 and R8; detected by A15.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_117.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `vaultops.constants` | `bot.py:_VAULT_OPS_TIMEOUT_SECS` | constants | none |
| `vaultops_outcomes` | `bot.py:CommandBot._run_vaultops`, `bot.py:CommandBot._deliver_vaultops` | adapter | a bot built with a synthetic token and chat id, whose subprocess call is replaced by a fake that completes, raises the bound's timeout, or raises `OSError`, and whose delivery is recorded, each case with a message id and without one |

The exit-status case is left out on purpose (R8).

## 8. Tables and the v9 import

`vault_passes` (coordination, `migrations/011701_coordination_vault_passes.sql`). The predecessor
kept no table for its passes, so W8's import maps nothing into it and it starts empty.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11701-ORDER` | `crates/coordination/src/vault_pass.rs` | the curator runs before the daily note | `vault_pass::the_pass_runs_its_duties_in_order_and_past_a_discard` |
| `S11702-CONTINUE` | `crates/coordination/src/vault_pass.rs` | a discard does not stop the pass | `vault_pass::the_pass_runs_its_duties_in_order_and_past_a_discard` |
| `S11703-ONE-RUNNING` | `migrations/011701_coordination_vault_passes.sql` | the partial unique index over `running` (a script-mutation row) | `vault_pass::a_second_pass_is_refused_while_one_runs` |
| `S11704-ABANDON-AFTER` | `crates/coordination/src/vault_pass.rs` | more than 660 seconds; the test names 660 and 661 | `vault_pass::a_stale_running_pass_is_abandoned_at_the_next_claim` |
| `S11705-BOUND` | `crates/coordination/src/vault_pass.rs` | 600 seconds; the test runs on a paused clock, so a longer wait costs no real time and fails its instant | `vault_pass::a_pass_past_its_bound_records_timed_out` |
| `S11706-APPLY-UNCANCELLED` | `crates/coordination/src/vault_pass.rs` | an apply in progress is shielded from the bound | `vault_pass::an_apply_in_progress_finishes_past_the_bound` |
| `S11707-RETENTION` | `crates/coordination/src/vault_pass.rs` | more than 90 days; the test names 90 and 91 | `vault_pass::the_claim_drops_passes_older_than_ninety_days` |
| `S11708-OWNER-ONLY` | `crates/bot/src/vaultops_command.rs` | the owner gate before the pass | `vaultops_command::vaultops_from_anyone_but_the_owner_runs_nothing` |
| `S11709-NO-TEXT` | `crates/bot/src/vaultops_command.rs` | the command's text is dropped | `vaultops_command::vaultops_passes_no_text_to_the_pass` |
| `S11710-CONTENT-FREE` | `crates/bot/src/vaultops_command.rs` | a failure edits to the failed line only | `vaultops_command::a_failed_vaultops_reply_carries_no_diagnostic` |
| `S11711-REPLY-NOT-OCCASION` | `crates/bot/src/vaultops_command.rs` | the reply is a command reply, never a router occasion | `vaultops_command::vaultops_answers_inside_quiet_hours` |
| `S11712-LATEST` | `crates/api/src/vault_pass_route.rs` | newest by start | `vault_pass_route::the_latest_pass_is_answered` |
