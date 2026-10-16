# SPEC-433: four state-machine lints start at warn, over CHECK lists, wildcard arms, unreachable arms and uncovered enums

Path:
`docs/specs/SPEC-433-four-state-machine-lints-start-at-warn-over-check-lists-wildcard-arms-unreachable-arms-and-uncovered-enums.md`.

- **Issue:** none is closed. The pull request refers to #822, the release pull request, and lands
  on dev only after it.
- **Context(s):** the script tests (`scripts/tests/`), one register under `config/`, the mutation
  band and the documents. No crate, migration, formal entry or workflow changes.
- **Decided by:** ADR-447 (each state-machine lint is its own test over one register, starting at
  warn, and a CHECK list is read from the schema SQLite builds). No SPEC or ADR is amended.
- **Schematic:** `docs/schematics/state-machine-lints.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-433.md`. Every rule it adds is at warn.

## 1. The problem, measured

Nothing ties a stored state's CHECK list to its enum. Nothing notices a catch-all arm over a state
enum, or counts the arms that claim a variant impossible, or says which state enums no formal
model is told about. Every figure below was read at dev `b25ed04a` (`D` below). `X` stands for the
four exclude pathspecs `':(exclude,icase)*drill*' ':(exclude,icase)*readings_tree*'
':(exclude,icase)*_standin_checks*' ':(exclude,icase)*test_deploy_standin*'`, each beside the
positive `.`. The tests read the whole tree, so each examined figure they print is at least the
figure here, and the file counts read by name are exact.

| fact | at `b25ed04a` | read by |
|---|---|---|
| `.rs` files under `crates/*/src` | 340 | `git ls-tree -r --name-only D crates/ \| grep -c -E '^crates/[^/]+/src/.*\.rs$'` |
| migrations | 33 | `git ls-tree --name-only D migrations/ \| grep -c -E '\.sql$'` |
| formal modules (`.tla`, `.lean`) and their cover lines | 55 modules, 251 cover lines | `git ls-tree -r --name-only D formal/ \| grep -c -E '\.(tla\|lean)$'`; `git grep -n '@phx covers' D -- . X`, the lines under `formal/` |
| Rust enums named as states | 18: 12 under `crates/*/src`, 6 under `crates/*/tests` | `git grep -n -E '\benum\s+[A-Za-z0-9_]*(State\|Status\|Phase\|Stage\|Step\|Lifecycle\|Mode)\b' D -- . X`, its `.rs` lines (2 more are Swift) |
| matches on a product state enum | 13 | the arm patterns `<Enum>::<Variant> ... =>` and `Self::<Variant> ... =>` in each enum's `impl`, by `git grep -n` and `git show D:<path> \| sed -n` ranges |
| catch-all arms among them | 2: `crates/readings/src/state.rs:302` and `:312` | the same ranges |
| wildcard arms whose value is a state enum | 3, none a match on one | `git grep -n -E '(^\|[^A-Za-z0-9_])_\s*(if\s.*)?=>' D -- . X`, the arms whose value names a state enum |
| `unreachable!`, `panic!`, `todo!`, `unimplemented!` under `crates/*/src` | 2, neither in product code | `git grep -n -E '\b(panic\|unimplemented\|todo\|unreachable)!' D -- . X` |
| whole-body column-domain CHECK lists | 59 in 32 migrations, folding to 54 columns (42 text, 12 integer); 5 rewritten by `004102`'s rebuild | the `CHECK (<col> IN (...))` forms of each migration, comments stripped, renames applied |
| state columns among the 54 | 9: 7 paired, 2 with no enum | the text columns named `state`, `status`, `outcome`, `verdict`, `phase`, `stage`, `step` or `mode`, or ending in `_` and one |
| state enums deriving `sqlx::Type` or carrying a serde rename | 0 | `git grep -n -E 'sqlx::Type\|#\[sqlx\(\|serde\(rename' D -- . X` |
| enums marked `#[non_exhaustive]`; state-enum variants imported bare | 0; 0 | `git grep -n -E 'non_exhaustive\|use\s+[A-Za-z_:]*(State\|Status\|Phase\|Stage\|Step\|Lifecycle\|Mode)::(\*\|\{)' D -- . X` |
| product files holding `#[cfg(test)]` | 27 | `git grep -c -E '#\[cfg\(test\)\]' D -- . X`, the `crates/*/src` lines |
| product state enums anchored by a formal entry | 1 (`Phase`); 6 if a cover of the enum's file counted | the cover lines under `formal/` whose path is an enum's file, and those whose `anchor=` is its name |

