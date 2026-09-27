# Schematic: the bot's update loop and its outbound transport

Kind: state machine and data flow. Read at DeckStreak `main` e05dfa5 (ADR-006, ADR-007, the
telegram-platform pack), and at the predecessor's `27ee2bc` for the behaviour it ports
(`bot.py:CommandBot.run`, `poll_once`, `_drain_offset`, `_backoff_delay`, `_process_update`,
`_process_callback`, `telegram.py:TelegramNotifier`). Decided by ADR-026; built by SPEC-026.

```mermaid
stateDiagram-v2
  [*] --> Draining: deleteWebhook, then advance the offset past every queued update
  Draining --> Polling: READY=1
  Polling --> Gate: getUpdates(offset, timeout, allowed_updates) returned a batch
  Polling --> Backoff: the poll failed
  Backoff --> Polling: the predecessor's backoff (golden)
  Gate --> Dropped: not the owner, not the owner's private chat, or oversized
  Gate --> Dispatch: the owner's message or callback
  Dispatch --> Answered: a callback is always answered
  Dispatch --> Sending: a command replies
  Dropped --> Confirm
  Answered --> Confirm
  Sending --> Confirm
  Confirm --> Polling: offset = last update_id + 1
  Polling --> Stopping: SIGTERM: finish the batch in hand, confirm, STOPPING=1
  Stopping --> [*]
```

```mermaid
flowchart LR
  text[rendered HTML, every dynamic value escaped] --> chunk[chunk.rs: at most 4096 UTF-16 units, cut at paragraph, line or word, tags closed and reopened]
  chunk --> send[transport.rs: sendMessage, link previews off]
  send -->|429| wait[wait retry_after seconds, repeat the SAME request, at most 3]
  wait --> send
  send -->|5xx| dbl[doubling wait, at most 3]
  dbl --> send
  send --> attempted[attempted count +1 for every send]
  send -->|a message id came back| delivered[delivered count +1]
  attempted & delivered --> marker[DeliveryMarker: attempted without delivered is a failed send]
```
