# SPEC-434: a transition table per product state enum, and four graph lints over it, start at warn

Path:
`docs/specs/SPEC-434-a-transition-table-per-product-state-enum-and-four-graph-lints-over-it-start-at-warn.md`.

- **Issue:** none is closed. The pull request refers to #883, whose scope it discharges, and lands
  on dev only after the release pull request #822 and after SPEC-433.
- **Context(s):** the script tests (`scripts/tests/`), one register under `config/`, the mutation
  band and the documents. No crate, migration, formal entry or workflow changes.
- **Decided by:** ADR-448 (each product state enum's transition table is data in one register beside
  the state-machine lints, and each graph lint is its own test). No SPEC or ADR is amended.
- **Schematic:** `docs/schematics/state-machine-transition-tables.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-434.md`. Every rule it adds is at warn.

## 1. The problem, measured

SPEC-433 lists the workspace's product state enums, and nothing declares the moves any of them
admits, so no check can say a state is reached, a state is not a dead end, a terminal is never left,
or one event moves one state one way. Every figure below was read at dev `b25ed04a` (`D` below). `X`
stands for the four exclude pathspecs `':(exclude,icase)*drill*' ':(exclude,icase)*readings_tree*'
':(exclude,icase)*_standin_checks*' ':(exclude,icase)*test_deploy_standin*'`, each beside the
positive `.`. The tests read the whole tree, so each examined figure they print is at least the
figure here.

| fact | at `b25ed04a` | read by |
|---|---|---|
| `.rs` files under `crates/*/src` | 340 | `git ls-tree -r --name-only D crates/ \| grep -c -E '^crates/[^/]+/src/.*\.rs$'` |
| Rust enums named as states | 18: 12 under `crates/*/src`, 6 under `crates/*/tests` | `git grep -n -E '\benum\s+[A-Za-z0-9_]*(State\|Status\|Phase\|Stage\|Step\|Lifecycle\|Mode)\b' D -- . X`, its `.rs` lines |
| state enums deriving or implementing `Default` | 0 | `git grep -n -E 'impl\s+Default\s+for\s+[A-Za-z0-9_]*(State\|Status\|Phase\|Stage\|Step\|Lifecycle\|Mode)\b\|#\[default\]' D -- . X`: the two `#[default]` lines are in no state enum's file |
| product writers of the four stored state columns | 12 statements: `skip_days` 4, `chests` 5, `sync_runs` 2, `reading_topic_days` 1 | `git grep -n -E 'UPDATE +(sync_runs\|reading_topic_days\|chests\|skip_days)\b\|INTO +(sync_runs\|reading_topic_days\|chests\|skip_days)\b' D -- . X`, the `crates/*/src` lines |
| cover lines under `formal/`, and those naming a state enum's file | 251 in 40 files; 30 name one of six enum files | `git grep -n '@phx covers' D -- . X`, the lines under `formal/` |
| `DYNAMIC_IMPORTS` in `scripts/tests/test_ci_workflows.py` | declared at `:7172` | `git show D:scripts/tests/test_ci_workflows.py \| grep -n DYNAMIC_IMPORTS` |

The twelve product state enums split by one test: does any write read the state it leaves? Three
do, and their tables carry moves. Nine do not, and their tables carry none; `all` below means every
variant, each listed by name in the register, in `initial` and in `terminal`.