The twelve product state enums, with their variants:

| enum | declared at | variants |
|---|---|---|
| `Phase` | `crates/coordination/src/recompute/mod.rs:56` | RollupAndScore, BaseXp, StreaksAndGovernor, DaySteps, DerivedBonuses, CoinMint, Awards |
| `BandStep` | `crates/curriculum/src/progress.rs:240` | FirstSighting, BandUp, Unchanged |
| `NotifyState` | `crates/daemon/src/lifecycle.rs:46` | Ready, Watchdog, Stopping |
| `Step` | `crates/daemon/src/wiring.rs:605` | Rescore, Settings, Credentials, Scope, History, Sync, Obligations, GateProbe, GateRecord, WindowRead, WindowBase, Recompute |
| `AnchorState` | `crates/ingest/src/gate.rs:102` | Missing, Unreadable, Present |
| `SkipState` | `crates/ingest/src/skip.rs:232` | Pending, Applied, Failed |
| `RunStatus` | `crates/ingest/src/sync_runs.rs:159` | Ok, Error, Skipped |
| `ChestState` | `crates/quests/src/chest_store.rs:50` | Sealed, Vaulted, Opened, Resolved |
| `TopicState` | `crates/readings/src/state.rs:257` | Ready, NoNewCards, CouldNotTell, Paused, Failed, AiRouteAbsent |
| `State` | `crates/insights/src/registry.rs:8` | Live, Inert |
| `State` | `crates/identity/src/passkeys.rs:135` | Registration, SignIn |
| `Step` | `crates/bot/src/poll.rs:228` | Done, Again, Stop |

The six test enums are `HoldState` (`crates/coordination/tests/held_flush.rs:179`), `DayState`
(`crates/streaks/tests/formal_vectors_open_lapse.rs:18`) and four `Step`s
(`crates/ingest/tests/support/mod.rs:442`, `crates/vault/tests/atomic.rs:24`,
`crates/vault/tests/confinement.rs:362`, `crates/vault/tests/inbox_capture.rs:221`).

The 13 matches on a product state enum: `crates/bot/src/poll.rs:167` and `:188`;
`crates/coordination/src/recompute/progress.rs:70`; `crates/daemon/src/lifecycle.rs:59`;
`crates/daemon/src/wiring.rs:650` and `:670`; `crates/ingest/src/gate.rs:221`;
`crates/ingest/src/sync_runs.rs:172`; `crates/quests/src/chest_store.rs:65`;
`crates/readings/src/gates.rs:33` (on `Option<RunStatus>`); and `crates/readings/src/state.rs:287`,
`:300` and `:309`. Eleven are exhaustive. The two at `:300` (`TopicState::class`) and `:309`
(`TopicState::reason`) end in `_ => None`, at `:302` and `:312`.

The three wildcard arms whose value is a state enum decode another type into one, so none is a
match on a state enum: `crates/curriculum/src/progress.rs:263` (`_ => BandStep::Unchanged`, over a
tuple of `Option<usize>`), `crates/ingest/src/skip.rs:540` (`_ => SkipState::Pending`, over a
`&str`) and `crates/ingest/src/state.rs:165` (`_ => AnchorState::Unreadable`, over a tuple of
`Option`s).

The two panicking macros under `crates/*/src`: `crates/kernel/src/verdict.rs:29`, an `unreachable!`
arm in a `//!` doc example, and `crates/daemon/src/wiring.rs:1515`, a `panic!` arm inside the
`#[cfg(test)] mod tests` that opens at `:817`.

The nine state columns, each read from the schema the migrations leave:

