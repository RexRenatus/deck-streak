# Schematic: the cutover checklist, one item at a time, and the order after the owner's go

Kind: state machine and sequence. Read at DeckStreak `dev` (ADR-011, ADR-034, ADR-037,
docs/schematics/notification-router.md, docs/schematics/vault-write-paths.md,
docs/schematics/cron-fire-ledger-and-catch-up.md, docs/schematics/alert-and-slo-path.md). Added
by the W8 plan for SPEC-143, SPEC-144, SPEC-145 and SPEC-146; ADR-143, ADR-144 and ADR-145 decide
it. Every step after the go names the owner gate #164. DeckStreak never stops the predecessor:
the predecessor stops, and the private rail (#41) carries it out; DeckStreak records that it did.

## One item of the checklist

An item is a notification kind, a vault contract or a job that DeckStreak and the predecessor
would both write. Its gates stay closed until it switches, so each contract has one writer at every
step (ADR-011). Only one item is in flight (stopped or switched, not yet verified or reverted).

```mermaid
stateDiagram-v2
  [*] --> waiting
  waiting --> stopped: stopped, after the go, the rail having stopped that writer
  stopped --> switched: switch, every gate of the item opened
  switched --> verified: verify passes, the output's sha256 recorded
  switched --> reverted: revert, every gate closed first
  reverted --> stopped: stopped again, once the rail restarted and stopped the writer
  verified --> reverted: revert on the owner's decision after a failed or void day, gates closed first
  verified --> [*]
```

A refusal writes nothing: `no_go` for a stop before the go, `go_recorded` for a second go,
`unknown_item` for an item the checklist does not list, `not_movable` for a stop of a
DeckStreak-only item or a verified one, `in_flight` for a stop while an item is in flight,
`not_stopped` and `not_switched` out of order, and `output_exists` for a verification whose output
file exists. A verified item reverts only on the owner's decision (#164). A switch set by hand
before its item switched is drift, and a drift fails every verification.

## What a closed gate holds

```mermaid
flowchart LR
  gate{"the item switched?"}
  kind["kind: a notification kind's switch"] --> gate
  duty["duty: a staged vault duty"] --> gate
  job["job: a job of the table"] --> gate
  arch["archive: the readings archive switch"] --> gate
  gate -- "no" --> held["held: the router withholds, the duty does not start, the runner returns before claiming, the archive writes nothing"]
  gate -- "yes" --> runs["runs, and its runs are the item's evidence"]
```

## The order after the go (#164)

```mermaid
sequenceDiagram
  participant O as the owner
  participant R as the private rail
  participant C as deckstreakd cutover
  participant I as deckstreakd import
  participant S as the SLO day report
  O->>C: go, the owner's reference recorded
  loop each moving item, one at a time
    R->>C: stopped, the predecessor's writer of that contract stopped
    O->>C: switch
    O->>C: verify, pass recorded by its digest
  end
  R->>C: retired, the predecessor stopped as a whole and kept disabled, not removed
  O->>I: the dry run of the final copy, then the apply over a verified backup
  O->>S: the report over the first full day after the apply
  O->>C: alone, the passing report's digest recorded
  O->>O: v1.0.0 from main, then the read-only release check
```

A failed or void day is the owner's decision (#164): the ways back are the import's rollback and
an item's revert. The predecessor's removal waits for the owner's own approval.
