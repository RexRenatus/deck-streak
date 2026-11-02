# The native client's first sync through the full-sync choice (SPEC-503)

The sequence of a fresh install's first sync on iPhone and iPad, from the owner's tap to the
normal sync after the download. Each arrow is one call; the core decides every rule.

```mermaid
sequenceDiagram
    actor Owner
    participant View as SyncView
    participant Model as SyncModel
    participant Keychain as Keychain item
    participant Session as EngineSession
    participant Facade as ffi Engine (sync facade)
    participant Core as engine core
    participant Service as sync service

    Owner->>View: Sync
    View->>Model: sync()
    Model->>Keychain: hostKey()
    Keychain-->>Model: host key (no password)
    Model->>Session: sync(hostKey, endpoint)
    Session->>Facade: sync(host_key, endpoint)
    Facade->>Service: GET the statement at the endpoint's origin
    Service-->>Facade: minimum client level
    Facade->>Core: handshake(statement)
    Facade->>Core: run(1,5, normal sync, media off)
    Core->>Core: endpoint rule, media refusal, handshake check
    Core->>Service: normal sync
    Service-->>Core: full sync required
    Core-->>Model: required (via facade and session)
    Model->>Session: unsynced()
    Session->>Facade: unsynced()
    Facade->>Core: one fixed offline read
    Core-->>Model: unsynced reviews and changes
    Model->>Session: countChoice(required)
    Session->>Facade: full_sync_count(folder)
    Facade->>Core: one_way::count (server copy into a new file)
    Core-->>View: what the download loses
    Owner->>View: Download
    View->>Model: download()
    Model->>Session: download(hostKey, endpoint)
    Session->>Facade: full_sync_confirm(1, no snapshot answer, folder)
    Facade->>Core: one-way gesture, back_up, download_ready, write
    Core->>Service: one-way download after the backup
    Core-->>Model: written
    Model->>Session: sync() again
    Session->>Facade: sync
    Facade->>Core: run(1,5)
    Core-->>Model: no full sync needed
```

## Where each rule lives

| Rule | Where |
| --- | --- |
| Endpoint rule, media refusal, handshake check | `crates/engine-core/src/dispatch.rs::run` |
| The normal sync's request and the statement's URL | `crates/engine-core/src/native_sync.rs` |
| Counts, backup, re-check, write | `crates/engine-core/src/one_way.rs`, `full_sync.rs` |
| The one-way gesture | `crates/ffi/src/engine.rs` (the adapter's entry file) |
| The host key | the app's one Keychain item, `SyncCredentialStore` |
