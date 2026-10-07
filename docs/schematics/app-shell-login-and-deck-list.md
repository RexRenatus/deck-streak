# Schematic: the app shell, its login and its deck list (SPEC-347, ADR-358)

Read at DeckStreak `dev` `96eae6afd97384108cb31d66e7d3243d2d70f5af` (DEV), #656 at
`d295cd886a520ea61387723c0b081fed29a33c5d` (HARNESS) and #659 at
`01773951348d8c14fe9f0ba186f5e78c2244d954` (APPLE); the engine's files at the revision root
`Cargo.toml` line 133 names. Every `path:line` below is at those commits. The core
(`crates/engine-core`) and the umbrella's shape are as #623's and #624's designs place them; this
schematic draws only what #625 adds to them.

## 1. Components, after

```mermaid
flowchart LR
  subgraph App["ios/App: the DeckStreak target (ios/app.yml)"]
    Entry["DeckStreakApp#59; entry"]
    Views["DeckListView, AccountView#59; view"]
    Model["AppModel#59; model, main actor"]
    Session["EngineSession#59; session, an actor"]
    Store["SyncCredentialStore#59; credential"]
    Config["SyncConfiguration#59; config"]
  end
  subgraph Packages["local packages only"]
    Wire["HarnessWire#59; wire, the client's codec"]
    Bindings["EnginePackage: DeckStreakFFI, the generated bindings"]
    XCF["deck_streak_ffiFFI: the XCFramework, this run's artifact"]
  end
  subgraph Rust["one Rust static library"]
    FFI["deck-streak-ffi: the umbrella, allow-list with (1,3)"]
    Core["deck-streak-engine-core: Dispatcher, native column, login guard"]
    Engine["the engine: Backend, its HTTP client with bundled roots"]
  end
  Plist["Info.plist: DSSyncEndpoint, DSSyncUser"]
  Settings["App.xcconfig placeholders, the lane's command line, or the ignored local include"]
  Keychain[("Keychain: one generic password, this device only")]
  Server["the sync server, HTTPS (#628)"]
  Disk[("Application Support: collection.anki2")]

  Entry --> Views --> Model
  Model --> Session
  Model --> Store --> Keychain
  Model --> Config --> Plist
  Settings -->|rendered at build| Plist
  Session --> Wire
  Session --> Bindings --> XCF --> FFI --> Core --> Engine
  Engine --> Disk
  Engine -->|outside App Transport Security| Server
```

What each edge may carry is the census's door (SPEC-347 R11): only `session` reaches the
bindings, the codec and the file system; only `credential` reaches the Keychain; only `config`
reads the info dictionary. No Swift file opens a network connection or a store of its own.

## 2. The Apple job body, before and after

```mermaid
flowchart TB
  subgraph Callers["callers (APPLE)"]
    OnChange["apple-on-change.yml: a pull request into dev touching crates/ffi, ios, the manifests"]
    OnTag["apple-on-tag.yml: a SemVer tag"]
  end
  subgraph Body["xcframework.yml, workflow_call (APPLE line 14)"]
    X["xcframework: two slices, bindings, the harness's collection, artifact"]
    HW["harness-wire: the codec's tests and mutants (HARNESS line 165)"]
    subgraph H["harness, needs xcframework (HARNESS line 254)"]
      H1["place this run's framework and bindings"]
      H2["the project, generated: ios/project.yml, and after: ios/app.yml"]
      H3["the harness's tests, measurements, size, required-reason symbols (lines 292 to 362)"]
      H4["after: the app's tests, Debug, on the iPhone and then the iPad"]
      H5["after: the app, archived unsigned for a device, and its required-reason imports"]
      H6["the report, with the app's rows (line 363)"]
    end
  end
  OnChange --> Body
  OnTag --> Body
  X --> H1 --> H2 --> H3 --> H4 --> H5 --> H6
```

