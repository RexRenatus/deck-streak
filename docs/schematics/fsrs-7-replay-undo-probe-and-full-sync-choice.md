# FSRS-7 replay, the undo probe and the full-sync choice path (SPEC-342, ADR-353)

Read at DeckStreak `dev` 1eec0870 and at the engine's fork commit c538de5, the commit the workspace's
`[patch]` entry pins. Every `rslib/...:line` below is read at c538de5, whose native code at each of
those lines is the earlier pin's, 57382da (ADR-058's note). The pinned FSRS-7 revision is
`4bc0a0979f95dd01cc653031b6f2a01429e4c32b`, and its facts are re-read there by the build
(ADR-353).

## 1. Where the crate sits (component)

The scheduler crate is a context with no DeckStreak edge. Its one external dependency is the
pinned revision, and the engine keeps the released crate. Dev-dependencies are drawn dotted: they
are not context edges.

```mermaid
flowchart LR
    subgraph ws[DeckStreak workspace]
        fsrs7[deck-streak-fsrs7<br/>depends on: nothing]
        ffi[deck-streak-ffi<br/>depends on: nothing]
        ingest[deck-streak-ingest<br/>depends on: kernel]
    end
    engine[anki, the engine<br/>fork commit c538de5]
    rel[scheduler crate, released<br/>registry, by checksum]
    pin[scheduler crate, FSRS-7<br/>fsrs-rs at the pinned revision]
    fsrs7 --> pin
    fsrs7 -. dev: coexistence test .-> rel
    ffi --> engine
    ingest --> engine
    engine --> rel
    deny[deny.toml allow-git<br/>engine fork, rust-url, fsrs-rs] -. admits .-> pin
```

## 2. The replay (data flow)

History goes in. Memory state, a checksum and timings come out. Within a card, the history is
taken in id order.

```mermaid
flowchart TD
    grid[the grid<br/>reviews 10000, 100000, 1000000<br/>mean 8 or 32, method single or batch] --> hist
    hist[measure history<br/>card lengths cycle 1 to 2m-1, total exactly R<br/>fixed rating and interval cycles] --> rows
    rows[review-log rows<br/>card id, id in ms, ease, kind] --> conv
    conv[convert<br/>drop manual, rescheduled, ease 0<br/>first interval 0, then ms difference / 86400000] --> items
    items[one FSRS-7 item per card] --> replay
    params[the pinned revision's FSRS-7 defaults<br/>34 parameters] --> replay
    replay[replay, single or batch] --> states[memory state per card]
    states --> sum[checksum: the sum of stabilities]
    clock[monotonic clock<br/>1 warm-up, 5 timed runs] --> line
    replay --> clock
    sum --> line[one fixed line per cell<br/>median, min, max ms, cost per review, checksum]
    line --> native[native, one thread]
    line --> wasm[wasm32-wasip1 under Node V8 WASI]
    native --> report[replay_report<br/>refuses a missing or doubled cell, a bad line,<br/>checksums apart by more than 1 in 10000,<br/>no web-target line]
    wasm --> report
    webcheck[cargo check, web target<br/>pass, or fail naming crates] --> report
    report --> out[step summary and the fsrs7-measure-report artifact]
```

## 3. The undo probe (sequence)

U1 runs with no server. U2 and U3 run against the engine's own server, started by the test; the
recording relay sits in front of it. The engine's undo of an answer removes its row
(`rslib/src/revlog/undo.rs:14-17`). A normal sync empties the queue as it starts
(`rslib/src/sync/collection/normal.rs:85-86`, `rslib/src/undo/mod.rs:228`).

