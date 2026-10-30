---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The bot receives updates by long polling, not by webhook

## Context and Problem Statement

ADR-007 puts the API behind Caddy on loopback and leaves the bot's transport open:
"the API and the bot's webhook (if chosen over long polling in W0) listen on loopback only". The
Bot API offers two exclusive ways to receive updates: a webhook Telegram calls over HTTPS, or
`getUpdates` long polling the bot calls. SPEC-026 builds the bot; which does it use?

## Decision Drivers

- The anti-goals: no tunnels, no opened inbound ports (CHARTER 10); every change to Caddy's
  configuration is an owner gate (ADR-007).
- One owner: a handful of updates a day, so latency and throughput are not constraints.
- Secrets: every new credential is provisioned by the private rail and rotated by hand.
- The predecessor's proven runtime long-polls, drains stale updates at start, and backs off on errors.
- The telegram-platform pack's rules for each mode (a webhook needs a secret token and idempotent
  handling of Telegram's retries; a poll needs a positive timeout and offset confirmation).

## Considered Options (the alternatives it was chosen against)

- Long polling from the bot's own unit: `deleteWebhook` at start, a drain of queued updates, `getUpdates` with a 50-second timeout, offset confirmation, the predecessor's backoff — chosen: no inbound path at all, no Caddy change, no new credential, and the predecessor's behaviour ports as it is.
- A webhook behind Caddy at a path on the Mini App's host — rejected because it adds a public route (an owner gate), a webhook secret as a new credential, and idempotency by `update_id` for Telegram's retries, to save one outbound connection that one owner's traffic never strains.
- Long polling from inside the API process — rejected because the bot would then share the API's unit, memory ceiling and restarts; a bot fault would take the Mini App's API down with it.
- Short polling (`getUpdates` with no timeout) — rejected because the Bot API documents it for testing only, and it would spend requests every few seconds for nothing.

## Decision Outcome

Chosen option: long polling from `deck-streak-bot.service`, the only poller for the token. The
drain at start (advance the offset without dispatching) keeps stale taps from replaying after a
deploy, as the predecessor's default did. The telegram-platform rows `tg-poll-offset`,
`tg-poll-long`, `tg-allowed-updates` and `tg-update-mode` judge the loop; `ws.tg-webhook-secret` has
no subject.

### Decided at delivery (SPEC-026 §7)

The delivery measured what this record had assumed, and decided each open question against its
alternatives:

- **The bot's waits go through a `Waits` the transport holds: tokio's timer in the service, a
  recorder in the tests.** Chosen against running the tests on tokio's paused time, as SPEC-026's
  plan had it: every request to the loopback fake failed after exactly 60 virtual seconds, while the
  same request completed in about a millisecond in real time, because the paused runtime moves its
  clock to the next timer whenever it parks, a request in flight included, and the HTTP client's own
  timeout was that timer. Also against sleeping for real, which would make the golden's 30-second
  waits real ones. A test on paused time with no socket holds `TokioTimer` to its duration.
- **The export is uploaded from memory.** frankenstein's `sendDocument` uploads only from a path, so
  the transport builds the `multipart/form-data` body from the bytes with frankenstein's own client
  and its re-export of reqwest. Chosen against writing the owner's export to a temporary file to hand
  frankenstein a path, which would put the owner's whole data on the host's disk to send it, and
  against adding reqwest as a dependency of its own, a crate this record does not name.
- **`getUpdates` is read update by update.** Each update is decoded by frankenstein in turn, so one it
  cannot read is still confirmed by its `update_id`. Chosen against frankenstein's `Vec<Update>`, with
  which one unreadable update fails the whole batch, and the poll, which confirms nothing it could
  not read, would fetch the same batch forever.
- **A `/delete` confirmation is single-use and the latest question's alone.** The button erases only
  when it is on the latest `/delete` question the bot sent in this process, and only once. Chosen
  against erasing on any tap of the confirming button, which would let a stale question, tapped by
  mistake, erase everything; and against a time window, which would put a clock on a command the
  owner takes in their own time.
- **`/sync` marks the owner's rescore first.** The flag is set before the sync's settings are read,
  so the next cycle serves the owner's request even when this one cannot run. Chosen against setting
  it only once the cycle can start, which would drop the owner's request on a failed start.
- **The Bot API's base URL is a setting**, `DECKSTREAK_BOT_API_URL`: `https:`, or `http:` to a
  loopback host only, defaulting to Telegram's own. Chosen against a base URL fixed in the code and a
  test-only build, either of which would mean the tested binary is not the shipped one, and against
  any `http:` host, which would send the token in the clear.