| enum | declared at | variants | initial | terminal | moves |
|---|---|---|---|---|---|
| `Phase` | `crates/coordination/src/recompute/mod.rs:56` | RollupAndScore, BaseXp, StreaksAndGovernor, DaySteps, DerivedBonuses, CoinMint, Awards | RollupAndScore | Awards | 6, each to the next of `PHASES` (`:74`), on `evaluate` |
| `ChestState` | `crates/quests/src/chest_store.rs:50` | Sealed, Vaulted, Opened, Resolved | Sealed, Vaulted | Resolved | 7 (below) |
| `SkipState` | `crates/ingest/src/skip.rs:232` | Pending, Applied, Failed | Pending | Applied, Failed | 2 (below) |
| `BandStep` | `crates/curriculum/src/progress.rs:240` | FirstSighting, BandUp, Unchanged | all | all | none: the outcome of one comparison of a stored band with a current one (`band_step`, `:256`) |
| `NotifyState` | `crates/daemon/src/lifecycle.rs:46` | Ready, Watchdog, Stopping | all | all | none: a datagram, never a held value; `WATCHDOG=1` can follow `STOPPING=1` while a role drains (`crates/daemon/src/role_api.rs:253`, before its heartbeat stops at `:260`) |
| `Step` | `crates/daemon/src/wiring.rs:605` | Rescore, Settings, Credentials, Scope, History, Sync, Obligations, GateProbe, GateRecord, WindowRead, WindowBase, Recompute | all | all | none: the label of the sync step whose failure refuses it, built when it fails (`Step::of`, `:635`) |
| `State` | `crates/identity/src/passkeys.rs:135` | Registration, SignIn | all | all | none: a ceremony's kind, fixed at its insert (`:418`, `:519`) |
| `AnchorState` | `crates/ingest/src/gate.rs:102` | Missing, Unreadable, Present | all | all | none: how one read found the gate's anchor (`crates/ingest/src/state.rs:133-165`) |
| `RunStatus` | `crates/ingest/src/sync_runs.rs:159` | Ok, Error, Skipped | all | all | none: inserted with its run's row and never updated (`:368`, `:432`) |
| `State` | `crates/insights/src/registry.rs:8` | Live, Inert | all | all | none: a row's setting in the source (`ROWS`, `:27-31`) |
| `TopicState` | `crates/readings/src/state.rs:257` | Ready, NoNewCards, CouldNotTell, Paused, Failed, AiRouteAbsent | all | all | none: a topic's outcome for a study day, overwritten by a later run that day whatever it held (`crates/readings/src/store.rs:396`) |
| `Step` | `crates/bot/src/poll.rs:228` | Done, Again, Stop | all | all | none: the outcome of one start request (`pause`, `:239`) |

The fifteen rows. Each `on` names the function whose call makes the move, by convention:
- `Phase`: RollupAndScore to BaseXp, BaseXp to StreaksAndGovernor, StreaksAndGovernor to DaySteps,
  DaySteps to DerivedBonuses, DerivedBonuses to CoinMint and CoinMint to Awards, each on `evaluate`
  (the fold keeps its steps in phase order at `mod.rs:487-493` and runs them at `:631`).
- `ChestState`: Sealed to Opened and Vaulted to Opened on `mark_opened` (`chest_store.rs:300`);
  Opened to Resolved on `mark_resolved` (`:316`) and on `settle_choice` (`:337`); Sealed, Vaulted
  and Opened to Resolved on `sweep_resolve` (`:355`). A chest is created Sealed or Vaulted
  (`crates/quests/src/chests.rs:135-138`, `:218`, `:245`) through `insert_chest` (`chest_store.rs:405`).
- `SkipState`: Pending to Applied on `settle_applied_on` (`skip.rs:563`), Pending to Failed on
  `settle_failed` (`:409`). `begin` inserts Pending (`:363`). `mark_undone_on` (`:588`) writes the
  `undone` column of an applied skip and leaves its state as it is, so it is no row.

The storage each machine can hold: `chests.state` is `CHECK (state IN ('sealed', 'vaulted',
'opened', 'resolved'))` (`migrations/008101_quests_chests_and_tokens.sql:20`) and `skip_days.state`
is `CHECK (state IN ('pending', 'applied', 'failed'))` (`migrations/008301_ingest_skip_days.sql:13`),
each its enum's variants; `reading_topic_days.state` holds TopicState's six
(`migrations/004501_readings_topic_days_and_runs.sql:51-52`) and `sync_runs.status` RunStatus's
three (`migrations/002201_ingest_sync_runs.sql:14`). No stored state lies outside its enum.