Before: H2 generates `ios/project.yml` alone, and H4 and H5 do not exist. After: no job, workflow,
runner or cargo command is added; the tag caller runs the same steps, so a SemVer tag's build is
archived the same way the lane will sign it (#634).

## 3. The login, before

```mermaid
sequenceDiagram
  participant S as Swift (harness)
  participant F as deck-streak-ffi
  S->>F: run(1, 3, bytes)
  F-->>S: EngineRefusal.NotAllowed, service 1 method 3
```

No ref holds a sync pair on the native path (`crates/ffi/src/allow_list.rs`, DEV line 23, HARNESS
line 25), and no Swift file names the Keychain (SPEC-347 M5).

## 4. The login, after

```mermaid
sequenceDiagram
  actor L as Learner
  participant V as AccountView
  participant M as AppModel
  participant C as SyncConfiguration
  participant S as EngineSession
  participant F as deck-streak-ffi
  participant K as engine core
  participant E as the engine
  participant N as the sync server
  participant Q as SyncCredentialStore
  L->>V: types the password, taps Sign in
  V->>M: signIn(password)
  M->>C: endpoint, user (from the Info.plist)
  M->>M: state signingIn#59; controls disabled
  M->>S: login(user, password, endpoint)
  S->>F: run(1, 3, Requests.syncLogin bytes)
  F->>F: allow-list holds (1,3)
  F->>K: run(1, 3, bytes)
  K->>K: native column holds (1,3)
  K->>K: login guard decodes the request
  alt endpoint absent, unparseable, carrying credentials, or not https (loopback http aside)
    K-->>F: BackendError, kind INVALID_INPUT, the rule's sentence
    F-->>S: EngineRefusal.Engine(bytes)
  else admitted
    K->>E: SyncLogin
    E->>N: HTTPS, the engine's own client
    alt accepted
      N-->>E: host key
      E-->>K: SyncAuth bytes
      K-->>F: bytes
      F-->>S: bytes
      S->>S: Responses.syncAuth gives the host key
      S-->>M: host key
      M->>Q: save(host key, service endpoint, account user)
      Q->>Q: SecItemAdd, after first unlock, this device only, not synchronizable
      M->>M: password cleared#59; state signedIn
    else refused (wrong password, no network)
      N-->>E: refusal
      E-->>F: BackendError bytes (SYNC_AUTH_ERROR, NETWORK_ERROR)
      F-->>S: EngineRefusal.Engine(bytes)
    end
  end
  S-->>M: the refusal's message (Responses.engineMessage)
  M->>M: password cleared#59; state signedOut#59; nothing stored
```

The password lives in the field and the model for the call alone. The guard's sentence, like the
engine's, names neither the endpoint, the user nor the password (SPEC-347 R2, R3).

## 5. The account's states

```mermaid
stateDiagram-v2
  [*] --> SignedOut: launch, no item for this endpoint and user
  [*] --> SignedIn: launch, an item for this endpoint and user
  SignedOut --> SigningIn: Sign in tapped
  SigningIn --> SignedIn: host key stored
  SigningIn --> SignedOut: refusal shown, nothing stored
  SignedIn --> SignedOut: Sign out deletes the item
```

Sign-out is offered only in `SignedIn`, and the sheet's controls are disabled in `SigningIn`, so no
sign-out can race a login. A build with another endpoint or user starts in `SignedOut`, because its
key finds no item (ADR-358 D2).

## 6. The deck list, before (the harness)

```mermaid
sequenceDiagram
  participant H as HarnessModel
  participant S as EngineSession (harness)
  participant E as engine via deck-streak-ffi
  H->>S: open()
  S->>S: copy the bundled synthetic collection to a fresh directory
  S->>E: run(3, 0, OpenCollection)
  H->>S: deckNames()
  S->>E: run(7, 13, include_filtered)
  E-->>S: deck names
  S-->>H: names, in the engine's order
```

HARNESS `ios/Harness/Sources/EngineSession.swift` lines 60 to 90: every launch starts from the same
bundled card.

## 7. The deck list, after (the app)

```mermaid
sequenceDiagram
  participant M as AppModel
  participant S as EngineSession (app)
  participant E as engine via deck-streak-ffi and the core
  participant D as Application Support
  M->>S: open()
  S->>D: the collection's fixed path, its media folder
  S->>E: run(3, 0, OpenCollection)
  E->>D: creates the collection when absent (its default deck)
  M->>S: decks()
  S->>E: run(7, 13, include_filtered)
  E-->>S: deck names
  S-->>M: names, in the engine's order
  M-->>M: the list, or the refusal's sentence in its place
```

Nothing of the list is cached outside the engine's collection (SPEC-347 R7). The detail pane
reads "Choose a deck" until the review screen exists (#632).

## 8. The `harness` job after part 2 (SPEC-347 R13, ADR-358 D10)

Read at `dev` `1a3bdcf3`, where `xcframework.yml` has grown since section 2 was drawn: the jobs
are `xcframework` (line 30), `harness-wire` (178), `card-isolation` (267) and `harness` (356), so
section 2's line numbers are HARNESS's, not this tree's. In `harness`, "the project, generated" is
line 394, the harness's tests 397, the card view's planted suite 410, the Release measurements 421,
the Release app built alone 434, its size 443, its required-reason symbols 449, the report 479 and
the upload 599. Section 2's H2 is superseded: the app's project is generated by its own step, and
"the project, generated" keeps its one line.

```mermaid
flowchart TB
  subgraph H["harness, needs xcframework"]
    P["place this run's framework, bindings and collection, the icon, the generator"]
    G["the project, generated: ios/project.yml alone, the lanes' copy ends here"]
    T["the harness's tests, the card view's suite, the measurements, the Release app, its size and symbols"]
    A1["the app's tests, Debug, on the iPhone and then the iPad: first line generates ios/app.yml"]
    A2["the app, archived unsigned for a device: Release, generic iOS, imports against ios/App/PrivacyInfo.xcprivacy"]
    R["the report: the harness's rows and the app's rows, whatever the outcome"]
    U["upload the report and the result bundles"]
  end
  Lanes["testflight lanes: copy P through G (SPEC-352 A20)"]
  P --> G --> T --> A1 --> A2 --> R --> U
  G -. copied by .-> Lanes
```

| step | runs | signing | reads |
|---|---|---|---|
| the app's tests, Debug, on the iPhone and then the iPad | `xcodegen generate --spec ios/app.yml`, then `xcodebuild test` of the `DeckStreak` scheme on both simulators, one after the other | ad hoc on its command line, no team (SPEC-347 section 6's fallback) | its own derived data and result bundle `app.xcresult` |
| the app, archived unsigned for a device | `xcodebuild archive` of the `DeckStreak` scheme, Release, `generic/platform=iOS`, then `nm -u` of the archived executable | off on its command line | `ios/App/PrivacyInfo.xcprivacy`, against the categories the harness's check names |
| the report | as before, plus the app's minutes, its test cases on each simulator and its required-reason lines | none | the app's result bundle and its symbols file |

No job, workflow, runner or cargo command is added. The harness's own steps are unchanged, so a
failure in them stops the job before the app's steps run, and the report still runs.
