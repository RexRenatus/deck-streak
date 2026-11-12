# Schematic: the bot's update loop and its outbound transport

Kind: state machine and data flow. Read at DeckStreak `main` e05dfa5 (ADR-006, ADR-007, the
telegram-platform pack), and at the predecessor's `27ee2bc` for the behaviour it ports
(`bot.py:CommandBot.run`, `poll_once`, `_drain_offset`, `_backoff_delay`, `_process_update`,
`_process_callback`, `telegram.py:TelegramNotifier`). Decided by ADR-026; built by SPEC-026.

```mermaid
stateDiagram-v2
  [*] --> Starting: settings, the owner and the token (credentials), the database under its open lock
  Starting --> Draining: deleteWebhook, the owner's menu, then advance the offset past every queued update
  Draining --> Polling: READY=1 as the first long poll is issued
  Polling --> Gate: getUpdates(offset, timeout, allowed_updates) returned a batch, read update by update
  Polling --> Backoff: the poll failed
  Backoff --> Polling: the predecessor's backoff (golden)
  Gate --> Dropped: not the owner, not the owner's private chat, oversized text, or unreadable
  Gate --> AnswerOnly: the owner's callback whose data is over 64 bytes
  Gate --> Dispatch: the owner's message or callback
  Dispatch --> Answered: a callback is answered first
  Dispatch --> Sending: a command replies
  AnswerOnly --> Confirm
  Dropped --> Confirm
  Answered --> Confirm
  Sending --> Confirm
  Confirm --> Polling: offset = last update_id + 1, confirmed by the next request
  Polling --> Stopping: SIGTERM: the poll in flight abandoned, the batch in hand finished, the offset confirmed, STOPPING=1
  Stopping --> [*]
```

```mermaid
flowchart LR
  text[rendered HTML, every dynamic value escaped] --> chunk[chunk.rs: at most 4096 UTF-16 units, cut at paragraph, line or word, tags closed and reopened]
  chunk --> send[transport.rs: sendMessage, link previews off, the keyboard on the last chunk]
  send -->|429| wait[wait retry_after seconds, 1 without it, on the transport's waits; repeat the SAME request, at most 3]
  wait --> send
  send -->|5xx, 4xx or a network error| again[the next attempt at once, at most 3: goldens/send_retry.json]
  again --> send
  send --> attempted[attempted count +1 for every message]
  send -->|every chunk came back with a message id| delivered[delivered count +1]
  attempted & delivered --> marker[wiring's TransportMarker: coordination's DeliveryMarker]
```

The owner's `/delete` holds one pending question at a time, in memory:

```mermaid
stateDiagram-v2
  [*] --> None
  None --> Pending: /delete, and the question came back with its message id
  Pending --> Pending: /delete again: the newer question replaces it
  Pending --> Erased: the owner taps the button on the pending question
  Pending --> Pending: a tap on another message: nothing erased, told it expired
  None --> None: any tap: nothing erased, told it expired
  Erased --> None: erase_all, then its result
```

The owner's `/sync` crosses the context map through one port, so the cycle it runs can move out of
the bot's unit without the bot changing:

```mermaid
flowchart LR
  sync["/sync, after typing"] --> port["the bot's OwnerSync port"]
  port --> wiring[daemon wiring's OwnerSyncCycle]
  wiring --> rescore[ingest: the owner's rescore marked pending]
  rescore --> cycle[coordination::sync_cycle with the trigger owner]
  cycle --> answer[the sync's outcome and the recompute's, or the reason code it could not run]
  answer --> reply[bot: one reply, escaped]
```