```mermaid
sequenceDiagram
    participant T as the probe
    participant C as local collection
    participant R as recording relay
    participant S as engine sync server (child)
    Note over T,C: U1
    T->>C: answer card Good
    T->>C: count review-log rows for the card (expect 1)
    T->>C: undo
    T->>C: count rows (expect 0), card row byte-equal to before
    Note over T,S: U2
    T->>C: answer card Good
    C->>R: normal sync
    R->>S: hostKey, meta, start, applyChunk, ...
    T->>C: can_undo (expect none), undo (expect UndoEmpty)
    T->>C: count rows (expect 1)
    T->>S: stop the server, read its collection: the row is there
    Note over T,S: U3
    T->>C: answer card Good, then undo
    C->>R: normal sync
    T->>R: pushed chunks carry no review-log row for the card
    T->>S: stop the server, read its collection: no row for the card
```

## 4. The full-sync choice path (sequence)

The choice is offered at the `meta` answer (`rslib/src/sync/collection/meta.rs:70-78`). The
upload changes the local collection, closes it, and sends one request
(`rslib/src/sync/collection/upload.rs:44-69`). The server checks integrity and replaces its
collection (`rslib/src/sync/collection/upload.rs:73-104`). The download closes the local
collection, fetches to a temporary file, checks it and renames it into place
(`rslib/src/sync/collection/download.rs:21-44`).

```mermaid
sequenceDiagram
    participant O as the choice
    participant A as local client
    participant B as another client
    participant R as recording relay
    participant S as engine sync server
    A->>R: hostKey
    A->>R: meta
    R->>S: meta
    S-->>A: schema differs
    Note over A: FullSyncRequired upload_ok, download_ok<br/>upload_ok false only if local empty and server not<br/>download_ok false only if server empty and local not
    alt F5: a normal sync in the window
        B->>S: hostKey, meta, start, applyChunk with one review, finish
    end
    alt upload chosen (F2)
        O->>A: upload
        Note over A: before_upload changes the local collection, then close
        A->>R: upload, one request, no second meta
        R->>S: upload
        Note over S: integrity check, then replace whole
        Note over S: F4: server's own rows are gone<br/>F5: the other client's review is gone
    else download chosen (F3)
        O->>A: download
        Note over A: close
        A->>R: download, one request
        R->>S: download
        S-->>A: the server's file
        Note over A: temporary file, integrity check, atomic rename
        Note over A: F4: local-only rows are gone<br/>F6: a review synced earlier is gone too,<br/>when the server was replaced without it
    end
```

What the probe records for the choice's TLA+ model, which the full-sync choice's own delivery
writes first (SPEC-334 section 9):
- the actions, as the requests each choice sends (F2, F3);
- the window, the gap between the `meta` answer and the write, where nothing re-checks the
  server (F5);
- the losses, as id differences, not counts of unsynced rows (F4, F6).

## 5. The CI job graph

```mermaid
flowchart LR
    pr[pull request into dev] --> ci[ci.yml]
    pr -->|crates/fsrs7, Cargo.lock, the workflow| fm[fsrs7-measure.yml]
    ci --> rust[rust job: check.sh fmt, clippy, test, doctest, audit-rust<br/>test runs undo_and_full_sync and the fsrs7 crate's tests<br/>audit-rust runs cargo deny with the new allow-git entry]
    ci --> eng[engine job: the engine set, unchanged]
    ci --> py[hygiene job: check.sh python<br/>test_fsrs7_pin, test_engine_pin]
    ci --> mut[mutation jobs: the crate's src and the band's rows]
    ci --> agg[ci, the aggregate]
    rust --> agg
    eng --> agg
    py --> agg
    mut --> agg
    fm --> n1[native: replay_timing, one thread]
    fm --> w1[add wasm32-wasip1, build replay_timing]
    w1 --> w2[run under node:wasi]
    fm --> wc[cargo check, web target, recorded]
    n1 --> rep[replay_report]
    w2 --> rep
    wc --> rep
    rep --> art[summary and fsrs7-measure-report artifact]
```

`fsrs7-measure.yml` is a workflow of its own, after `engine-measure.yml`'s pattern:
- a read-only token;
- actions pinned by full commit;
- no secret;
- the pull request's concurrency group.

It compiles neither the engine nor protoc, so it needs neither. `ci.yml` changes no job: the
probes enter the `test` stage, and the crate enters every stage that runs over the workspace.
