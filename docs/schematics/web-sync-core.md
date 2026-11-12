# Schematic: the web client's sync and its full-sync write (`SPEC-364`)

Read at DeckStreak `dev` `dee337bc` and the engine fork at `c538de55`. Every `path:line` below is
from those trees. Parts b1, b2 and b3 are marked where a component arrives.

## 1. Components

```mermaid
flowchart LR
  subgraph Page["the page (web/app/src)"]
    study["study page<br/>session start and end"]
    persist["persistence.ts<br/>requestPersistence (b2: called each session)"]
    syncui["sync screens (part c)"]
    snaproute["snapshot answer (part c)"]
  end

  subgraph Worker["the dedicated Worker (web/app/src/lib/engine)"]
    serve["worker.ts serve<br/>own-origin messages only"]
    proto["protocol.ts OPS<br/>(b2: sync-login, sync; b3: full-sync ops)"]
    syncmod["sync.ts (b2)<br/>Login, sync, settle"]
    cred["credential.ts<br/>obtain / forSend / settle"]
    subgraph Engine["web engine, wasm32 (crates/web-engine)"]
      exports["wasm.rs exports<br/>(b2: sync_login, sync; b3: full_sync_*)"]
      stage["the choice's stage<br/>held between taps (b3)"]
      lists["study.rs<br/>STUDY_CALLS (run_method); SYNC_CALLS (b1)"]
      subgraph Core["engine core (crates/engine-core)"]
        table["table.rs<br/>(1,3) and (1,5) web; (1,6) exempt (b1)"]
        guard["login_guard.rs<br/>one endpoint rule, no media (b1)"]
        oneway["one_way.rs (b1)<br/>count, back_up, recheck, write"]
        fullsync["full_sync.rs<br/>the choice's types (part a)"]
        credrule["credential.rs<br/>the generation rule"]
        dispatch["dispatch.rs<br/>run, run_exempt; run_one_way (b1, crate-private)"]
      end
      subgraph Fork["the engine, pinned fork"]
        xhr["io_monitor.rs wasm32 twin<br/>synchronous XMLHttpRequest (b2)"]
        gates["runtime, IoMonitor, threads<br/>clock and thread gates (b2)"]
        files["upload.rs, download.rs<br/>serialize, deserialize, backup (b3)"]
      end
    end
  end

  subgraph Storage["the origin's storage"]
    idb["IndexedDB<br/>deck-streak-credential: the sealed record"]
    pool["OPFS pool, default VFS<br/>collection, its log, server copies, backups"]
  end

  server["the sync server<br/>own origin, /anki-sync/"]
  release["the service's release route<br/>/api/sync/seal-key"]

  study --> serve
  syncui --> serve
  snaproute --> syncui
  persist -. asks the browser .-> pool
  serve --> proto --> syncmod
  syncmod --> cred
  cred --> idb
  cred --> release
  cred --> credrule
  syncmod --> exports
  exports --> dispatch
  exports --> lists
  lists -. "parity: equals the web column" .- table
  exports --> stage --> oneway
  dispatch --> table
  dispatch --> guard
  oneway --> fullsync
  oneway --> dispatch
  dispatch --> Fork
  xhr --> server
  files --> pool
  Fork --> pool
```

What crosses each boundary:

| boundary | crosses | never crosses |
|---|---|---|
| page to Worker | operation names, the sync user and password once (`sync-login`), the owner's direction and the snapshot's `found` | the key, the host key, ids or paths |
| Worker to page | status words, the counts, the sync's required change | any key, any id set, any path |
| `sync.ts` to `wasm.rs` | the key and endpoint `forSend` answered, for one call | a stored copy of the key |
| core to engine | requests the core built or checked (`run`, `run_one_way`), fixed statements | a caller's one-way request bytes |
| engine to network | the engine's own request, the `anki-sync` header among its headers, to the page's own origin | any other origin (`connect-src 'self'`) |

## 2. The fork pin

```mermaid
flowchart LR
  fix["57382da<br/>ADR-058's fix"] --> ten["ten wasm32 patches<br/>c538de55, the current tag"]
  ten --> b2p["browser-xhr<br/>wasm-clock-threads<br/>(b2's new tag)"]
  b2p --> b3p["browser-full-sync-files<br/>(b3's new tag)"]
  ten -. "[patch] rev today" .- root["root Cargo.toml lines 163-171"]
  b2p -. "[patch] rev after b2" .- root
  b3p -. "[patch] rev after b3" .- root
```

Each step is a fast-forward of the pin branch with a new tag; no tag moves or is deleted while
pinned (ADR-348). Each patch is a `cfg` on `wasm32`; the native engine is the one pinned before.

## 3. A normal sync (part b2)

Part b2 as built is section 7: the export is `sync_collection`, and every send settles.

```mermaid
sequenceDiagram
  participant P as page
  participant W as Worker sync.ts
  participant C as credential.ts
  participant E as wasm.rs + core
  participant F as engine (fork twin)
  participant S as sync server
  P->>W: sync
  W->>C: forSend(origin + /anki-sync/)
  C-->>W: {generation, key} or a status word
  alt a status word
    W-->>P: the status word (absent, sealed, needs-sign-in, offline)
  else a key
    W->>E: sync(key, endpoint)
    E->>E: decide (1,5) web: Admit, guard the endpoint, sync_media false
    E->>F: run_service_method(1,5)
    F->>S: synchronous XMLHttpRequest, anki-sync header, zstd body
    S-->>F: zstd body + size header, or a status
    F-->>E: the answer, or the engine's error bytes
    E-->>W: required change, or error bytes
    W->>C: settle(generation, error bytes or none)
    C-->>W: status word (a 403 drops the key)
    W-->>P: status word + required change
  end
```

