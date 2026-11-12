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
  C->>E: debit the tariff, floor-clipped, outside the daily loss cap, keyed to the skip's own study day
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

## Amended by SPEC-083 section 10 (2026-10-03)

Read at DeckStreak `dev` 8fba25c. SPEC-083 lands in three parts (ADR-321): E4a the record, the
preview without its card list, the tariff and its refund, and the skip set's port with its seven
readers; E4b the take's write with its backup, restore drill, counts and the class's stop; E4c the
undo, the use cases, the bot, the API, the Mini App and the daemon's wiring. The diagrams below are
drawn before E4b's code. Where they differ from the ones above, section 10 governs.

### The settlement: the charge and the refund, once per skip (T3, T4; E4a)

```mermaid
flowchart TD
  take["the take's outcome, or the start-up settlement of a pending row"] --> applied{"applied?"}
  applied -- "no" --> failed["settle failed: no charge"]
  applied -- "yes" --> tx["one write transaction, BEGIN IMMEDIATE"]
  tx --> count["count the month's other applied skips not undone on an earlier study day"]
  count --> price["price: the ladder from economy.json at that count, clamped to its last entry"]
  price --> debit["floor-clipped debit, source skip_tariff, the skip's id, the skip's own study day"]
  debit --> settle["settle applied, unfunded when paid is below the price"]
  undo["the undo, accepted"] --> utx["one write transaction"]
  utx --> mark["mark undone at the undo's instant"]
  mark --> paid["read what the skip paid, by source and the skip's id"]
  paid --> refund["refund, source skip_tariff_refund, the skip's id, the undo's recorded study day"]
  refund --> commit["commit"]
```

A retry on any later study day finds the same ledger key `(study_day, source, reference)`, so the
debit and the refund each move coins once. The skip set's port, `skip/days.rs`, is the one reader
of `skip_days` in coordination: the streaks step twice, the XP step, the day bonuses, the streak
view and the level view call it, and the open lapse takes the set its caller read from it.

### The class's backup, restore drill and counts (R34, R35; E4b)

```mermaid
sequenceDiagram
  participant I as ingest skip write
  participant W as working copy
  participant B as backup beside the private copy
  participant T as throwaway collection
  participant L as ledger
  I->>L: read the class's stop, and refuse writes_stopped when it is set
  W->>W: converge by one normal sync
  I->>W: count the cards, the notes, the review-log rows, each queue and type pair, and the due count the wrapped search selects
  I->>W: close the converged working copy
  I->>B: copy it whole, mode 0600, named for the skip
  B->>T: copy the backup to a throwaway collection
  I->>T: open it with the engine, and count it the same way
  Note over I,T: a backup not written, or counts that differ, fails the take before any card changes
  I->>B: replace the last backup only now, after this one's check passed
  I->>L: commit each card's prior state
  W->>W: Set Due Date on the previewed cards
  I->>W: count again
  Note over I,W: only the review-log rows, up by the cards moved, and the due count, down by them, may move
  I->>L: any other count moved: set the class's stop, write nothing, push nothing
  I->>L: read the class's stop again before the push
  W->>W: push by a second normal sync
```

The backup is host-only: never in a bucket, never in Litestream's replica and never in ADR-064's
daily copy. DeckStreak never restores it; the owner restores it by the owner's own import, because
a restore reaches the server only by a full upload. The data-rights erase removes it, and the
export leaves it out.

### The class's stop (R36; E4b, E4c)

```mermaid
stateDiagram-v2
  [*] --> clear
  clear --> stopped: a count moved, recorded with who and why
  clear --> stopped: the owner's command, after a confirm
  stopped --> clear: only the owner's authenticated command handler, after a confirm
  stopped --> stopped: a take or an undo refuses writes_stopped
```

While the stop is set nothing writes, an undo included, and the preview says so.

### Amended by E4b (2026-10-03): the fourth hook, the stop's fail-closed read and the backup's erase

Drawn before E4b's code, at E4a's committed head `2669bc8a`, and appended: nothing above is edited.
The take runs its steps on a row the caller began, under the exclusive collection lock; every
refusal settles that row `failed` with one code and writes neither the collection nor a request,
and every exit discards the working copy. Its test seam has four hooks, each a no-op in production.

```mermaid
sequenceDiagram
  participant I as ingest skip write
  participant P as private copy
  participant W as working copy
  participant B as backup beside the private copy
  participant L as ledger
  participant S as sync server
  I->>L: read the stop, a missing row read as set: writes_stopped
  I->>P: the pin, the zone, the offset and the engine's day, then the list and its digest
  P->>W: copy
  W->>S: converge, one normal sync
  Note over I,W: hook after the converge
  I->>W: the offset and the engine's day again, then the moved set and counts C0
  W->>B: the partial backup, mode 0600
  Note over I,B: hook after the backup's write, before its restore check
  I->>B: the restore check, then rename and remove the older backup
  I->>L: each moved card's prior state
  Note over I,L: hook after the snapshot's commit
  I->>W: Set Due Date, counts C1, then the state each card was left in
  Note over I,W: hook before the push
  I->>L: read the stop again: writes_stopped
  W->>S: push, one normal sync
```

```mermaid
stateDiagram-v2
  [*] --> read
  read --> clear: the row is present and not stopped
  read --> stopped: the row is present and stopped
  read --> stopped: the row is missing, read fail-closed
  clear --> [*]: the take goes on
  stopped --> [*]: writes_stopped, nothing written
```

The backup's erase is ingest's `erase_backups` over the private copy's directory: it removes every
skip backup and partial backup there and no other file. E4c calls it from both of the erase's
callers, the bot's delete and the daemon's erase role, after the ledger's erase commits; the
ledger's own erase keeps the stop's row, which is exempt.

```mermaid
flowchart LR
  E[the ledger's erase commits] --> X[erase_backups over the private copy's directory]
  X --> R1[every skip-backup file removed]
  X --> R2[every partial backup removed]
  X --> K[every other file kept]
```
