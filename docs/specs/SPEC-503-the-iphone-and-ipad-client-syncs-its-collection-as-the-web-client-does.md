# SPEC-503: the iPhone and iPad client syncs its collection, as the web client does

- **Context(s):** `deck-streak-engine-core` (`crates/engine-core`), `deck-streak-ffi`
  (`crates/ffi`), the app (`ios/App`).
- **Decided by:** ADR-521 (this SPEC's own), which amends ADR-356 D2, under ADR-358 (one Keychain
  item and the endpoint guard in the core), ADR-368 (the full-sync choice is one rule in the core),
  ADR-375 (the web client syncs from its Worker; the one-way sync is an exempt write) and ADR-385
  (the minimum-client handshake).
- **Beside:** SPEC-347 (the app shell and its Keychain item), SPEC-364 (the web client's sync),
  SPEC-374 (the minimum-client handshake) and SPEC-377 (the web sync page's choice).
- **Schematic:** `docs/schematics/native-first-sync.md` (the sequence of a first sync through the
  full-sync choice on iPhone and iPad).
- **Mutation band:** S50300-S50399. **Models:** `formal/tla/MinimumClientHandshake`,
  `formal/tla/FullSyncChoice` and `formal/tla/SyncCredential`, re-read with the native actor and
  re-stamped (section 8).

## 1. The problem, measured

At the base, the core's native column refuses the normal sync: `crates/engine-core/src/table.rs`
marks `BackendSyncService.SyncCollection` (1,5) `native: false`, and the native adapter's
allow-list (`crates/ffi/src/allow_list.rs`) holds ten pairs without it. The app can sign in and
keep the host key in its Keychain item (SPEC-347), but it cannot sync. The web client syncs from its
Worker through the same core (SPEC-364, SPEC-377): the normal sync with media off and the endpoint
guard, the handshake before it, and the full-sync choice's counts, backup, re-check and write.

## 2. Requirements

- **R1.** The native column admits `SyncCollection` (1,5), and the native adapter's allow-list
  carries it, held equal to the column by the parity test.
- **R2.** A native normal sync runs with media off and meets the login's endpoint rule before the
  engine sees it. The core builds its request (`native_sync::normal_sync`); the adapter passes no
  request bytes of the app's own for it.
- **R3.** Before a native normal sync, the adapter reads the service's statement of its minimum
  client level at the endpoint's origin and hands it to the core, which refuses the sync below the
  stated level, or with no statement read, in the engine's error shape. An endpoint the rule refuses
  is not read.
- **R4.** A fresh install's first sync goes through the full-sync choice: the counts of what each
  offered direction loses, a backup of the side the write replaces, then the download, each run by
  the core in the model's order. The choice's files are kept in the collection's directory. The
  native client offers the download only; an upload waits for the server's snapshot answer, which
  this client does not read (section 5).
- **R5.** After the first sync, a sync is a normal one: the app runs a normal sync after the
  download is written, and every later sync starts as a normal sync.
- **R6.** The app reads the host key from its one Keychain item before each sync and hands only
  the key to the adapter. The password is never stored: not in the Keychain, the model, or the
  adapter.
- **R7.** The app reads what the device has not synced (the core's offline read) and shows it
  before a full sync can lose reviews: the choice's buttons appear only after that read.
- **R8.** The app's sync model is tested by XCTest on one iPhone and one iPad simulator.
- **R9.** MinimumClientHandshake, FullSyncChoice and SyncCredential are re-read with the native
  actor and re-stamped.

## 3. Acceptance criteria

```acceptance
A1: cargo test -p deck-streak-engine-core --test table
A1: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column
A1: cargo test -p deck-streak-ffi --test review_pairs -- --exact the_allow_list_carries_the_review_pairs
A2: cargo test -p deck-streak-engine-core --test login_guard -- --exact a_native_normal_sync_reaches_only_a_guarded_endpoint_and_never_media
A2: cargo test -p deck-streak-engine-core --test native_sync
A2: cargo test -p deck-streak-ffi --test sync -- --exact a_native_normal_sync_keeps_the_endpoint_rule_and_reads_no_statement_for_a_refused_endpoint
A3: cargo test -p deck-streak-engine-core --test handshake -- --exact a_client_below_the_minimum_is_refused_every_sync_before_the_engine
A3: cargo test -p deck-streak-ffi --test sync -- --exact a_native_normal_sync_with_no_statement_read_is_refused_before_the_engine
A4: cargo test -p deck-streak-ffi --test sync -- --exact a_fresh_installs_first_sync_counts_backs_up_and_downloads_then_syncs_normally
A4: cargo test -p deck-streak-ffi --test sync -- --exact the_unsynced_count_is_read_offline_before_any_full_sync
A5: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
A5: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncModelTests
A6: phxd formal check --root . --base <the base> --kind tla --gate --format json
```

A1 holds R1; A2 holds R2; A3 holds R3; A4 holds R4, R5 and R7 at the adapter. A5's XCTest holds R4
to R8 at the app: the choice, then the download from the owner's tap, then a normal sync; the
unsynced read before the counts; the stored host key on every call and no password held; and,
signed out, no sync call. It runs on the macOS lane only. A6 holds R9.

## 4. File manifest

- `crates/engine-core/src/table.rs`: (1,5) `native: true`.
- `crates/engine-core/src/native_sync.rs` (new) and `crates/engine-core/src/lib.rs`: the native
  normal sync's request, its reply's requirement, the choice's answer and the statement's URL.
- `crates/ffi/src/allow_list.rs`: (1,5) joins the allow-list.
- `crates/ffi/src/engine.rs`: the sync facade, `Engine::sync`, `unsynced`, `full_sync_count`,
  `full_sync_confirm` and `full_sync_cancel`, and their records.
- `crates/ffi/tests/sync.rs` (new), `crates/ffi/tests/review_pairs.rs`,
  `crates/engine-core/tests/table.rs`, `login_guard.rs`, `handshake.rs` and `native_sync.rs` (new).
- `ios/App/Sources/SyncModel.swift` and `SyncView.swift` (new), `EngineSession.swift`,
  `AppModel.swift`, `AccountView.swift`; `ios/AppTests/SyncModelTests.swift` (new);
  `ios/swift-roles.json`.
- `formal/tla/MinimumClientHandshake/MinimumClientHandshake.tla`,
  `formal/tla/FullSyncChoice/FullSyncChoice.tla`, `formal/tla/SyncCredential/SyncCredential.tla`.
- `docs/decisions/ADR-521-*.md` (new), `docs/decisions/ADR-356-*.md` (amendment),
  `docs/schematics/native-first-sync.md` (new), `changelog.d/native-sync-503.md` (new).

## 5. What this does NOT cover

- The upload from the native client: it needs the server's snapshot answer, which the web client
  reads under an owner session; the native client has none yet, so the core's snapshot rule refuses
  an upload and the app offers none.
- Media sync, the backups' list, export and retention on the device, and a background sync.

## 6. Risks

- The first sync's backup and server copy stay in the collection's directory, uncounted against
  the device's storage, until retention is built for the native client.
- The facade reads the statement before every normal sync: a service that does not answer stops
  every native sync, by design (SPEC-374 R6).

## 7. Mutation rows

Band S50300-S50399: S50300 (1,5)'s native mark; S50301 the facade's statement read; S50302 the
request's `sync_media: false`; S50303 the full-sync threshold in the app's model; S50304 the stage
dropped on the write. Each row's killer is named in the acceptance block above.

## 8. Formal models

The three models are re-read with the native actor. MinimumClientHandshake: the facade's
`Engine::sync` is a `Read` then a `Sync`, and its `native_sync::statement_url` is the login's
origin rule. FullSyncChoice: `Engine::full_sync_count` and `full_sync_confirm` drive the same steps
as the web exports, in the same order. SyncCredential: the native actor keeps the key in one
Keychain item and has no Worker, seal or generation, so the model's rules are the web client's
alone; the facade takes a key and never a password. Their covers are re-stamped (A6).

## 9. What only a device or a person proves

A5's XCTest on the iPhone and iPad simulators, and a first sync against the deployed service.
