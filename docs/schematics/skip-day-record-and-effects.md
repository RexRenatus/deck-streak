# Schematic: the skip day, recorded by DeckStreak, written to the collection on the owner's confirm, and read by every context it touches

Kind: data flow, sequence and state machine. Read at DeckStreak `dev` 53184dd and at the
predecessor's `27ee2bc` (`pipeline_layers/skip.py:SkipDaysLayer`, `sync.py:AnkiSyncer.skip_day`
and `AnkiSyncer.undo_skip`, `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` and
`_refund_skip_tariff`, `skip.py`, and every reader of `database.py:GamifyStore.skip_days_set`).
Added by SPEC-083, under ADR-083's option (a), the owner's decision at #266 (ADR-089).

It extends `docs/schematics/data-flow.md`, whose dashed arrow from coordination to the sync server
is the skip day's write. That arrow carries only the skip day's reschedule of the study day's due
review cards and its exact inverse, beside the settings a push carries whole (below), each pushed
by a normal (incremental) sync of a working copy
that is discarded afterwards. The private collection copy stays SPEC-022's: no step below writes
it, so it never holds a local change for another sync to send.

## Taking a skip

```mermaid
sequenceDiagram
  participant O as owner
  participant S as bot or Mini App
  participant C as coordination skip use cases
  participant A as analytics rollup
  participant I as ingest skip record and write
  participant W as working copy
  participant V as sync server
  participant E as economy wallet
  O->>S: skip, or cheat
  S->>C: preview
  C->>A: the study day's due review count, absent when the day has no rollup yet
  C->>I: an active skip today, the skips applied this month, the cards to move and their digest
  C->>E: the tariff's price from economy.json, and the balance
  C-->>S: due count, the cards to move, tariff, funded or not
  O->>S: confirm, carrying the digest
  S->>C: take
  C->>I: list the cards again, and stop with preview_changed when the digest is missing or differs
  C->>I: record the skip as pending, once per study day
  I->>W: copy the private copy beside it, under the exclusive collection lock
  W->>V: converge by one normal sync
  Note over W,V: a full or one-way sync demand aborts here, and nothing is written
  I->>W: check the engine's day and the configured UTC offset again, reading the offset without a day computation (the daylight-saving refusal and the pin's refusal were made before the request)
  Note over I,W: a day that is not the study day, or an offset that differs or is missing, fails the take here, before any card changes
  I->>I: commit each card's prior due date and prior state
  W->>W: Set Due Date on the previewed cards still due, with the day spec
  I->>I: commit the state the reschedule left each card in
  W->>V: push by a second normal sync, the moved cards and their review-log rows
  Note over W,V: a full or one-way sync demand aborts here too, and nothing was pushed
  I->>I: read each moved card back, record applied, and discard the working copy
  C->>E: purchase the tariff, clipped to the wallet, outside the daily loss cap
  C-->>S: cards moved, and every card left alone listed
```

The push also carries the collection's settings and creation stamp whole, because the working copy
is newer, each as the server held it at the converge, the engine's own last-unburied day aside
(SPEC-083 R23, R24). A preview, a take or an undo refuses before any request or write while the
engine's own day is not the study day, or while the collection's configured UTC offset is missing
or is not the process's zone, or while the process's zone observes daylight saving or is not pinned as a fixed rule (a POSIX rule that names no zone file) by `TZ` in the service's environment, and the take and the undo check the first two again on the converged working
copy before any card changes, so a converge that brings another client's setting that fails either
ends the take before its push and the undo with nothing written (R3). A take that aborts, or fails
before its push's first request, records the skip
as `failed` with its reason, discards the working copy and charges nothing. A take whose push fails
after its first request answers that its outcome is not known yet, never that nothing was written,
and its row stays `pending` (R25). A take whose write outlives the answer's wait answers `pending`,
and the write's own outcome settles the row; a row still `pending` is settled after the private
copy's next sync (R26).

