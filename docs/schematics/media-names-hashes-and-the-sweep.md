# Schematic: a media file's hash, its names and their bindings, the names a note writes, and the sweep

Kinds: component, and the binding state machine. Read at the delivery's base `554a223e`, and at the pinned engine
`a078820f` for the grammar it ports (`rslib/src/text.rs`, `rslib/src/media/check.rs`). Decided by ADR-531; built by
SPEC-513.

## Component

The media core is one crate with no workspace edge. A collection hands it texts and stored files through
`MediaIndex`; it hands back names, bindings and a sweep plan. It reads no file, clock or environment.

```mermaid
flowchart LR
  subgraph collection["a collection (implements MediaIndex)"]
    fields["note fields and card templates (HTML)"]
    styles["note types' CSS"]
    store["the media store's files: name, added instant, imported"]
  end
  subgraph media["deck-streak-media"]
    hash["hash.rs: MediaHash::of (SHA-256), hex form, blob path"]
    name["name.rs: MediaName: certified ds-&lt;hex&gt;.ext, or imported as written"]
    bind["bind.rs: step, release_step, Bindings (write-once)"]
    scan["scan.rs: scan (the engine's tag and sound tag grammar)"]
    css["css.rs: scan_css (url( as the font pass reads it)"]
    sweep["sweep.rs: fate, in_grace, plan_sweep"]
    index["index.rs: MediaIndex, references, plan"]
  end
  sha2["sha2 (workspace, ADR-024)"]
  bytes["a file's bytes"] --> hash
  sha2 --> hash
  hash --> name
  name --> bind
  hash --> bind
  fields --> index
  styles --> index
  store --> index
  index --> scan
  index --> css
  scan --> index
  css --> index
  index --> sweep
  sweep --> plan["SweepPlan: delete, kept with reasons, missing"]
  plan --> release["the store deletes the planned files and releases their bindings"]
  release --> bind
```

## The binding state machine

One name's slot, under `step` (a bind of hash `h`, with the claim of a certified name) and `release_step`. A refusal
leaves the slot as it was. A slot never returns to unbound, and its hash never changes.

```mermaid
stateDiagram-v2
  [*] --> Unbound
  Unbound --> Live_h: bind h (claim none or h) / New
  Unbound --> Unbound: bind with a claim other than h / Mismatch
  Unbound --> Unbound: release / Unbound
  Live_h --> Live_h: bind h / Unchanged
  Live_h --> Live_h: bind g, g not h / Conflict
  Live_h --> Live_h: bind with a claim other than the hash / Mismatch
  Live_h --> Released_h: release / Released
  Released_h --> Released_h: release / AlreadyReleased
  Released_h --> Live_h: bind h / Revived
  Released_h --> Released_h: bind g, g not h / Conflict
```

## What a sweep keeps

Each stored file is asked in this order, and the first answer that holds is its fate.

```mermaid
flowchart TD
  file["a stored file"] --> live{"does a note, template or CSS name it?"}
  live -- yes --> keepLive["keep: live"]
  live -- no --> pinned{"does its name start with _?"}
  pinned -- yes --> keepPinned["keep: pinned"]
  pinned -- no --> grace{"added at or after the instant, or less than the window before it?"}
  grace -- yes --> keepGrace["keep: in grace"]
  grace -- no --> imported{"did an import bring it?"}
  imported -- yes --> keepImported["keep: imported"]
  imported -- no --> delete["delete"]
```
