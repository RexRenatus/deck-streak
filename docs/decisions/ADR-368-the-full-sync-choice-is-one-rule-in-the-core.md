# ADR-368: the full-sync choice is one rule in the engine core, modelled first, that counts by ids, backs up the side it replaces and re-checks the server before an upload

- **Status:** proposed
- **SPEC:** SPEC-357 (issue #631; the app campaign's SPEC-334 row 1.4, R5 and R8, and its
  section 9)
- **Under:** ADR-337 (the owner's tap and its token: shown, then confirmed), ADR-340 (the offsite
  snapshot an upload checks for), ADR-356 (the core both clients share and its tables), ADR-058
  (the fork and its patch table) and ADR-361 (the OPFS media directory the sync fills).
  SPEC-342 measured the path (F1 to F6); SPEC-350 left the sync screens to this issue.

## Context and Problem Statement

ADR-337 decides that a one-way sync is an owner's tap, shown and then confirmed, carrying a token.
SPEC-334 R8 adds three guards: show what each side loses, write an on-device backup first, and
find the server's offsite snapshot before an upload. SPEC-342 measured what the engine does: the
offer (F1), one request per direction (F2, F3), the loss as a difference of ids (F4, F6), and a
window in which another client's sync is overwritten (F5). None of these records decides where the
guards live, how a client learns what each side holds, what the backup holds, how the window is
narrowed, where the snapshot answer comes from, how the unsynced warning is read, how the browser
reaches the sync server at all (the fork refuses it on wasm32), or what the model abstracts.
Both clients need the same answers: the web client now, the iOS client in IOS-3 (#633).

This record decides part a's rule: where it lives, how the counts are read, what the backup
holds, the re-check, the warning's read, the model's abstraction and the write's last check.
Where an upload's snapshot answer comes from, how the browser reaches the sync server, and how a
client stores its backups are decided when parts b and c are built (#631).

## Decision Drivers

- One copy for both clients: a rule both need is written once, where the engine they share lives.
- No review is lost while offline, and none is lost silently by a choice.
- The core stays the one client-side holder of the engine: no adapter passes SQL, and the core
  depends on no crate of this workspace (ADR-356 D4).
- The model comes first, and covers the rule it states.

## Decisions, and the alternatives each was chosen against

D5, D7 and D10 are not decided here: parts b and c decide them when they are built (#631).

### D1. The rule lives in the engine core, as a typed state machine

`crates/engine-core/src/full_sync.rs` holds the offer, the counts and the choice's states as types
(`Counted`, `Confirmed`, `BackedUp`, `Checked`, `Ready`, `Write`). Each transition consumes its
state, and only `Ready::at_write` makes a `Write`, which part b's one-way call consumes beside the
owner's gesture. The clients' adapters store files and run calls; their screens render.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The web page's TypeScript | the iOS client would write it a second time in Swift, and the page would hold engine logic |
| The web engine crate | it builds for wasm32 only, so the iOS client cannot reach it |
| A new workspace crate | it would need the engine's ids and a second database door, against ADR-356 D4's one holder |
| The native adapter crate | the web client cannot reach it |
| Runtime flags on one state value | a client could skip a step by setting a flag; a consumed type cannot be skipped |

### D2. The counts are id differences against a scratch copy of the server's collection

The core reads the device's review-log, card and note ids, and the same ids of a scratch copy of
the server's collection that the engine's own full download fetches into a file the adapter
stores (part b). `Losses::between` counts what the replaced side holds and the kept side lacks.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The device's count of unsynced rows | F6 refutes it: a review already synced is lost by a download once the server was replaced without it |
| A server route listing the collection's ids | a patch to the fork's sync server and a new protocol for one screen |
| A dry run of a normal sync | the engine refuses a normal sync while a full sync is required, and has no dry run |
| The counts the sync's `meta` answer carries | counts, not ids: they cannot say which rows differ |

### D3. The backup holds the side the write replaces, verified by ids

A download's backup is the device's collection; an upload's is the scratch server copy. The core
accepts a backup only when its ids hold every id of that side. Where a client stores its backups,
and how many it keeps, is part c's (#631).

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Always the device's collection | an upload replaces the server's side, so a device backup protects nothing it loses |
| No verification | a truncated or stale backup would pass |

### D4. The server is re-checked after the snapshot check, and a change returns to the counts

After the snapshot is found, the adapter fetches a fresh server copy; `Checked::rechecked`
compares its ids and modified stamp with the counted copy's. A difference returns a new `Counted`:
new counts, a new confirm, a new backup. The window left is the fetch's own duration before the
write.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| No re-check | F5: every sync during the confirm, the backup and the snapshot check would be overwritten uncounted |
| A server that refuses an upload after a change | it closes the window whole, but it is a fork patch to the sync server; recorded as the server's own surface (#617) |
| The server's sequence number alone | the scratch download itself advances it, and it says nothing about what changed |

### D6. The unsynced warning is one fixed read in the core

`Dispatcher::unsynced` reads, in one statement, the reviews whose sequence number marks them
unsynced, and whether the collection or its schema changed since its last sync. It needs no
network, so it answers offline.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A counter the page keeps | lost when the tab closes, and a second copy for the iOS client |
| The engine's sync status call | it takes the sync credential and may ask the server, so it does not answer offline |
| The browser's online flag | it says whether a network exists, not what is unsynced |

### D8. The model abstracts collections to sets of review ids

`formal/tla/FullSyncChoice` keeps the device, the server, the second client, its unsynced rows,
the backup, the server copy and the snapshot as sets of review ids. The choice is a program
counter; a second client's normal sync is atomic; a write is one step. Cards and notes follow the
same rule as reviews, so one kind of row stands for all three. The second client's own full sync
is outside the model: the model shows that it is forced, not what it chooses.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Collections with rows of three kinds | the state space triples and no property separates the kinds |
| Request-level steps (`meta`, `upload`, `download`) | SPEC-342 already proved each direction is one request; the order of the choice's steps is what is unproven |
| Modelling the second client's choice | it is a stock client's own choice, with its own counts on a DeckStreak client |
| SPEC-334 section 9's clause as written | "no normal sync between the check and the write" cannot be held by a client (F5); the model states what a client can hold |

### D9. The write re-reads the device; study is not held during the choice

`Ready::at_write` re-reads the device's ids at the write and refuses a download whose device side
gained a row its backup lacks. The owner may keep studying while a choice is open.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Holding study while a choice is open | a screen rule each client would write, and a Worker that ends mid-choice drops the hold |
| No check at the write | a review made after the backup would be lost by the download, silently |

## Consequences

- Part a touches no file the open study-screen work touches and needs no new dependency.
- IOS-3 reuses `full_sync.rs`, the reads and the model unchanged, and writes its adapter's storage
  and its screens.
- An upload changes the server's schema, so every other client makes a full sync of its own after
  it. That is the price of `AWindowSyncIsNotSilent`, and it is the engine's own behaviour.
- An edit to a row both sides hold is not counted; the choice screen says so.

## What would make this wrong

- A server patch that refuses an upload after a change would make D4's re-check a second guard
  rather than the only one.
- A measurement that a scratch download costs more than the owner accepts on a large collection
  would move D2 to a server route listing ids.
- An engine release whose full sync no longer changes the schema would void
  `AWindowSyncIsNotSilent` and needs the model re-read.