Two nearby shapes look like tables and are not adopted. `crates/bot/src/poll.rs:176-178` and
`:189-191` map each `Step` outcome to a loop action, so the start's requests are code positions and
`Step` is their event. `crates/identity/src/passkeys.rs:443` refuses a ceremony of the other kind
at a registration's finish, which keeps a kind fixed.

The two state columns SPEC-433 lists with no enum: `notification_queue.state` (held, sending,
abandoned; `migrations/004102_notifications_queue_claim.sql:16`) is moved by SQL literals in
`crates/notifications/src/ledger.rs`, one with a bound target (`:359`); `inbox_captures.state`
(captured, filed; `migrations/011801_vault_inbox_captures.sql:24`) has no `UPDATE` in product source.

Formal covers naming each enum's file: `Phase`'s file 10 (`formal/tla/MintReadsTheFinalBase` anchors
`Phase` and `PHASES`); `ChestState`'s 6 (`formal/tla/ChestOpenedOnce` covers all four of its
moving functions); `SkipState`'s 7 (`formal/tla/SkipDayOnce` covers `begin` and
`settle_applied_on`, and no cover names `settle_failed`); `BandStep`'s 3 (`formal/lean`'s
`RoadToC2` covers `band_step`); the wiring `Step`'s 2 and the passkeys `State`'s 2, none on the
enum; the other six files 0.

## 2. Requirements

- **R1. One register.** `config/fsm-tables.json` holds exactly `{"rules": {...}, "tables": [...]}`.
  `rules` has exactly five keys, `FSM-TABLE`, `FSM-REACH`, `FSM-DEAD-END`, `FSM-TERMINAL` and
  `FSM-DETERMINISTIC`, each exactly `{"level": ..., "listed": [...]}` with a `level` of `warn` or
  `fail`; every level is `warn` in this delivery. An unknown or missing key, rule or level refuses
  the register.
- **R2. A table.** Each entry of `tables` holds exactly `file`, `enum`, `initial`, `terminal` and
  `moves`, and `no_moves`, a non-empty reason, exactly when `moves` is empty. `initial` is a
  non-empty list of state names, `terminal` a list, and each row of `moves` exactly
  `{"from", "to", "on"}` with `on` a non-empty name. No two tables share a `file` and `enum`, and no
  two rows of a table are equal. Any other shape refuses the register.
- **R3. The ratchet.** Each `listed` entry is keyed as its rule's hit is keyed (R6, R8 to R11) and
  carries `ruling`, a path under `docs/rulings/` that exists in the tree. An entry without one, or
  naming no such file, refuses the register at every level. Deleting an entry owes nothing. A listed
  entry no hit matches reads `stale`.
- **R4. The product state enums.** A product file is a `.rs` file under `crates/*/src`, at any
  depth. Before reading it, the helper masks comments (line, block, nested and doc), string,
  raw-string, byte-string and character literals, and every item under `#[cfg(test)]`. A product
  state enum is an enum declared in that masked text whose name ends in `State`, `Status`, `Phase`,
  `Stage`, `Step`, `Lifecycle` or `Mode`; it is keyed by its file and its name.
- **R5. Its states.** An enum's states are the names of its body's top-level variants: a unit,
  tuple `(...)` or struct `{...}` variant, with or without `= <discriminant>`, after any outer
  attributes. A variant whose outer attributes include `#[cfg(test)]` is not a state.
- **R6. FSM-TABLE.** A product state enum no table names is a hit, keyed by its file and name. A
  table whose `file` is absent, whose `enum` is not declared in that file's masked text, or which
  names in `initial`, `terminal` or a row a state its enum lacks, is VOID.
- **R7. The walk.** A table's reached set is its `initial` states and every state a row leads to
  from a reached state, found breadth first.