- **frankenstein's licence is admitted for frankenstein alone.** Its licence is WTFPL, a permissive
  licence compatible with the GNU GPL, and `deny.toml` admits it by one exception naming the crate.
  Chosen against adding WTFPL to the workspace's allow list, which would admit it for any crate
  unseen. The TLS stack frankenstein's client brings, reqwest's rustls with its aws-lc-rs provider,
  passes the allow list as it stands.
- **The golden replies carry no notification kind.** Each is `phx.duty.message.v1` with `duty`
  `bot-commands` and no `kind`: a command reply is not a notification. notifications-policy's
  `message-metadata` row reads every envelope as one, so the box run defers that row to #257 until
  it judges notifications only. Chosen against naming a kind the policy declares, which would make
  a reply look like an alert or a nudge, and against moving the replies out of `*.msg.json`, which
  the telegram-platform payload rows read.
- **frankenstein's async client, `client-reqwest`.** frankenstein 0.52.1 offers two clients:
  `client-reqwest`, its `AsyncTelegramApi` over reqwest and tokio, and `client-ureq`, its blocking
  `TelegramApi` over ureq. The bot runs in the daemon's tokio runtime, and the stop abandons a long
  poll in flight by dropping its future (R13). Chosen against `client-ureq`, because each
  `getUpdates` would then hold a thread for up to the 50-second long poll: called on the runtime, it
  would block one of its worker threads for as long; moved to tokio's blocking pool, it would hold a
  thread that cannot be aborted once it has started, so the stop could not abandon the poll and
  would wait it out before it confirmed its offset and exited.
- **The TLS provider is aws-lc-rs, the one frankenstein's client brings.** frankenstein 0.52.1
  depends on reqwest 0.13 with its default features off and `rustls` on, and reqwest 0.13.5 defines
  `rustls` as `__rustls-aws-lc-rs` plus `rustls-platform-verifier`: its client is built on
  `rustls::crypto::aws_lc_rs::default_provider()`, and it has no ring feature, since a ring provider
  takes `rustls-no-provider` and a provider the caller installs before it builds a client. Cargo's
  features only add, so no setting of this workspace takes `rustls` back from frankenstein (measured
  with `cargo tree -e features -i reqwest@0.13.5` and the two crates' published manifests). Chosen against
  ring, which would need a fork of frankenstein that asks for `rustls-no-provider`, or reqwest as a
  dependency of the bot's own, to hand frankenstein a client built around ring while aws-lc-rs is
  still built beside it; this record declines both, the second as it declines reqwest for the
  export. ring is in the bot's build all the same: the engine's reqwest 0.12.28, reached through
  coordination, ingest and anki, turns on its `__rustls-ring`, and so rustls's own `ring` feature,
  which also turns on rustls-webpki's; `rustls-platform-verifier` names no ring. This bullet declines
  ring as the provider of frankenstein's client, not ring in the tree.
- **An offset stands confirmed only once the server answers a request that carried it.** The poller
  marks a long poll's offset confirmed on the server's answer, as the stop's confirmation already
  did, never as the poll is issued. A stop that wins the race abandons a poll whose request may
  never have left the process, and the stop's own confirming request then covers it, so R13's stop
  confirms its offset in every interleaving, at the cost of one request, answered at once, when the
  last poll was still waiting. Chosen against making the lifecycle test wait for the confirmed
  offset before it sends SIGTERM and weakening R13's wording to match, which would fit the
  requirement and the test to the defect, and leave the poller's state claiming a confirmation the
  server may never have received.

### Consequences

- Good, because nothing new listens on the host, and the Caddy block serves only the Mini App and its
  API.
- Good, because a restart of the bot never replays a tap the owner made before it.
- Bad, because an update waits for the next poll to return (at most the long-poll timeout, usually
  far less); for one owner's commands that is invisible.

### Confirmation

SPEC-026's A7 and A8; the telegram-platform bot-api rows in the box run (ADR-069).

## What would make this wrong

- DeckStreak serves many users (it serves one by design), where a webhook's push and horizontal
  scaling would matter.
- Telegram deprecates `getUpdates` (tracked by the telegram-platform pack's Bot API notes).

## More Information

ADR-007; SPEC-026; the telegram-platform pack ("Webhook or long polling"); the predecessor's
`bot.py:CommandBot.run` and `_drain_offset`.

## Amendment (ADR-323, #257)

The deferral of notifications-policy's `message-metadata` row to #257, recorded above, ended at
ADR-323: the policy declares the duty `bot-commands` in its `replies` list, so the row skips the
command replies and counts them, and a reply is still not a notification kind.
