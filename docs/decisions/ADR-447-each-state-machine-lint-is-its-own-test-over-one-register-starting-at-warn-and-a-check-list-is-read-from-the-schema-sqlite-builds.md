---
status: "accepted"
---

# ADR-447: each state-machine lint is its own test over one register, starting at warn, and a CHECK list is read from the schema SQLite builds

Decides SPEC-433. Path:
`docs/decisions/ADR-447-each-state-machine-lint-is-its-own-test-over-one-register-starting-at-warn-and-a-check-list-is-read-from-the-schema-sqlite-builds.md`.
Amends nothing. Every figure was read at dev `b25ed04a` by the commands SPEC-433 section 1 lists,
over the tree less the four paths those commands exclude; the tests read the whole tree, so each
examined figure they print is at least the one written here.

## Context

The workspace keeps its state machines as Rust enums, and several store their state in a SQLite
text column whose `CHECK (<column> IN (...))` list is written by hand in a migration. Nothing ties
the two together. A new variant can be added and stored while the CHECK list still refuses it.
Or it can be added while a wildcard arm answers for it unread, or while a panicking arm claims it
is impossible. And no formal model need ever be told about it.

At `b25ed04a` the tree holds 18 Rust enums whose names end in `State`, `Status`, `Phase`, `Stage`,
`Step`, `Lifecycle` or `Mode`: 12 in product source under `crates/*/src` and 6 under
`crates/*/tests`. Their spellings are hand-written: 6 of the 7 enums stored in a column spell
each variant in a `const fn` whose arms are `Self::<Variant> => "<text>"`. The seventh,
`SkipState`, is spelled only in its store's SQL literals (`crates/ingest/src/skip.rs:364`, `:409`,
`:563`) and read back by a string match. No state enum derives `sqlx::Type` or carries a serde
rename. The migrations hold 59 whole-body column-domain CHECK lists, which fold to 54 columns. One
rebuild (`migrations/004102_notifications_queue_claim.sql`) rewrites five of them and renames its
table into place.

This ADR decides four lints, each a test of its own, and how each starts, reports, moves up and is
retired.

## Decision Drivers

- A rule whose precision is unread must not block a pull request, so every rule starts at warn
  and moves to fail only by the promotion law.
- A rule that examined nothing must never read as a pass.
- A listed site must survive a line shift above it, so sites are keyed by path and item, never by
  line.
- No product code changes in this delivery; a hit that looks like a defect is a question.
- The rules read only committed text, run in CI's existing script-test step, and add no dependency.

## Decisions

**D1. FSM-CHECK-LIST: a declared pair list, the schema SQLite builds, and the enum's own
spelling.** Each pair in `config/fsm-lints.json` names a table, a column, an enum, the enum's file
and its spelling rule: `{"fn": "<name>"}` reads the arms `Self::<Variant>[(..)] => "<text>"` of
that `const fn` in the enum's `impl`, and `"snake_case"` reads each variant's name in snake case.
A pair may name `unstored` variants, which the comparison drops, each with its reason. The SQL side
executes every `migrations/*.sql` in filename order into an in-memory SQLite database. It then
reads each table's final `CREATE` text from `sqlite_master`, so SQLite itself applies every
rebuild, rename and added column. In that text the rule masks `--` and `/* */` comments and
single-quoted literals (a doubled `''` is one quote). It reads a domain only in the whole-body
form `CHECK (<column> IN (<literal>, ...))`. A CHECK that joins an IN list to anything else, such
as `CHECK ((state IN ('could_not_tell', 'failed')) = (reason IS NOT NULL))`, is a condition, not a
domain. A pair is equal when the set of its spellings equals the column's list. A hit names the
column and each spelling missing from either side. Every text domain column whose name is, or
ends in `_` and, one of `state`, `status`, `outcome`, `verdict`, `phase`, `stage`, `step` and
`mode` is a state column, and it must be paired or listed under `unpaired` with its reason. An
integer list such as `passkeys.backup_state IN (0, 1)` is a flag, not a state column.

Seven pairs at `b25ed04a`, every one equal:

| table.column | enum | spelling | list |
|---|---|---|---|
| `sync_runs.status` | `RunStatus`, `crates/ingest/src/sync_runs.rs:159` | `as_str` `:171` | ok, error, skipped |
| `chests.state` | `ChestState`, `crates/quests/src/chest_store.rs:50` | `name` `:64` | sealed, vaulted, opened, resolved |
| `skip_days.state` | `SkipState`, `crates/ingest/src/skip.rs:232` | snake case | pending, applied, failed |
| `reading_topic_days.state` | `TopicState`, `crates/readings/src/state.rs:257` | `name` `:286` | ready, no_new_cards, could_not_tell, paused, failed, ai_route_absent |
| `cron_fires.last_outcome` | `Outcome`, `crates/coordination/src/ledger.rs:22` | `as_str` `:39` | ok, error, catchup, missed |
| `reading_runs.outcome` | `RunOutcome`, `crates/readings/src/state.rs:359` | `name` `:382` | resolved, paused, could_not_tell, ai_route_absent |
| `agent_runs.verdict` | `Verdict`, `crates/agent/src/verdict.rs:96` | `name` `:110` | delivered, withheld, unavailable, ai_route_absent |

Nine state columns: the seven, plus two listed `unpaired`. `notification_queue.state` (held,
sending, abandoned) is written as SQL literals in `crates/notifications/src/ledger.rs` by no enum,
and `004102` widened it from (held, abandoned). `inbox_captures.state` (captured, filed) has no enum.

Five columns overlap a state enum's spellings without being its column, and each is triaged:
- `cron_fires.last_outcome` shares `ok` and `error` with `RunStatus`: a different enum,
  coordination's `Outcome`, equal to it (pair 5).
- `reading_runs.outcome` shares `paused`, `could_not_tell` and `ai_route_absent` with `TopicState`
  and `resolved` with `ChestState`: a different enum, `RunOutcome` (pair 6).
- `agent_runs.verdict` shares `ai_route_absent` with `TopicState`: a different enum, the agent's
  `Verdict` (pair 7).
- `reading_topic_days.state` shares `failed` with `SkipState` and `ready` with `NotifyState`: it is
  `TopicState`'s (pair 4). `NotifyState` is never stored. Its `message` spells `sd_notify`
  datagrams (`crates/daemon/src/lifecycle.rs:58`), so these are two enums sharing a spelling, not
  a shared column.
- `notification_queue.state` was rewritten by a later migration: the latest list stands, and it has
  no enum (unpaired).

None is a real drift. The known false positives: a CHECK a later migration rewrote (closed by
reading SQLite's final schema), and a variant that is never stored (closed by `unstored`; none at
`b25ed04a`). The known false negatives: a CHECK in conditional form only; a domain held by a
trigger (0 triggers at `b25ed04a`) or by code alone; and a state column whose name escapes the
rule, such as `notification_queue.hold`. A rename never passes silently: a column renamed in SQL
or an enum renamed in Rust leaves its pair naming something absent, which is VOID.

**D2. FSM-WILDCARD: no catch-all arm in a match on a state enum, in product source.** A state enum
is an enum declared in product source, any `.rs` file under `crates/*/src` outside an item under
`#[cfg(test)]`, whose name ends in one of the seven suffixes. Its identity is its name and its
variant set, so the two enums named `State` and the two named `Step` are told apart by their
variants. A match is on a state enum when one of its arm patterns names a variant as
`<Enum>::<Variant>` anywhere in the pattern (so `Some(RunStatus::Ok)` counts), or as
`Self::<Variant>` inside an `impl <Enum>` or `impl <Trait> for <Enum>` block. A hit is an arm
whose whole top-level pattern is a catch-all: `_`, `_ if <guard>`, a lone lower-case binding
(`other`, `ref x`, `mut x`) or `<binding> @ _`. Before matching, the rule masks comments, string
and character literals, and items under `#[cfg(test)]`.

At `b25ed04a`: 12 product state enums (SPEC-433 section 1), 13 matches on them, 2 hits, both in
`crates/readings/src/state.rs`. They are `:302`, `_ => None` in `TopicState::class`, and `:312`,
`_ => None` in `TopicState::reason`, listed by path, enum and enclosing fn. A reading that counts a
wildcard arm whose VALUE is a state enum finds three other sites: `crates/curriculum/src/progress.rs:263`
(`_ => BandStep::Unchanged` over a tuple of `Option<usize>`), `crates/ingest/src/skip.rs:540`
(`_ => SkipState::Pending` over a `&str`) and `crates/ingest/src/state.rs:165`
(`_ => AnchorState::Unreadable` over a tuple of `Option`s). None is a match on a state enum: each
decodes another type into one, and is no hit.

