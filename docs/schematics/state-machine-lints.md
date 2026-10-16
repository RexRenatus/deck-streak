# State-machine lints: the four rules' data flow

Path: `docs/schematics/state-machine-lints.md`. Kind: data flow. Every `path:line` below was read
at dev `b25ed04a`. Decided by ADR-447 for SPEC-433.

## 1. Where the rules run

Each rule is a test module under `scripts/tests`. `scripts/check.sh`'s python stage runs
`python3 -m unittest discover -s scripts/tests -p 'test_*.py'` and prints every suite's output
(`scripts/check.sh:185-188`), and the `hygiene` job runs that stage
(`.github/workflows/ci.yml:351`). So every `FSM-LINT` line lands in the `hygiene` job's log, where
the promotion reader counts it.

```mermaid
flowchart LR
  REG["config/fsm-lints.json<br/>levels, pairs, unpaired, listed"]
  HELP["scripts/tests/_fsm_lints.py<br/>masker, item and match reader,<br/>schema reader, four rules, printer"]
  T5["test_fsm_check_list.py"]
  T2["test_fsm_wildcard.py"]
  T3["test_fsm_unreachable.py"]
  T50["test_fsm_uncovered.py"]
  TS["test_fsm_lints.py<br/>shared behaviour"]
  STAGE["check.sh python stage<br/>unittest discover"]
  LOG["hygiene job log<br/>FSM-LINT lines"]
  REG --> HELP
  HELP --> T5
  HELP --> T2
  HELP --> T3
  HELP --> T50
  HELP --> TS
  T5 --> STAGE
  T2 --> STAGE
  T3 --> STAGE
  T50 --> STAGE
  TS --> STAGE
  STAGE --> LOG
```

## 2. FSM-CHECK-LIST

Population: every `migrations/*.sql` (33 files), executed in filename order into one in-memory
SQLite database. The schema is read from `sqlite_master` once all have run. Matcher: the
whole-body form `CHECK (<column> IN (<literal>, ...))`, read with comments and literals masked.
Examined: the domain lists of the final schema, at least 54 at `b25ed04a` (42 text, 12 integer);
59 written lists fold to them, and `migrations/004102_notifications_queue_claim.sql` rewrites 5.

```mermaid
flowchart TD
  MIG["migrations/*.sql<br/>filename order"] --> DB["in-memory SQLite"]
  DB --> MASTER["sqlite_master<br/>final CREATE text per table"]
  MASTER --> DOM["whole-body CHECK IN lists<br/>per table.column"]
  DOM --> STATECOL{"text list, name is or ends in<br/>state status outcome verdict<br/>phase stage step mode?"}
  STATECOL -- "no" --> SKIP["not a state column"]
  STATECOL -- "yes" --> PAIRED{"in pairs?"}
  PAIRED -- "no" --> UNP{"in unpaired?"}
  UNP -- "yes" --> L1["listed"]
  UNP -- "no" --> N1["new"]
  PAIRED -- "yes" --> CMP["compare with the enum's spellings"]
  RS["the enum's file<br/>fn arms or snake case,<br/>less unstored"] --> CMP
  CMP -- "equal" --> OK1["no hit"]
  CMP -- "differs" --> D1["drift hit:<br/>missing and extra spellings"]
  CMP -- "absent table, column,<br/>enum, fn or literal" --> V1["VOID"]
```

Sites at `b25ed04a`: 9 state columns; 7 pairs, each equal (`sync_runs.status` with `RunStatus`,
`chests.state` with `ChestState`, `skip_days.state` with `SkipState`, `reading_topic_days.state`
with `TopicState`, `cron_fires.last_outcome` with `Outcome`, `reading_runs.outcome` with
`RunOutcome`, `agent_runs.verdict` with `Verdict`); 2 listed unpaired (`notification_queue.state`,
`inbox_captures.state`); 0 drift.

## 3. FSM-WILDCARD

