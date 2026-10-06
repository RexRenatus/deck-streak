# SPEC-347: the universal app's shell lists the local decks over the engine and keeps the sync login's host key in the Keychain

- **Wave:** the app campaign. **Issue:** #625. **Campaign row:** SPEC-334 row 1.5 (R4, R17, R19,
  R20). **Context(s):** `deck-streak-engine-core` and `deck-streak-ffi` (one pair and one guard; no
  edge between DeckStreak crates changes), `ios/` (the app beside the harness; no bounded context),
  and the Apple job body and its censuses.
- **Decided by:** ADR-335 (one umbrella crate behind `run(service, method, bytes)`; the sync login
  in the Keychain), ADR-342 (one universal SwiftUI target; a split view on the iPad and a stack on
  the iPhone), ADR-344 (internal builds under the dev app id, syncing to the staging sync user;
  the build number), ADR-350 (the generator, the engine's package fed by its own run's artifact,
  the hand-written codec, the signing seam), and ADR-358 (the app's own generator spec, the
  credential's one Keychain item, the sync configuration as build settings, the endpoint guard in
  the core, the codec as the whole client's, the thin-Swift census, and the unsigned device
  archive).
- **Status:** two pull requests. Part 1 (R1 to R3) is the engine's login pair and its guard, on
  Linux. Part 2 (R4 to R14) is the app, on the macOS job; its criteria sit in section 7 until it
  lands. **Base:** live `dev`; part 1 after #623's first pull request, part 2 after part 1, #656,
  #659 and #624 have landed. **Mutation band:** `S34700-S34799`.

## 1. The problem, measured

Read at DeckStreak `dev` `96eae6afd97384108cb31d66e7d3243d2d70f5af` (DEV), with #656 at
`d295cd886a520ea61387723c0b081fed29a33c5d` (HARNESS; its merge-base with DEV is `7507d8a2`, so its
own change is the three-dot diff `DEV...HARNESS`) and #659 at
`01773951348d8c14fe9f0ba186f5e78c2244d954` (APPLE; merge-base `ccd36df6`). `R=<the DeckStreak
checkout>`; `E=<the engine's checkout>` and `PIN=<the engine revision root Cargo.toml's [patch]
names, line 133>` for the engine's files.

Part 1 was built after #656, #659 and #623's first part landed; the build re-read every figure it
relies on at its cut, and the figures below stay as read at DEV.

### 1.1 What `ios/` and the Apple job already hold

| # | measured | figure | command |
|---|---|---|---|
| M1 | Swift files under `ios/` | DEV 0. HARNESS 16: two package manifests (`ios/EnginePackage`, `ios/HarnessWire`), seven harness sources, three harness tests, two codec sources and two codec tests | `git -C $R ls-tree -r --name-only <sha> -- ios \| grep -c '\.swift$'` |
| M2 | the project generator | HARNESS `ios/project.yml` (82 lines): project `Harness`, targets `Harness`, `HarnessTests`, `HarnessUITests`, iPhone and iPad (`TARGETED_DEVICE_FAMILY "1,2"`, line 41), strict concurrency (line 42), local packages only (lines 17, 19), a scheme archiving Release; the generated project is ignored (`.gitignore` line 41, `ios/*.xcodeproj/`) | `git -C $R show HARNESS:ios/project.yml \| cat -n`; `git -C $R show HARNESS:.gitignore \| grep -n xcodeproj` |
| M3 | the signing seam | HARNESS `ios/Config/Harness.xcconfig` (16 lines): `PRODUCT_BUNDLE_IDENTIFIER = $(DS_APP_ID)`, `DS_APP_ID` a placeholder whose first label is the reserved `invalid`, and `#include? "Signing.local.xcconfig"`, which git ignores (`.gitignore` line 50) | `git -C $R show HARNESS:ios/Config/Harness.xcconfig \| cat -n` |
| M4 | property lists | HARNESS `ios/Harness/Info.plist` (48 lines): the bundle id, version and build from build settings, `ITSAppUsesNonExemptEncryption` false, no `NSAppTransportSecurity` key; `ios/Harness/PrivacyInfo.xcprivacy` (31 lines): tracking false, two required-reason categories (file timestamps, disk space) | `git -C $R show HARNESS:ios/Harness/Info.plist \| cat -n`; `… PrivacyInfo.xcprivacy \| cat -n` |
| M5 | entitlements, Keychain, app id | 0 entitlements files; 0 files under `ios/` naming `SecItem` or `kSec`; no app id beyond the placeholder | `git -C $R ls-tree -r --name-only HARNESS -- ios \| grep -c '\.entitlements$'`; `git -C $R grep -l -E 'SecItem\|kSec' HARNESS -- ios \| wc -l` |
| M6 | the job that links the engine into an app | HARNESS `xcframework.yml` (491 lines): jobs `xcframework` (line 39), `harness-wire` (165) and `harness` (254, `needs: xcframework`). `harness` downloads this run's framework, bindings and synthetic collection, verifies the generator by digest, runs one `xcodegen generate --spec ios/project.yml` (step at line 289), tests Debug on one iPhone and one iPad simulator (292), measures Release (305), builds the Release app alone (318), reads its size (327) and its required-reason imports against the manifest (333), then reports (363) and uploads (483); every build passes `CODE_SIGNING_ALLOWED=NO` on its command line. `ios/**` starts the workflow (line 26) | `git -C $R show HARNESS:.github/workflows/xcframework.yml \| grep -n -E '^  [a-z-]+:$\|- name:\|ios/\*\*'` |
| M7 | the Apple job body | APPLE turns `xcframework.yml` into a called workflow (`workflow_call`, line 14) with no concurrency block; `apple-on-change.yml` (31 lines) calls it on a pull request into `dev` touching `crates/ffi/**`, `ios/**`, the manifests, the toolchain file or either workflow; `apple-on-tag.yml` (24 lines) calls it on a SemVer tag | `git -C $R show APPLE:.github/workflows/xcframework.yml \| sed -n '13,20p'`; `git -C $R show APPLE:.github/workflows/apple-on-change.yml \| cat -n` |
| M8 | a census over Swift sources | none: 0 modules under `scripts/tests` name `.swift` at DEV, HARNESS or APPLE; #624's design leaves "the app shell … brings its Swift and its census" to this issue | `git -C $R grep -l -F '.swift' <sha> -- scripts/tests \| wc -l` |
| M9 | the harness tree census | HARNESS `scripts/tests/test_ios_harness_tree.py` (250 lines): any file under `ios/` naming `DEVELOPMENT_TEAM`, `CODE_SIGN_IDENTITY` or `CODE_SIGNING_ALLOWED` is refused, comments included (line 36, lines 82 to 84); every xcconfig under `ios/` must set `PRODUCT_BUNDLE_IDENTIFIER = $(DS_APP_ID)` with a `DS_APP_ID` whose first label is `invalid` (lines 101 to 130); the Info.plist seam is read for the harness's plist alone (line 25) | `git -C $R show HARNESS:scripts/tests/test_ios_harness_tree.py \| sed -n '20,131p'` |