- **R8. FSM-REACH.** It examines every state of every table. A state outside the reached set is a
  hit, keyed `<Enum>::<State>` with its file.
- **R9. FSM-DEAD-END.** It examines every reached state outside `terminal`. Such a state with no row
  whose `from` is it is a hit, keyed as R8's. A self-loop is a row.
- **R10. FSM-TERMINAL.** It examines every reached state in `terminal`. Such a state with a row
  whose `from` is it, a self-loop included, is a hit, keyed as R8's. A state outside `terminal` is
  never its hit.
- **R11. FSM-DETERMINISTIC.** It examines every distinct `(from, on)` pair of every table's rows. A
  pair whose rows name two or more `to` states is a hit, keyed `<Enum>::<State> on <event>` with its
  file.
- **R12. One defect, one rule.** No state is a hit of two of FSM-REACH, FSM-DEAD-END and
  FSM-TERMINAL.
- **R13. What each rule prints, and what fails.** Each reading prints one summary line,
  `FSM-LINT <rule> level=<level> examined=<n> <unit> hits=<n> new=<n> listed=<n> stale=<n>`, with
  the unit `state-enums`, `states`, `states`, `terminals` or `pairs` in R6, R8, R9, R10, R11's
  order, and one line per hit, `FSM-LINT <rule> <level> <new|listed|stale> <path>:<line> <key>`,
  where the line is the enum's declaration (R6, R11) or the state's variant (R8 to R10). A hit line
  carries no source text. At `warn` nothing a hit says fails. At `fail` a `new` or `stale` hit
  fails the test.
- **R14. VOID, at every level.** A reading fails as VOID when it examined nothing, when the tree's
  `crates/` or the register is absent, on R6's VOID, or, for FSM-TABLE, when its walk's product file
  count differs from an independent listing's.
- **R15. What does not change.** No product code, migration, formal entry, workflow or required check
  changes. No module imports another by path, so `scripts/tests/test_ci_workflows.py` is unchanged,
  and every name each new module uses is one its name placement test already places (ADR-447 D5).
- **R16. The tables at `b25ed04a`.** The register holds the twelve tables section 1 lists, with
  their fifteen rows, and every `listed` is empty.

## 3. Acceptance criteria of SPEC-434

Every test runs over a fixture tree it writes under a temporary directory, except A1, A5, A7, A9 and
A11, which read the repository's own tree. Each test is in the module its fence line names.