The known false negatives: a `_` in a tuple position (`(false, _)`), a wrapped wildcard (`Some(_)`),
a match a macro builds, an arm naming a variant imported bare by `use <Enum>::*` (0 at
`b25ed04a`), and `matches!`, `if let` and `let ... else`, which name the variants they want by
design. The 2 `let ... else` sites at `b25ed04a`, `crates/identity/src/passkeys.rs:443` and
`crates/identity/src/linking.rs:266`, refuse the other ceremony on purpose. The known false
positive: a catch-all over an enum from another crate that is `#[non_exhaustive]`. It cannot occur
here, because a state enum is this workspace's and none is `#[non_exhaustive]` at `b25ed04a`. Test
enums and test files are not read.

**D3. FSM-UNREACHABLE: a listed-set ratchet on panicking arms in product source.** A hit is a
match arm in product source whose value is a call of `unreachable!`, `panic!`, `todo!` or
`unimplemented!`, with or without arguments, either bare or as the first statement of the arm's
block. The same masks as D2 apply, so a doc-comment example is not read. At `b25ed04a` product
source holds 0 such arms. Two of these macros sit under `crates/*/src`, and neither is product
code. `crates/kernel/src/verdict.rs:29` is a `//!` doc example, which runs as a doctest.
`crates/daemon/src/wiring.rs:1515` is inside the `#[cfg(test)] mod tests` that opens at `:817`.
The ratchet is a listed set keyed by path, enclosing fn and macro, and it is empty at `b25ed04a`.
It moves down when a listed entry is deleted, which owes no ruling. It moves up only by an entry
whose key is outside the test's frozen seed and which names a ruling file under `docs/rulings/`
that exists. Any other addition fails at every level.

**D4. FSM-UNCOVERED: a state enum is covered when a formal entry anchors the enum itself.** Covered
means a line `@phx covers <the enum's file> anchor=<the enum's name>` in a file under `formal/`.
An enum is an anchor item, so a new variant moves the anchor's digest and stales the entry, and its
model must then be re-read. A cover that names the enum's file at another anchor does not cover
the enum. At `b25ed04a` the rule reads 251 cover lines in 55 formal modules and 12 product state
enums. One is covered: `Phase` (`crates/coordination/src/recompute/mod.rs:56`, anchored by
`formal/tla/MintReadsTheFinalBase/MintReadsTheFinalBase.tla:2`). The other 11 are listed:
`BandStep`, `NotifyState`, `AnchorState`, `SkipState`, `RunStatus`, `ChestState`, `TopicState`,
the insights `State`, the passkeys `State`, the poll `Step` and the wiring `Step`. A state enum
that is neither covered nor listed is a `new` hit; a listed enum now covered, or gone, is `stale`.
The ratchet moves as D3's does.

**D5. One home per rule, one helper and one register.** Each rule is a test module of its own:
`scripts/tests/test_fsm_check_list.py`, `test_fsm_wildcard.py`, `test_fsm_unreachable.py` and
`test_fsm_uncovered.py`. Their shared behaviour is held by `scripts/tests/test_fsm_lints.py`. The
masker, the item and match reader, the four rules' functions, the register reader and the line
printer live in `scripts/tests/_fsm_lints.py`, imported statically as `_support` and `_units` are,
so no module loads another by path. The register is `config/fsm-lints.json`. Each tree test prints
one line per rule, `FSM-LINT <rule> level=<level> examined=<n> <unit> hits=<n> new=<n> listed=<n>
stale=<n>`. It also prints one line per hit, `FSM-LINT <rule> <level> <new|listed|stale>
<path>:<line> <key>`, which never carries source text. `examined=0` fails as VOID at every level,
and each walk's file count must equal an independent listing's. At warn a hit is printed and
nothing fails. At fail a new or stale hit fails the test. A VOID, or a register that does not
parse, names an unknown rule or level, or admits an entry D3 refuses, fails at every level.

