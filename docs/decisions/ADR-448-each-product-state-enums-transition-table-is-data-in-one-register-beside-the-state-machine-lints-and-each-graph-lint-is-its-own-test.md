---
status: "accepted"
---

# ADR-448: each product state enum's transition table is data in one register beside the state-machine lints, and each graph lint is its own test

Decides SPEC-434. Path:
`docs/decisions/ADR-448-each-product-state-enums-transition-table-is-data-in-one-register-beside-the-state-machine-lints-and-each-graph-lint-is-its-own-test.md`.
Amends nothing. Every figure was read at dev `b25ed04a` by the commands SPEC-434 section 1 lists,
over the tree less the four paths those commands exclude; the tests read the whole tree, so each
examined figure they print is at least the one written here.

## Context

SPEC-433 reads the workspace's twelve product state enums for four lints, and its FSM-UNCOVERED
reads only the formal half of coverage. Nothing declares which moves a state enum admits, so
nothing can say that every state is reached, that no state is a dead end, which states end a
machine, or that one event moves one state one way (#883).

At `b25ed04a` the twelve enums split by one test: does any write read the state it leaves?

- Three do. `ChestState` (`crates/quests/src/chest_store.rs:50`) is stored in `chests.state`, and
  each of its four moves is a guarded update that names the states it leaves:
  `mark_opened` (`:300`, from sealed or vaulted), `mark_resolved` (`:316`, from opened),
  `settle_choice` (`:337`, from opened) and `sweep_resolve` (`:355`, from sealed, vaulted or
  opened). `SkipState` (`crates/ingest/src/skip.rs:232`) is stored in `skip_days.state`: `begin`
  inserts `pending` (`:363`), and `settle_failed` (`:409`) and `settle_applied_on` (`:563`) each
  leave `pending` only. `Phase` (`crates/coordination/src/recompute/mod.rs:56`) is an order:
  `PHASES` (`:74`) lists the seven, and the fold keeps its steps in that order (`:487-493`) and runs
  them so (`:631`).
- Nine do not. Each value is built fresh, or overwritten whatever it held. `TopicState` is a topic's
  outcome for a study day, and a later run that day overwrites it in one upsert
  (`crates/readings/src/store.rs:396`) that never reads the state it replaces. `RunStatus` is
  inserted with its run's row and never updated (`crates/ingest/src/sync_runs.rs:368`, `:432`).
  `NotifyState` names a datagram, and `WATCHDOG=1` can follow `STOPPING=1` while a role drains
  (`crates/daemon/src/role_api.rs:253`, before its heartbeat stops at `:260`). `BandStep`, `AnchorState`, the sync's `Step`
  and the poll's `Step` are each the outcome of one comparison or one request. The passkeys `State`
  is a ceremony's kind, fixed when it is inserted (`crates/identity/src/passkeys.rs:418`, `:519`).
  The insights `State` is a row's setting in the source (`crates/insights/src/registry.rs:27-31`).

Two nearby code shapes look like tables. `crates/bot/src/poll.rs:176-178` and `:189-191` map each
`Step` outcome to a loop action (next request, retry, return), so `Step` is the event and the
start's requests are code positions, not an enum. The passkeys finish refuses the other kind
(`passkeys.rs:443`), which keeps a kind fixed, not a move between kinds.

## Decision Drivers

- A table is a claim about the code, so its states are the enum's own variants, read from its file.
- A rule whose precision is unread must not block a pull request, so every rule starts at warn and
  moves to fail only by the promotion law.
- A rule that examined nothing must never read as a pass.
- One defect is reported by one rule.
- No product code changes, and the rules read only committed text in CI's existing script-test step,
  with no new dependency.

## Decisions

**D1. FSM-TABLE: one table per product state enum, as data in `config/fsm-tables.json`.** A table
is keyed by the enum's `file` and `enum`. It holds `initial` (a non-empty list of states),
`terminal` (a list, possibly empty), `moves` (rows `{"from", "to", "on"}`) and, only when `moves`
is empty, `no_moves`, the reason the enum makes no move. A table's states are its enum's variants,
read from the enum's declaration in its file: unit, tuple and struct variants, with any
discriminant, outer attribute or `#[cfg(test)]` variant handled as SPEC-434 R5 says. The table does
not list its states, so a variant added to the enum is a state the table must place. The
population is SPEC-433's product state enums (R6 there): every enum declared in a `.rs` file under
`crates/*/src`, outside an item under `#[cfg(test)]`, whose name ends in `State`, `Status`, `Phase`,
`Stage`, `Step`, `Lifecycle` or `Mode`. A product state enum with no table is a `new` hit. A table
whose file or enum is absent, or that names a state its enum lacks, is VOID: a rename never passes
silently. A row's `on` names the function whose call makes the move, by convention; whether that
function makes that move, and whether any other code writes the state, is the writers rule of #885,
which this rule does not read. The six test enums are not read (#888). The poll's and the passkeys'
nearby shapes are listed, not adopted. The two state columns SPEC-433 lists as unpaired,
`notification_queue.state` and `inbox_captures.state`, get no enum and no table here: an enum is a
product change, and their machines are written only as SQL literals, one with a bound target
(`crates/notifications/src/ledger.rs:359`), so each is a product machine of #885.

Twelve tables at `b25ed04a`: three machines with 15 rows (`Phase` 6, `ChestState` 7, `SkipState` 2)
and nine with no moves, each with its reason (SPEC-434 section 1).

**D2. FSM-REACH (check 13): every state is reached from the declared initial set.** The initial set
is declared per table, never derived. The walk is a breadth-first walk from `initial` over `moves`;
a state it never reaches is a hit, keyed by the enum and the state. A table with no moves declares
every state initial, because each value is built fresh. At `b25ed04a` the walk reads 51 states and
lists none. The known false positives: a state reached only by a load from storage, which the table
declares initial because a stored row is an entry point (none at `b25ed04a`: every stored state of
the three machines is created or moved to); and a state built only in a test, which is a true hit
for product code. The known false negatives: an initial set declared wider than the code creates,
and a move the table admits that the code never makes, both #885's writers rule.

**D3. FSM-DEAD-END (check 14): every reached state outside the terminal set has an outgoing row.**
The population is the reached states outside `terminal`, so an unreached state is FSM-REACH's alone.
A reached state outside `terminal` with no row leaving it is a hit. A self-loop is an outgoing row.
At `b25ed04a` the rule reads 10 such states (`Phase` 6, `ChestState` 3, `SkipState` 1) and lists
none. The known false positive: a state whose only exit is an error or recovery path the table does
not name, where the hit is true of the table and the cure is its row. The known false negatives: a
retry loop, a state whose only exits lead back to it, which is never left and reads as no dead end;
and a state whose declared exits a crash can skip, as a `pending` skip whose take dies in flight,
which is liveness and not a property of the table (#885).

**D4. FSM-TERMINAL (check 15): a declared terminal is never left.** Every table declares
`terminal`, and a missing key refuses the register. A reached declared terminal with an outgoing
row, a self-loop included, is a hit. The other half of the rule as first stated, a sink not declared
terminal, is FSM-DEAD-END's hit by construction, because such a sink is a reached state outside the
terminal set with no row; so it is reported there once and never here. An unreached terminal is
FSM-REACH's. Each state is therefore a hit of at most one of FSM-REACH, FSM-DEAD-END and
FSM-TERMINAL. At `b25ed04a` the rule reads 41 reached terminals and lists none. Where this rule ends,
#885's terminal rows begin: this rule reads the table's rows, never a stored row, so a write that
changes another column of a terminal row, as `mark_undone_on` (`crates/ingest/src/skip.rs:580`)
sets `undone` on an applied skip, is no row and no hit.

**D5. FSM-DETERMINISTIC (check 16): one state and one event move to at most one state.** Every row
carries `on`, so the rule reads each `(from, on)` pair and lists one whose rows name two or more `to`
states, keyed by the enum, the state and the event. At `b25ed04a` the rule reads 15 pairs over the
15 rows and lists none. The known false negative: an overwrite that writes any state is no row, as
`TopicState`'s upsert is not, so the rule never reads it. The known false positive: one function
whose target depends on a guard the event's name omits; its cure is a row per guard, each event
named for its guard.

**D6. One home beside SPEC-433's, apart from its files.** The five rules sit where SPEC-433's four
do: one test module each under `scripts/tests` (`test_fsm_table.py`, `test_fsm_reach.py`,
`test_fsm_dead_end.py`, `test_fsm_terminal.py`, `test_fsm_deterministic.py`), their shared behaviour
in `test_fsm_table_lints.py`, one helper `_fsm_tables.py` imported statically as `_support` and
`_units` are, and one register under `config/`. They are apart from SPEC-433's files because that
SPEC's R1 fixes its register at exactly four rules, its A18 refuses a fifth, and ADR-447 D5 keeps
its masker for its own four rules; so this helper carries its own small Rust masker and enum reader,
and a lexer shared by every Rust text guard stays #886's. The register takes SPEC-433's shape for
levels and listed sites, and every reading prints SPEC-433's lines, `FSM-LINT <rule>
level=<level> examined=<n> <unit> hits=<n> new=<n> listed=<n> stale=<n>` and one
`FSM-LINT <rule> <level> <new|listed|stale> <path>:<line> <key>` per hit, so the promotion reader
counts all nine rules from one log. A warn prints its lines and passes. The lines reach the same
job log as SPEC-433's, through the same script-test step. No module imports another by path, so
`scripts/tests/test_ci_workflows.py`'s `DYNAMIC_IMPORTS` (`:7172`) gains nothing and that file is
not changed.

**D7. The listings land at warn, and no product code changes.** Each rule lands with the hits it
lists at `b25ed04a`, which are none: twelve tables, 0 `new`, 0 `listed`, 0 `stale` for each rule. A
hit that looks like a real defect, an unreachable stored state, a dead end or two moves from one
event, is put as a question with options and is never cured here (#889).

**D8. Kill criteria, the same law for each rule.**
- Promotion: a rule moves from warn to fail after three iterations at a precision of at least 0.8
  over at least 5 hits, by its own delivery changing the rule's `level` in the register.
- Demotion: moving a fail rule back to warn, or listing a new site under a fail rule, is a weakening
  and needs a signed ruling under `docs/rulings/`.
- VOID blocks at every level: `examined=0`; FSM-TABLE's walk count differing from an independent
  listing's; the tree's `crates/` or the register absent; a table naming a file, an enum or a state
  that is absent. The rules call no external tool, so a missing tree or register is the tooling
  VOID.
- The kill window is the last 30 merged pull requests into dev. A rule whose triaged precision reads
  below 0.8 over that window is redesigned by a new ADR, never demoted by an edit. A rule with fewer
  than 5 hits in the window stays at warn: its precision is unread.
- Per rule: a FSM-TABLE hit is a fact, and whether the new enum is a machine is the triage. A
  FSM-REACH, FSM-DEAD-END or FSM-TERMINAL hit is true of the table, and the triage is whether the
  table or the code is wrong. A FSM-DETERMINISTIC hit is true unless the event's name hides a guard.

**D9. What this delivery does not do.** It builds none of the client rules (#884), none of the later
rules: writers, triggers, terminal rows, formal parity, journaled moves, mutation per arm, the
product machines and the client contract (#885), no shared lexer (#886), no promotion reader (#887),
no widened population (#888), none of SPEC-433's four rules, and no cure of a hit (#889).

**D10. Tests first, rows, no model, two pushes.** Each acceptance criterion's test is red first by
its own reason, on a table or site planted in a fixture tree under a temporary directory, never in
the product tree. The tree tests read the real tree. The walk reads product files that this
repository's delivery practice reads only in CI, so every red and green is read there, and the
delivery pushes twice: the red commit alone, its failures read by name, then the green, the rows
and the red-first record. Rows S43400 onward sit in `scripts/mutation-rows.d/S43400-S43499.json` as
`SCRIPT_MUTATIONS`. No TLA+ model and no Lean proof is added, for the reasons under D10 below. Each
check's walk is closed instead by an independent oracle over an exhaustive population of small
tables, which never calls the helper. No SPEC or ADR on dev is amended, and the schematic is new.

## What it was chosen against

### D1

- A Rust `const` table beside each enum: rejected because it edits twelve product files in nine crates, each owing a build, a lint and its own mutants, for data no product code reads.
- A function returning the table: rejected because a table read by calling code is code, which must be compiled and run to be read, and it edits the same twelve files.
- Listing each table's states beside its moves: rejected because a second list of the variants could disagree with the enum, while reading them from the enum ties the table to the code.
- Tables for the six test enums: rejected because a test's enum is its own oracle and stores nothing.
- Adopting the poll's outcome dispatch and the passkeys' kind refusal as tables: rejected because each is keyed by an enum whose values are events or kinds, over states no enum names.
- Reading each move's writer now and listing a writer no row admits: rejected because the writers here take three forms, a guarded SQL literal, an SQL literal with a bound target and a Rust value built in another file, each a reader of its own, which is #885's writers rule.
- A column-keyed table for `notification_queue.state` and `inbox_captures.state`: rejected because neither has an enum to read its states from, and naming one is a product change.

### D2

- Deriving the initial set from a `Default`: rejected because none of the twelve enums derives or implements one at `b25ed04a`.
- Deriving it from a constructor: rejected because which variant a constructor builds is control flow, which is the writers rule.
- Walking from every state that no row enters: rejected because a state no row enters and no code creates would then read reached.

### D3

- Reading every state, reached or not: rejected because an unreached sink would then be reported twice, by FSM-REACH and here.
- Treating a self-loop as no outgoing row: rejected because a state that loops on a retry is left by its row in the table's own terms; the loop is a named false negative.

### D4

- Reporting a sink not declared terminal here: rejected because it is exactly FSM-DEAD-END's hit, so one defect would be reported twice.
- An implicit terminal set, every sink: rejected because a sink that should not be one would then never be a hit.
- Reading stored terminal rows for later writes: rejected because a stored row's other columns are #885's terminal rows.

### D5

- Rows with no event, the rule reading `(from, to)` pairs: rejected because every such table is deterministic by construction, so the rule would examine nothing.
- An event equal to the target state: rejected because it makes every table deterministic by its spelling.

### D6

- Adding the five rules to `config/fsm-lints.json`: rejected because SPEC-433 R1 fixes that register at four rules and its A18 refuses a fifth.
- Importing SPEC-433's helper for its masker and enum reader: rejected because ADR-447 D5 keeps that masker for its four rules, and sharing one is #886's decision.
- A table kept only in a TLA+ model: rejected because six of the twelve enum files carry no model, and a model's table cannot be read by a lint without a second parser.
- A table generated from the code at test time: rejected because a table derived from the code agrees with the code by construction, so no graph rule could find a defect in it.
- One module for the five rules: rejected because it gives one examined count and one red for five rules the promotion law counts apart.
- A Rust test in a crate: rejected because the rules read every crate, which no one crate owns.

### D7

- Curing a hit in this delivery: rejected because a cure is a product change owing its own SPEC and tests, which a lint delivery must not carry.
- Landing at fail: rejected because no rule's precision has been read yet.

### D8

- A kill window in days: rejected because merges, not days, give a rule hits to read.
- Demotion by a plain register edit: rejected because a demotion is a weakening.
- Promotion on hit count alone: rejected because a noisy rule reaches 5 hits first.

### D9

- Building the writers rule with the tables: rejected because each writer form needs a reader of its own, and the tables are needed first.

### D10

- One push: rejected because the walk reads files read only in CI, so the red can be read only after a push, and the green must follow it.
- A TLA+ model of the rules: rejected because no two actors share state: each module reads committed files in one process and writes nothing.
- A Lean proof of the walk: rejected because at warn the walk decides no verdict, and an exhaustive population of small tables judged by a transitive-closure oracle that never calls the helper decides every reach, dead-end, terminal and determinism answer with literal verdicts.
- Amending the SPECs that own the enums: rejected because no requirement they own changes.
- Amending an existing schematic of a machine: rejected because three owners' schematics would each carry one lint's data, while the lint's data flow belongs beside SPEC-433's.

## Consequences

- A variant added to a machine's enum with no row reads unreached in the next pull request's script-test log.
- A new product state enum with no table prints a `new` line, so the next table to write is named by the tree.
- A renamed enum or variant that a table names fails as VOID until the table follows it.
- Each rule costs one walk of `crates/*/src` (340 files at `b25ed04a`) and a read of one register in CI's script-test step.

## Confirmation

SPEC-434's acceptance criteria decide each rule. The red-first record, `docs/red-first/SPEC-434.md`,
holds each criterion's red and green. The band's rows are proved in CI by their own ids.

## What would make this wrong

- A product machine whose state is moved only by code that reads no state, such as an upsert, would read as a value; `TopicState` is that case at `b25ed04a`, and its table says so.
- An enum whose variants are read wrong by the masker would make its table VOID or its states wrong; SPEC-434 A3 plants each variant form.
- A promotion reader that cannot tell a `listed` line from a `new` one would count a listed site every iteration.

## More Information

- SPEC-434 holds the requirements, the acceptance criteria, the manifest and the rows.
- The schematic is `docs/schematics/state-machine-transition-tables.md`.
- SPEC-433 and ADR-447 hold the four lints these sit beside, and land first.
