# Schematic: iPhone and iPad Phase 1 parity (SPEC-358, ADR-369)

Read at `D` = `f9381e14311a0cbe7d812946e33355c13d678e01` (dev, with the app shell (#684), the
review screen (#709) and SPEC-350 part 2 (#690) merged). The figures were first read at dev
`3fe90969` and at the shell's red stub (`73a6bda1`); the shell's and the review screen's surfaces
below are read at `D` and corrected where they moved: the review reaches `EngineSession` through
the `ReviewSession` actor (`ReviewSession.swift:24-25`), which imports the codec beside it, and
the sidebar's one toolbar button opens the account sheet (`DeckListView.swift:35`, `:60-63`) that
part b's R14 makes the settings sheet. Nothing here names a host, an identity or a schedule.

## 1. Component diagram

Arrows point from the caller to the callee. The Swift side holds no rule: the adapter and the core
decide, Swift shows. Dashed edges are tests and gates.

```mermaid
flowchart TB
  subgraph Screens["Screens: iPhone stack, iPad two columns (by size class)"]
    DL["DeckListView + sync status line"]
    RV["ReviewView / ReviewChrome / AnswerBar"]
    SS["Settings sheet: Account, Remote mapping"]
    SC["Sync choice sheet (Cancel first, focused)"]
  end

  subgraph Input["Input (census role: input)"]
    RI["RemoteInput (GameController)"]
    KC["Key commands (ReviewCommands)"]
    IT["Idle timer"]
  end

  subgraph Models["Models (state lives here)"]
    AM["AppModel"]
    RM["ReviewModel (perform)"]
    MM["MappingModel"]
    SM["SyncModel"]
  end

  ES["EngineSession (the one DeckStreakFFI importer), reached by the review through ReviewSession"]

  subgraph Adapter["Native adapter: crates/ffi"]
    EN["Engine: run, face, bury, flag, sync, unsynced, one-way entry (engine.rs)"]
    RE["remote.rs: map, RemoteMapping store, Awake"]
    VO["voices.rs"]
    ST["storage: backups, server copy"]
  end

  subgraph Core["Core: crates/engine-core"]
    DP["Dispatcher: tables, login guard on every SyncAuth call, id_sets, unsynced"]
    RW["review.rs: RED, toggled_red, BURY_USER, BuryOf, bury_of; bury_request, flag_request"]
    FS["full_sync.rs: Counted, Confirmed, BackedUp, Checked, Ready, Write; retention"]
  end

  subgraph Local["Local storage"]
    COL["Collection + media folder"]
    BK["Backups/ (two newest)"]
    SCP["Server copy"]
    MAPF["Mapping file"]
    VOF["Voices file"]
    KEY["Keychain host key"]
  end

  SRV["Sync server"]

  subgraph Gates["Gates"]
    LCI["Linux CI: cargo tests; censuses: parity, thin-Swift, one review rule, one-way call, CI workflows"]
    AJ["Apple job: harness steps, iPhone then iPad"]
    FM["Formal: FullSyncChoice"]
  end

  DL --> AM
  RV --> RM
  SS --> MM
  SC --> SM
  RI --> RM
  KC --> RM
  RM --> ES
  MM --> ES
  SM --> ES
  AM --> ES
  RM -.sets from Awake.-> IT
  ES --> EN
  EN --> RE
  EN --> VO
  EN --> ST
  EN --> DP
  DP --> RW
  DP --> FS
  FS --> ST
  ST --> BK
  ST --> SCP
  EN --> COL
  RE --> MAPF
  VO --> VOF
  ES --> KEY
  DP --> SRV
  LCI -.-> Adapter
  LCI -.-> Core
  AJ -.-> Screens
  FM -.-> FS
```

What each box owns:

| box | owns | does not own |
|---|---|---|
| Screens | layout, text, the shown choice | any rule, any call |
| Input | binding a control to its NAME; setting the idle timer from the adapter's answer | the control's meaning |
| Models | the review's card and side, the sheet's state | the action a control fires |
| `EngineSession`, `ReviewSession` | the one import of the adapter (`EngineSession`); the codec (both) | any decision |
| Adapter | the remote's map and store, `Awake`, the backup's file steps, the one-way entry | the choice's order, the flag and bury rules |
| Core | the tables, the login guard, the flag and bury rules, the choice's order and retention | any file path or platform call |

## 2. Data flows

### 2.1 A session's sync

```mermaid
sequenceDiagram
  participant M as AppModel / SyncModel
  participant S as EngineSession
  participant E as Engine (adapter)
  participant D as Dispatcher (core)
  participant V as Sync server
  M->>S: open the collection
  S->>E: open
  M->>S: signed in? start a sync
  S->>E: sync (1,5) with media
  E->>D: SyncCollection, login guard on the endpoint
  D->>V: normal sync request
  alt NO_CHANGES or NORMAL
    V-->>D: answer
    D-->>E: done#59; a new endpoint is not followed
    E->>D: unsynced
    D-->>M: reviews not yet synced, collection changed
    Note over M: status line at the sidebar's foot
  else offline or refused
    D-->>E: refused by name
    E-->>M: status says so#59; every review stays on the device
  else FULL
    D-->>M: the choice (2.2)
  end
```

### 2.2 The full-sync choice

```mermaid
sequenceDiagram
  participant O as Owner
  participant SC as Choice sheet
  participant E as Engine (adapter)
  participant D as Core (full_sync)
  participant V as Sync server
  E->>D: download the server copy (login guard)
  D->>V: fetch
  V-->>D: the server's collection
  D->>D: count by ids (Counted)
  D-->>SC: each direction with what each side loses
  Note over SC: no preselection, Cancel first and focused
  O->>SC: taps one direction (Confirmed)
  alt download
    E->>E: close (3,1), copy whole to a temp file, rename
    E->>E: reopen with the kept open request
    E->>D: ids of the copy
    D->>D: backed_up, retention (keep two, delete oldest after accept)
    D->>D: download_ready, at_write
    O->>SC: the confirm tap is the gesture
    E->>D: the one-way call (Write)
    E->>E: reopen#59; status refreshed
  else upload
    E->>D: backed_up(server copy)
    D-->>SC: refused by name at the snapshot step (owner session not yet native)
    Note over SC: both sides untouched
  end
  Note over SC: a refusal at any step names the step and leaves both sides untouched
```

### 2.3 A remote-driven review

```mermaid
sequenceDiagram
  participant R as Remote (gamepad or key)
  participant I as RemoteInput / key command
  participant RM as ReviewModel
  participant S as ReviewSession over EngineSession
  participant E as Engine (adapter, remote.rs)
  participant T as Idle timer
  R->>I: press (never the release)
  I->>RM: control name
  RM->>S: intent(mode, control)
  S->>E: intent
  E-->>RM: the intent, or nothing
  RM->>S: resolve(intent, side)
  S->>E: resolve
  E-->>RM: the action (question side: confirm only#59; answer side: confirm is Good)
  RM->>RM: perform(action)
  RM->>S: engine call (answer, bury, flag, undo, replay)
  RM->>E: Awake::wanted(review shown, scene, gamepad, key drove)
  E-->>T: set (only from this answer, only in the input role)
```

### 2.4 An undo (moved)

Part a builds no native undo: native Undo leaves the allow-list and the native run refuses (3,8)
(#714). The undo sequence drafted here moved to SPEC-358 section 7.

## 3. Boundaries this schematic holds

- Swift imports the adapter in `EngineSession` alone; GameController and the idle timer are named
  in the `input` role alone; the one-way call is made from the choice sheet's confirm handler
  alone. Each is a census, planted-control tested.
- No Swift file names a preference store or a network client; every call to the engine passes the
  adapter's allow-list, which equals the core's native column.
- The flag and bury rules, the choice's order and retention each exist once, in the core.
- A moved rule's mutation rows move with it; none is retired.