| column | list | enum |
|---|---|---|
| `sync_runs.status` (`migrations/002201_ingest_sync_runs.sql:14`) | ok, error, skipped | `RunStatus`, by `as_str` (`crates/ingest/src/sync_runs.rs:171`) |
| `chests.state` (`migrations/008101_quests_chests_and_tokens.sql:20`) | sealed, vaulted, opened, resolved | `ChestState`, by `name` (`crates/quests/src/chest_store.rs:64`) |
| `skip_days.state` (`migrations/008301_ingest_skip_days.sql:13`) | pending, applied, failed | `SkipState`, by snake case: no fn spells it, its store writes SQL literals (`crates/ingest/src/skip.rs:364`, `:409`, `:563`) |
| `reading_topic_days.state` (`migrations/004501_readings_topic_days_and_runs.sql:51`) | ready, no_new_cards, could_not_tell, paused, failed, ai_route_absent | `TopicState`, by `name` (`crates/readings/src/state.rs:286`) |
| `cron_fires.last_outcome` (`migrations/002701_coordination_cron_fires.sql:21`) | ok, error, catchup, missed | `Outcome`, by `as_str` (`crates/coordination/src/ledger.rs:39`) |
| `reading_runs.outcome` (`migrations/004501_readings_topic_days_and_runs.sql:23`) | resolved, paused, could_not_tell, ai_route_absent | `RunOutcome`, by `name` (`crates/readings/src/state.rs:382`) |
| `agent_runs.verdict` (`migrations/004301_agent_runs.sql:14`) | delivered, withheld, unavailable, ai_route_absent | `Verdict`, by `name` (`crates/agent/src/verdict.rs:110`) |
| `notification_queue.state` (`migrations/004102_notifications_queue_claim.sql:16`, widened from `004101`'s held, abandoned) | held, sending, abandoned | none: SQL literals in `crates/notifications/src/ledger.rs` |
| `inbox_captures.state` (`migrations/011801_vault_inbox_captures.sql:24`) | captured, filed | none |

Every pair is equal at `b25ed04a`. ADR-447 D1 triages the columns whose lists overlap a different
state enum's spellings.

## 2. Requirements

- **R1. One register.** `config/fsm-lints.json` holds `{"rules": {...}}` with exactly four keys,
  `FSM-CHECK-LIST`, `FSM-WILDCARD`, `FSM-UNREACHABLE` and `FSM-UNCOVERED`, each with a `level` of
  `warn` or `fail`; every level is `warn` in this delivery. `FSM-CHECK-LIST` holds `pairs` and
  `unpaired`; the other three hold `listed`. An unknown rule, level or key refuses the register.
- **R2. The ratchet's admission.** Each `listed` and `unpaired` entry is keyed by its rule's key
  (R6, R8, R9; a column for `unpaired`). An entry with no `ruling` must be in its test module's
  frozen seed. An entry outside the seed must carry `ruling`, a path under `docs/rulings/` that
  exists in the tree. Any other entry refuses the register at every level. Deleting an entry
  owes nothing.
- **R3. The schema SQLite builds.** FSM-CHECK-LIST executes every `migrations/*.sql`, in filename
  order, into one in-memory SQLite database, and reads each table's final `CREATE` text from
  `sqlite_master`. In that text it masks `--` and `/* */` comments and single-quoted literals,
  where a doubled `''` is one quote. It reads a column domain only in the whole-body form
  `CHECK (<column> IN (<literal>, ...))`, so a CHECK joining an IN list to anything else is not a
  domain. Each literal is one spelling, commas inside it included.
- **R4. The enum's spellings.** A pair names `table`, `column`, `enum`, `file`, a `spelling` of
  `{"fn": "<name>"}` or `"snake_case"`, and optionally `unstored`, a list of variants each with a
  reason. `{"fn": ...}` reads the arms `Self::<Variant>[(...)] => "<text>"` of that fn inside the
  enum's `impl` in `file`. `"snake_case"` turns each variant's name into lower case with an
  underscore before each inner capital (`NoNewCards` to `no_new_cards`). Variants named in
  `unstored` are dropped.
- **R5. The comparison.** A pair is equal when its set of spellings equals its column's list. A
  hit names the column, each spelling the list lacks and each list entry no variant spells. A text
  domain column whose name is, or ends in `_` and, one of `state`, `status`, `outcome`, `verdict`,
  `phase`, `stage`, `step` or `mode` is a state column. A state column that is neither paired nor
  listed in `unpaired` is a `new` hit, and a listed one is `listed`. An integer list is never a
  state column.
- **R6. FSM-WILDCARD's population.** A product file is a `.rs` file under `crates/*/src`, at any
  depth. Before reading it, the rule masks comments (line, block, nested and doc), string,
  raw-string, byte-string and character literals, and every item under `#[cfg(test)]`. A state
  enum is an enum declared in product source whose name ends in `State`, `Status`, `Phase`,
  `Stage`, `Step`, `Lifecycle` or `Mode`. A match is on a state enum when an arm pattern names
  `<Enum>::<Variant>` anywhere in it, or `Self::<Variant>` inside `impl <Enum>` or
  `impl <Trait> for <Enum>`, where the variant is that enum's.
- **R7. FSM-WILDCARD's hit.** A hit is an arm of such a match whose whole top-level pattern is
  `_`, `_ if <guard>`, a lone lower-case binding (with `ref` or `mut` or neither) or
  `<binding> @ _`. Its key is the match's path, the enum and the enclosing fn. A `_` inside a
  tuple or a wrapper, `matches!`, `if let` and `let ... else` are not hits.
- **R8. FSM-UNREACHABLE.** Over R6's masked product source, a hit is a match arm whose value is a
  call of `unreachable!`, `panic!`, `todo!` or `unimplemented!`, either bare or as the first
  statement of the arm's block. Its key is the path, the enclosing fn and the macro. The rule is a
  ratchet over `listed`: empty in this delivery, growing only by R2.
- **R9. FSM-UNCOVERED.** A product state enum is covered when a file under `formal/` holds the line
  `@phx covers <the enum's file> anchor=<the enum's name>`; a cover naming its file at another
  anchor, or its name in another file, does not cover it. Its key is the enum's file and name. An
  enum neither covered nor listed is a `new` hit. A listed enum that is covered or gone is
  `stale`. `listed` grows only by R2.
- **R10. What each rule prints, and what fails.** Each reading prints one summary line,
  `FSM-LINT <rule> level=<level> examined=<n> <unit> hits=<n> new=<n> listed=<n> stale=<n>`, and
  one line per hit, `FSM-LINT <rule> <level> <new|listed|stale> <path>:<line> <key>`. A hit line
  carries no source text. At `warn` nothing a hit says fails. At `fail` a `new` or `stale` hit fails
  the test.
- **R11. VOID, at every level.** A reading fails as VOID when it examined nothing. It also fails
  when its walk's file count differs from an independent listing's, or when `crates/`,
  `migrations/` or `formal/` is absent. For FSM-CHECK-LIST it fails too on a Python without
  `sqlite3`, a SQLite that cannot create a `STRICT` table, a migration that does not execute, or a
  pair naming a table, column, enum or fn that is absent, or whose spelling arm is not a literal.
- **R12. What does not change.** No product code, migration, formal entry, workflow or required
  check changes. No module imports another by path, so `scripts/tests/test_ci_workflows.py` is
  unchanged, and every name each new module uses is one its name placement test already places.

## 3. Acceptance criteria of SPEC-433

Every test runs over a fixture tree it writes under a temporary directory, except A1, A7, A10 and
A12, which read the repository's own tree. Each test is in the module its fence line names.

| id | criterion | decided by |
|---|---|---|
| A1 | the FSM-CHECK-LIST reading of the tree reads every migration an independent listing counts and every pair the register names, and prints its summary line | `test_the_tree_reading_reads_every_migration_and_every_pair` |
| A2 | a planted drift names its column, each spelling the list lacks and each list entry no variant spells, across `{"fn": ...}` and `"snake_case"` spellings, a doubled `''`, a literal holding a comma and an `unstored` variant | `test_a_planted_drift_names_its_column_and_each_missing_and_extra_spelling` |
| A3 | a table rebuilt under a new name, renamed into place and given an added column is read as the last migration leaves it | `test_a_rebuilt_and_renamed_table_is_read_as_the_last_migration_leaves_it` |
| A4 | only the whole-body domain form is read: a conditional CHECK, a commented-out CHECK of either comment form and an integer list read as no state domain, beside one domain that is read | `test_only_a_whole_body_check_is_a_column_domain` |
| A5 | a state column of each of the eight names, and of the `_` ending, neither paired nor listed is a `new` hit, and a listed one reads `listed` | `test_a_state_column_neither_paired_nor_listed_is_a_new_hit` |
| A6 | a pair naming an absent table, column, enum or fn, or a spelling arm that is not a literal, is VOID | `test_a_pair_naming_what_is_absent_is_void` |
| A7 | the FSM-WILDCARD reading of the tree reads every product file an independent listing counts and at least one match on a state enum, and prints its summary line | `test_the_tree_reading_reads_every_product_file_and_its_matches` |
| A8 | each catch-all form is a hit over a state enum of each suffix, through `<Enum>::`, a wrapped `Some(<Enum>::...)`, `Self::` in `impl <Enum>` and `Self::` in `impl <Trait> for <Enum>` | `test_each_catch_all_arm_over_a_state_enum_is_a_hit` |
| A9 | beside one planted hit, a `_` in a tuple position, an item under `#[cfg(test)]`, a comment, a string, `matches!`, `if let`, `let ... else` and a non-state enum are not hits | `test_only_the_planted_catch_all_is_a_hit` |
| A10 | the FSM-UNREACHABLE reading of the tree reads every product file an independent listing counts, and prints its summary line | `test_the_tree_reading_reads_every_product_file` |
| A11 | each of the four macros as an arm's value, bare or first in its block, is a hit, and the same macro in a doc comment, under `#[cfg(test)]` or outside an arm is not | `test_each_panicking_arm_is_a_hit_and_nothing_else_is` |
| A12 | the FSM-UNCOVERED reading of the tree reads every formal module an independent listing counts, every product state enum and at least one cover line, and prints its summary line | `test_the_tree_reading_reads_every_formal_module_and_state_enum` |
| A13 | an anchor on the enum covers it; a cover of its file at another anchor, or of its name in another file, does not; a listed enum now covered is `stale`, and an unlisted uncovered one is `new` | `test_only_an_anchor_on_the_enum_covers_it` |
| A14 | each rule is VOID on a tree with nothing to examine, and FSM-CHECK-LIST is VOID on a SQLite that cannot create a `STRICT` table | `test_every_rule_is_void_with_nothing_to_examine` |
| A15 | an entry outside its seed is refused without a `ruling`, and with one that names no file under `docs/rulings/`, and admitted with one that does | `test_an_entry_outside_the_seed_needs_a_ruling_that_exists` |
| A16 | at `warn` a `new` and a `stale` hit print and pass; at `fail` each fails the test | `test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them` |
| A17 | a hit line carries its rule, level, kind, path, line and key, and no text of the line it names | `test_a_hit_line_names_its_site_and_never_its_source` |
| A18 | the register names exactly the four rules, each at `warn`, and refuses an unknown rule, level or key | `test_the_register_names_four_rules_at_warn_and_refuses_the_unknown` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_the_tree_reading_reads_every_migration_and_every_pair
A2: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_planted_drift_names_its_column_and_each_missing_and_extra_spelling
A3: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_rebuilt_and_renamed_table_is_read_as_the_last_migration_leaves_it
A4: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_only_a_whole_body_check_is_a_column_domain
A5: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_state_column_neither_paired_nor_listed_is_a_new_hit
A6: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_pair_naming_what_is_absent_is_void
A7: python3 -m unittest discover -s scripts/tests -p test_fsm_wildcard.py -k test_the_tree_reading_reads_every_product_file_and_its_matches
A8: python3 -m unittest discover -s scripts/tests -p test_fsm_wildcard.py -k test_each_catch_all_arm_over_a_state_enum_is_a_hit
A9: python3 -m unittest discover -s scripts/tests -p test_fsm_wildcard.py -k test_only_the_planted_catch_all_is_a_hit
A10: python3 -m unittest discover -s scripts/tests -p test_fsm_unreachable.py -k test_the_tree_reading_reads_every_product_file
A11: python3 -m unittest discover -s scripts/tests -p test_fsm_unreachable.py -k test_each_panicking_arm_is_a_hit_and_nothing_else_is
A12: python3 -m unittest discover -s scripts/tests -p test_fsm_uncovered.py -k test_the_tree_reading_reads_every_formal_module_and_state_enum
A13: python3 -m unittest discover -s scripts/tests -p test_fsm_uncovered.py -k test_only_an_anchor_on_the_enum_covers_it
A14: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_every_rule_is_void_with_nothing_to_examine
A15: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_an_entry_outside_the_seed_needs_a_ruling_that_exists
A16: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them
A17: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_a_hit_line_names_its_site_and_never_its_source
A18: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_the_register_names_four_rules_at_warn_and_refuses_the_unknown
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/_fsm_lints.py` | scripts | added: the masker, the item and match reader, the schema reader, the four rules, the register and the line printer (R1-R11) |
| `scripts/tests/test_fsm_check_list.py` | scripts | added: FSM-CHECK-LIST's seed and A1-A6 |
| `scripts/tests/test_fsm_wildcard.py` | scripts | added: FSM-WILDCARD's seed and A7-A9 |
| `scripts/tests/test_fsm_unreachable.py` | scripts | added: FSM-UNREACHABLE's empty seed and A10-A11 |
| `scripts/tests/test_fsm_uncovered.py` | scripts | added: FSM-UNCOVERED's seed and A12-A13 |
| `scripts/tests/test_fsm_lints.py` | scripts | added: the shared behaviour, A14-A18 |
| `config/fsm-lints.json` | config | added: the four rules at `warn`, the seven pairs, the two unpaired columns and the listed sites (R1) |
| `scripts/mutation-rows.d/S43300-S43399.json` | scripts | added: rows S43300-S43371 (section 9) |
| `docs/specs/SPEC-433-four-state-machine-lints-start-at-warn-over-check-lists-wildcard-arms-unreachable-arms-and-uncovered-enums.md` | docs | added: this SPEC |
| `docs/decisions/ADR-447-each-state-machine-lint-is-its-own-test-over-one-register-starting-at-warn-and-a-check-list-is-read-from-the-schema-sqlite-builds.md` | docs | added: its decision |
| `docs/schematics/state-machine-lints.md` | docs | added: the four rules' data flow |
| `docs/red-first/SPEC-433.md` | docs | added: the red-first record |
| `changelog.d/fsm-core-433.md` | changelog | added: the fragment |

The register's seeded entries, which each test module freezes as its seed:
- `pairs`: the seven of section 1, each with its spelling as section 1 names it and no `unstored`.
- `unpaired`: `notification_queue.state`, which its store writes as SQL literals with no enum, and
  `inbox_captures.state`, which has no enum.
- FSM-WILDCARD `listed`: (`crates/readings/src/state.rs`, `TopicState`, `class`) and
  (`crates/readings/src/state.rs`, `TopicState`, `reason`).
- FSM-UNREACHABLE `listed`: none.
- FSM-UNCOVERED `listed`: the eleven product state enums other than `Phase`, each by its file and
  name.

## 5. What this does NOT cover

- It builds no transition table per state enum and no graph check over one (every state
  reachable, no dead end, a declared terminal set, deterministic moves); FSM-UNCOVERED reads the
  formal half of coverage alone (#883).
- It adds no TypeScript exhaustive-switch rule and no Swift switch-without-default rule (#884).
- It adds none of the later state-machine rules: one writer per state field, a trigger per
  transition, terminal rows that never change, formal transition parity, journaled moves, mutation
  per transition arm, the product machines, and the client state list and contract (#885).
- It shares no lexer with the other Rust text guards; the masker serves these four rules (#886).
- It adds no reader that counts each rule's lines across merged pull requests and moves a rule to
  `fail`; the line it prints is that reader's input (#887).
- It reads no enum a column pairs but no state name marks (`RunOutcome`, `Outcome`, `Verdict`) under
  FSM-WILDCARD or FSM-UNCOVERED. It reads no `_` in a tuple position, no wrapped wildcard and no
  macro-built match (#888).
- It cures no hit it lists: the two `TopicState` wildcards, `SkipState`'s decode default
  (`crates/ingest/src/skip.rs:540`) and the two state columns with no enum stay as they are (#889).

## 6. Risks

- **A sibling's new state enum or match turns a reading's figures.** At `warn` a new site only
  prints, so no pull request is blocked; a listed site that moves item or file reads `stale`, which
  prints too. Detected by the summary line in each pull request's `hygiene` log.
- **A migration SQLite cannot execute here.** Every migration runs under the kernel's own tests, so
  one that will not execute in a bare in-memory database is a VOID that names its file, never a
  silent skip. Detected by A1 in CI.
- **The masker misreads a Rust form it does not know** (a raw string with many `#`, a nested block
  comment, a lifetime beside a character literal). A9 and A11 plant each form, and a misread
  surfaces as a `new` line naming a site no human finds.