| id | criterion | decided by |
|---|---|---|
| A1 | the FSM-TABLE reading of the tree reads every product file an independent listing counts, every table the register names and at least one product state enum, and prints its summary line | `test_the_tree_reading_reads_every_product_file_and_every_table` |
| A2 | a planted enum of each of the seven suffixes with no table is a `new` hit, and a table for it clears the hit; an enum under `#[cfg(test)]`, in a comment, in a string, under `crates/*/tests` and one with no suffix are not read | `test_only_a_product_state_enum_with_no_table_is_a_hit` |
| A3 | a unit, tuple, struct, discriminant-carrying and attributed variant are each a state, and a `#[cfg(test)]` variant and a struct variant's fields are not | `test_every_variant_form_is_read_and_a_test_variant_is_not` |
| A4 | a table naming an absent file, an enum its file lacks, or a state its enum lacks in `initial`, `terminal` or a row is VOID | `test_a_table_naming_what_is_absent_is_void` |
| A5 | the FSM-REACH reading of the tree examines at least as many states as the register names, and prints its summary line | `test_the_tree_reading_reads_every_state_of_every_table` |
| A6 | over a chain, a cycle, a self-loop and two initial states, only an unreached state and a state reached only from an unreached one are hits | `test_only_a_state_no_walk_from_the_initial_set_reaches_is_a_hit` |
| A7 | the FSM-DEAD-END reading of the tree reads every table and prints its summary line | `test_the_tree_reading_reads_every_reached_state_outside_the_terminal_set` |
| A8 | a reached sink outside the terminal set is a hit; a terminal sink, an unreached sink and a state whose only row is a self-loop are not | `test_only_a_reached_sink_outside_the_terminal_set_is_a_hit` |
| A9 | the FSM-TERMINAL reading of the tree reads every table and prints its summary line | `test_the_tree_reading_reads_every_reached_terminal` |
| A10 | a reached terminal with an outgoing row and one with a self-loop are hits; a terminal sink, an unreached terminal with a row and a sink not declared terminal are not | `test_only_a_reached_terminal_with_an_outgoing_row_is_a_hit` |
| A11 | the FSM-DETERMINISTIC reading of the tree reads every row's state and event pair and prints its summary line | `test_the_tree_reading_reads_every_state_and_event_pair` |
| A12 | one state on one event to two states is a hit naming the state and the event; one state on two events and two states on one event are not | `test_only_one_event_moving_one_state_two_ways_is_a_hit` |
| A13 | each rule is VOID on a tree with nothing to examine, on a tree with no `crates/` and with no register, and FSM-TABLE is VOID when its walk's count differs from the independent listing's | `test_every_rule_is_void_with_nothing_to_examine` |
| A14 | a listed entry is refused without a `ruling`, with one naming no file and with one outside `docs/rulings/`, is admitted with one that exists, and reads `stale` when no hit matches it | `test_a_listed_entry_needs_a_ruling_that_exists_and_reads_stale_without_its_hit` |
| A15 | at `warn` a `new` and a `stale` hit print and pass; at `fail` each fails the test | `test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them` |
| A16 | a hit line carries its rule, level, kind, path, line and key, and no text of the line it names | `test_a_hit_line_names_its_site_and_never_its_source` |
| A17 | the register names exactly the five rules, each at `warn`, and refuses an unknown rule, level or key, a missing key, an empty initial set, a duplicate table, a duplicate row, a table with no moves and no `no_moves`, and one with moves and a `no_moves` | `test_the_register_names_five_rules_at_warn_and_refuses_the_malformed` |
| A18 | over every move relation on three states with one event, with every non-empty initial set and every terminal set (28,672 tables), and every move relation on two states with two events (256 tables), each graph rule's hits equal an oracle's that never calls the helper | `test_every_graph_rule_agrees_with_an_independent_oracle_over_every_small_table` |
| A19 | over the same population, no state is a hit of two of FSM-REACH, FSM-DEAD-END and FSM-TERMINAL | `test_no_state_is_a_hit_of_two_graph_rules` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_the_tree_reading_reads_every_product_file_and_every_table
A2: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_only_a_product_state_enum_with_no_table_is_a_hit
A3: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_every_variant_form_is_read_and_a_test_variant_is_not
A4: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_a_table_naming_what_is_absent_is_void
A5: python3 -m unittest discover -s scripts/tests -p test_fsm_reach.py -k test_the_tree_reading_reads_every_state_of_every_table
A6: python3 -m unittest discover -s scripts/tests -p test_fsm_reach.py -k test_only_a_state_no_walk_from_the_initial_set_reaches_is_a_hit
A7: python3 -m unittest discover -s scripts/tests -p test_fsm_dead_end.py -k test_the_tree_reading_reads_every_reached_state_outside_the_terminal_set
A8: python3 -m unittest discover -s scripts/tests -p test_fsm_dead_end.py -k test_only_a_reached_sink_outside_the_terminal_set_is_a_hit
A9: python3 -m unittest discover -s scripts/tests -p test_fsm_terminal.py -k test_the_tree_reading_reads_every_reached_terminal
A10: python3 -m unittest discover -s scripts/tests -p test_fsm_terminal.py -k test_only_a_reached_terminal_with_an_outgoing_row_is_a_hit
A11: python3 -m unittest discover -s scripts/tests -p test_fsm_deterministic.py -k test_the_tree_reading_reads_every_state_and_event_pair
A12: python3 -m unittest discover -s scripts/tests -p test_fsm_deterministic.py -k test_only_one_event_moving_one_state_two_ways_is_a_hit
A13: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_every_rule_is_void_with_nothing_to_examine
A14: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_a_listed_entry_needs_a_ruling_that_exists_and_reads_stale_without_its_hit
A15: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them
A16: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_a_hit_line_names_its_site_and_never_its_source
A17: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_the_register_names_five_rules_at_warn_and_refuses_the_malformed
A18: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_every_graph_rule_agrees_with_an_independent_oracle_over_every_small_table
A19: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_no_state_is_a_hit_of_two_graph_rules
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/_fsm_tables.py` | scripts | added: the masker, the enum and variant reader, the register reader, the walk, the five rules and the line printer (R1-R14) |
| `scripts/tests/test_fsm_table.py` | scripts | added: FSM-TABLE, A1-A4 |
| `scripts/tests/test_fsm_reach.py` | scripts | added: FSM-REACH, A5-A6 |
| `scripts/tests/test_fsm_dead_end.py` | scripts | added: FSM-DEAD-END, A7-A8 |
| `scripts/tests/test_fsm_terminal.py` | scripts | added: FSM-TERMINAL, A9-A10 |
| `scripts/tests/test_fsm_deterministic.py` | scripts | added: FSM-DETERMINISTIC, A11-A12 |
| `scripts/tests/test_fsm_table_lints.py` | scripts | added: the shared behaviour and the oracle, A13-A19 |
| `config/fsm-tables.json` | config | added: the five rules at `warn`, empty `listed`, and the twelve tables of section 1 (R1, R16) |
| `scripts/mutation-rows.d/S43400-S43499.json` | scripts | added: rows S43400-S43463 (section 9) |
| `docs/specs/SPEC-434-a-transition-table-per-product-state-enum-and-four-graph-lints-over-it-start-at-warn.md` | docs | added: this SPEC |
| `docs/decisions/ADR-448-each-product-state-enums-transition-table-is-data-in-one-register-beside-the-state-machine-lints-and-each-graph-lint-is-its-own-test.md` | docs | added: its decision |
| `docs/schematics/state-machine-transition-tables.md` | docs | added: the tables' shapes and the five rules' data flow |
| `docs/red-first/SPEC-434.md` | docs | added: the red-first record |
| `changelog.d/fsm-tables-434.md` | changelog | added: the fragment |

## 5. What this does NOT cover

- It adds no TypeScript exhaustive-switch rule and no Swift switch-without-default rule (#884).
- It does not read whether a row's `on` makes its move, whether one writer holds each state field,
  or a writer no row admits; a row names its function by convention only (#885).
- It adds none of the other later rules: a trigger per transition, terminal rows that never change,
  formal transition parity (a table against a model's moves, `settle_failed` left uncovered among
  them), journaled moves, mutation per transition arm, the client state list and contract (#885).
- It builds no product machine: no enum or table for `notification_queue.state` or
  `inbox_captures.state`, none for the bot start's requests, the passkeys ceremony's life or the
  role lifecycle that orders `NotifyState`'s datagrams (#885).
- It shares no lexer with the other Rust text guards; its masker serves these five rules (#886).
- It adds no reader that counts each rule's lines across merged pull requests and moves a rule to
  `fail`; the line it prints is that reader's input (#887).
- It reads no test enum and no enum the seven suffixes do not name, such as `RunOutcome`, `Outcome`
  and `Verdict` (#888).
- It changes none of SPEC-433's four rules or their register (#898).
- It cures no hit; none is listed at `b25ed04a`, and a later one is a question first (#889).

## 6. Risks

- **A sibling's new state enum or variant turns a reading.** At `warn` a new enum or an unplaced
  variant only prints. A renamed enum or variant that a table names is VOID, which fails the
  sibling's pull request until the table follows it. Detected by the summary line in each pull
  request's script-test log.
- **The masker misreads a Rust form it does not know** (a raw string with many `#`, a nested block
  comment, a lifetime beside a character literal, a variant attribute). A2 and A3 plant each form,
  and a misread surfaces as a `new` line naming a site no human finds, or as a VOID naming the table.