### 1.2 The engine surface the login and the deck list need

| # | measured | figure | command |
|---|---|---|---|
| M10 | the native allow-list | DEV `[Call; 5]` (line 23): (3,0), (7,13), (13,3), (13,4), (3,8); HARNESS `[Call; 6]` (line 25) adds (27,6). No sync pair at any ref; #623's design gives the core a native column equal to the adapter's list, held by a parity test | `git -C $R show <sha>:crates/ffi/src/allow_list.rs \| grep -n ALLOW_LIST` |
| M11 | the sync service's methods | `BackendSyncService` (service 1, `sync.proto` lines 15 to 26), in order: SyncMedia, AbortMediaSync, MediaSyncStatus, **SyncLogin (method 3)**, SyncStatus, SyncCollection, FullUploadOrDownload (method 6, the one-way sync), AbortSync, SetCustomCertificate | `git -C $E show $PIN:proto/anki/sync.proto \| grep -n -E 'service \|rpc '` |
| M12 | the login's messages | `SyncLoginRequest {username 1, password 2, optional endpoint 3}` (lines 35 to 38); `SyncAuth {hkey 1, optional endpoint 2, …}` (lines 29 to 31) | `git -C $E show $PIN:proto/anki/sync.proto \| sed -n '29,40p'` |
| M13 | a login with no endpoint | the engine's HTTP client falls back to its built-in default server when the endpoint is absent (`rslib/src/sync/http_client/mod.rs`, the `unwrap_or_else` near line 45); the login passes the request's endpoint through unchanged (`rslib/src/backend/sync.rs` lines 288 to 311). A login request without an endpoint sends the password to a server that is not DeckStreak's | `git -C $E show $PIN:rslib/src/sync/http_client/mod.rs \| sed -n '40,50p'`; `git -C $E show $PIN:rslib/src/backend/sync.rs \| sed -n '288,311p'` |
| M14 | whether a status call proves a credential | it does not: `SyncStatus` returns without the network whenever the collection has local changes (`sync.rs` lines 313 to 321) | `git -C $E show $PIN:rslib/src/backend/sync.rs \| sed -n '313,332p'` |
| M15 | the engine's refusal | `BackendError {message 1, kind 2, …}`; kinds `INVALID_INPUT 0`, `NETWORK_ERROR 6`, `SYNC_AUTH_ERROR 7`, `SYNC_OTHER_ERROR 8` (`backend.proto` lines 23 to 60) | `git -C $E show $PIN:proto/anki/backend.proto \| sed -n '23,60p'` |
| M16 | the engine's TLS | the workspace builds the engine with its `rustls` feature (root `Cargo.toml` line 89), which the engine maps to its HTTP client's rustls with bundled web roots (`rslib/Cargo.toml` line 14); the lockfile's HTTP client entry depends on the bundled roots. The engine's requests do not pass through the system's URL loading, so App Transport Security does not govern them | `git -C $R show DEV:Cargo.toml \| sed -n 89p`; `git -C $E show $PIN:rslib/Cargo.toml \| sed -n 14p`; `git -C $R show DEV:Cargo.lock \| grep -n -A12 '^name = "reqwest"$'` |
| M17 | a network-free login round trip | `crates/ingest/tests/support/mod.rs` re-runs its test binary as the engine's own sync server on loopback HTTP with a synthetic user from `SYNC_USER1` (lines 43, 227 to 304; the endpoint `http://127.0.0.1:<port>/`, line 280) | `git -C $R show DEV:crates/ingest/tests/support/mod.rs \| sed -n '236,282p'` |
| M18 | a credential that never reaches a log | `crates/ingest/src/engine.rs` line 62: the server's `SyncLogin` prints none of its three values in `Debug` (SPEC-022 R9) | `git -C $R show DEV:crates/ingest/src/engine.rs \| sed -n '55,75p'` |
| M19 | the crates a URL check could use | no workspace member names `url`; the lockfile carries it as one of the engine's own dependencies (lines 5438 to 5440); `tokio` is a workspace dependency (root `Cargo.toml` line 62); the native adapter declares no dev-dependencies at DEV | `git -C $R show DEV:Cargo.toml \| grep -n -E '^(url\|tokio)'`; `git -C $R show DEV:Cargo.lock \| grep -n '^name = "url"$'`; `git -C $R show DEV:crates/ffi/Cargo.toml` |
| M20 | the Swift codec | HARNESS `ios/HarnessWire/Sources/HarnessWire/Messages.swift` (194 lines): `Requests` (line 37) builds six requests, `Responses` (line 140) decodes three; none is a sync message. `ios/HarnessWire/swift-mutants.json` (189 lines) is swept by `harness-wire`'s step "the codec's mutants, each against its one killer" | `git -C $R show HARNESS:ios/HarnessWire/Sources/HarnessWire/Messages.swift \| grep -n -E 'enum (Requests\|Responses)\|static func'` |

### 1.3 The conventions this delivery meets