## The skip's states

```mermaid
stateDiagram-v2
  [*] --> pending: the owner confirms the previewed cards
  pending --> applied: the server accepts the push, or no card is due
  pending --> failed: a full-sync demand, or any failure before the first request of the push
  applied --> undone: the undo is accepted, or has no card to write
  failed --> [*]
  undone --> [*]
```

Only `applied` rows not undone are in the skip set. A `failed` row leaves its study day free, and
an `undone` one does too.

## What reads the skip, at the next recompute

```mermaid
flowchart LR
  rec[("skip_days, owned by ingest")] --> port{{"the skip set: study days with an applied skip not undone"}}
  port --> ls["language streak: bridged, no freeze consumed"]
  port --> law["law streak: bridged"]
  port --> run["consistency run: left unchanged"]
  port --> asc["Ascendant: never armed on the day"]
  port --> gov["governor: the silent run neither counts nor ends at the day"]
  port --> qs["quests: the day's quests voided, never failed, SPEC-080"]
  port --> ch["chests: none rolled on the day, SPEC-081"]
  port --> gh["ghost race: a week of two or more skip days is exempt, SPEC-080"]
  port -.-> w5["discipline and evening nudges: read the same set in their wave"]
```

Each reader holds its own rule and proves it in its own SPEC; the skip set is the one port they all
read, through coordination, so no context queries `skip_days` a second way.

## Undoing a skip

```mermaid
flowchart TD
  undo["undo, asked for by the owner in the bot or the Mini App"] --> pend{"is the study day's take pending?"}
  pend -- "yes" --> wait["refused: the take's outcome is not known yet, and nothing is requested or written"]
  pend -- "no" --> latest{"an applied skip not undone?"}
  latest -- "no" --> none["nothing_to_undo"]
  latest -- "yes" --> ask["ask, naming the study day it undoes, and the owner confirms"]
  ask --> check{"the engine's day the study day, the configured UTC offset present and the process's zone, and the zone pinned as a fixed rule, a POSIX rule that names no zone file, without daylight saving?"}
  check -- "no" --> refused["refused with its reason, before any request or write"]
  check -- "yes" --> conv["copy the private copy, then converge by one normal sync"]
  conv --> full1{"a full or one-way sync demand?"}
  full1 -- "yes" --> abort["abort: nothing pushed, the skip stays applied, no refund"]
  full1 -- "no" --> recheck{"the day and offset checks again, on the converged working copy?"}
  recheck -- "no" --> held["ended with its own reason: nothing written or pushed, the skip stays applied, no refund"]
  recheck -- "yes" --> each["each card the skip moved"]
  each --> same{"still as the skip left it, with no study event since?"}
  same -- "no" --> listed["left alone, and listed to the owner"]
  same -- "yes" --> restore["restore its recorded prior due date and prior state"]
  restore --> push["push the restored cards by a second normal sync, when there are any"]
  listed --> push
  push --> full2{"a full or one-way sync demand?"}
  full2 -- "yes" --> abort
  full2 -- "no" --> lost{"did the push fail after its first request?"}
  lost -- "yes" --> unknown["the outcome is not known yet, and the skip stays applied"]
  lost -- "no" --> readback["read each restored card back, and list each one that no longer equals what the undo wrote or holds a review since the converge, its review-log row kept"]
  readback --> mark["mark the skip undone, at the undo's instant"]
  mark --> refund["credit what it paid, on the undo's study day"]
  mark --> next["next recompute: the set no longer holds the day"]
  next --> miss["the day is a missed day wherever a rule counts one"]
  next --> kept["transitions already settled stay as they were"]
```

The skip may be taken again on the same study day after an undo: the migration's key allows one
skip `pending` or `applied` and not undone per study day, not one row. The review-log rows the
reschedule wrote stay after an undo, because an incremental sync carries none away, and the read
never counts them as study events (SPEC-023 R2).