- **The name placement test in `test_ci_workflows.py` cannot place a name a new module uses.** It
  reads every module in `scripts/tests`, so such a name reds that test in CI. Detected at the red
  push; the modules name only builtins it already places.
- **The oracle's population is slow.** A18 and A19 read 28,928 small tables each; each prints its
  examined count, and its time is read from the job log at the red push.
- **A table's `on` goes stale.** A renamed function leaves its event name behind, and nothing here
  reads it. Detected only by #885's writers rule.

## 7. What only CI proves

The tree readings are read from the script-test job's log, never asserted as listings, because a
listing a test pinned would turn a sibling's new site into a red at `warn`. At the delivery's head
the summary lines must read, each with `hits=0 new=0 listed=0 stale=0`: FSM-TABLE `examined=12
state-enums`, FSM-REACH `examined=51 states`, FSM-DEAD-END `examined=10 states`, FSM-TERMINAL
`examined=41 terminals` and FSM-DETERMINISTIC `examined=15 pairs`. Each examined figure is at least
section 1's. The red-first record holds every criterion's red and green, each read in CI.

## 8. Formal model

None is added and none changes. Each rule reads committed files in one process and writes nothing,
so no two actors share state, and there is nothing for a TLA+ model to interleave. The walk and the
three graph sets are pure functions over a table, and at `warn` they decide no verdict; A18 compares
each with an oracle that never calls the helper, over every table on three states and one event
and every table on two states and two events, with literal verdicts. No path this delivery adds is
named by a cover under `formal/`, so no entry's digest moves. Three tables describe machines a model
also covers: `Phase` (`formal/tla/MintReadsTheFinalBase` anchors the enum and `PHASES`),
`ChestState` (`formal/tla/ChestOpenedOnce` covers its four moving functions) and `SkipState`
(`formal/tla/SkipDayOnce` covers `settle_applied_on`, and no model covers `settle_failed`).
Whether a table equals its model's moves is #885's formal parity, never read here.