| rule | where | what it asks of this delivery |
|---|---|---|
| rows | `scripts/mutation-rows.json`: a SPEC owns `S<NNN>00` to `S<NNN>99` in `scripts/mutation-rows.d/`; `MUTATIONS` rows have seven cells (stem, crate, crate-relative path, find, replace, killer `<target>::<test>`, behaviour); `SCRIPT_MUTATIONS` rows six (stem, path from the root, find, replace, behaviour, killer `module.Class.method`). HARNESS's `S33900` row on `crates/ffi/src/allow_list.rs` is the template for a pair's row | one band fragment, section 8's rows |
| Swift mutants | `ios/HarnessWire/swift-mutants.json`, swept by `harness-wire` (ADR-350) | the codec's new lines carry rows there |
| workflow reads | `test_ci_workflows.py` (APPLE): reads go through `workflow_file_text` (line 97); any other read in a module that imports it is listed in `NOT_WORKFLOW_READS` (line 4103); `ADMITTED_RUNNERS` (line 43) admits the macOS runner in `xcframework.yml` alone | the app's steps join the existing `harness` job; no new workflow, job or runner |
| the harness tree census | M9 | the app's xcconfig and every app file meet it unchanged |
| one library | #624's design: one binary target; `ios/project.yml` declares only local packages; no built library committed | the app's generator spec is held to the same reading (R12) |

## 2. Requirements

Part 1, delivered by the first pull request:

R1. **The login pair.** (1,3) `SyncLogin` joins the native adapter's allow-list
    (`crates/ffi/src/allow_list.rs`) and the core's native column in the same delivery, so #623's
    parity test holds. No other pair joins: `SyncStatus` (1,4), `SyncCollection` (1,5), the one-way
    sync (1,6), `AbortSync` (1,7) and `SetCustomCertificate` (1,8) stay refused with
    `NotAllowed`.
R2. **The endpoint guard.** Before the engine sees (1,3), the core decodes the request and refuses
    it when the endpoint is absent or empty, does not parse as a URL, carries a username or a
    password in the URL, or has a scheme other than `https`, with one exception: `http` to a
    loopback IP literal (127.0.0.0/8 or `::1`), which is the engine's own test server. The refusal
    is the engine's own error shape, a `BackendError` of kind `INVALID_INPUT` whose message names
    the rule broken, returned exactly as an engine refusal is (`EngineRefusal::Engine` at the
    adapter). Its text names neither the endpoint, the username, nor the password.
R3. **The round trip.** Through the adapter's `Engine::run`, a login to the engine's own sync
    server on loopback returns a `SyncAuth` whose host key is non-empty and whose endpoint is the
    one sent. A wrong password returns `EngineRefusal::Engine` whose `BackendError` kind is
    `SYNC_AUTH_ERROR`. Neither the password nor the host key appears in any refusal's text.

Part 2, delivered by the next pull request (section 7):

R4. **The app's project.** `ios/app.yml` generates `ios/DeckStreak.xcodeproj` (ignored by
    `ios/*.xcodeproj/`): an application target `DeckStreak` for iPhone and iPad, at the harness's
    deployment target, with complete strict concurrency; a unit-test target `DeckStreakTests`
    hosted by the app; a UI-test target `DeckStreakUITests`; one shared scheme `DeckStreak` whose
    archive action builds Release. Its packages are local paths alone, `EnginePackage` (product
    `DeckStreakFFI`) and `HarnessWire`; it declares no remote package, no framework or library
    dependency and no entitlements file. `ios/project.yml` and the harness are unchanged.
R5. **The seam.** `ios/Config/App.xcconfig` sets `PRODUCT_BUNDLE_IDENTIFIER = $(DS_APP_ID)`, a
    `DS_APP_ID` whose first label is `invalid`, `DS_SYNC_ENDPOINT` to an `https` URL on a host
    under `.invalid` (written `https:/$()/…`, because `//` opens an xcconfig comment),
    `DS_SYNC_USER` to a placeholder, `CURRENT_PROJECT_VERSION` and `MARKETING_VERSION`, and
    includes the ignored `Signing.local.xcconfig` when present. `ios/App/Info.plist` holds the
    harness's five seam keys, `DSSyncEndpoint = $(DS_SYNC_ENDPOINT)` and
    `DSSyncUser = $(DS_SYNC_USER)`, and no `NSAppTransportSecurity` key. `ios/App/PrivacyInfo.xcprivacy`
    declares tracking false, no collected data type, and exactly the required-reason categories
    the Release archive's executable imports.
R6. **The shell.** One `NavigationSplitView`: the deck list as its sidebar and a detail pane that
    reads "Choose a deck" until the review screen exists. It collapses to a stack in compact width
    with the deck list first; the chosen deck is the model's state, so it survives the collapse.
    The toolbar carries one account button.
R7. **The deck list.** At launch the session opens the collection at one fixed path under
    Application Support (the engine creates it when it is absent), then lists the decks by name
    in the engine's order through (7,13), filtered decks included. A refusal shows its sentence in
    place of the list. A fresh install lists exactly one deck, the engine's default. No deck name
    is kept anywhere but the engine's collection.
R8. **The login.** The account sheet shows the configured sync user as text, one secure password
    field and a sign-in button. Signing in calls (1,3) with the configured user, the configured
    endpoint and the typed password. On success the host key is stored (R9), the password is
    cleared from the field and the model, and the sheet reads "Signed in as <user>" with a
    sign-out button. On a refusal the sheet shows the engine's message and stores nothing. While
    a login runs, the sheet's controls are disabled and sign-out is not offered. Sign-out deletes
    the stored item. At launch, a stored item for the configured endpoint and user reads as
    signed in, with no network call.
R9. **The credential.** One Keychain generic-password item: its service is the configured
    endpoint, its account the configured user, its value the host key; accessible after first
    unlock, on this device only; not synchronizable; no access group named. The password is never
    written anywhere: not the Keychain, user defaults, a file nor a log. Nothing of the login is
    written outside that item.
