# Schematic: the app clients, the engine's two transports, the sync server and the one router

Kind: component diagram and data flow, with the full-sync choice as a sequence. Read at DeckStreak
`dev` df2a4cd (`crates/notifications/src/transport.rs:84`, the router's one outbound port;
`crates/ingest/Cargo.toml` and `crates/readings/Cargo.toml`, the engine's only consumers;
`docs/CONTEXT-MAP.md`). Added by SPEC-334, the app campaign's PRD. Every component drawn with a
dashed edge or named "planned" does not exist yet: it enters the tree, and the context map, through
the delivery that builds it, under the ADR named beside it.

## The components

```mermaid
flowchart LR
  subgraph devices["the owner's devices"]
    ios["iPhone and iPad app: SwiftUI, universal (ADR-335, ADR-342), planned"]
    iosCard["card frame: WKWebView, no network, no message handler (UX-05)"]
    web["web client: SvelteKit study screens (ADR-336), planned"]
    webCard["card frame: sandboxed iframe under its own CSP (UX-05)"]
    worker["Worker: the engine on WASM over OPFS storage, planned"]
    remote["8BitDo remote: gamepad mode or keyboard mode (ADR-342)"]
    desktop["Anki desktop and AnkiMobile"]
  end
  subgraph core["shared Rust, planned"]
    engine["engine core: an allow-listed dispatcher over the engine's backend; OwnerGesture (ADR-337)"]
    ffi["FFI umbrella crate: one static library (ADR-335)"]
    wasm["WASM build of the engine core (ADR-336)"]
    fsrs7["FSRS-7 crate, isolated, never linked into the engine (ADR-338)"]
    xp["XP crate, I/O-free (SPEC-334 R13)"]
  end
  subgraph host["the service's own host (ADR-340)"]
    proxy["HTTPS reverse proxy, one origin"]
    sync["sync server, built from the pinned fork, hashed passwords"]
    daemon["DeckStreak daemon: ingest, api, the one router (ADR-041)"]
    snap["offsite snapshot and its drilled restore (BAK-01)"]
  end
  apns["APNs"]
  wpush["web push service"]
  remote -- "GameController, UIKeyCommand" --> ios
  remote -- "Gamepad API, keydown" --> web
  ios --> iosCard
  web --> webCard
  web --> worker
  ios --> ffi
  worker --> wasm
  ffi --> engine
  wasm --> engine
  ffi --> fsrs7
  ffi --> xp
  wasm --> fsrs7
  wasm --> xp
  ios -- "sync, HTTPS" --> proxy
  worker -- "sync, HTTPS, same origin" --> proxy
  desktop -- "sync, HTTPS" --> proxy
  web -- "sign-in, XP, nudge settings" --> proxy
  ios -- "sign-in, XP, device token" --> proxy
  proxy --> sync
  proxy --> daemon
  daemon -- "ingest reads the synced copy, read-only" --> sync
  sync --> snap
  daemon -- "router transport (ADR-341), planned" --> apns
  daemon -- "router transport (ADR-341), planned" --> wpush
  apns --> ios
  wpush --> web
```

- The engine has one port and two transports: FFI on iPhone and iPad, WASM in the browser
  (SPEC-334 R3). If the browser engine spike says NO-GO, the Worker is replaced by ADR-336's
  fallback: the server-side study collection behind the daemon's study API, and the web client
  calls it over HTTPS instead.
- The FSRS-7 crate and the XP crate join the same umbrella static library on iPhone and iPad and
  the same WASM build in the browser. FSRS-7 never links into the engine, and it writes only the
  stock fields through it (ADR-338).
- The daemon's router keeps its Telegram transport (`BotTransport`,
  `crates/notifications/src/transport.rs:84`) until native push carries it; then the Mini App and
  the bot retire (ADR-341).

## The study flow

```mermaid
flowchart TD
  open["open the collection"] --> start["normal sync at session start"]
  start --> replay["FSRS-7 preset only: rebuild memory state from review history (ADR-338)"]
  replay --> queue["today's due queue from the stock scheduler"]
  queue --> scores{"external due-card scores enabled?"}
  scores -- "yes" --> reorder["reorder today's due cards only (ADR-339)"]
  scores -- "no" --> card
  reorder --> card["show the card in its sandboxed frame"]
  card --> reveal["reveal; the persona sheet opens only now, card timer paused"]
  reveal --> answer["answer: the engine records the review"]
  answer --> shown["XP crate: per-review XP shown at once"]
  shown --> queue
  answer --> stop{"session ends?"}
  stop -- "yes" --> endsync["normal sync at session end; warn while reviews are unsynced"]
  endsync --> ledger["daemon: ingest reads the reviews; the ledger confirms XP and day-level bonuses"]
  ledger --> check["confirmed XP is never below shown XP (SPEC-334 R13)"]
```

## The full-sync choice

```mermaid
sequenceDiagram
  participant O as owner
  participant C as client (UI layer)
  participant E as engine core
  participant S as sync server
  participant B as offsite snapshot
  S->>C: a normal sync is refused: a one-way sync is required
  C->>O: what each side loses, as counts
  O->>C: tap: upload or download (an exempt tap, ADR-337)
  C->>E: write the on-device backup, with the OwnerGesture
  E-->>C: backup written
  alt upload
    C->>B: does the server's offsite snapshot exist?
    B-->>C: yes, or the upload stops here
    C->>E: upload, with the OwnerGesture
    E->>S: one-way upload
  else download
    C->>E: download, with the OwnerGesture
    S->>E: one-way download
  end
```

No normal sync may run between the backup and the snapshot check and the one-way write. SPEC-334
section 9 records the TLA+ model of that ordering as a candidate for the delivery that builds it.

## The sync-server move

```mermaid
flowchart LR
  old["the current third-party host: the sync server today"] -- "final sync on every client; freeze" --> snapshot["offsite snapshot; restore drilled"]
  snapshot --> copy["copy the sync data"]
  copy --> new["the service's own host: sync server behind HTTPS"]
  new --> repoint["repoint desktop, then AnkiMobile, then the app; set a new sync password"]
  old -. "kept as the rollback until the owner retires it" .-> repoint
```

## Builds to the owner's devices

```mermaid
flowchart LR
  devtip["dev tip"] -- "manual dispatch after an iOS delivery lands (ADR-344)" --> devjob["macOS job: build number = dev's first-parent commit count"]
  devjob --> internal["internal TestFlight: dev app id, the owner the only tester"]
  internal -. "until both rulings are on dev and the sync cutover is done" .-> staging["staging sync user"]
  tag["SemVer tag on main"] --> release["release job: the tag's version, main's first-parent count"]
  release --> realapp["internal TestFlight: the app id that holds real data"]
```

A TestFlight upload publishes a signed build for the owner's devices; it changes no host and is
not a deploy (ADR-335, RELEASING.md). The team id, the app ids, the associated-domains file and
the upload key come from the private deploy rail, never from this repository.
