# The sync core and a pending operation's life

Read at `554a223e`. SPEC-509 and ADR-527 decide what this draws: the sync core's modules and who links the crate
(component), and the life of one operation in a device's pending queue (state machine).

## 1. The crate and its users (component)

`deck-streak-sync` depends on nothing and does no I/O. Its users are drawn dashed: the server's log, the umbrella FFI
crate and the web engine gain their edges in the changes that first use the crate (#15), so no manifest names it yet.
The Lean entries and the Kani harnesses check the modules they cover.

```mermaid
flowchart TB
  subgraph users[its users, each edge drawn by the change that first uses it]
    server["the server's log: judge, writer, routes"]
    ffi["ffi: the iPhone and iPad client's static library"]
    web_engine["web-engine: the web client's wasm32 module"]
  end
  subgraph sync[deck-streak-sync: no dependency, no I/O]
    op["op: Op, fourteen kinds; OpHead; OpKind and its tags"]
    codec["codec: encode, encode_body, decode; refusals by name"]
    key["key: Key (t, rank, origin, seq); compare; sort_keys"]
    clock["clock: tick; Hlc with recv, tick_at, next_id_at"]
    ids["ids: next_id; Slot; SLOTS = 64"]
    merge["merge: Edit, merge, apply, Register with its conflicts"]
    pending["pending: PendingQueue; Verdict; Settled"]
  end
  subgraph proofs[the proofs that cover it]
    lean["Lean: SyncClock, ReviewKey, FieldMerge, IdSlot, each with its vectors"]
    kani["Kani: K-sync-codec, K-key-order, K-hlc, K-next-id"]
  end
  server -.-> pending
  server -.-> codec
  ffi -.-> codec
  web_engine -.-> codec
  codec --> op
  op --> key
  merge --> key
  clock --> ids
  pending --> key
  lean -.->|covers| clock
  lean -.->|covers| key
  lean -.->|covers| merge
  lean -.->|covers| ids
  kani -.->|harnesses| codec
  kani -.->|harnesses| key
  kani -.->|harnesses| clock
  kani -.->|harnesses| ids
```

## 2. A pending operation's life (state machine)

An operation enters the queue when the device makes it and leaves only on its judgement. Sending changes its stage;
a retryable refusal sends it back. Acknowledged operations are part of the base; a rejected one is kept, bytes and
reason, for the owner.

```mermaid
stateDiagram-v2
  [*] --> Queued: made (enqueue)
  Queued --> Sent: send
  Sent --> Sent: send again after a lost answer
  Sent --> Queued: Refused (gap, clock ahead, blob missing, below minimum client, busy)
  Queued --> Acknowledged: its own operation in a pull
  Sent --> Acknowledged: Accepted or Duplicate verdict
  Sent --> Acknowledged: its own operation in a pull
  Sent --> Rejected: Rejected (malformed, id taken, gone, reviewed note delete, undo stale, seq reuse)
  Acknowledged --> [*]
  Rejected --> [*]: kept among the rejections the owner sees
```
