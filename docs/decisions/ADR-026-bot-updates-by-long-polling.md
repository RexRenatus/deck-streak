---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The bot receives updates by long polling, not by webhook

## Context and Problem Statement

ADR-007 puts the API behind the shared Caddy on loopback and leaves the bot's transport open:
"the API and the bot's webhook (if chosen over long polling in W0) listen on loopback only". The
Bot API offers two exclusive ways to receive updates: a webhook Telegram calls over HTTPS, or
`getUpdates` long polling the bot calls. SPEC-026 builds the bot; which does it use?

## Decision Drivers

- The anti-goals: no tunnels, no opened inbound ports (CHARTER 10); the shared Caddy is shared
  infrastructure, and every change to it is an owner gate (ADR-007).
- One owner: a handful of updates a day, so latency and throughput are not constraints.
- Secrets: every new credential is provisioned by the private rail and rotated by hand.
- The predecessor's proven runtime long-polls, drains stale updates at start, and backs off on errors.
- The telegram-platform pack's rules for each mode (a webhook needs a secret token and idempotent
  handling of Telegram's retries; a poll needs a positive timeout and offset confirmation).

## Considered Options (the alternatives it was chosen against)

- Long polling from the bot's own unit: `deleteWebhook` at start, a drain of queued updates, `getUpdates` with a 50-second timeout, offset confirmation, the predecessor's backoff — chosen: no inbound path at all, no Caddy change, no new credential, and the predecessor's behaviour ports as it is.
- A webhook behind the shared Caddy at a path on the Mini App's host — rejected because it adds a public route to shared infrastructure (an owner gate), a webhook secret as a new credential, and idempotency by `update_id` for Telegram's retries, to save one outbound connection that one owner's traffic never strains.
- Long polling from inside the API process — rejected because the bot would then share the API's unit, memory ceiling and restarts; a bot fault would take the Mini App's API down with it.
- Short polling (`getUpdates` with no timeout) — rejected because the Bot API documents it for testing only, and it would spend requests every few seconds for nothing.

## Decision Outcome

Chosen option: long polling from `deck-streak-bot.service`, the only poller for the token. The
drain at start (advance the offset without dispatching) keeps stale taps from replaying after a
deploy, as the predecessor's default did. The telegram-platform rows `tg-poll-offset`,
`tg-poll-long`, `tg-allowed-updates` and `tg-update-mode` judge the loop; `ws.tg-webhook-secret` has
no subject.

### Consequences

- Good, because nothing new listens on the host, and the Caddy block serves only the Mini App and its
  API.
- Good, because a restart of the bot never replays a tap the owner made before it.
- Bad, because an update waits for the next poll to return (at most the long-poll timeout, usually
  far less); for one owner's commands that is invisible.

### Confirmation

SPEC-026's A7 and A8; the telegram-platform bot-api rows in `scripts/check.sh`.

## What would make this wrong

- DeckStreak serves many users (it serves one by design), where a webhook's push and horizontal
  scaling would matter.
- Telegram deprecates `getUpdates` (tracked by the telegram-platform pack's Bot API notes).

## More Information

ADR-007; SPEC-026; the telegram-platform pack ("Webhook or long polling"); the predecessor's
`bot.py:CommandBot.run` and `_drain_offset`.