R10. **The codec grows.** `HarnessWire` gains `Requests.syncLogin(username:password:endpoint:)`,
    which always writes fields 1, 2 and 3 (the endpoint is not optional in Swift);
    `Responses.syncAuth(_:)`, which reads field 1 and skips unknown fields; and
    `Responses.engineMessage(_:)`, which reads a `BackendError`'s message (field 1) and kind
    (field 2). Each is pinned by literal bytes and carries mutant rows.
R11. **The thin-Swift census.** `scripts/tests/test_ios_thin_swift.py`, over the register
    `ios/swift-roles.json`, holds four readings of every committed `.swift` under `ios/`:
    - a closed register: every file is listed once, with one role of `entry`, `view`, `model`,
      `session`, `credential`, `config`, `wire`, `manifest`, `test` or `harness`, and the register
      lists no file that is absent;
    - doors, for the app's own roles (`entry`, `view`, `model`, `session`, `credential`,
      `config`, `wire`): only `session` imports `DeckStreakFFI` or `HarnessWire` or names
      `FileManager`; only `credential` imports `Security` or names `SecItem` or a `kSec`
      constant; only `config` reads the info dictionary; none imports `WebKit` (the card's web
      view arrives with the review screen and its own role); none outside `wire` names a JSON or
      property-list coder;
    - names no file may hold, `harness` and `test` files included: `URLSession`, `URLRequest`,
      `NWConnection`, `import Network`, `SQLite3`, `sqlite3_`, `UserDefaults`, `@AppStorage`,
      `NSUbiquitousKeyValueStore`;
    - budgets: each non-harness, non-test, non-manifest file's decision count, read after its
      comments and string text are removed (an interpolation's contents are kept), equals the
      count the register records, and is at most its role's ceiling, which the census holds:
      `entry` 0, `view` 3, `model` 6, `session` 4, `credential` 4, `config` 2. `wire` has no
      ceiling, because the hand-written codec is the logic ADR-350 admits in Swift and its
      literal-bytes tests and mutants hold it; its counts are exact all the same. The decision tokens are `if`, `guard`, `case`, `while`, `for`, `repeat`, `catch`, `&&`,
      `||`, `??`, a ternary ` ? `, and the calls `.filter(`, `.sorted(`, `.sort(`, `.reduce(`,
      `.min(`, `.max(`, `.contains(`, `.first(where:`, `.allSatisfy(`. A raw string literal
      (`#"`) in a budgeted file is refused by name, so the stripper never has to read one.
    The harness's files are bound by the closed register and the forbidden names alone, which
    they meet unchanged. It prints the files, doors and decisions examined; planted trees
    breaking each rule are refused by the rule's name.
R12. **The app tree census.** `scripts/tests/test_ios_app_tree.py` holds: `ios/App/Info.plist`
    carries the seven keys of R5 with their values, the boolean false export answer, and no
    `NSAppTransportSecurity`; `App.xcconfig`'s `DS_SYNC_ENDPOINT` reads, with `$()` removed, as an
    `https` URL whose host ends in `.invalid`, and its `DS_SYNC_USER` is the placeholder; the
    privacy manifest parses with tracking false and no collected data type; no entitlements file
    exists under `ios/`; the credential store's one file names
    `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, `kSecAttrSynchronizable` with
    `kCFBooleanFalse` and `kSecClassGenericPassword`, and no other accessibility constant or
    access group; and every generator spec under `ios/` (`ios/*.yml`), not `ios/project.yml`
    alone, declares local packages only and no framework dependency, which is #624's reading
    extended to the app's spec.
R13. **CI.** The Apple job body's `harness` job gains, and nothing else gains: a second
    `xcodegen generate --spec ios/app.yml` line in the step "the project, generated"; a step "the
    app's tests, Debug, on the iPhone and then the iPad" (the `DeckStreak` scheme, both
    simulators, code signing off on its command line, its own derived data and result bundle);
    a step "the app, archived unsigned for a device" (Release, a generic iOS device destination,
    code signing off on its command line), whose same step compares the archived executable's
    required-reason imports with `ios/App/PrivacyInfo.xcprivacy`; and the report's rows for both.
    No job, workflow, runner, cargo command or required context is added, and no harness step
    changes.
R14. **What the lane receives (#634).** The scheme `DeckStreak`, its Release archive for a generic
    iOS device, and the settings the lane renders: `DS_APP_ID` (the dev app id for internal
    builds; the release app id only from a SemVer tag on `main`), `DS_SYNC_ENDPOINT` and
    `DS_SYNC_USER` (the staging sync user for internal builds), `CURRENT_PROJECT_VERSION`
    (ADR-344's build number) and `MARKETING_VERSION`, with the team and signing on its own command
    line or in its own ignored include. The app reads no other configuration.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | Through the adapter, a login to the engine's sync server on loopback with the server's synthetic user returns a `SyncAuth` whose host key is non-empty and whose endpoint is the one sent. Red first: `EngineRefusal::NotAllowed { service: 1, method: 3 }` | `cargo test -p deck-streak-ffi --test login -- --exact a1_a_login_through_the_adapter_returns_the_servers_host_key` |
| A2 | A wrong password is the engine's `SYNC_AUTH_ERROR`, and the refusal's text holds neither the password nor any host key. Red first: `NotAllowed` for (1,3) | `cargo test -p deck-streak-ffi --test login -- --exact a2_a_wrong_password_is_the_engines_auth_refusal_and_names_no_secret` |
| A3 | The guard admits `https` endpoints and `http` to `127.0.0.1` and `[::1]`, and refuses, each by its rule: an absent and an empty endpoint, an unparseable one, `http` to a named host, `http` to a non-loopback IP literal, `http://127.0.0.1@example.invalid/`, a URL carrying a username or a password, and a non-HTTP scheme; it prints the endpoints examined. Red first: over a guard that admits everything, the absent endpoint is admitted | `cargo test -p deck-streak-engine-core --test login_guard -- --exact a3_the_guard_admits_https_and_loopback_http_alone` |
| A4 | Through the dispatcher, (1,3) with the endpoint `ftp://127.0.0.1:1/` returns a `BackendError` of kind `INVALID_INPUT` carrying the guard's sentence, and the sentence names neither the endpoint, the user nor the password. Red first: over the stub guard that admits every endpoint, the engine's own client refuses the scheme with another kind, and no request leaves the process | `cargo test -p deck-streak-engine-core --test login_guard -- --exact a4_a_refused_login_never_reaches_the_engine_and_names_no_secret` |
| A5 | The (1,3) pair is in both the adapter's list and the core's native column, and #623's parity test still holds | `cargo test -p deck-streak-engine-core --test parity` |
| A6 | `Requests.syncLogin` writes the user, the password and the endpoint as fields 1, 2 and 3, byte for byte, an empty endpoint included | `swift test --package-path ios/HarnessWire --filter HarnessWireTests.RequestBytesTests/test_a6_the_login_request_writes_user_password_and_endpoint` (`harness-wire`, step "the codec's tests, on the host") |
| A7 | `Responses.syncAuth` reads the host key past unknown fields, and `Responses.engineMessage` reads a `BackendError`'s message and kind | `swift test --package-path ios/HarnessWire --filter HarnessWireTests.ResponseDecodingTests/test_a7_the_host_key_and_the_engines_message_decode` (same step) |
| A8 | On a fresh install the deck list shows exactly one row, the engine's default deck, on the iPhone and on the iPad | `xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ShellFlowTests/test_a8_a_fresh_install_lists_the_engines_default_deck` (`harness`, step "the app's tests, Debug, on the iPhone and then the iPad") |
| A9 | A login to the placeholder endpoint shows the engine's message on the sheet and stays signed out, with no stored item | the same shape, `-only-testing:DeckStreakUITests/ShellFlowTests/test_a9_a_refused_login_shows_the_engines_message_and_stays_signed_out` |
| A10 | The host key round-trips through the store under the configured endpoint and user, reads as absent under another endpoint, carries the accessibility and synchronizable attributes of R9, and sign-out deletes it | the same shape, `-only-testing:DeckStreakTests/CredentialStoreTests/test_a10_the_host_key_round_trips_and_sign_out_deletes_it` |
| A11 | On the iPad the deck list and the detail show side by side; on the iPhone the deck list shows first and choosing a deck shows the detail | the same shape, `-only-testing:DeckStreakUITests/ShellFlowTests/test_a11_the_split_view_follows_the_size_class` |
| A12 | R11 over the tree, and planted trees breaking each reading refused by its rule's name | `python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py -k test_every_swift_file_keeps_its_role_its_doors_and_its_budget` |
| A13 | R12 over the tree, and planted trees breaking each rule refused by name | `python3 -m unittest discover -s scripts/tests -p test_ios_app_tree.py -k test_the_app_tree_carries_the_seam_and_keeps_the_credential_in_the_keychain` |
| A14 | The `harness` job generates both specs, runs the app's tests on both simulators and archives the app for a generic iOS device with code signing off, all after the harness's steps and before the report; no other job, workflow or runner changes | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_app_is_generated_tested_and_archived_in_the_harness_job` |
| A15 | The Release archive builds, and its executable's required-reason imports are all declared | `xcodebuild archive -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Release -destination generic/platform=iOS -archivePath "$RUNNER_TEMP/DeckStreak.xcarchive" CODE_SIGNING_ALLOWED=NO` (`harness`, step "the app, archived unsigned for a device") |

A1 and A2 are red over the lists without (1,3). A3 and A4 are red over a stub guard that admits
every endpoint, wired where the guard will sit and committed alone before the guard. A5 is #623's
parity test, run unchanged: it adds no test, so it has no red of its own, and rows 00 and 01 each
break it.

A6 and A7 are red over stubs that write and read nothing; A8 to A11 over an app whose first
commit opens no collection and stores nothing; A12 and A13 over a census whose positive artifact
(the register's files, the app's plist) is not yet written. The macOS reds are read from the pull
request's own run (ADR-350). A14's red comes from CI with its red commit pushed alone first. A15
is a build, not a test: if the test-selection probe flags it, it leaves the fence and its run's
lines are quoted in the pull request's body, in the same commit, disclosed.

```acceptance
A1: cargo test -p deck-streak-ffi --test login -- --exact a1_a_login_through_the_adapter_returns_the_servers_host_key
A2: cargo test -p deck-streak-ffi --test login -- --exact a2_a_wrong_password_is_the_engines_auth_refusal_and_names_no_secret
A3: cargo test -p deck-streak-engine-core --test login_guard -- --exact a3_the_guard_admits_https_and_loopback_http_alone
A4: cargo test -p deck-streak-engine-core --test login_guard -- --exact a4_a_refused_login_never_reaches_the_engine_and_names_no_secret
A5: cargo test -p deck-streak-engine-core --test parity
A6: swift test --package-path ios/HarnessWire --filter HarnessWireTests.RequestBytesTests/test_a6_the_login_request_writes_user_password_and_endpoint
A7: swift test --package-path ios/HarnessWire --filter HarnessWireTests.ResponseDecodingTests/test_a7_the_host_key_and_the_engines_message_decode
A8: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ShellFlowTests/test_a8_a_fresh_install_lists_the_engines_default_deck
A9: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ShellFlowTests/test_a9_a_refused_login_shows_the_engines_message_and_stays_signed_out
A10: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/CredentialStoreTests/test_a10_the_host_key_round_trips_and_sign_out_deletes_it
A11: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ShellFlowTests/test_a11_the_split_view_follows_the_size_class
A12: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py -k test_every_swift_file_keeps_its_role_its_doors_and_its_budget
A13: python3 -m unittest discover -s scripts/tests -p test_ios_app_tree.py -k test_the_app_tree_carries_the_seam_and_keeps_the_credential_in_the_keychain
A14: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_app_is_generated_tested_and_archived_in_the_harness_job
A15: xcodebuild archive -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Release -destination generic/platform=iOS -archivePath "$RUNNER_TEMP/DeckStreak.xcarchive" CODE_SIGNING_ALLOWED=NO
```

## 4. File manifest

Part 1:

| file | context | change |
|---|---|---|
| `crates/ffi/src/allow_list.rs` | `deck-streak-ffi` | (1,3) `SyncLogin` joins `ALLOW_LIST` |
| `crates/ffi/Cargo.toml` | `deck-streak-ffi` | `tokio` (workspace) as a dev-dependency, for the test server |
| `crates/ffi/tests/login.rs` | `deck-streak-ffi` | A1, A2 |
| `crates/ffi/tests/support/sync_server.rs` | `deck-streak-ffi` | the engine's sync server on loopback, after `crates/ingest/tests/support/mod.rs` |
| `crates/engine-core/src/table.rs` | `deck-streak-engine-core` | (1,3) joins the native column |
| `crates/engine-core/src/login_guard.rs` | `deck-streak-engine-core` | R2's guard |
| `crates/engine-core/src/dispatch.rs` | `deck-streak-engine-core` | (1,3) passes the guard before the engine |
| `crates/engine-core/src/lib.rs` | `deck-streak-engine-core` | `pub mod login_guard`, its one documented `pub fn` |
| `crates/engine-core/Cargo.toml` | `deck-streak-engine-core` | `url` (workspace), the engine's own URL parser (ADR-358 D4) |
| `crates/engine-core/tests/login_guard.rs` | `deck-streak-engine-core` | A3, A4 |
| `crates/engine-core/tests/table.rs` | `deck-streak-engine-core` | the native set gains (1,3) (R1) |
| `Cargo.toml` | workspace | `url` joins `[workspace.dependencies]` at the version the lockfile already holds |
| `Cargo.lock` | workspace | the two members' dependency lists; no new package |
| `scripts/mutation-rows.d/S34700-S34799.json` | rows | section 8's part 1 rows |
| `docs/specs/SPEC-347-the-universal-apps-shell-lists-the-local-decks-and-keeps-the-sync-host-key-in-the-keychain.md`, `docs/decisions/ADR-358-the-app-shell-its-own-project-one-keychain-item-and-the-endpoint-guard-in-the-core.md`, `docs/schematics/app-shell-login-and-deck-list.md` | documents | this SPEC, its ADR, its schematic |
| `docs/red-first/SPEC-347.md` | documents | the red-first record |
| `changelog.d/ios-app-347.md` | documents | the delivery's entry |

Part 2:

| file | context | change |
|---|---|---|
| `ios/app.yml` | `ios/` | R4 |
| `ios/Config/App.xcconfig` | `ios/` | R5 |
| `ios/App/Info.plist`, `ios/App/PrivacyInfo.xcprivacy` | `ios/` | R5 |
| `ios/App/Sources/DeckStreakApp.swift` | `ios/` (entry) | the scene and the split view |
| `ios/App/Sources/AppModel.swift` | `ios/` (model) | the deck names, the chosen deck, the account state |
| `ios/App/Sources/EngineSession.swift` | `ios/` (session) | the one owner of the engine: open, deck names, login |
| `ios/App/Sources/SyncCredentialStore.swift` | `ios/` (credential) | R9 |
| `ios/App/Sources/SyncConfiguration.swift` | `ios/` (config) | the endpoint and user from the info dictionary |
| `ios/App/Sources/DeckListView.swift`, `ios/App/Sources/AccountView.swift` | `ios/` (view) | R6 to R8 |
| `ios/AppTests/CredentialStoreTests.swift` | `ios/` (test) | A10 |
| `ios/AppUITests/ShellFlowTests.swift` | `ios/` (test) | A8, A9, A11 |
| `ios/HarnessWire/Sources/HarnessWire/Messages.swift` | `ios/` (wire) | R10 |
| `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift`, `ResponseDecodingTests.swift` | `ios/` (test) | A6, A7 |
| `ios/HarnessWire/swift-mutants.json` | `ios/` | the codec's new rows |
| `ios/swift-roles.json` | `ios/` | R11's register |
| `scripts/tests/test_ios_thin_swift.py`, `scripts/tests/test_ios_app_tree.py` | censuses | R11, R12 |
| `scripts/tests/test_ci_workflows.py` | censuses | R13's class; each new read listed in `NOT_WORKFLOW_READS` |
| `.github/workflows/xcframework.yml` | the Apple job body | R13 |
| `scripts/mutation-rows.d/S34700-S34799.json` | rows | section 8's part 2 rows |
| this SPEC, `changelog.d/ios-app-347.md` | documents | section 7's rows move into section 3 |

## 5. What this does NOT do

- It syncs no collection. `SyncStatus`, `SyncCollection`, `AbortSync`, media sync, the sync
  screens and the full-sync choice are #633's; the one-way sync (1,6) stays refused until #631 and
  #633 build its measurement and its guards.
- It applies R2's endpoint rule to the login alone. Every later call that carries a `SyncAuth` is
  admitted with the same rule by the delivery that admits it (#633).
- It shows no card and answers none: the review screen, its WKWebView and the deck tree's due
  counts are #632's.
- It lays out no iPad screen beyond the split shell, and adds no remote, keyboard command or undo
  (#633).
- It builds no upload lane and signs nothing: the team, signing, the upload, the build number's
  rendering, the dev app id's and the staging user's values, and the cargo build's deployment
  target for a device release are the TestFlight pipeline's (#634).
- It proves no login against the staging sync user: that user, its server's certificate and the
  cutover's new sync password are #628's, and the first device login follows them.
- It adds no service sign-in and no bearer token: the passkey sign-in starts on the web (#627),
  and the native bearer token joins the Keychain with the native sign-in that follows it (#627).
- It adds no Swift mutation row to the gate's tables: the codec's rows stay in
  `ios/HarnessWire/swift-mutants.json`, and Swift rows proper are #650's.
- It takes no figure on a device (#629).
- It declares no collected data type: user content leaves the device first with the sync that
  uploads it, which declares it (#633).
- It changes no umbrella, binary target or library count (#624) and no dispatcher shape beyond one
  pair and one guard (#623).

## 6. Risks

- **An unsigned simulator build may be refused the Keychain** (`errSecMissingEntitlement`,
  -34018). Detected by A9's first run in the harness job. The fallback is ad-hoc signing for the
  simulator steps on the workflow's command line, never a team and never a file under `ios/`
  (M9 refuses the settings' names there).
- **TLS on iOS is not proven by CI.** A8 fails at name resolution, before TLS. The engine's client
  carries its own roots (M16); the first staging login on a device (#628, #634) proves it, and its
  failure shows the engine's message on the sheet.
- **A staging certificate the bundled roots do not trust fails every login**, since the custom
  certificate pair (1,8) stays refused. #628's server needs a publicly trusted certificate.
- **The login holds the session for its duration.** The deck list's calls queue behind it on the
  session actor, bounded by the engine's network timeout. The sheet disables its controls; the
  list stays readable from the model.
- **A host key stays valid on the server until the sync password changes.** A lost device is
  answered by a new sync password (#628's cutover sets one).
- **The census's reading of Swift is lexical.** A construct it misreads reads as a wrong count or
  a refused name, never a silent pass, because the counts are exact. Raw strings are refused
  outright.
- **The export answer.** The committed answer is false, as the harness's is; the engine's TLS is
  standard HTTPS through a library the app links, not the system's. The lane's first upload
  surfaces any question about it (#634).
- **Parity with a sibling pair.** #656 adds (27,6) to the adapter's list; whichever of it and this
  delivery lands second carries both pairs in both lists.
- **Concurrent edits to `test_ci_workflows.py`.** Several deliveries add classes; the build re-reads
  it at its base.

## 7. Delivered by the next pull request

Part 2's criteria, A6 to A15, now sit in section 3 with their lines in its fence (section 10).

## 8. Mutation rows (band `S34700-S34799`)

Part 1, `MUTATIONS`:

| stem | crate, path | mutates | killer |
|---|---|---|---|
| `S34700-THE-LOGIN-PAIR-IS-THE-ENGINES-LOGIN-CALL` | ffi, `src/allow_list.rs` | the pair's method 3 to 4 | `login::a1_a_login_through_the_adapter_returns_the_servers_host_key` |
| `S34701-THE-CORE-ADMITS-THE-LOGIN-PAIR` | engine-core, `src/table.rs` | the native column's (1,3) to (1,4) | `parity::each_adapter_table_equals_its_transport_column` (the core's column no longer equals the adapter's list) |
| `S34702-AN-ABSENT-ENDPOINT-IS-REFUSED` | engine-core, `src/login_guard.rs` | the absent-endpoint arm admits | `login_guard::a3_the_guard_admits_https_and_loopback_http_alone` |
| `S34703-ONLY-HTTPS-IS-ADMITTED` | engine-core, `src/login_guard.rs` | the scheme test's `==` to `!=` | `login_guard::a3_the_guard_admits_https_and_loopback_http_alone` |
| `S34704-PLAIN-HTTP-ONLY-TO-LOOPBACK` | engine-core, `src/login_guard.rs` | the loopback test forced true | `login_guard::a3_the_guard_admits_https_and_loopback_http_alone` |
| `S34705-NO-CREDENTIALS-IN-THE-URL` | engine-core, `src/login_guard.rs` | the userinfo arm deleted | `login_guard::a3_the_guard_admits_https_and_loopback_http_alone` |
| `S34706-THE-LOGIN-PASSES-THE-GUARD` | engine-core, `src/dispatch.rs` | the guard's call before the engine removed | `login_guard::a4_a_refused_login_never_reaches_the_engine_and_names_no_secret` |
| `S34707-THE-GUARD-REFUSES-AS-INVALID-INPUT` | engine-core, `src/login_guard.rs` | the refusal's kind to `SYNC_OTHER_ERROR` | `login_guard::a4_a_refused_login_never_reaches_the_engine_and_names_no_secret` |

Part 2, `SCRIPT_MUTATIONS` (killers are A12 to A14's methods), and the codec's rows in
`swift-mutants.json`:

| stem or id | path | mutates | killer |
|---|---|---|---|
| `S34710-THE-REGISTER-IS-CLOSED` | `scripts/tests/test_ios_thin_swift.py` | an unlisted file admitted | A12 |
| `S34711-ONLY-THE-CREDENTIAL-FILE-NAMES-SECURITY` | same | `Security` dropped from the credential door | A12 |
| `S34712-A-BUDGET-IS-EXACT` | same | the count's `==` to `<=` | A12 |
| `S34713-NO-FILE-NAMES-USER-DEFAULTS` | same | `UserDefaults` dropped from the forbidden names | A12 |
| `S34714-THE-ENDPOINT-PLACEHOLDER-IS-INVALID` | `scripts/tests/test_ios_app_tree.py` | the `.invalid` host test dropped | A13 |
| `S34715-THE-ITEM-STAYS-ON-THIS-DEVICE` | same | the accessibility constant test dropped | A13 |
| `S34716-APP-TRANSPORT-SECURITY-IS-UNTOUCHED` | same | the `NSAppTransportSecurity` absence test dropped | A13 |
| `S34717-THE-ARCHIVE-IS-UNSIGNED` | `scripts/tests/test_ci_workflows.py` | the archive's signing-off test dropped | A14 |
| codec: the endpoint is field 3 | `Messages.swift` | the endpoint's tag to field 2 | A6 |
| codec: the host key is field 1 | `Messages.swift` | the host key's field to 2 | A7 |
| codec: the message is field 1 | `Messages.swift` | the message's field to 4 | A7 |

Each row's `find` is one exact line of the code the build writes; a find that occurs more than
once is VOID, so the build fixes each find after the code is written.

## 9. Amendments: what part 1 leaves unchanged

- `ios/app.yml`: unchanged in this part; delivered by part 2
- `ios/Config/App.xcconfig`: unchanged in this part; delivered by part 2
- `ios/App/Info.plist`: unchanged in this part; delivered by part 2
- `ios/App/PrivacyInfo.xcprivacy`: unchanged in this part; delivered by part 2
- `ios/App/Sources/DeckStreakApp.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/AppModel.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/EngineSession.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/SyncCredentialStore.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/SyncConfiguration.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/DeckListView.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/AccountView.swift`: unchanged in this part; delivered by part 2
- `ios/AppTests/CredentialStoreTests.swift`: unchanged in this part; delivered by part 2
- `ios/AppUITests/ShellFlowTests.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/Sources/HarnessWire/Messages.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/swift-mutants.json`: unchanged in this part; delivered by part 2
- `ios/swift-roles.json`: unchanged in this part; delivered by part 2
- `scripts/tests/test_ios_thin_swift.py`: unchanged in this part; delivered by part 2
- `scripts/tests/test_ios_app_tree.py`: unchanged in this part; delivered by part 2
- `scripts/tests/test_ci_workflows.py`: unchanged in this part; delivered by part 2
- `.github/workflows/xcframework.yml`: unchanged in this part; delivered by part 2

## 10. Amendments: what part 2 delivers

Part 2 delivers R4 to R14, and section 7's rows A6 to A15 now sit in section 3, with their lines
in its fence. It was cut at `dev` `1a3bdcf3`. Seven points move from the text above; ADR-358 D9 and
D10 decide the two that change a requirement, and the rest are named here.

- **R11's isolation role.** The register gains the role `isolation`, for the card view's sources
  under `ios/CardIsolation/Sources/` (SPEC-348 R17). Like the harness's files, they are bound by
  the closed register and the forbidden names alone, with no door and no budget. Each package
  manifest is `manifest`; every file of a test target (`ios/*/Tests/`, `ios/HarnessTests/`,
  `ios/HarnessUITests/`, `ios/CardProbeTests/`, `ios/AppTests/`, `ios/AppUITests/`) is `test`; the
  harness's sources and the card probe's host are `harness`; the codec's two sources are `wire`.
  The register is read over the files git tracks, so a file on disk that git does not track is
  never judged.
- **R11's network names (ADR-358 D9).** `import Network`, `NWConnection` and `NWConnectionGroup`
  are admitted only in `isolation` files and in `test` files under `ios/CardIsolation/Tests/` and
  `ios/CardProbeTests/`, where the card probe's listeners hold them, and the census prints each
  admission by its file and its name. Those names stay refused in every other file, and every other
  name R11 forbids (`URLSession`, `URLRequest`, `SQLite3`, `sqlite3_`, `UserDefaults`,
  `@AppStorage`, `NSUbiquitousKeyValueStore`) stays refused in every file, admitted ones included.
- **R13's generate line (ADR-358 D10).** `xcodegen generate --spec ios/app.yml` is the first line
  of the new step "the app's tests, Debug, on the iPhone and then the iPad", which runs after the
  harness's steps. The step "the project, generated" keeps its one line, so the TestFlight lanes'
  copy of the harness's steps (SPEC-352 A20) stays equal and no lane file changes. A14 holds the
  step and its one line.
- **The destinations.** A8 to A11's `-destination` strings carry `,OS=$SIM_OS`, as every
  destination in the `harness` job does.
- **The fragment.** This part's entry is `changelog.d/ios-app-shell-347.md`, a new file, because
  `changelog.d/ios-app-347.md` is part 1's.
- **The seam's values and the app's property list.** `DS_SYNC_USER`'s placeholder is
  `invalid-sync-user`. Besides R5's seven keys, `ios/App/Info.plist` carries the harness's launch
  screen, scene manifest and both orientation keys, so the app fills the screen on the iPhone and
  the iPad. The app has no asset catalog.
- **The lane.** Section 5 leaves the scheme move and the product icon to #634; SPEC-352's section 5
  names them #625's. This part does neither, and leaves both texts as they are.

Files this part changes that section 4 does not name:

- `docs/decisions/ADR-358-the-app-shell-its-own-project-one-keychain-item-and-the-endpoint-guard-in-the-core.md`: D9 and D10
- `docs/schematics/app-shell-login-and-deck-list.md`: section 8, the app's two steps
- `docs/red-first/SPEC-347.md`: part 2's lines
- `changelog.d/ios-app-shell-347.md`: part 2's entry
- `scripts/tests/test_one_static_library.py`: its planted second generator spec moves from `ios/app.yml`, which R4 makes a tracked file, to `ios/planted.yml`, a path no tree holds, with the same plants and the same refusals; at `ios/app.yml` each plant would be listed twice

`scripts/tests/test_ci_workflows.py`'s new class reads the workflow through `load`, as its
neighbours do, so it lists nothing in `NOT_WORKFLOW_READS`.
`scripts/tests/test_ci_workflows.py` lists two sites of `scripts/tests/test_ios_app_tree.py` in `DYNAMIC_IMPORTS`, the table the read census sums with `NOT_WORKFLOW_READS`: the exception class the property-list parser raises, caught to refuse a malformed property list by name, and the split of a settings value into its scheme and host. Neither reads a file, and neither reads a workflow.
`scripts/tests/test_ios_thin_swift.py` counts each call R11 lists in both of its spellings, the call form (`.filter(`, `.first(where:`) and the trailing closure (`.filter {` or `.filter{`, `.first {` or `.first{`), so a call counts once however it is written; a planted tree for each trailing spelling is refused by the budgets' name.

Part 1's rows, which this part leaves as they are:

- `crates/ffi/src/allow_list.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/Cargo.toml`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/login.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/support/sync_server.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/table.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/login_guard.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/dispatch.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/lib.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/Cargo.toml`: unchanged in this part; delivered by part 1
- `crates/engine-core/tests/login_guard.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/tests/table.rs`: unchanged in this part; delivered by part 1
- `Cargo.toml`: unchanged in this part; delivered by part 1
- `Cargo.lock`: unchanged in this part; delivered by part 1
- `changelog.d/ios-app-347.md`: unchanged in this part; delivered by part 1
