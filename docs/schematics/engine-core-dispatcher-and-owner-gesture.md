# The engine core: one dispatcher for both clients, and the owner-gesture token

Component diagram and call paths for SPEC-345 and ADR-356. Every `path:line` citation was read
at `dev` `1eec0870e67e801fc3aa279318219d6986116ec7`; the engine's at the pinned fork rev
`c538de55a23e695234e794029fce0dafff2d36a9`.

## Components, before

At `dev`, each adapter holds the engine's `Backend` itself, and the web engine's named exports
reach it without consulting its own table (`crates/web-engine/src/wasm.rs:61-68`, `:74-86`).

```mermaid
flowchart LR
  SUI["SwiftUI harness"] -->|"UniFFI"| FFI["deck-streak-ffi<br/>Engine.run, ALLOW_LIST: 5 pairs"]
  PAGE["web page"] -->|"postMessage"| WK["Worker: session.ts"]
  WK -->|"named exports"| WE["deck-streak-web-engine<br/>wasm.rs call and query"]
  WE -.->|"run_method only"| ADMIT["study.rs admit: STUDY_CALLS, 8 pairs"]
  FFI -->|"allowed, then run_service_method"| ENG["Anki engine: Backend"]
  WE -->|"run_service_method, unchecked"| ENG
  WE -->|"run_db_command_bytes with SQL"| ENG
  ING["deck-streak-ingest<br/>the mirror's engine port"] -->|"Collection API"| ENG
```

## Components, after

The core is the only client-side holder of the engine. Only the two adapters may depend on it, so
only they can name `OwnerGesture::from_tap`; a crate without the edge fails to compile, and the
graph census refuses the edge.

```mermaid
flowchart LR
  SUI["SwiftUI screens"] -->|"UniFFI"| FFI["deck-streak-ffi<br/>Engine.run, Engine.run_exempt<br/>ALLOW_LIST checked first"]
  PAGE["web page"] -->|"postMessage"| WK["Worker: session.ts"]
  WK -->|"named exports"| WE["deck-streak-web-engine<br/>wasm.rs call, query, run_exempt"]
  FFI -->|"Transport Native"| D
  WE -->|"Transport Web, wasm32 only"| D
  subgraph CORE["deck-streak-engine-core"]
    D["Dispatcher<br/>start, run, read, run_exempt"]
    T["table<br/>ORDINARY with native and web marks<br/>EXEMPT: six writes and their target kinds"]
    G["OwnerGesture<br/>one write, one target<br/>from_tap only"]
    R["Read<br/>NoteCount, CardSnapshot"]
    D --> T
    G --> D
    R --> D
  end
  D -->|"run_service_method"| ENG["Anki engine: Backend"]
  D -->|"run_db_command_bytes, fixed statements"| ENG
  ING["deck-streak-ingest<br/>the mirror's engine port"] -->|"Collection API, 7 lines held by text"| ENG
  SERVER["daemon, api, bot, mcp, coordination"] -.->|"no edge: compile error"| CORE
```

| unit | owns | may not |
|---|---|---|
| `deck-streak-engine-core` | the `Backend`, the ordinary and exempt tables, `OwnerGesture`, the fixed reads | depend on any DeckStreak crate; declare a feature or a `staticlib` |
| `deck-streak-ffi` | the native transport, its allow-list, `EngineRefusal`, the native exempt entry | name `anki` outside its tests |
| `deck-streak-web-engine` | the browser transport, its study calls, the `wasm32` exports | name the core or the engine in its native build |
| `deck-streak-ingest` | the mirror's port over the engine's `Collection` API | add an engine write line the containment census does not hold |
| every other member | nothing of the engine | name the core, the engine or an adapter |

## An ordinary call: AnswerCard, (13,4)

Before, on each transport:

```mermaid
sequenceDiagram
  participant S as Swift screen
  participant F as ffi Engine
  participant W as web wasm.rs
  participant E as engine Backend
  S->>F: run(13, 4, request bytes)
  F->>F: allowed(13, 4) finds SchedulerService.AnswerCard
  F->>E: run_service_method(13, 4, bytes)
  E-->>F: response bytes, or error bytes
  F-->>S: bytes, or EngineRefusal.Engine
  Note over W,E: the Worker's answer export calls call(SCHEDULER, 4) and reaches run_service_method without admit
  W->>E: run_service_method(13, 4, bytes)
  E-->>W: response bytes
```

After:

```mermaid
sequenceDiagram
  participant S as Swift screen
  participant F as ffi Engine
  participant W as web wasm.rs
  participant D as core Dispatcher
  participant E as engine Backend
  S->>F: run(13, 4, request bytes)
  F->>F: allowed(13, 4), unchanged
  F->>D: run(13, 4, bytes) on Transport Native
  D->>D: ORDINARY row 13,4 is marked native
  D->>E: run_service_method(13, 4, bytes)
  E-->>D: response bytes
  D-->>F: bytes
  F-->>S: bytes
  W->>D: run(13, 4, bytes) on Transport Web
  D->>D: ORDINARY row 13,4 is marked web
  D->>E: run_service_method(13, 4, bytes)
  E-->>D: response bytes
  D-->>W: bytes
```

A pair the transport's column lacks returns `Refusal.NotAllowed` before the engine is reached; the
native adapter's own `allowed()` refuses it first, with the text `tests/refusal_text.rs` pins.

## An exempt call: Forget, (13,17)

Before: no exempt path exists. The native allow-list refuses (13,17) as `NotAllowed`; the web engine
has no export for it, and `run_method` refuses it (`crates/web-engine/tests/study.rs:81`). Any crate
that names `anki` could still call the engine's `Collection` API for it directly.

```mermaid
sequenceDiagram
  participant S as Swift screen
  participant F as ffi Engine
  S->>F: run(13, 17, bytes)
  F->>F: allowed(13, 17) is None
  F-->>S: EngineRefusal.NotAllowed, service 13 method 17
```

After part 1: the ordinary path holds it for a gesture.

```mermaid
sequenceDiagram
  participant W as web wasm.rs
  participant D as core Dispatcher
  W->>D: run(13, 17, bytes)
  D->>D: 13,17 is in EXEMPT
  D-->>W: Refusal.NeedsGesture, service 13 method 17
```

After part 2: the owner's tap, through the adapter's one exempt entry.

```mermaid
sequenceDiagram
  participant S as Swift gesture handler
  participant F as ffi Engine
  participant G as core OwnerGesture
  participant D as core Dispatcher
  participant E as engine Backend
  S->>F: run_exempt(Forget, card c, request bytes)
  F->>G: from_tap(Forget, Target Card c)
  G-->>F: gesture, or a refusal when the target is not a card
  F->>D: run_exempt(gesture, bytes), the gesture moved in
  D->>D: decode as ScheduleCardsAsNewRequest
  D->>D: card_ids equal to the one card c, or refuse
  D->>E: run_service_method(13, 17, the checked message re-encoded)
  E-->>D: response bytes
  D-->>F: bytes
  F-->>S: bytes, or the exempt refusal
```

A request whose `card_ids` is empty, names another card, or names `c` and another is refused at the
check, and the engine never sees it. A daemon, bot, coordination or API caller has no edge to the
core, so it cannot name `from_tap` at all; the containment census refuses an added edge, a
`#[path]` or `include!` of a core file, and a new engine write line, each by name.

## The containment census's population

```mermaid
flowchart TB
  M["every workspace member's Cargo.toml"] --> GR["graph census<br/>core named by ffi and web-engine only<br/>anki named outside dev-deps by core and ingest only<br/>no member names an adapter"]
  SRC["every member's src, examples, tests, benches, build.rs"] --> CC["containment census<br/>from_tap and run_exempt only in the two entry files<br/>engine write names only at the 7 held ingest lines<br/>no path or include of a core file"]
  PL["planted callers in a scratch tree<br/>daemon, bot, coordination, ingest"] --> CC
  GR --> V["examined counts printed<br/>a planted caller refused by name"]
  CC --> V
```
