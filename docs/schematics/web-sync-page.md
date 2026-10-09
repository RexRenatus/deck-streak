# Schematic: the web sync page, the Worker, the web engine, the core and the fork (SPEC-377)

Kind: sequence. Decided by ADR-388. The `path:line` citations were read at `dev` `f48a977c` and in
the engine fork at its pin `2cfa7047`; SPEC-377 section 1 gives the command for each.

Participants, the same in every diagram:

| id | is | where |
|---|---|---|
| `Pg` | the page: the study engine and the sync screen | `web/app/src/lib/study/engine.ts`, `web/app/src/lib/sync/` |
| `Wk` | the Worker: its session, its credential store, its sync and its choice | `web/app/src/lib/engine/` |
| `En` | the web engine: its exports, the pool's port and a choice's held stage | `crates/web-engine/src/wasm.rs`, `crates/web-engine/src/files.rs` |
| `Co` | the core: `one_way`, the dispatcher and the `Files` port | `crates/engine-core/src/` |
| `Fk` | the engine at the fork's new pin | the fork's `rslib/src/sync/` |
| `Sv` | the sync server | the service's sync route |
| `Ap` | the service's snapshot route (part c2; a test route stands in for it in part c1's browser tests) | `crates/api/src/` |

## 1. A session's two syncs (part c1, R7)

The page posts each sync and awaits neither; the Worker's one queue orders them with study.

```mermaid
sequenceDiagram
    participant Pg as page
    participant Wk as Worker
    participant En as web engine
    participant Co as core
    participant Sv as sync server
    Pg->>Wk: open
    Wk->>En: open
    En-->>Wk: existed and notes
    Pg-)Wk: sync, not awaited
    Wk->>En: sync_collection with the settled key
    En->>Co: the normal sync through the dispatcher
    Co->>Sv: the engine's own transport
    Sv-->>Co: no-changes, normal-sync or full-sync
    Co-->>En: status
    En-->>Wk: status word
    Wk-->>Pg: status word and the unsynced count
    Pg->>Wk: study requests, queued behind the sync
    Pg-)Wk: sync before close, not awaited
    Pg->>Wk: close
```

## 2. The choice, a download (part c1, R5, R9)

The page passes a direction and nothing else. Every path is a new pool name the web engine mints
after a reserve.

```mermaid
sequenceDiagram
    participant Pg as page
    participant Wk as Worker
    participant En as web engine
    participant Co as core
    participant Fk as engine at the pin
    participant Sv as sync server
    Pg->>Wk: choice-count
    Wk->>En: full_sync_count
    En->>En: reserve the pool and mint a new name
    En->>Co: one_way count into that name
    Co->>Co: fillable asks the Files port
    Co->>Fk: a private engine's full download
    Fk->>Sv: download
    Sv-->>Fk: bytes
    Fk->>Fk: deserialize, check integrity, set ls to mod, back up into the name
    Fk-->>Co: the counted server copy
    Co-->>En: Counted, held as the stage
    En-->>Wk: counts per offered direction
    Wk-->>Pg: counts
    Pg->>Wk: choice-confirm download, the owner's tap
    Wk->>En: full_sync_confirm download
    En->>Co: the one-way gesture, back_up into a new name, then write
    Co->>Co: VACUUM INTO the backup
    Co->>Fk: full download over the open collection
    Fk->>Fk: deserialize and replace by the backup interface, one transaction
    Fk-->>Co: written
    Co-->>En: written
    En-->>Wk: written
    Wk-->>Pg: written, with the backup kept
```

## 3. The choice, an upload (part c1, R5, R6, R9)

The Worker reads the snapshot answer itself; `unknown` refuses the upload as `not found` does.

```mermaid
sequenceDiagram
    participant Pg as page
    participant Wk as Worker
    participant Ap as snapshot route
    participant En as web engine
    participant Co as core
    participant Fk as engine at the pin
    Pg->>Wk: choice-confirm upload, the owner's tap
    Wk->>Ap: GET with the owner's session
    alt found
        Ap-->>Wk: found and age
        Wk->>En: full_sync_confirm upload, found
        En->>Co: back_up, snapshot_found, recheck
        alt the server changed
            Co-->>En: changed, with the reason
            En-->>Wk: back to the counts
            Wk-->>Pg: counts and a sentence saying why
        else unchanged
            Co->>Fk: full upload
            Fk->>Fk: serialize the closed collection
            Fk-->>Co: uploaded
            Co-->>En: written
            En-->>Wk: written
            Wk-->>Pg: written
        end
    else not found, refused or unreachable
        Ap-->>Wk: not found or no answer
        Wk-->>Pg: the upload waits for a snapshot
    end
```

## 4. A collection the browser evicted (part c1, R10)

```mermaid
sequenceDiagram
    participant Pg as page
    participant Wk as Worker
    participant En as web engine
    participant Co as core
    Pg->>Wk: open
    Wk->>En: open
    En-->>Wk: existed false
    Wk-->>Pg: the browser lost its copy
    Pg-)Wk: sync
    Wk->>En: sync_collection
    En-->>Wk: full-sync
    Pg->>Wk: choice-count
    Wk->>En: full_sync_count
    En->>Co: count
    Co-->>En: the download offered alone
    En-->>Wk: download alone
    Wk-->>Pg: restore from the server, on the owner's tap
```

## 5. The snapshot route (part c2, R12 to R14)

```mermaid
sequenceDiagram
    participant Wk as Worker
    participant Ap as snapshot route
    participant Ls as lister port
    participant Cm as list command
    Wk->>Ap: GET with the owner's session
    alt no owner session
        Ap-->>Wk: 401
    else over the rate window
        Ap-->>Wk: 429
    else admitted
        opt the cached answer is stale
            Ap->>Ls: list the archive
            alt no lister wired
                Ls-->>Ap: unknown
            else wired
                Ls->>Cm: arguments, no shell, bounded
                Cm-->>Ls: names, or a refusal or a timeout
                Ls-->>Ap: stamps with a sealed archive and its sealed manifest
            end
        end
        Ap-->>Wk: found and age, not found, or unknown, naming no object
    end
```
