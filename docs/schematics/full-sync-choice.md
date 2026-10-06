# Schematic: the full-sync choice and the web sync screens

SPEC-357, ADR-368, issue #631. Every `path:line` below was read at DeckStreak `dev` `adb19bbd`,
and every engine path in the fork at its pin, commit `c538de55`. Parts are marked (a) to (d): part
a builds the model, the rule and the core's reads; parts b to d are excluded in SPEC-357 section 5
(#631).

## 1. Components

The page renders and forwards the owner's taps; the Worker owns the session, its Web Lock and its
storage; the core owns every rule both clients share; the model covers the core's rule.

```mermaid
flowchart LR
  subgraph Page["web page (web/app/src)"]
    Study["study page: session start and end (c)"]
    Status["unsynced status, role=status (c)"]
    Screen["choice screen: counts, Cancel focused, confirm (c)"]
  end
  subgraph Worker["dedicated Worker, one Web Lock (session.ts:7)"]
    Session["Session: lock, storage, engine (session.ts:85)"]
    Ops["protocol OPS (protocol.ts:7) + sync, choice, backup, write (c)"]
    subgraph Wasm["web engine module (crates/web-engine)"]
      Exports["wasm exports (wasm.rs) + sync exports (b)"]
      subgraph Core["engine core (crates/engine-core)"]
        Dispatcher["Dispatcher: table, run, read (dispatch.rs)"]
        Reads["id_sets and unsynced fixed reads (a)"]
        Rule["full_sync.rs: Offer, Losses, Counted to Write (a)"]
      end
      Engine["Anki engine (fork pin)"]
      Transport["wasm32 sync transport: synchronous XHR (b)"]
    end
  end
  subgraph Store["OPFS pool 'deck-streak' (wasm.rs:53-61)"]
    Col["collection.anki2"]
    Copy["server copy, scratch (b, c)"]
    Backups["two newest backups (c)"]
    Media["collection.media (d)"]
  end
  subgraph Origin["web origin"]
    Edge["edge: /anki-sync/ (SPEC-337 R4)"]
    SyncServer["sync server, its own user (ADR-351 D1)"]
  end
  Offsite[("offsite snapshot, sealed (ADR-340)")]
  subgraph Gates["CI and formal gates"]
    Rust["CI rust job: engine-core tests, rows"]
    WebJob["CI web-engine job: size budget (SPEC-338 M3)"]
    WebTests["CI web tests: page and Worker"]
    Formal["formal check: tla/FullSyncChoice"]
  end

  Study --> Ops
  Screen --> Ops
  Ops --> Session --> Exports --> Dispatcher
  Dispatcher --> Engine
  Rule --> Reads --> Dispatcher
  Exports --> Rule
  Engine --> Transport --> Edge --> SyncServer
  Engine --> Col
  Exports --> Copy
  Exports --> Backups
  Engine --> Media
  SyncServer -. "daily window, sealed" .-> Offsite
  Status --> Ops
  Formal -. "covers" .-> Rule
  Rust -. "tests" .-> Rule
  WebJob -. "bytes" .-> Wasm
  WebTests -. "tests" .-> Page
```

The iOS client (#633) replaces only the page, the Worker, the web engine and the OPFS pool with
its own screens, adapter (`crates/ffi`) and container; the core and the model are the same files.

## 2. A session's sync (part c, on parts a and b)

```mermaid
sequenceDiagram
  participant P as study page
  participant W as Worker session
  participant C as engine core
  participant E as engine
  participant S as sync server
  P->>W: open, then sync (session start)
  W->>C: run SyncCollection (1,5)
  C->>E: admitted on the web column (b)
  E->>S: synchronous requests
  alt online, normal sync
    S-->>E: changes
    E-->>W: done
  else offline or refused
    E-->>W: network error
    W-->>P: offline, keep studying
  else a full sync is required
    E-->>W: FULL_SYNC, FULL_UPLOAD or FULL_DOWNLOAD
    W-->>P: open the choice screen (section 3)
  end
  W->>C: unsynced read (a)
  C-->>P: reviews not synced, changed, schema changed
  P->>W: rate (each review)
  W->>C: unsynced read
  C-->>P: status updated
  P->>W: sync (session end, pagehide)
  W->>C: run SyncCollection (1,5)
```

A sync never blocks study: a refused sync leaves the collection as it was, and the status names
what is unsynced until a later sync succeeds.

## 3. The choice's states (part a's rule; the model's program counter)

```mermaid
stateDiagram-v2
  [*] --> Counted: engine answers a full sync, ids read, server copy read
  Counted --> Counted: confirm a direction not offered (refused)
  Counted --> Confirmed: owner confirms an offered direction
  Confirmed --> Confirmed: backup lacks an id of the replaced side (refused)
  Confirmed --> BackedUp: backup holds the replaced side
  BackedUp --> Ready: download_ready (download)
  BackedUp --> Checked: snapshot_found, found is true (upload)
  BackedUp --> BackedUp: snapshot not found (refused)
  Checked --> Ready: rechecked, fresh copy unchanged
  Checked --> Counted: rechecked, fresh copy changed (new counts)
  Ready --> Write: at_write, device side still backed
  Ready --> Counted: at_write, device gained a row the backup lacks
  Write --> [*]: run_one_way with the owner's gesture (b)
```

Cancel at any state before `Write` leaves both sides untouched. `Write` has no public constructor:
only `Ready::at_write` makes one.

## 4. An upload's order (the model's actions)

```mermaid
sequenceDiagram
  participant O as owner
  participant P as choice screen
  participant C as core rule
  participant A as adapter (Worker)
  participant S as sync server
  participant X as snapshot answer of part c
  participant B as second client
  A->>S: scratch full download (server copy)
  A->>C: device ids, copy ids
  C-->>P: counts per offered direction
  O->>P: confirm upload
  P->>C: confirm(Upload)
  A->>C: backup ids (the server copy, kept in the pool)
  C-->>A: BackedUp
  A->>X: is a sealed snapshot found
  X-->>A: found
  A->>C: snapshot_found
  B->>S: normal sync (a window row)
  A->>S: fresh scratch download
  A->>C: rechecked(fresh)
  alt the fresh copy differs
    C-->>P: new counts, confirm again
  else unchanged
    C-->>A: Ready
    A->>C: at_write(device ids)
    C-->>A: Write
    A->>S: upload (one request, schema changed)
    B->>S: next sync: full sync required, its rows kept on B
  end
```

The row the second client syncs before the re-check is counted (the fresh copy differs). A row it
syncs after the re-check and before the upload is overwritten on the server, stays on the second
client, and that client's next sync is forced to a full sync by the upload's schema change
(`rslib/src/collection/mod.rs:192-203`). That is `CountsCoverTheUpload` and
`AWindowSyncIsNotSilent`.

## 5. The model's shape (`formal/tla/FullSyncChoice`, part a)

| variable | holds |
|---|---|
| `local`, `server`, `other` | review ids on the device, on the server, on the second client |
| `pending` | the second client's reviews not yet synced |
| `backup`, `copy`, `snap` | review ids in the backup, the scratch server copy and the newest sealed snapshot (or none) |
| `pc`, `dir`, `counted` | the choice's state, its direction, and the ids the counts showed as lost |
| `held` | the ids the write's last check reads: the counted device side, and the backup's ids once the backup is accepted |
| `made`, `window`, `lostUncounted`, `lostUnbacked`, `needFull`, `snapFound`, `uploadedNoSnap` | history: every review made, rows the second client synced after the re-check, what a write removed uncounted or unbacked, whether the second client must make a full sync, whether this choice found a snapshot, whether an upload ran without one |

| action | step |
|---|---|
| `AReview(r)`, `BReview(r)` | a review on the device, on the second client |
| `BSync` | the second client's normal sync, atomic; disabled once it must make a full sync |
| `Snapshot` | the daily window seals the server's rows |
| `Count`, `Confirm(d)`, `Backup`, `SnapCheck`, `Rechecked`, `DownloadReady`, `AtWriteRefused`, `Write` | the choice, one step each, guarded by the switches below; `Rechecked` and `AtWriteRefused` return to new counts |
| `Cancel` | the owner's Cancel at any step before the write; nothing is written |
| `Finished` | stutter once the choice is done or refused |

| switch (TRUE in `MCFullSyncChoice.cfg`) | FALSE in witness | kills |
|---|---|---|
| `BackupFirst` | `a-download-before-its-backup` | `BackupBeforeReplace` |
| `SnapshotFirst` | `an-upload-with-no-snapshot-found` | `SnapshotBeforeUpload` |
| `Recheck` | `an-upload-with-no-re-check` | `CountsCoverTheUpload` |
| `SchemaBump` | `an-upload-that-keeps-the-servers-schema` | `AWindowSyncIsNotSilent` |
| `VerifyAtWrite` | `a-write-that-does-not-re-read-the-device` | `NoReviewLost` |