## 4. The full-sync write (part b3 over b1's driver)

```mermaid
sequenceDiagram
  participant P as page (part c)
  participant W as Worker
  participant E as wasm.rs (stage holder)
  participant O as core one_way.rs
  participant T as full_sync.rs types
  participant F as engine
  participant L as pool files
  participant S as sync server
  P->>W: full-sync-count
  W->>E: full_sync_count(key, endpoint)
  E->>O: count(answer, auth, new copy path)
  O->>F: private engine: open the empty copy, full download
  F->>S: download
  F->>L: copy written (one transaction)
  O->>O: read device ids and copy ids
  O->>T: Counted::show
  E-->>P: the counts (upload, download)
  P->>W: full-sync-confirm(direction) [the owner's tap]
  W->>E: full_sync_confirm: OwnerGesture::from_tap(OneWaySync)
  E->>T: Counted::confirm(direction)
  E->>O: back_up(confirmed, new backup path, copy path)
  alt download
    O->>F: VACUUM INTO backup (fixed statement, path bound)
    F->>L: backup file
    O->>O: read the backup's ids from the file
  else upload
    O->>O: read the counted copy's ids again from its file
  end
  O->>T: Confirmed::backed_up(ids read)
  alt download
    E->>T: download_ready
  else upload
    E-->>P: needs the snapshot answer
    P->>W: full-sync-snapshot(found)
    E->>T: BackedUp::snapshot_found (found false: refused)
    E->>O: recheck(checked, auth, new fresh path)
    O->>F: fetch the fresh copy
    O->>T: Checked::rechecked (changed: back to the counts)
  end
  E->>O: write(ready, gesture, auth)
  O->>O: re-read the device ids
  O->>T: Ready::at_write (a new device row: back to the counts)
  O->>F: run_one_way (crate-private): the core's own request, upload = direction, no media
  F->>S: upload or download
  F->>L: download: replace in one transaction
  E-->>P: written, or the new counts
```

## 5. The choice's states and who drives each step

```mermaid
stateDiagram-v2
  [*] --> Counted: count (core fetches the copy)
  Counted --> Confirmed: the owner's tap
  Confirmed --> BackedUp: back_up (core writes or reads the backup)
  BackedUp --> Ready: download_ready
  BackedUp --> Checked: snapshot_found (found)
  Checked --> Ready: recheck unchanged (core fetches fresh)
  Checked --> Counted: recheck changed
  Ready --> Written: write (core re-reads, at_write, run_one_way)
  Ready --> Counted: at_write refuses a new device row
  Counted --> [*]: cancel
  Confirmed --> [*]: cancel
  BackedUp --> [*]: cancel
  Written --> [*]
```

## 6. The gates

| gate | holds | where it runs |
|---|---|---|
| `table`, `login_guard`, `exempt`, `one_way` tests (b1) | the rows, the endpoint rule, the one-way refusal, each step's order against the engine's own sync server | the repository's `rust` job |
| `containment`, `graph` (b1) | `run_one_way` named outside the core only in the entry files; no new dependent of the core | the `rust` job |
| `parity` (b1), the web engine's `study` test (b1) | the web column equals `STUDY_CALLS` with `SYNC_CALLS`; `SYNC_CALLS` is exactly the login and the normal sync | the `rust` job |
| mutation rows `S36401` to `S36421` (b1) | each step's check against its killer | the `mutation-rows` job |
| `boundary` census (b2, b3) | each new web export's owed statements | the `rust` job |
| `credential-reach`, `sync.test.ts`, `engine.test.ts` (b2) | the key's one holder, settle after every send, persistence each session | the `web` job |
| the engine's browser tests (b2, b3) | the transport and the write in Chromium and WebKit against the engine's own sync server | the `web-engine` job, after the build and its size bound |
| `formal/tla/FullSyncChoice` | the order; every covered anchor's digest unchanged | the formal checker outside this repository's CI |

## 7. A normal sync as built (part b2)

A sequence, read at dev `a6a44fa6` and the fork's `c538de55`. The Worker's `sync` takes the key at
the send, the engine's export sends through the fork's synchronous transport, and the send settles
on every path (ADR-375 D14 to D19).

```mermaid
sequenceDiagram
  participant P as page
  participant W as Worker session
  participant S as sync module
  participant C as credential module
  participant E as web engine export sync_collection
  participant X as fork transport browser-xhr
  participant V as sync server
  P->>W: sync
  W->>S: the sync step, on the session queue
  S->>C: forSend
  C-->>S: key and generation
  S->>E: sync_collection(key, endpoint)
  E->>X: the engine's request, timeout the stall duration
  X->>V: synchronous request from the Worker
  V-->>X: answer
  alt response URL is not the request's
    X-->>E: MISDIRECTED_REQUEST, no source, read as Failed
  else non-success status
    X-->>E: the status source, downcast to error_for_status_code
  else success
    X-->>E: bytes, decoded by the size header
  end
  E-->>S: required, or the engine's error bytes
  alt success
    S->>C: settle(gen)
  else the engine's error bytes
    S->>C: settle(gen, bytes)
  else any other throw
    S->>C: settle(gen, empty bytes), then throw again
  end
  S-->>W: status and required
  W-->>P: reply
```