The lines reach the `hygiene` job's log: `scripts/check.sh:185-188` runs `unittest discover` over
`scripts/tests` and prints every suite's output, and `.github/workflows/ci.yml:351` runs that stage.
The promotion reader counts each rule's lines there, in the run of each merged pull request into
dev; the reader is not in this delivery. No module imports dynamically, so
`scripts/tests/test_ci_workflows.py`'s `DYNAMIC_IMPORTS` (`:7172`) gains nothing. That file's name
placement test (`:4599-4655`) still reads every new module, so each module names only builtins it already
places. The masker serves these four rules alone; one lexer shared by every Rust text guard is a
later decision.

**D6. The listings land at warn, and no product code changes.** Each rule lands with the sites D1
to D4 list. A hit that looks like a defect is put as a question, with options and a recommended
one, and is never cured here. Three are put: the two `TopicState` wildcards; `SkipState`'s decode,
which reads any text it does not name as `Pending` (`crates/ingest/src/skip.rs:540`); and the two
unpaired state columns.

**D7. Kill criteria, the same law for each rule.**
- Promotion: a rule moves from warn to fail after three iterations at a precision of at least 0.8
  over at least 5 hits, by its own delivery changing the rule's `level` in the register.
- Demotion: moving a fail rule back to warn, or listing a new site under a fail rule, is a weakening
  and needs a signed ruling under `docs/rulings/`.
- VOID blocks at every level: `examined=0`; the tree's `crates/`, `migrations/` or `formal/` absent;
  a Python without `sqlite3`, or a SQLite that cannot create a `STRICT` table (FSM-CHECK-LIST); a
  pair naming a table, column, enum or spelling fn that is absent, or a spelling arm that is not a
  literal.
- The kill window is the last 30 merged pull requests into dev. A rule whose triaged precision reads
  below 0.8 over that window is redesigned by a new ADR, never demoted by an edit. A rule with fewer
  than 5 hits in the window stays at warn: its precision is unread, so it cannot be promoted.
- Per rule: a FSM-CHECK-LIST drift is true by construction unless a variant is never stored. A
  FSM-WILDCARD hit can be a deliberate catch-all, which is the precision the law reads. A
  FSM-UNREACHABLE hit is true unless the arm's impossibility is proved by the types. A
  FSM-UNCOVERED hit is a fact, and whether the enum is a machine worth a model is the triage.

**D8. What this delivery does not do.** It builds no transition table and no graph check, no
TypeScript or Swift exhaustiveness rule, none of the later rules, and no cure of a hit D6 lists.
SPEC-433 section 5 names each with its issue.

**D9. Tests first, rows, no model, two pushes.** Each acceptance criterion's test is red first by
its own reason, on a site planted in a fixture tree under a temporary directory, never in the
product tree. The tree tests read the real tree. The rules read paths that this repository's
delivery practice reads only in CI, so every red and green is read there. The delivery pushes
twice: the red commit alone, its failures read by name, then the green, the rows and the red-first
record. Rows S43300 onward sit in `scripts/mutation-rows.d/S43300-S43399.json` as
`SCRIPT_MUTATIONS`. No formal model and no proof is added, for the reasons under D9 below. No
SPEC or ADR on dev is amended: no requirement they own changes.

## What it was chosen against

### D1

- Deriving each pair from overlapping spellings: rejected because spellings are shared across enums (`ok` and `error` by `RunStatus` and `Outcome`, `failed` by `TopicState` and `SkipState`), which is how every overlap at `b25ed04a` arose.
- Folding the migration files by text: rejected because every statement form SQLite can apply (a rebuild, a table or column rename, an added column) would need a second implementation, while SQLite's own final schema applies them all.
- Reading spellings from `Display`, a serde rename or `sqlx::Type`: rejected because no paired enum derives one at `b25ed04a`, and each pair names the fn its store calls.
- A database trigger refusing a spelling outside the enum: rejected because it checks a write at run time, after the code and the CHECK already disagree, and costs a migration per variant.
- Comparing case-folded variant names with the list: rejected because `TopicState::NoNewCards` is stored as `no_new_cards` through `name`, and the fn is what the store calls.

### D2

