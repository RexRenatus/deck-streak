# SPEC-143: the cutover checklist moves each contract one at a time, and no contract has two writers at any step

- **Wave:** W8. **Issue:** #62 (epic #9), both criteria: every item has a command and its recorded
  output, and no contract has two writers at any step. **Context(s):** `deck-streak-coordination`
  (the checklist, the ledger `cutover_steps`, the steps' use cases, the verification and the job
  runner's gate), `deck-streak-notifications` (the kinds' switches, opened and closed through a
  port), `deck-streak-readings` (the archive switch's port), `deck-streak-vault` (the staged
  executor refuses a duty that has not moved), `deck-streak-daemon` (the `cutover` role) and
  `deploy` (the runbook).
- **Decided by:** ADR-011 (side by side: one writer per vault contract, each contract and kind
  moved one at a time with a verification run), ADR-027 (the job table and its ledger), ADR-065
  (every other writer is fenced from the readings folder before DeckStreak writes it) and ADR-143
  (the checklist is a ledger of steps: the go, the private rail's stop, DeckStreak's switch and a
  recorded verification, one item in flight).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-027, SPEC-041, SPEC-042 and SPEC-043 (landed);
  SPEC-049, SPEC-053, SPEC-100, SPEC-101, SPEC-102, SPEC-105, SPEC-110, SPEC-111 and SPEC-116
  (planned, unlanded: each adds a kind, a duty's writer or a job this checklist gates); SPEC-142
  (this wave, unlanded: the `import` role beside which the `cutover` role is added).
  **Mutation band:** `S14300-S14399`. SPEC-146 (this wave) builds after this SPEC and adds its own
  item.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-143.md` (ADR-016).

## 1. The problem, measured

- **The contracts are switched off one by one, and switched on by nobody.** At `dev` 703097b the
  router withholds a kind whose setting reads `"0"` with `nudges_disabled` (SPEC-041 R4), and the
  planned SPECs seed the kinds the predecessor's code also raises as `"0"` until each moves:
  `comeback_enabled` (SPEC-049 R7), `morning_enabled`, `evening_enabled`, `last_chance_enabled`,
  `habit_enabled`, `focus_enabled`, `quest_offer_enabled` and `chests_vaulted_enabled` (SPEC-100
  R27), `digest_enabled` and `weekly_enabled` (SPEC-101 R28), and `widget_enabled` (SPEC-102 R23).
  The readings archive is off until the owner's go (SPEC-053 R8). No SPEC names the command that
  turns any of them on, or what proves it went on alone.
- **Four kinds are not seeded off.** `notifications-policy.json` names nine kinds at `dev`, and the
  planned SPECs add eight (SPEC-100 R8, SPEC-101 R26, SPEC-102 R21, SPEC-105 R2, SPEC-111 R10 and
  SPEC-116 R6). `celebrations_enabled` (the predecessor's `CelebrationsLayer.celebrate` gates every
  celebration on its own switch, SPEC-102 §1), `discipline_notices_enabled`, `drill_ready_enabled`
  and `inbox_filed_enabled` are seeded by no migration, so each speaks as soon as its SPEC lands.
- **Three writers have no switch.** The vault's staged duty runs (SPEC-042 R4) apply any duty the
  layout names (`crates/vault/data/layout.json`: `daily-note`, `weekly-synthesis`,
  `inbox-curator`, `daily-reading` and `drill-coach`); the job `drill_postback` pays graded drills
  hourly (SPEC-110 R9); and the stats bridge (#153, SPEC-146) replaces its file nightly. Each is a
  contract the predecessor's code also writes (`vault_bridge.py:poll_drill_postbacks`,
  `vault_bridge.py:write_vault_stats`), and each would write the moment its timer runs.
- **Nothing records a move.** ADR-011's confirmation is "the W8 cutover SPEC's checklist, each item
  a command with its output; the owner's recorded go". No table, command or file holds either.

## 2. Requirements

The checklist (#62)

R1. The checklist `crates/coordination/data/cutover.json` (schema `deckstreak.cutover.v1`, read
    with `include_str!`) lists every item in the order it moves. An item has an `id`
    (`[a-z0-9-]{1,40}`), a `side` (`moves` or `deckstreak_only`), for `deckstreak_only` a `reason`
    naming its ADR, for `moves` the `source` (`module.py:function` of the predecessor's code that
    writes the same contract) and one or more `gates`. A gate is one of `kind:<policy kind>` (the
    kind's switch), `archive:readings_vault_archive` (SPEC-053 R8's switch), `duty:<layout duty>`
    (a staged duty) or `job:<job id>` (a job of `coordination::jobs::TABLE`). A top-level
    `jobs_outside` names every job of the table that writes no contract the predecessor's code also
    writes, each with its reason.
R2. The planned items, confirmed against the populations R3 reads at the build's head:
    - `moves`, one item per kind: `celebration`, `digest`, `morning`, `streak_risk`,
      `last_chance`, `habit`, `comeback`, `focus`, `quest_offer`, `chests_vaulted`, `weekly`,
      `widget`, `discipline`, `drill_ready` and `inbox_filed` (15);
    - `moves`, one item per vault contract: `readings-folder` (`archive:readings_vault_archive`,
      `duty:daily-reading`; its `stopped` is the rail's record that ADR-065's fences are
      proved), `drills` (`duty:drill-coach`, `job:drill_postback`), `inbox`
      (`duty:inbox-curator`), `daily-note` (`duty:daily-note`) and `weekly-synthesis`
      (`duty:weekly-synthesis`) (5). SPEC-146 adds a sixth, `vault-stats` (`job:vault_stats`),
      in the delivery that adds the job, so the job is gated from its first commit;
    - `deckstreak_only`: `reading_ready` (ADR-041: DeckStreak's own kind) and `alert` (ADR-050:
      DeckStreak's pages about its own health) (2).
    A kind whose predecessor source the builder cannot name stays `moves` (the conservative answer:
    it waits for the go).
R3. The census holds at every head, as tests:
    - the checklist's `kind:` gates and `deckstreak_only` kind items together equal the kinds of
      `notifications-policy.json`, as sets, each kind once;
    - the `duty:` gates equal the duties of `crates/vault/data/layout.json`, as sets, each once;
    - every job of `coordination::jobs::TABLE` is in exactly one `job:` gate or in `jobs_outside`,
      and every `job:` gate names a job of the table;
    - every `kind:` gate of a `moves` item names a switch that some migration seeds `"0"` with
      `INSERT OR IGNORE`.
R4. The migration `migrations/014302_notifications_side_by_side_defaults.sql` seeds
    `celebrations_enabled`, `discipline_notices_enabled`, `drill_ready_enabled` and
    `inbox_filed_enabled` as `"0"` with `INSERT OR IGNORE` into `notification_settings`, so a value
    the owner set is kept and a kind that has not moved stays silent.

The ledger and the steps (#62, gated on #164)

R5. The table `cutover_steps` (`migrations/014301_coordination_cutover_steps.sql`, STRICT,
    append-only): `id`, `item` (an item id, or `*` for the go and SPEC-144's two steps), `step`
    (`go`, `stopped`, `switched`, `verified`, `reverted`, and SPEC-144's `retired` and `alone`),
    `reference` (for `go`, `stopped` and `retired`: at most 64 characters of
    `[A-Za-z0-9#:._/-]`), `command` (for `verified` and `alone`: the command line run),
    `output_sha256` (for `verified` and `alone`: 64 lowercase hex), `verdict` (for `verified` and
    `alone`: `pass`), `created_at`. CHECK constraints bind each column to its step. A trigger refuses UPDATE and
    DELETE.
R6. The steps, each a `deckstreakd cutover` command and one `Db::write` transaction that reads the
    ledger and appends one row, so two runs cannot interleave:
    - `go --reference <ref>` records the owner's go (#164). It is refused when a go is recorded.
    - `stopped <item> --reference <ref>` records that the private rail (#41) stopped the
      predecessor's writer of that contract. It is refused before the go, for an unknown or a
      `deckstreak_only` item, for an item already stopped and not reverted, and while another item
      is in flight (stopped or switched, not yet verified or reverted): one item at a time.
    - `switch <item>` opens every gate of the item and records `switched`. It is refused unless the
      item's last step is `stopped`. A `kind:` gate is opened through notifications'
      `switches::open`, an `archive:` gate through readings' `archive_switch::open`; a `duty:` or
      `job:` gate is open once the row is recorded.
    - `verify <item> --output <file>` runs R9's verification, writes its output to `<file>` (mode
      0600, created new, refused if it exists) and, on `pass`, records `verified` with the command
      line and the output's sha256. `pending` and `fail` record nothing and exit 3 and 1.
    - `revert <item>` closes every gate of an item whose last step is `switched` or `verified`,
      then records `reverted`, so the rail restarts the predecessor's writer only after `status`
      reads the item reverted. A verified item reverts only on the owner's decision (#164), after a
      failed or void day (SPEC-144 R5).
    - `status` prints one line per item, `item=<id> side=<side> step=<step>`, then R8's drift
      lines, and ends `CUTOVER STATUS OK` or `CUTOVER STATUS DRIFT: <n>` (exit 1).
R7. Every refusal names its reason and writes nothing: `no_go` (a stop before the go),
    `go_recorded` (a second go), `unknown_item` (an item the checklist does not list),
    `not_movable` (a stop of a `deckstreak_only` item or of a verified one), `in_flight` (a stop
    while an item, itself included, is stopped or switched), `not_stopped` (a switch whose item's
    last step is not `stopped`), `not_switched` (a verify whose item's last step is not `switched`,
    or a revert whose item's last step is neither `switched` nor `verified`) and `output_exists` (a
    verify whose output file exists). Exit codes: 0 done, 1 refused or failed, 2 usage, 3 pending.

The gates hold (#62's second criterion)

R8. **Drift.** A gate is open for an item whose last step is not `switched` or `verified` when a
    `kind:` or `archive:` switch reads other than `"0"` before its item switched. `status` and
    every `verify` report each as `drift item=<id> gate=<gate>`, and a drift fails every
    verification, so an owner's hand-set switch never passes as a move.
R9. **Verification.** `verify <item>` prints, in this order and with no row value:
    `item=<id> side=<side> step=<step>`; one `gate=<gate> state=<open|closed>` per gate; one
    `evidence=<gate> runs=<n> delivered=<n> withheld=<n> failed=<n>` per gate, counted since the
    item's `switched` row (a kind from the router's decisions, a duty from `agent_runs`, a job from
    `cron_fires`, the archive from the readings runs' outcomes); the drift lines; `verdict=<v>`; and
    `CUTOVER VERIFY <PASS|PENDING|FAIL>: <id>`. `pass` needs every gate open, no drift, at least
    one delivered or ok run per gate and no failed one; `pending` is no run yet; anything else is
    `fail`.
R10. **The job runner's gate.** `coordination::runner` returns before claiming a fire of a job
    named by a `job:` gate whose item's last step is not switched or verified, writing no
    `cron_fires` row and logging `job=<id> not_moved item=<item>`, and exits 0.
R11. **The duty's gate.** `deck_streak_vault::staged::Executor::new` takes an `InForce` (the duties
    whose items' last step is switched or verified, from `coordination::cutover::in_force`), and
    `apply` refuses a run of a duty outside it with `RunRefusal::NotMoved`, leaving the vault
    untouched, before the gate runs. A test scans every non-test Rust source for `Executor::new(` and requires
    `cutover::in_force` in the same call.
R12. **The switches' ports.** `deck_streak_notifications::switches::{open, close, is_open}` write
    and read `notification_settings` by key (`"1"` and `"0"`), refusing a key no policy kind names;
    `deck_streak_readings::archive_switch::{open, close, is_open}` do the same for SPEC-053 R8's
    switch. Each has a test in its own package.

The role and the runbook

R13. `crates/daemon/src/role_cutover.rs` runs the steps; `Role::Cutover` joins `crates/daemon/src/
    main.rs`, whose `NAMES` gains `cutover` and whose usage line names its commands.
R14. The runbook `deploy/cutover.md` lists the items in R1's order, and for each the four commands
    in order (the rail's `stopped`, `switch`, `verify --output`, and `revert` under `## Revert`),
    with the output's shape from R9. It says that every output file stays in the maintainer's
    private record and that only its sha256 enters the ledger; that every step after the go names
    #164 as its gate; and that the import (SPEC-142) and the day alone (SPEC-144) follow the last
    item.
R15. The eleven anti-goals hold (CHARTER): the checklist grants no XP, sends no message of its own
    and deletes no vault file; the steps are run by hand, never by a timer.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the checklist's kind items equal the policy's kinds as sets, each once, and one kind added to a copy of the policy without an item fails naming it | `every_policy_kind_has_one_item` |
| A2 | the checklist's duty gates equal the layout's duties as sets, each once | `every_layout_duty_has_one_gate` |
| A3 | every job of the table is in exactly one job gate or in `jobs_outside`, and every job gate names a job of the table | `every_job_is_gated_or_outside` |
| A4 | every kind gate of a moving item names a switch some migration seeds `"0"` | `every_moving_kind_is_seeded_off` |
| A5 | a fresh database reads each of R4's four switches as `"0"`, and a value set before the migration is kept | `the_four_switches_are_seeded_off` |
| A6 | a stop recorded before the go is refused with `no_go` and writes nothing | `a_stop_before_the_go_is_refused` |
| A7 | a switch of an item whose last step is not `stopped` is refused with `not_stopped`, and its gates stay closed | `a_switch_before_its_stop_is_refused` |
| A8 | a stop of a second item while one is stopped or switched and not verified is refused with `in_flight` | `one_item_moves_at_a_time` |
| A9 | a revert closes every gate of the item before its row is recorded, and a gate that cannot be closed records nothing | `a_revert_closes_the_gates_before_it_records` |
| A10 | a kind switch set to `"1"` before its item switched reads as drift, and `status` exits 1 | `a_switch_open_before_its_move_is_drift` |
| A11 | a verify with no run since the switch is `pending`, exits 3 and records nothing | `a_verify_with_no_run_is_pending` |
| A12 | a passing verify writes its output with mode 0600, refuses an existing file with `output_exists`, and records the command line and the output's sha256 | `a_passing_verify_records_its_output_digest` |
| A13 | a verify's output over synthetic runs holds only R9's lines, and none of the synthetic rows' values | `the_verify_output_carries_no_row_value` |
| A14 | a job gated by an item that has not switched, or whose last step is `reverted`, writes no fire row and exits 0 | `a_job_that_has_not_moved_does_not_run` |
| A15 | a staged run of a duty outside the in-force set, a reverted item's duty included, is refused with `NotMoved` and leaves the vault untouched | `a_run_of_a_duty_not_moved_is_refused` |
| A16 | every non-test call of `Executor::new` passes `cutover::in_force` | `every_executor_takes_the_in_force_duties` |
| A17 | the switches port opens and closes a policy kind's switch and refuses a key no kind names | `a_switch_opens_and_closes_only_a_policy_kind` |
| A18 | the archive switch port opens and closes the readings archive switch | `the_archive_switch_opens_and_closes` |
| A19 | the ledger refuses an update and a delete, and a row whose columns do not fit its step | `the_ledger_is_append_only_and_shaped_by_step` |
| A20 | only the name `cutover` runs the cutover role, and it refuses arguments it does not take with the usage code | `only_the_name_cutover_runs_the_cutover_role` |
| A21 | a second go is refused with `go_recorded`, a step on an item the checklist does not list with `unknown_item`, a stop of a `deckstreak_only` item or of a verified one with `not_movable`, a verify of an item not switched and a revert of a stopped one with `not_switched`, each writing nothing, and a revert of a verified item closes its gates and records `reverted` | `each_out_of_order_step_names_its_reason` |

```acceptance
A1: cargo test -p deck-streak-coordination --test cutover_census -- --exact every_policy_kind_has_one_item
A2: cargo test -p deck-streak-coordination --test cutover_census -- --exact every_layout_duty_has_one_gate
A3: cargo test -p deck-streak-coordination --test cutover_census -- --exact every_job_is_gated_or_outside
A4: cargo test -p deck-streak-coordination --test cutover_census -- --exact every_moving_kind_is_seeded_off
A5: cargo test -p deck-streak-notifications --test switches -- --exact the_four_switches_are_seeded_off
A6: cargo test -p deck-streak-coordination --test cutover -- --exact a_stop_before_the_go_is_refused
A7: cargo test -p deck-streak-coordination --test cutover -- --exact a_switch_before_its_stop_is_refused
A8: cargo test -p deck-streak-coordination --test cutover -- --exact one_item_moves_at_a_time
A9: cargo test -p deck-streak-coordination --test cutover -- --exact a_revert_closes_the_gates_before_it_records
A10: cargo test -p deck-streak-coordination --test cutover -- --exact a_switch_open_before_its_move_is_drift
A11: cargo test -p deck-streak-coordination --test cutover_verify -- --exact a_verify_with_no_run_is_pending
A12: cargo test -p deck-streak-daemon --test cutover -- --exact a_passing_verify_records_its_output_digest
A13: cargo test -p deck-streak-coordination --test cutover_verify -- --exact the_verify_output_carries_no_row_value
A14: cargo test -p deck-streak-coordination --test runner -- --exact a_job_that_has_not_moved_does_not_run
A15: cargo test -p deck-streak-vault --test in_force -- --exact a_run_of_a_duty_not_moved_is_refused
A16: cargo test -p deck-streak-coordination --test cutover_census -- --exact every_executor_takes_the_in_force_duties
A17: cargo test -p deck-streak-notifications --test switches -- --exact a_switch_opens_and_closes_only_a_policy_kind
A18: cargo test -p deck-streak-readings --test archive_switch -- --exact the_archive_switch_opens_and_closes
A19: cargo test -p deck-streak-coordination --test cutover -- --exact the_ledger_is_append_only_and_shaped_by_step
A20: cargo test -p deck-streak-daemon --test roles -- --exact only_the_name_cutover_runs_the_cutover_role
A21: cargo test -p deck-streak-coordination --test cutover -- --exact each_out_of_order_step_names_its_reason
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/data/cutover.json` | `deck-streak-coordination` | added: the checklist (R1, R2) |
| `crates/coordination/src/cutover.rs` | `deck-streak-coordination` | added: the checklist's reader, the steps, drift, the verification and `in_force` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the `cutover` module |
| `crates/coordination/src/runner.rs` | `deck-streak-coordination` | changed: the not-moved gate (R10) |
| `crates/coordination/tests/cutover.rs` | `deck-streak-coordination` | added: A6-A10, A19, A21 |
| `crates/coordination/tests/cutover_census.rs` | `deck-streak-coordination` | added: A1-A4, A16 |
| `crates/coordination/tests/cutover_verify.rs` | `deck-streak-coordination` | added: A11, A13 |
| `crates/coordination/tests/runner.rs` | `deck-streak-coordination` | changed: A14 |
| `migrations/014301_coordination_cutover_steps.sql` | `deck-streak-coordination` | added: `cutover_steps` and its append-only triggers |
| `migrations/014302_notifications_side_by_side_defaults.sql` | `deck-streak-notifications` | added: R4's four switches seeded `"0"` |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row `cutover_steps`, exempt |
| `crates/coordination/src/data_rights.rs` | `deck-streak-coordination` | changed: `cutover_steps` exempt from export and erase |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: coordination's port names `cutover_steps` |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `cutover_steps` row |
| `privacy.json` | repo | changed: `cutover_steps` among coordination's exempt stores |
| `PRIVACY.md` | docs | changed: one line for the cutover record |
| `crates/notifications/src/switches.rs` | `deck-streak-notifications` | added: `open`, `close`, `is_open` (R12) |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the `switches` module |
| `crates/notifications/tests/switches.rs` | `deck-streak-notifications` | added: A5, A17 |
| `crates/readings/src/archive_switch.rs` | `deck-streak-readings` | added: `open`, `close`, `is_open` (R12) |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed: the `archive_switch` module |
| `crates/readings/tests/archive_switch.rs` | `deck-streak-readings` | added: A18 |
| `crates/vault/src/staged.rs` | `deck-streak-vault` | changed: `InForce` and `RunRefusal::NotMoved` (R11) |
| `crates/vault/tests/in_force.rs` | `deck-streak-vault` | added: A15 |
| `crates/daemon/src/role_cutover.rs` | `deck-streak-daemon` | added: the `cutover` role |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | changed: `Role::Cutover`, `NAMES` and the usage line |
| `crates/daemon/tests/cutover.rs` | `deck-streak-daemon` | added: A12 |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A20 |
| `deploy/cutover.md` | deploy | added: the runbook (R14) |
| `.sqlx/` | repo | changed: the refreshed query cache |
| `scripts/mutation-rows.d/S14300-S14399.json` | repo | added: §9's rows |
| `docs/schematics/cutover-checklist-and-sequence.md` | docs | added by the W8 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-143.md` | docs | added: the red-first record |
| `docs/specs/SPEC-143-the-cutover-checklist-moves-each-contract-one-at-a-time-and-no-contract-has-two-writers-at-any-step.md` | docs | moved from `docs/specs/planned/` |
| `changelog.d/feat-cutover-checklist.md` | repo | added: the fragment |

## 5. What this does NOT do

- It stops, disables and removes nothing of the predecessor: the private rail carries out every
  stop on the owner's go and records it with `stopped` (#41, #164).
- It imports no row: the import follows the last item (#61).
- It judges no day of DeckStreak alone and retires nothing (#63).
- It builds none of the writers it gates: each is its own SPEC's (#136, #49, #50, #153).
- It moves no inbox capture: a capture lands in the inbox, which the owner's devices also write,
  and the curator that moves it is the `inbox` item (#154, #50).
- It changes no repository setting and tags no release (#64).

## 6. Risks

- **Two writers of one contract**, the predecessor's and DeckStreak's. Prevented by R6's order
  (the rail's stop before the switch) and one item in flight, and detected by A7, A8 and rows
  S14302 and S14303.
- **A switch the owner set by hand**, which would open a gate before its move. Detected by R8's
  drift in `status` and every verification, and by A10 and row S14305.
- **A kind that speaks before its move** because its SPEC seeded no switch. Prevented by R3's
  census and R4's seeds, and detected by A1, A4, A5 and row S14309. SPEC-102, SPEC-105, SPEC-111
  and SPEC-116 may land before this SPEC; a kind of theirs speaks from its landing to this
  delivery, so each should seed its switch itself (an owner question in the plan).
  Until SPEC-143 lands, each builder of SPEC-102 (celebrations), SPEC-105, SPEC-111 and SPEC-116 seeds
  its own switch "0" with `INSERT OR IGNORE` (ADR-011) and a criterion, the jobs and duties of SPEC-110,
  SPEC-111 and SPEC-116 stay off the box, and no release is cut from dev while any of them is on dev
  without SPEC-143.
- **A move recorded without proof.** Prevented by R9's `pass` rule and R6's refusal to record a
  pending or failed run, and detected by A11, A12 and rows S14306 and S14308.
- **A personal value in a public record.** The output stays in the private record and only its
  digest enters the ledger; detected by A13 and row S14310.
- **A revert that leaves both writing.** Prevented by R6's gates closed before the row, and
  detected by A9 and row S14304.

## 7. Parity goldens

None. The checklist decides who writes, not what is written; each writer's own SPEC holds its
goldens.

## 8. Tables and the v9 import

| table | owner | the predecessor's table | import rule |
|---|---|---|---|
| `cutover_steps` | `coordination` | none | empty: the checklist's record starts with DeckStreak's own go |

`cutover_steps` is exempt from export and erase, as `cron_fires` is (SPEC-027): an erase must never
reopen a moved contract's gate or re-admit a second writer. It holds item ids, references and
digests, never a personal value.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S14301-GO-FIRST` | `crates/coordination/src/cutover.rs` | a stop before the go is refused | `cutover::a_stop_before_the_go_is_refused` |
| `S14302-STOP-FIRST` | `crates/coordination/src/cutover.rs` | a switch needs the rail's stop | `cutover::a_switch_before_its_stop_is_refused` |
| `S14303-ONE-IN-FLIGHT` | `crates/coordination/src/cutover.rs` | one item moves at a time | `cutover::one_item_moves_at_a_time` |
| `S14304-REVERT-ORDER` | `crates/coordination/src/cutover.rs` | the gates close before the revert is recorded | `cutover::a_revert_closes_the_gates_before_it_records` |
| `S14305-DRIFT` | `crates/coordination/src/cutover.rs` | an open gate before its move is drift | `cutover::a_switch_open_before_its_move_is_drift` |
| `S14306-PENDING` | `crates/coordination/src/cutover.rs` | no run is pending, never pass | `cutover_verify::a_verify_with_no_run_is_pending` |
| `S14307-RUNNER-GATE` | `crates/coordination/src/runner.rs` | a job not moved does not run | `runner::a_job_that_has_not_moved_does_not_run` |
| `S14308-DIGEST` | `crates/daemon/src/role_cutover.rs` | the recorded digest is the output's | `cutover::a_passing_verify_records_its_output_digest` |
| `S14309-SEED` | `migrations/014302_notifications_side_by_side_defaults.sql` | the four switches start at `"0"` (a script-mutation row whose cargo killer is in `deck-streak-notifications`) | `switches::the_four_switches_are_seeded_off` |
| `S14310-NO-VALUE` | `crates/coordination/src/cutover.rs` | the output carries counts only | `cutover_verify::the_verify_output_carries_no_row_value` |
| `S14311-DUTY-GATE` | `crates/vault/src/staged.rs` | a duty not moved is refused | `in_force::a_run_of_a_duty_not_moved_is_refused` |
| `S14312-KIND-KEY` | `crates/notifications/src/switches.rs` | only a policy kind's switch is written | `switches::a_switch_opens_and_closes_only_a_policy_kind` |

No killer calls a network: every test runs against a temporary database, a temporary vault and a
stub staged gate.
