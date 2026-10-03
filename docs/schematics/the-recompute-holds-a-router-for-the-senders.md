# Schematic: the recompute holds a router for the senders

Kind: sequence. Read at DeckStreak `dev` `6773f6e` (`crates/coordination/src/sync_cycle.rs`
`sync_cycle` and `flush`; `crates/daemon/src/wiring.rs` `RecomputeSetup` and `OwnerSyncCycle::run`;
`crates/daemon/src/role_job.rs` the scheduled sync and the `held_flush` job;
`crates/daemon/src/sync_request.rs` `answer`; `crates/notifications/src/router.rs` `decide`,
`after_claim` and `flush_with`). Added by SPEC-319; decided by ADR-319.

## The actors

- **The job process** runs both recompute cycles: the scheduled sync and the owner's sync when a
  request is stored (SPEC-059). Every cycle it builds goes through `RecomputeSetup::cycle`, which
  attaches a holding router: `Router::new(..).holding()`, with no bot transport. It loads no bot
  credential (ADR-066).
- **The bot process** stores the owner's request, rings the doorbell, polls for the answer within
  its bound, and flushes its own bot-joined router when the answer says a sync ran and succeeded.
- **The `held_flush` job** runs at 07:36, outside the quiet window, with its own bot-joined router
  (SPEC-041 R14, R17). It is the backstop for a hold no answer flushed.
- **`notification_queue`** is the shared state. Every flush takes it under the flush lease and
  claims each row it sends (SPEC-041 R16).

## The owner's sync, outside the quiet window

```mermaid
sequenceDiagram
    participant Owner
    participant Bot as bot process
    participant Job as job process
    participant Router as holding router (job)
    participant Queue as notification_queue
    participant Flush as bot-joined router (bot)
    Owner->>Bot: /sync
    Bot->>Job: store the request, ring the doorbell
    Job->>Job: sync, then the cycle's flush answers NoNotifier
    Job->>Router: the fold offers each owed award, the level-up, the relights
    Router->>Router: decide: switch off -> withheld nudges_disabled (marked)
    Router->>Queue: switch on, outside the window -> deferred send, held, claim kept
    Router-->>Job: answered, so the award is marked (ADR-303)
    Job-->>Bot: the request is answered (Ran)
    Bot->>Flush: the sync ran and succeeded -> flush
    Flush->>Queue: take the lease, claim the held rows
    Flush->>Owner: push each held celebration (two in full, the rest in one recap)
    Flush->>Queue: settle each pushed row by its claim
```

## When no answer is observed, and the scheduled sync

```mermaid
sequenceDiagram
    participant Bot as bot process
    participant Job as job process
    participant Queue as notification_queue
    participant Held as held_flush job (07:36)
    participant Owner
    Bot->>Bot: the answer bound expires -> StillRunning, no flush
    Job->>Queue: the cycle still holds its celebrations
    Note over Job,Queue: the scheduled sync runs inside the quiet window, so its holds are quiet holds
    Held->>Queue: take the lease, abandon by name what is past 720 minutes
    Held->>Owner: push the rest
```

## What the model holds

`formal/tla/HeldFlush` models the hold as `HoldSend(i)`, enabled outside the window at a sync
trigger before the flush that follows the answer (`"sync" \notin fired`), and the expired answer as
`Unanswered`, which answers that trigger with no flush. `MCHeldSendHold.cfg` places a scheduled
trigger after the last sync trigger, as the next 07:36 always comes, and the witness
`a-hold-outside-the-window-with-no-later-flush.cfg` drops it: `HeldReachesOrAbandons` must then
find an item still held at the end of the run.