Population: every `.rs` file under `crates/*/src` (340), masked of comments, literals and items
under `#[cfg(test)]` (27 product files hold one). State enums: those declared there whose names end
in the seven suffixes, 12 at `b25ed04a`. Matcher: an arm whose whole pattern is `_`, `_ if`, a lone
binding or `<binding> @ _`, in a match whose arms name the enum's variant. Examined: 13 matches at
`b25ed04a`.

```mermaid
flowchart TD
  SRC["crates/*/src/**/*.rs"] --> MASK["mask comments, literals,<br/>#[cfg(test)] items"]
  MASK --> ENUMS["state enums:<br/>name, file, variants"]
  MASK --> MATCHES["match expressions and their arms"]
  ENUMS --> ON{"an arm names Enum::Variant,<br/>or Self::Variant in impl Enum?"}
  MATCHES --> ON
  ON -- "no" --> NOTON["not examined"]
  ON -- "yes" --> ARM{"an arm whose whole pattern<br/>is a catch-all?"}
  ARM -- "no" --> EX["exhaustive"]
  ARM -- "yes" --> KEY["key: path, enum, enclosing fn"]
  KEY --> LISTED{"in listed?"}
  LISTED -- "yes" --> L2["listed"]
  LISTED -- "no" --> N2["new"]
```

Sites at `b25ed04a`: 2 listed, `crates/readings/src/state.rs:302` (`TopicState::class`) and `:312`
(`TopicState::reason`).

## 4. FSM-UNREACHABLE

Population: FSM-WILDCARD's masked product source. Matcher: a match arm whose value is
`unreachable!`, `panic!`, `todo!` or `unimplemented!`, bare or first in its block. Examined: the
product files (340) and every match arm in them.

```mermaid
flowchart LR
  MASKED["masked product source"] --> ARMS["every match arm"]
  ARMS --> PANIC{"value starts with a<br/>panicking macro?"}
  PANIC -- "no" --> NONE["no hit"]
  PANIC -- "yes" --> K3["key: path, enclosing fn, macro"]
  K3 --> S3{"in listed?"}
  S3 -- "yes" --> L3["listed"]
  S3 -- "no" --> N3["new"]
```

Sites at `b25ed04a`: none. The two such macros under `crates/*/src` are masked:
`crates/kernel/src/verdict.rs:29` sits in a doc comment, and `crates/daemon/src/wiring.rs:1515`
sits under `#[cfg(test)]`.

## 5. FSM-UNCOVERED

Population: the product state enums (12) and every `@phx covers` line in the 55 modules under
`formal/` (251 at `b25ed04a`). Matcher: a cover whose path is the enum's file and whose anchor is
the enum's name.

```mermaid
flowchart TD
  ENUMS5["product state enums<br/>file and name"] --> COVER{"a cover line with path = file<br/>and anchor = name?"}
  FORMAL["formal/**<br/>@phx covers lines"] --> COVER
  COVER -- "yes" --> COV["covered"]
  COVER -- "no" --> L5{"in listed?"}
  L5 -- "yes" --> LST["listed"]
  L5 -- "no" --> NEW5["new"]
  COV --> WASL{"also in listed?"}
  WASL -- "yes" --> ST5["stale"]
```

Sites at `b25ed04a`: 1 covered, `Phase`, by
`formal/tla/MintReadsTheFinalBase/MintReadsTheFinalBase.tla:2`; 11 listed (`BandStep`,
`NotifyState`, `AnchorState`, `SkipState`, `RunStatus`, `ChestState`, `TopicState`, the insights
`State`, the passkeys `State`, the poll `Step` and the wiring `Step`).

## 6. Levels, the ratchet and VOID

```mermaid
stateDiagram-v2
  [*] --> warn
  warn --> fail: promotion law met, own delivery
  fail --> warn: signed ruling only
  state warn {
    [*] --> printed
    printed: new, listed, stale print, test passes
  }
  state fail {
    [*] --> checked
    checked: new or stale fails the test
  }
```

At either level, a reading that examined nothing, a walk whose count differs from an independent
listing, or a register entry outside its seed with no ruling file is VOID and fails. A listed entry
is deleted freely, and an entry outside the seed is added only with a `ruling` that names a file
under `docs/rulings/`.
