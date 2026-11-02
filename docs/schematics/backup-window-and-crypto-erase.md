# Backups keep at most seven days, and an erase destroys the learner's key first

Decided by ADR-603 for SPEC-584. Every `path:line` below was read at this delivery's base on `dev`. Kinds:
a component diagram of the backup layers before and after, and two sequences, the erase and the
restore. "Later" marks a step a later delivery builds (SPEC-584 section 9); everything else is
true at `dev` or is the text this delivery changes.

## 1. The backup layers, before

```mermaid
flowchart LR
  db[("the database<br/>one file on the data disk")]
  replicator["the replicator<br/>snapshot every 24 h, kept 48 h<br/>deploy/litestream.yml:9-11"]
  replica[("the replica bucket<br/>soft delete off<br/>host identity may delete")]
  daily["the daily copy<br/>newest three on the host<br/>deploy/scripts/backup.py:43"]
  syncstore[("the sync server's store")]
  archive["the sync archive<br/>newest three on the host<br/>sealed to the owner's offline key"]
  offsite[("the offsite bucket<br/>create-only identity<br/>rule deletes at 30 days")]
  db --> replicator --> replica
  db --> daily
  syncstore --> archive --> offsite
```

The declared window is `P3D` (`privacy.json:450`); the offsite period is `P30D`
(`ADR-351:125-132`). No copy is encrypted per learner.

## 2. The backup layers, after

```mermaid
flowchart LR
  stores[("each learner's store<br/>encrypted under that learner's data key (later)")]
  shared[("the shared records and the directory of accounts")]
  keys1[("key store beside the server<br/>one wrapped key per learner<br/>no history, never backed up")]
  keys2[("key store in a second region<br/>the same rows")]
  wrap["the key service<br/>one wrap key"]
  replicator["the replicator<br/>snapshot every 24 h<br/>its own retention off (later)"]
  replica[("the replica bucket<br/>rule deletes at 6 days<br/>create and read only")]
  daily["the daily copy<br/>newest six, none older than 6 days (later)"]
  disk[("disk snapshots of the data disk<br/>one a day, deleted at 6 days<br/>no key on the disk")]
  second[("the second-region bucket<br/>shared records and directory<br/>rule deletes at 4 days<br/>create-only identity")]
  archive["the sync archive<br/>newest three on the host, sealed"]
  offsite[("the offsite bucket<br/>rule deletes at 6 days<br/>create-only identity")]
  stores --> replicator --> replica
  stores --> daily
  stores --> disk
  shared --> second
  archive --> offsite
  keys1 -.-> wrap
  keys2 -.-> wrap
```

The second-region copy is pruned at 4 days old and every other copy at 6, so no copy is older than
7 days (`P7D`). Soft delete is off and no retention lock is set anywhere. The key stores are the only place a wrapped key lives.

## 3. The erase, from the learner's request to the bytes gone

```mermaid
sequenceDiagram
  participant L as the learner
  participant S as the server
  participant K1 as key store beside the server
  participant K2 as key store in a second region
  participant C as every backup copy
  L->>S: erase my data
  S->>K1: delete my wrapped key (later)
  S->>K2: delete my wrapped key (later)
  K1-->>S: deleted
  K2-->>S: deleted
  Note over C: from here every copy of my data is ciphertext with no key (later)
  S->>S: erase every port's rows in one transaction (crates/privacy/src/erase.rs:46)
  S-->>L: erased
  Note over C: each copy is deleted by its own rule within 6 days, so all are gone within 7 days
```

Until the keys exist, the erase is the transaction alone, and an erased row stays readable in a
copy until that copy is gone; the page then claims no crypto-erase (SPEC-584 R7).

## 4. A restore, through the wrap key

```mermaid
sequenceDiagram
  participant O as the owner
  participant S as the server
  participant B as a backup copy
  participant K as a key store
  participant W as the key service
  O->>S: restore a learner's store to a point in the last 5 days
  S->>B: restore the store to a private path
  S->>K: read the learner's wrapped key (the second-region store if the first is lost)
  K-->>S: the wrapped key, or none for an erased learner
  S->>W: unwrap it with the server's identity
  W-->>S: the data key
  S->>S: open and check the store
  Note over S: an erased learner has no wrapped key, so the store stays ciphertext
```

The weekly restore rehearsal runs this with a synthetic rehearsal learner and a synthetic erased
learner (SPEC-584 R8, later, after the park lifts).

## 5. Each step, its test and its row

| step | held by | red at the base | row |
|---|---|---|---|
| the declared window is `P7D` | A1 (`test_backup_window`) | the inventory reads `P3D` | `S58401` |
| the replica and daily-copies bullets state `P7D` and today's ages | A1; CI's `test_backup_units` window test | the bullets state `P3D` | `S58401` |
| the disk snapshots and the second-region copy are disclosed with `P7D` | A4 | neither bullet exists | `S58404` |
| the offsite archives' period is `P7D` | A2 (`test_privacy_policy`) | the bullet states `P30D` | `S58402` |
| the runbook checks the bucket rule with `P7D` | A3 (`test_sync_server_runbook`) | the runbook states `P30D` | `S58403` |
| every record that stated an older period carries its amendment | A5 | no amendment exists | `S58405` |
| the page claims crypto-erase exactly when the key table exists | A6 | not red: a guard with two plants | `S58406` |
| the key-first erase, the key stores, the restore through the wrap key | the key-layer delivery (later) | its own reds | its own band |
| the prune of each layer at its age (6 days, the second-region copy 4), the disk snapshots, the second-region copy | the deploy delivery (later) | its own reds | its own band |
