# Schematic: one sync cycle, from the retrying sync to the change gate's decision

Kind: state machine. Read at DeckStreak `main` e05dfa5 (ADR-008, ADR-009,
`docs/schematics/data-flow.md`), and at the predecessor's `27ee2bc` for the behaviour it ports
(`sync.py:AnkiSyncer.sync_now`, `pipeline.py:GamifyPipeline._sync_attempts`,
`pipeline.py:GamifyPipeline._maybe_skip_recompute`, `pipeline.py:GamifyPipeline._first_due_obligation`,
`anki_reader.py:probe_change_signal`). Decided by ADR-009 and ADR-022; built by SPEC-022 (the sync)
and SPEC-023 (the read and the gate).

```mermaid
stateDiagram-v2
  [*] --> Locked: take the collection lock (exclusive)
  Locked --> Attempting: attempt n of SYNC_RETRY_ATTEMPTS
  Attempting --> Synced: incremental ok
  Attempting --> Downloading: server demands a full sync
  Downloading --> Synced: download complete, swapped in
  Attempting --> Refused: server holds no collection (full_upload_required)
  Attempting --> Waiting: error or SYNC_TIMEOUT_SECS passed, attempts left
  Waiting --> Attempting: golden backoff with jitter (tokio timer)
  Attempting --> Failed: error on the last attempt (one bounded reason code)
  Refused --> Recorded
  Failed --> Recorded: sync_runs error row
  Synced --> Probing: unlock, then the cheap probe under a shared lock
  Probing --> Deciding: newest review id, card count, fingerprint
  Deciding --> Recomputing: Run(reason)
  Deciding --> Skipped: Skip
  Recomputing --> Recorded: read the window, recompute, write a fresh anchor
  Skipped --> Recorded: sync_runs skipped row, anchor unchanged
  Recorded --> [*]
```

`decide` runs the recompute when ANY of these holds, in this order, and skips only when none does:

| term | source |
|---|---|
| an owner rescore is pending (consumed once) | `ingest_state` |
| this cycle's sync failed | the sync outcome |
| no successful run on record, or the last run failed | `sync_runs` |
| the anchor is missing or unreadable | `ingest_state` |
| the settings generation changed | the kernel's `settings_generation` |
| the study day changed | the kernel's study-day rule and clock |
| the newest review id, the card count or the fingerprint changed | the probe |
| a registered deadline lies after the anchor's last recompute and at or before now | `coordination::obligations` |

The last term is the one an input-keyed gate cannot see: an obligation comes due precisely on a
cycle in which nothing in the collection changed. Every deadline-bearing feature registers its
source there in the delivery that builds it.

## The engine port and its measurement

Kind: component, then sequence. Built by SPEC-022's spike (A1 to A3), decided by ADR-009 and
ADR-022, read at Anki's tag `26.05` (`rslib/src/sync/collection/normal.rs`,
`rslib/src/sync/collection/download.rs`, `rslib/src/sync/http_server/mod.rs`).

```mermaid
flowchart LR
  others[every other context] -->|ingest's own types only| port
  subgraph ingest[deck-streak-ingest]
    port[[AnkiEngine: new_card_queue, normal_sync, full_download]]
    adapter[RslibEngine] -->|implements| port
    adapter -->|every failure| error[EngineError: one bounded kind, no text]
  end
  adapter -->|open, set each top-level deck current, get_queued_cards 1000| engine[(Anki's engine at the pinned tag)]
  adapter -->|sync_login, normal_sync, full_download; the engine's default HTTP client| engine
  engine -->|HTTP, media never synced| server[(the configured sync server)]
```

The port is the anti-corruption layer: nothing outside `ingest` names an engine type, and an
`EngineError` carries no text from the engine, so it cannot carry a path, the endpoint or a
credential. A full download is the engine's own: it holds the downloaded collection in memory,
writes it beside the copy, opens it with an integrity check and renames it over the copy, so the
old copy survives any failure before the rename.

Each budget test measures one operation in a process that does nothing else:

```mermaid
sequenceDiagram
  participant T as budget test (the parent)
  participant S as the same test binary as sync-server
  participant M as the same test binary as measure
  T->>T: build ADR-022's synthetic collection by bulk inserts
  T->>S: spawn, with SYNC_USER1 set by Command::env
  S-->>T: its loopback address, on stdout
  T->>M: spawn, handing it the endpoint and the copy's path
  M->>S: log in, then full download (or open and queue, with no server)
  M-->>T: VmHWM, read when the operation returns
  T->>T: the copy holds every card and review; VmHWM within 256 MiB
  T->>S: close its stdin, so it exits
```

| process | holds | why |
|---|---|---|
| the parent | the fixture's build, the checks | its memory is not the operation's |
| `sync-server` | the engine's server over the synthetic collection | the owner's server is another machine; the engine reads its users only from the process environment, which safe Rust sets only for a child |
| `measure` | one port call, then `VmHWM` | its peak is the operation's peak |

`engine-measure.yml` measures the rest on a hosted `ubuntu-24.04` runner with no cache: the
clock runs from the `protoc` download to the built release probe, then the probe is stripped and
sized, the budget tests run in the release profile the measured build compiled, and
`cargo deny check licenses` runs over the lockfile. Its report is ADR-009's Confirmation table.

## One sync run, and the census

Kind: flow. Built by SPEC-022's phase 2 (A4 to A17), decided by ADR-037 (one scheduled sync per
study day, the owner's triggers, never an upload), read at the predecessor's `27ee2bc`
(`pipeline.py:GamifyPipeline._sync_attempts`, `sync.py:AnkiSyncer._sync_blocking`).

```mermaid
flowchart TD
  cycle[coordination::sync_cycle: trigger] --> lock[take the collection lock, exclusive; wait for a running sync]
  lock --> which{trigger}
  which -->|scheduled| today{a scheduled run recorded this study day?}
  today -->|yes| refused[RefusedToday: no request, no row]
  which -->|owner| recent{a success finished under OWNER_SYNC_DEBOUNCE_SECS ago?}
  recent -->|yes| debounced[Debounced: that success, no request, no row]
  today -->|no| creds[load anki-sync-username and anki-sync-password]
  recent -->|no| creds
  creds -->|missing| failed
  creds --> attempt[attempt n of the schedule, bounded by its timeout]
  attempt -->|open finds the collection locked| reopen[wait, reopen: up to COLLECTION_OPEN_RETRIES]
  reopen --> attempt
  attempt -->|normal sync: synced or no change| ok[ok]
  attempt -->|full sync demanded, the server has a collection| download[full download beside the copy, then the swap] --> ok
  attempt -->|full sync demanded, the server is empty| upload[full_upload_required]
  attempt -->|error or timeout| wait{attempts left?}
  upload --> wait
  wait -->|yes| backoff[wait base 2^n-1 plus jitter, on tokio's timer] --> attempt
  wait -->|no| failed[one bounded reason code]
  ok --> record[record one sync_runs row: trigger, study day, status, attempts, full download]
  failed --> record
  record --> unlock[explicit unlock, then close the lock file]
```

The census (A15) puts a recording layer between the syncer and the engine's server. It keeps
every request, decompresses each body, and fails the test on `upload` or on any body that carries
a local change: a note type, deck, deck configuration, tag, configuration or creation stamp in
`applyChanges`, a grave in `applyGraves` or `start`, or a review, card or note in `applyChunk`.

```mermaid
flowchart LR
  syncer[Syncer over RslibEngine] -->|HTTP| recording[recording layer: keeps each request, decodes its body]
  recording -->|the same request| server[the engine's sync server, a child process]
  server -->|the response, unchanged| recording --> syncer
```