## 9. Mutation rows (S43400-S43499)

Rows S43400 to S43463, in `scripts/mutation-rows.d/S43400-S43499.json` as `SCRIPT_MUTATIONS`. Each
row's file is `scripts/tests/_fsm_tables.py` unless the row names `config/fsm-tables.json`. Each
find is taken from the green commit's text and occurs exactly once in its file. Each killer is the
criterion's test, spelled as the band's existing killers are (`<module>.<Class>.<test>`).

| row | file | mutant | killer |
|---|---|---|---|
| S43400 | helper | the product walk also reads `crates/*/tests` | A2 |
| S43401 | helper | the product walk reads only the top level of `src` | A1 |
| S43402 | helper | the walk's count is not compared with the independent listing | A13 |
| S43403 | helper | the `#[cfg(test)]` item mask is dropped | A2 |
| S43404 | helper | the Rust comment mask is dropped | A2 |
| S43405 | helper | the Rust string mask is dropped | A2 |
| S43406 | helper | `State` leaves the enum suffixes | A2 |
| S43407 | helper | `Status` leaves the enum suffixes | A2 |
| S43408 | helper | `Phase` leaves the enum suffixes | A2 |
| S43409 | helper | `Stage` leaves the enum suffixes | A2 |
| S43410 | helper | `Step` leaves the enum suffixes | A2 |
| S43411 | helper | `Lifecycle` leaves the enum suffixes | A2 |
| S43412 | helper | `Mode` leaves the enum suffixes | A2 |
| S43413 | helper | a tuple variant's fields are read as states | A3 |
| S43414 | helper | a struct variant's fields are read as states | A3 |
| S43415 | helper | a variant after a discriminant is dropped | A3 |
| S43416 | helper | an outer attribute hides its variant | A3 |
| S43417 | helper | a `#[cfg(test)]` variant is kept as a state | A3 |
| S43418 | helper | a product state enum with no table is not a hit | A2 |
| S43419 | helper | a table naming an absent file reads clean, not VOID | A4 |
| S43420 | helper | a table naming an enum its file lacks reads clean, not VOID | A4 |
| S43421 | helper | a state its enum lacks is dropped, not VOID | A4 |
| S43422 | helper | the walk takes one step from the initial set and stops | A6 |
| S43423 | helper | the walk starts from the first initial state alone | A6 |
| S43424 | helper | the walk also starts from every state no row enters | A6 |
| S43425 | helper | an unreached state is not a hit | A6 |
| S43426 | helper | FSM-REACH examines only the initial states | A5 |
| S43427 | helper | a reached sink outside the terminal set is not a hit | A8 |
| S43428 | helper | FSM-DEAD-END reads unreached states too | A19 |
| S43429 | helper | a terminal sink is a FSM-DEAD-END hit | A8 |
| S43430 | helper | a self-loop counts as no outgoing row | A8 |
| S43431 | helper | a reached terminal with an outgoing row is not a hit | A10 |
| S43432 | helper | a terminal's self-loop is not an outgoing row | A10 |
| S43433 | helper | a sink not declared terminal is a FSM-TERMINAL hit | A19 |
| S43434 | helper | FSM-TERMINAL reads unreached terminals too | A10 |
| S43435 | helper | two targets from one state and event are not a hit | A12 |
| S43436 | helper | pairs are keyed by the state alone | A12 |
| S43437 | helper | pairs are keyed by the event alone | A12 |
| S43438 | helper | `examined=0` is not refused | A13 |
| S43439 | helper | a tree with no `crates/` reads as empty, not VOID | A13 |
| S43440 | helper | a listed entry with no `ruling` is admitted | A14 |
| S43441 | helper | a `ruling` naming no file is admitted | A14 |
| S43442 | helper | a `ruling` outside `docs/rulings/` is admitted | A14 |
| S43443 | helper | a listed entry no hit matches is not `stale` | A14 |
| S43444 | helper | at `fail` a `new` hit does not fail | A15 |
| S43445 | helper | at `warn` a `new` hit fails | A15 |
| S43446 | helper | at `fail` a `stale` hit does not fail | A15 |
| S43447 | helper | a hit line carries the text of the line it names | A16 |
| S43448 | helper | an unknown level is read as `warn` | A17 |
| S43449 | helper | an unknown rule key is accepted | A17 |
| S43450 | helper | an unknown table key is accepted | A17 |
| S43451 | helper | a missing `terminal` reads as an empty set | A17 |
| S43452 | helper | an empty initial set is admitted | A17 |
| S43453 | helper | a duplicate table is admitted | A17 |
| S43454 | helper | a duplicate row is admitted | A17 |
| S43455 | helper | a table with no moves and no `no_moves` is admitted | A17 |
| S43456 | helper | a table with moves and a `no_moves` is admitted | A17 |
| S43457 | `config/fsm-tables.json` | FSM-TABLE's level reads `fail` | A17 |
| S43458 | `config/fsm-tables.json` | FSM-REACH's level reads `fail` | A17 |
| S43459 | `config/fsm-tables.json` | FSM-DEAD-END's level reads `fail` | A17 |
| S43460 | `config/fsm-tables.json` | FSM-TERMINAL's level reads `fail` | A17 |
| S43461 | `config/fsm-tables.json` | FSM-DETERMINISTIC's level reads `fail` | A17 |
| S43462 | `config/fsm-tables.json` | `ChestState`'s table names `crates/quests/src/chests.rs`, which does not declare it | A1 |
| S43463 | `config/fsm-tables.json` | a `ChestState` row names `Open` for `Opened` | A1 |