- clippy's `wildcard_enum_match_arm`: rejected because it fires on a wildcard over every enum, not the state enums, and a workspace lint level changes every crate's build for one rule.
- Counting a wildcard arm whose value is a state enum: rejected because the 3 such sites at `b25ed04a` decode another type into a state, which exhaustiveness does not concern.
- Counting `matches!`, `if let` and `let ... else`: rejected because each names the variants it wants by design, as the 2 `let ... else` refusals show.
- Reading test enums and test files: rejected because a test's catch-all is its own oracle (`crates/readings/tests/states.rs:141`), and the 6 test enums store nothing.
- A parser built on a Rust syntax library: rejected because it adds a third-party dependency to the script tests for four text rules.

### D3

- A count per file: rejected because a count hides which arm moved, and a new arm beside a removed one keeps it.
- A total over the tree: rejected for the same reason, across files.
- Reading doc-comment examples: rejected because a doctest is a test, as `crates/kernel/src/verdict.rs:29` is.
- `unreachable!` alone: rejected because `panic!`, `todo!` and `unimplemented!` in an arm claim the same impossibility.

### D4

- A cover naming the enum's file: rejected because it reads the wiring `Step` as covered by a model of the sensitive-deck gate, which never names it; that reading counts 6 covered at `b25ed04a`.
- A cover of any fn returning the enum: rejected because a new variant need not change that fn's text, so the model is never told.
- A count seeded from the tree rather than a list: rejected because a newly covered enum's place would hide a new uncovered one.

### D5

- One combined module for the four rules: rejected because it gives one examined count and one red for four rules the promotion law counts apart.
- clippy's `wildcard_enum_match_arm` as the home of FSM-WILDCARD: rejected for D2's reason.
- A database trigger as the home of FSM-CHECK-LIST: rejected for D1's reason.
- A Rust test in a crate: rejected because the rules read every crate, the migrations and `formal/`, which no one crate owns.
- A loader importing one module from another by path: rejected because it is a dynamic import `test_ci_workflows.py` must then list.
- A lexer shared with the other Rust text guards now: rejected because each is still landing, and sharing one is a decision of its own.

### D6

- Curing a hit in this delivery: rejected because a cure is a product change owing its own SPEC and tests, which a lint delivery must not carry.
- Landing at fail with today's hits listed: rejected because no rule's precision has been read yet.

### D7

- A kill window in days: rejected because merges, not days, give a rule hits to read.
- Demotion by a plain register edit: rejected because a demotion is a weakening.
- Promotion on hit count alone: rejected because a noisy rule reaches 5 hits first.

### D8

- Building the transition tables with these rules: rejected because each table is a design per enum, and these four rules need none.

### D9

- One push: rejected because the tests read paths read only in CI, so their red can be read only after a push, and the green must follow it.
- A TLA+ model of the rules: rejected because no two actors share state: each module reads committed files in one process and writes nothing.
- A Lean proof of the register's admission rule: rejected because the rule is membership in a frozen literal set or a cited file that exists, which a fixture population with literal verdicts decides; the schema fold is SQLite's own.
- Amending the SPECs that own the enums: rejected because no requirement they own changes.

## Consequences

- A new variant on a paired enum, stored without its CHECK list following, prints a drift line in
  the next pull request's `hygiene` log.
- A new state enum with no formal anchor prints a `new` line, so the first models to write are
  named by the tree.
- Each rule costs one walk of `crates/*/src` (340 files at `b25ed04a`) or of the 33 migrations in
  CI's script-test step.
- Four modules and a helper join `scripts/tests`, and the name placement test in
  `test_ci_workflows.py`
  reads each.

## Confirmation

SPEC-433's acceptance criteria decide each rule. The red-first record, `docs/red-first/SPEC-433.md`,
holds each criterion's red and green. The band's rows are proved in CI by their own ids.

## What would make this wrong

- SQLite's stored `CREATE` text failing to carry a column's CHECK after an `ALTER TABLE ... ADD
  COLUMN` would leave a column unread. `ingest_state.refused_reason` is that case at `b25ed04a`,
  and the tree test reads it.
- A state enum stored in more than one column would need two pairs; none is at `b25ed04a`.
- A promotion reader that cannot tell a `listed` line from a `new` one would count a listed site
  every iteration.

## More Information

- SPEC-433 holds the requirements, the acceptance criteria, the manifest and the rows.
- The schematic is `docs/schematics/state-machine-lints.md`.
