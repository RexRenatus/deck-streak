# Schematic: the nightly stats file, after the day's one sync, by one writer

Kind: data flow. Read at DeckStreak `dev` (ADR-011, ADR-027, ADR-037,
docs/schematics/cron-fire-ledger-and-catch-up.md, docs/schematics/vault-write-paths.md,
docs/schematics/sync-cycle-and-change-gate.md) and at the predecessor's `27ee2bc`
(`vault_bridge.py:build_vault_stats`, `_apply_degradation_signal`, `write_vault_stats`,
`unopened.py:read_unopened_rows`, `build_report`). Added by the W8 plan for SPEC-146; ADR-146
decides the slot. The job never syncs: it reads the study day's sync outcome (ADR-037), and it
writes nothing until its checklist item switches (SPEC-143).

```mermaid
flowchart TD
  timer["vault_stats at the rollover hour plus 12 minutes, after the sync's minute 7"] --> item{"the vault-stats item switched?"}
  item -- "no" --> held["returns before claiming: no fire row, nothing written"]
  item -- "yes" --> claim{"fire claimed for this study day?"}
  claim -- "already" --> none["nothing"]
  claim -- "claimed" --> lock["the collection lock taken shared, so a sync in flight ends first"]
  lock --> sync{"the study day's sync succeeded?"}
  sync -- "no" --> skip["skipped: nothing composed or written, the file kept, recorded ok"]
  sync -- "yes" --> first["the first blocks: day, language, law, leeches"]
  first -- "a read fails" --> err["error: the first of a streak pages, a repeat only logs"]
  first --> later["the later blocks, each on its own"]
  later --> subj{"subject table read?"}
  subj -- "no" --> dsub["law_subjects and drills named in degraded, weak subjects null"]
  subj -- "yes" --> drills["drill backlog, habits"]
  later --> weeks{"weeks read?"}
  weeks -- "no" --> dweeks["weeks named, history and trend cold"]
  weeks -- "yes, not empty" --> hist{"history and trend read?"}
  hist -- "no" --> dhist["weekly_history named"]
  later --> unopened{"the unopened read and projection ran?"}
  unopened -- "no" --> cold["the cold unopened block, never named"]
  unopened -- "yes" --> proj["each subject: too_short, non_stationary, limits_unknown, stalled, beyond_horizon or determined"]
  dsub --> signal["measured per block, stats_ok only when every block measured and nothing degraded, fail closed"]
  drills --> signal
  dweeks --> signal
  hist -- "yes" --> signal
  dhist --> signal
  cold --> signal
  proj --> signal
  signal --> write{"the file's path allowed?"}
  write -- "unset, root missing, folder missing, outside the root, rails refuse" --> err
  write -- "yes" --> atomic["sorted keys, indent 2, through a temp ending .tmp in the file's folder, renamed over it"]
```

The unopened read is ingest's: a read-only read of the copy inside the owner's scope, five sets,
and an unreadable copy is an error rather than an empty read. The projection is analytics', pure.
The composition, the degradation signal and the job are coordination's; the file's contract is
the vault's.