- **The name placement test in `test_ci_workflows.py` cannot place a name a new module uses.** It
  reads every module in `scripts/tests`, so such a name reds that test in CI. Detected at the red
  push; the modules name only builtins it already places.
- **A ruling-cited entry names a file that is later moved.** R2 refuses the register then, at every
  level, which names the entry.

## 7. What only CI proves

The tree readings are read from the `hygiene` job's log, never asserted as listings, because a
listing a test pinned would turn a sibling's new site into a red at `warn`. At the delivery's head,
the summary and hit lines must read: FSM-CHECK-LIST 7 pairs equal, 2 `listed` unpaired columns, 0
`new` and 0 `stale`; FSM-WILDCARD 2 `listed` (`TopicState`'s `class` and `reason`), 0 `new`;
FSM-UNREACHABLE 0 hits; FSM-UNCOVERED 1 covered, 11 `listed`, 0 `new`. Each examined figure is at
least section 1's. The red-first record holds every criterion's red and green, each read in CI.

## 8. Formal model

None is added and none changes. Each rule reads committed files in one process and writes nothing,
so no two actors share state, and there is nothing for a TLA+ model to interleave. The register's
admission is membership in a frozen literal seed or a cited file that exists, which A15's fixture
population decides with literal verdicts. The schema fold is SQLite's own, read through
`sqlite_master`. No path this delivery adds is named by a cover under `formal/`. FSM-UNCOVERED reads
those covers and edits none, so no entry's digest moves.

## 9. Mutation rows (S43300-S43399)

Rows S43300 to S43371, in `scripts/mutation-rows.d/S43300-S43399.json` as `SCRIPT_MUTATIONS`. Each
row's file is `scripts/tests/_fsm_lints.py` unless the row names `config/fsm-lints.json`. Each find
is taken from the green commit's text and occurs exactly once in its file. Each killer is the
criterion's test, spelled as the band's existing killers are (`<module>.<Class>.<test>`).

| row | file | mutant | killer |
|---|---|---|---|
| S43300 | helper | the domain form accepts text after the IN list's closing parenthesis | A4 |
| S43301 | helper | the domain form accepts text before the column's name | A4 |
| S43302 | helper | an integer list counts as a state column | A4 |
| S43303 | helper | `state` leaves the state-column names | A5 |
| S43304 | helper | `status` leaves the state-column names | A5 |
| S43305 | helper | `outcome` leaves the state-column names | A5 |
| S43306 | helper | `verdict` leaves the state-column names | A5 |
| S43307 | helper | `phase` leaves the state-column names | A5 |
| S43308 | helper | `stage` leaves the state-column names | A5 |
| S43309 | helper | `step` leaves the state-column names | A5 |
| S43310 | helper | `mode` leaves the state-column names | A5 |
| S43311 | helper | the `_<name>` ending no longer marks a state column | A5 |
| S43312 | helper | a spelling the list lacks goes unreported | A2 |
| S43313 | helper | a list entry no variant spells goes unreported | A2 |
| S43314 | helper | the migrations run in reverse filename order | A3 |
| S43315 | helper | the schema is read before the last migration runs | A3 |
| S43316 | helper | the `--` comment mask is dropped | A4 |
| S43317 | helper | the `/* */` comment mask is dropped | A4 |
| S43318 | helper | a doubled `''` ends the literal | A2 |
| S43319 | helper | a literal holding a comma splits in two | A2 |
| S43320 | helper | a pair naming an absent column reads equal, not VOID | A6 |
| S43321 | helper | a pair naming an absent enum reads equal, not VOID | A6 |
| S43322 | helper | a spelling arm that is not a literal is skipped, not VOID | A6 |
| S43323 | helper | snake case drops its underscores | A2 |
| S43324 | helper | a column listed `unpaired` reads `new` | A5 |
| S43325 | helper | an `unstored` variant stays in the spelling set | A2 |
| S43326 | helper | a SQLite that cannot create a `STRICT` table is not refused | A14 |
| S43327 | helper | a `_` arm is not a hit | A8 |
| S43328 | helper | a `_ if <guard>` arm is not a hit | A8 |
| S43329 | helper | a lone binding arm is not a hit | A8 |
| S43330 | helper | a `<binding> @ _` arm is not a hit | A8 |
| S43331 | helper | `Self::` inside `impl <Enum>` is not resolved | A8 |
| S43332 | helper | `Self::` inside `impl <Trait> for <Enum>` is not resolved | A8 |
| S43333 | helper | a variant inside `Some(...)` is not seen | A8 |
| S43334 | helper | `State` leaves the enum suffixes | A8 |
| S43335 | helper | `Status` leaves the enum suffixes | A8 |
| S43336 | helper | `Phase` leaves the enum suffixes | A8 |
| S43337 | helper | `Stage` leaves the enum suffixes | A8 |
| S43338 | helper | `Step` leaves the enum suffixes | A8 |
| S43339 | helper | `Lifecycle` leaves the enum suffixes | A8 |
| S43340 | helper | `Mode` leaves the enum suffixes | A8 |
| S43341 | helper | a `_` in a tuple position is a hit | A9 |
| S43342 | helper | the `#[cfg(test)]` mask is dropped | A9 |
| S43343 | helper | the Rust comment mask is dropped | A9 |
| S43344 | helper | the Rust string mask is dropped | A9 |
| S43345 | helper | `matches!` is read as a match | A9 |
| S43346 | helper | an `unreachable!` arm is not a hit | A11 |
| S43347 | helper | a `panic!` arm is not a hit | A11 |
| S43348 | helper | a `todo!` arm is not a hit | A11 |
| S43349 | helper | an `unimplemented!` arm is not a hit | A11 |
| S43350 | helper | a block arm's first statement is not read | A11 |
| S43351 | helper | a macro outside any arm is a hit | A11 |
| S43352 | helper | any anchor in the enum's file covers it | A13 |
| S43353 | helper | the enum's name anchored in another file covers it | A13 |
| S43354 | helper | a listed enum now covered is not `stale` | A13 |
| S43355 | helper | an unlisted uncovered enum is not `new` | A13 |
| S43356 | helper | `examined=0` is not refused | A14 |
| S43357 | helper | an entry outside the seed is admitted with no `ruling` | A15 |
| S43358 | helper | a `ruling` naming no file is admitted | A15 |
| S43359 | helper | at `fail` a `new` hit does not fail | A16 |
| S43360 | helper | at `warn` a `new` hit fails | A16 |
| S43361 | helper | at `fail` a `stale` hit does not fail | A16 |
| S43362 | helper | a hit line carries the text of the line it names | A17 |
| S43363 | helper | the product walk also reads `crates/*/tests` | A7 |
| S43364 | helper | the product walk reads only the top level of `src` | A7 |
| S43365 | helper | an unknown level is read as `warn` | A18 |
| S43366 | helper | an unknown rule key is accepted | A18 |
| S43367 | `config/fsm-lints.json` | FSM-CHECK-LIST's level reads `fail` | A18 |
| S43368 | `config/fsm-lints.json` | FSM-WILDCARD's level reads `fail` | A18 |
| S43369 | `config/fsm-lints.json` | FSM-UNREACHABLE's level reads `fail` | A18 |
| S43370 | `config/fsm-lints.json` | FSM-UNCOVERED's level reads `fail` | A18 |
| S43371 | `config/fsm-lints.json` | `RunStatus`'s spelling fn reads a name its `impl` lacks | A1 |
