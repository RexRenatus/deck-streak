# Schematic: the v9 import, from a verified copy to one apply and its way back

Kind: data flow and sequence. Read at DeckStreak `dev` (ADR-008, ADR-011, SPEC-020, SPEC-021,
docs/schematics/data-flow.md, docs/schematics/data-rights-export-and-erase.md,
docs/schematics/deployment.md) and at the predecessor's `27ee2bc` (its store's schema at
`user_version` 24, and the functions SPEC-140 §7 and SPEC-141 §7 prove). Added by the W8 plan for
SPEC-140, SPEC-141 and SPEC-142; ADR-140, ADR-141 and ADR-142 decide it. The import reads a COPY,
never the predecessor's live file: the private rail (#41) delivers the copy and its digest, and
nothing in the repository names where the predecessor keeps its database.

## The dry run, as often as the owner likes

It writes nothing live, so it runs side by side (#62), before the owner's go.

```mermaid
flowchart TD
  copy["the copy and its sha256, delivered by the private rail"] --> check{"sha256 equal, no -wal beside it, user_version 24, the snapshot's tables?"}
  check -- "any no" --> refuse["IMPORT PLAN REFUSED, by its reason, no row read"]
  check -- "yes" --> read["one read-only transaction on the copy, query_only on: SourceTables"]
  read --> vac["VACUUM INTO: a private copy of DeckStreak's database"]
  vac --> apply1["every owner's ImportPort write, in the plan's order, one transaction on that copy"]
  apply1 --> apply2["the same apply a second time: every pair unchanged, written 0"]
  apply2 --> integ{"integrity_check ok and foreign_key_check empty on the copy?"}
  integ -- "no" --> refuse
  integ -- "yes" --> report["one report line per source table, the unstamped count, IMPORT PLAN OK"]
  report --> rm["the private copy removed"]
```

## The apply, once, after the go and the retirement (#164)

```mermaid
sequenceDiagram
  participant O as the owner
  participant R as the private rail
  participant D as deckstreakd import
  participant L as the live database
  participant B as the backup
  O->>R: the go, recorded in the cutover ledger
  R->>R: every contract moved, then the predecessor stops and is retired
  R->>D: the final copy and its sha256, dry run first
  O->>L: DeckStreak's writers stopped, the backup daemon kept running
  O->>B: the backup restored from the replica, integrity_check ok
  D->>D: refused with not_retired unless the ledger holds retired
  D->>B: integrity_check ok and a per-table sha256
  D->>L: one write transaction, the per-table sha256 compared inside it
  D->>L: every owner writes its own rows, in the plan's order
  alt any refusal or a reconcile that differs
    D->>L: the whole transaction rolled back, nothing written
  else reconciled
    D->>L: commit, then integrity_check and foreign_key_check
  end
  O->>L: the writers started
```

## The rollback, by rename

```mermaid
flowchart TD
  start["deckstreakd import rollback, the backup named"] --> bchk{"the backup's integrity_check ok?"}
  bchk -- "no" --> no["refused, nothing renamed"]
  bchk -- "yes" --> aside["the live file and its -wal and -shm renamed aside, suffix .failed-import, kept for the owner"]
  aside --> swap["the backup renamed to the live name, in the same directory"]
  swap --> sync["the directory synced"]
  sync --> after{"integrity_check ok on the live name?"}
  after -- "yes" --> done["rolled back: no file copied, the replica's tracking reset by the runbook"]
  after -- "no" --> fail["exit 1, both files kept"]
```

## Who writes which rows

Each context writes its own rows through its own `ImportPort` (ADR-140): the migration crate
composes the ports and owns no table. A row the predecessor never recorded arrives NULL, never
invented (SPEC-140 R7), and a recovered card-state reading keeps its origin (ADR-141). The
reconcile judges each source table under the census's rule (`equal`, `filtered`, `sum`,
`declared`), and a table the plan drops is dropped with its reason (SPEC-142 R3).
