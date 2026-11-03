# The owner route and the companion loader

Read at `92755ef0`. SPEC-511 and ADR-529 decide what this draws: the order in which the web client's Worker loads the
owner's scheduling companion and fails closed (sequence), and where each file and each frame goes between the route,
its directory, the Worker and the web engine (data flow).

## 1. The loader (sequence)

Every step that fails reads `Absent`, and an `Absent` companion answers every request with the `Refused { Absent }`
frame, which the web engine's `decide` refuses. The Worker builds no frame: the web engine builds each one.

```mermaid
sequenceDiagram
  participant W as the Worker (public code)
  participant R as owner route /api/companion/v1/
  participant C as Web Crypto
  participant M as companion module
  participant E as web engine (Rust)
  W->>R: GET manifest.json (owner session cookie)
  R-->>W: 401 no session, 404 no directory or file, or 200 private no-store
  Note over W: not 200, or not exactly {module, weights} as 64 hex each: Absent
  W->>R: GET module.wasm, GET weights.bin
  R-->>W: 200 with the exact bytes, each at most 16 MiB
  W->>C: SHA-256 of each
  C-->>W: digests
  Note over W: a digest other than the manifest's: Absent
  W->>M: WebAssembly.instantiate(bytes, IMPORTS), IMPORTS a frozen empty object
  Note over W,M: a link error (any import asked for) or a missing export: Absent
  W->>M: alloc(len), copy the weights into memory, open(ptr, len)
  M-->>W: u64 packing the HelloReply frame's pointer and length
  W->>M: free both buffers
  W->>E: decide(hello, epoch, Preview frame, reply frame) on each press
  E-->>W: 17 bytes admitted, or 3 bytes refused
```

## 2. The stored checkpoint (sequence)

```mermaid
sequenceDiagram
  participant W as the Worker
  participant O as OPFS deck-streak-companion/
  participant E as web engine
  participant M as companion module
  W->>O: read state.bin
  O-->>W: the checkpoint frame verbatim, or none (genesis)
  W->>E: companion_restore(epoch, checkpoint)
  E-->>W: the Restore frame, built through Restore::from(CheckpointReply), or none
  W->>M: call(Restore frame)
  M-->>W: Ack frame
  W->>E: companion_load_check(hello, checkpoint, ack, head named, fold index)
  E-->>W: 0 keep, or a named discard
  Note over W,M: a discard: call(Purge frame), remove the directory, fold from genesis
```

## 3. Route, directory, Worker and engine (data flow)

```mermaid
flowchart LR
  dir[("the owner's directory, DECKSTREAK_COMPANION_DIR, outside the web root")]
  route["api: three literal routes, OwnerSession first"]
  site[("the public site: the web root, with the public engine under /engine/")]
  worker["the Worker: loader, Absent, checkpoint store"]
  opfs[("OPFS: deck-streak-companion/state.bin")]
  engine["web engine: decide, restore, load_check, purge"]
  module["companion module: memory, alloc, free, open, call; no import"]
  dir -->|manifest.json, module.wasm, weights.bin| route
  route -->|bytes, private no-store| worker
  site -->|the public engine's module and bindings| worker
  worker -->|instantiate with IMPORTS| module
  worker <-->|frames as bytes| module
  worker <-->|frames as bytes| engine
  worker <-->|checkpoint bytes verbatim| opfs
```

| crosses | from | to | content |
|---|---|---|---|
| the manifest, the module, the weights | the owner's directory, through the route | the owner's Worker only | no learner content; uncached |
| frames | the web engine and the module | each other, through the Worker | integers only (SPEC-501 R1) |
| the checkpoint frame | the module | OPFS, and back verbatim | derived, opaque; kept on the device only |
| nothing | the module | storage, network, clock | the import object gives it none |

The census of the built site (`scripts/companion-site-census.py`) holds that no file of the left-hand column ever sits
in the public site.
