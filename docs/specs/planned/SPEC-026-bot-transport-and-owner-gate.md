# SPEC-026: the bot long-polls Telegram, answers only the owner, sends valid chunked HTML through one transport, and waits out every 429

- **Wave:** W0. **Issue:** #19 (epic #1). **Context(s):** `deck-streak-bot`, `deck-streak-daemon` (the `bot` role and the delivery marker's wiring).
- **Decided by:** ADR-003 (frankenstein, tokio), ADR-006 (the bot's owner gate uses the same configured owner id), ADR-007 (no inbound port), ADR-010 (a `Type=notify` unit per role), ADR-025 (the shared lifecycle), ADR-037 (`/sync` is the owner's explicit sync trigger, with its 5-minute debounce), and this SPEC's ADR-026 (updates by long polling, not a webhook).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-026.md` (ADR-016).

## 1. The problem, measured

- **The predecessor's runtime** (predecessor `27ee2bc`, names only): one task long-polls
  `getUpdates` (`bot.py:CommandBot.run`, `poll_once`), confirms by offset, drains queued updates at
  start instead of replaying stale taps (`_drain_offset`, `config.py:bot_replay_on_start`), backs off
  on errors (`_backoff_delay`), answers only the owner's chat and only the owner's callbacks
  (`_process_update`, `_process_callback`), registers its menu for the owner's chat only and deletes
  the public menu (`set_commands`), drops text over 4096 characters and does not dispatch callback
  data over 64 bytes, shows a chat action for slow commands, honours a 429's `retry_after`, and
  treats "message is not modified" as success. Its notifier (`telegram.py:TelegramNotifier`,
  `_chunk`) sends HTML with every dynamic value escaped, link previews off, chunks at 4096
  characters, and keeps a monotonic attempted and delivered count the catch-up reads.
- **What the parity oracle proves** (registered in `tools/parity-oracle/registry/spec_026.py`): the
  error backoff (`goldens/poll_backoff.json`, an adapter over `bot.py:CommandBot._backoff_delay`
  driven through successive failures), the send retry (`goldens/send_retry.json`, an adapter over
  `telegram.py:TelegramNotifier._send_chunk` with a stub HTTP client answering each case's sequence
  of 429, 5xx and 200 responses and a recording sleep), and the constants
  (`goldens/bot.constants.json`: the long-poll and HTTP timeouts, the inbound caps and
  `telegram.py:_DEFAULT_RETRY_AFTER`).
- **What the skeleton deferred.** The box-run packs' wiring deferred the telegram-platform rows
  `tg-retry-after` and `tg-callback-answer` to this issue, because the probe read the vendored probes
  as bot source until a real bot existed. SPEC-056 removed those files, and the two deferrals with
  them; every other bot-api row is enforced.
- **Where DeckStreak improves on the predecessor:** the predecessor's chunker cut at a character
  count; DeckStreak's cuts at a paragraph, a line or a word in UTF-16 units and closes and reopens
  open tags, because Telegram refuses a chunk with an unclosed tag (the telegram-platform pack).
- **Nothing exists yet**: `crates/bot/src/lib.rs` is documentation only (read at `main` e05dfa5).

**Order.** After SPEC-024 (the owner type), SPEC-025 (the binary and the lifecycle), SPEC-027 (the
`DeliveryMarker` port and the sync cycle's rescore flag) and SPEC-021 (the export and erase use
cases `/export` and `/delete` call). SPEC-032's bot unit runs this role.

## 2. Requirements

R1. `deckstreakd bot` runs the bot through one transport module, `bot::transport`, holding
    frankenstein's async client; the bot token is the credential `telegram-bot-token`, and the
    owner is identity's owner (the credential `owner-user-id`).
R2. Updates arrive by long polling (ADR-026): at start the bot calls `deleteWebhook`, then advances
    the offset past every queued update without dispatching any (a drain, never a replay); each
    `getUpdates` passes `timeout` and `allowed_updates` (`message`, `callback_query`), and the offset
    is `update_id + 1` of the last handled update, confirmed by the next call.
R3. A failed poll waits per `goldens/poll_backoff.json` before the next, on tokio's timer.
R4. The owner gate: a message is dispatched only when it comes from the owner in the owner's private
    chat, and a callback only when it comes from the owner; anything else is dropped with no reply,
    no callback answer and no log of its content (the update kind and one reason code only).
R5. Inbound caps, as the predecessor: a text longer than 4096 characters is dropped; a callback whose
    data exceeds 64 bytes is answered, so the client's spinner stops, and not dispatched.
R6. Every outbound text is HTML (`parse_mode` HTML, never legacy Markdown); every dynamic value is
    escaped (`&` first, then `<` and `>`) where it enters the markup; link previews are off through
    `link_preview_options`, and no field a later Bot API version replaced is used. The bounds are
    named constants, `MAX_TEXT_UTF16 = 4096` and `MAX_CAPTION_UTF16 = 1024` (the telegram-platform
    pack).
R7. A text longer than `MAX_TEXT_UTF16` UTF-16 units after entity parsing is split at a paragraph,
    then a line, then a word boundary, never inside a tag or an entity; a tag open at a chunk's end
    is closed there and reopened at the next chunk's start, so every chunk parses on its own.
R8. A chunk is sent in at most three attempts (the predecessor's
    `telegram.py:TelegramNotifier.send_html` default): a 429 is waited out for its
    `parameters.retry_after` seconds (or `_DEFAULT_RETRY_AFTER` when the body has none) on tokio's
    timer before the SAME request is repeated; a 5xx or a network error uses the same attempts; the
    attempts and waits equal `goldens/send_retry.json`. An edit answered "message is not modified"
    counts as delivered.
R9. Every callback from the owner is answered with `answerCallbackQuery`.
R10. The transport counts attempted and delivered sends (a delivery is a send that returned a message
    id); the daemon's wiring implements coordination's `DeliveryMarker` with these counts
    (SPEC-027).
R11. At start the bot registers its commands for the owner's chat only (`setMyCommands` with the
    chat scope) and deletes the default-scope menu. At W0 the commands are:
    - `/start`: its first line says the coach is an AI; it offers the Mini App through a `web_app`
      inline button whose URL is `DECKSTREAK_MINI_APP_URL` (an `https` URL, configuration);
    - `/privacy`: links the published privacy policy;
    - `/export`: sends the export (`coordination::export_all`, SPEC-021) as a JSON document;
    - `/delete`: asks for confirmation with an inline button, and erases
      (`coordination::erase_all`) only on the owner's confirming callback;
    - `/sync`: the owner's explicit sync trigger (ADR-037), and at W0 the only sync besides the
      daily scheduled one: it runs a sync cycle now with the trigger `owner` and the rescore flag set
      (SPEC-023), and answers with the outcome. Less than 5 minutes after a successful sync it
      contacts no server and answers with that sync's result (SPEC-022's debounce, R17 there); the
      rescore flag still forces its one recompute.
    `/export` and `/sync` send the `typing` chat action before they work.
R12. Every message the bot renders at W0 is committed as a golden,
    `crates/bot/tests/messages/<name>.msg.json`, in nudge-duties' envelope `phx.duty.message.v1`
    (the `/start` greeting, `/privacy`, the `/delete` confirmation and its result, a sync reply, and
    a 6000-character sample with nested tags and its chunks); a test re-renders each and compares, and
    the telegram-platform payload rows judge the files.
R13. The bot role follows the shared lifecycle (SPEC-025): `READY=1` once the first poll is issued,
    watchdog pings, and on SIGTERM it finishes the batch in hand, confirms its offset and exits 0.
R14. No telegram-platform row is deferred (SPEC-056 removed the two deferrals); every
    telegram-platform bot-api row is green over the workspace. `privacy.json` names the bot's command
    table (`crates/bot/src/commands.rs`, text `privacy`) as a policy entry point, and `PRIVACY.md`
    names the bot's `/export` and `/delete`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | an update from anyone but the owner is dropped with no reply | `gate` test against a fake Bot API |
| A2 | a 429 waits `retry_after` seconds and repeats the same request, within the predecessor's attempts | `transport` test over `goldens/send_retry.json`; telegram-platform `tg-retry-after` |
| A3 | a 6000-character HTML message is sent as chunks that each parse, none cut inside a tag | `chunking` test over the committed golden; `payload-length`, `payload-markup` |
| A4 | every callback from the owner is answered | `gate` test; `tg-callback-answer` |
| A5 | `/start` says the coach is an AI and offers the Mini App | `commands` test over the golden |
| A6 | the menu registers `/privacy`, `/export`, `/delete` and `/sync` for the owner's chat only, and the public menu is deleted | `commands` test |
| A7 | the poll confirms each update by its offset | `poll` test; `tg-poll-offset` |
| A8 | updates queued before start are drained, never replayed | `poll` test |
| A9 | the poll backoff equals the predecessor's | `poll` test over `goldens/poll_backoff.json` |
| A10 | oversized inbound text and callback data are not dispatched | `gate` test over `goldens/bot.constants.json` |
| A11 | every dynamic value is HTML-escaped where it enters the markup | `transport` test; `tg-escape` |
| A12 | the delivery marker tells a failed send from a delivered one | `transport` test |
| A13 | `/export` sends the owner's data as a JSON document | `commands` test |
| A14 | `/delete` erases only after the owner's confirming callback | `commands` test |
| A15 | `/sync` runs a cycle now as the owner's trigger and forces one recompute | `commands` test |
| A16 | every committed golden message parses as the Bot API would parse it | `test_bot_messages.py`; telegram-platform `payload-*` rows |

```acceptance
A1: cargo test -p deck-streak-bot --test gate -- --exact an_update_from_anyone_but_the_owner_is_dropped_without_a_reply
A2: cargo test -p deck-streak-bot --test transport -- --exact a_429_waits_retry_after_seconds_and_repeats_the_same_request
A3: cargo test -p deck-streak-bot --test chunking -- --exact a_long_html_message_is_sent_as_chunks_that_each_parse
A4: cargo test -p deck-streak-bot --test gate -- --exact every_callback_from_the_owner_is_answered
A5: cargo test -p deck-streak-bot --test commands -- --exact start_says_the_coach_is_an_ai_and_offers_the_mini_app
A6: cargo test -p deck-streak-bot --test commands -- --exact the_menu_is_registered_for_the_owners_chat_only
A7: cargo test -p deck-streak-bot --test poll -- --exact the_poll_confirms_each_update_by_its_offset
A8: cargo test -p deck-streak-bot --test poll -- --exact updates_queued_before_start_are_drained_and_not_replayed
A9: cargo test -p deck-streak-bot --test poll -- --exact the_poll_backoff_matches_the_predecessors_golden
A10: cargo test -p deck-streak-bot --test gate -- --exact oversized_inbound_input_is_not_dispatched
A11: cargo test -p deck-streak-bot --test transport -- --exact every_dynamic_value_is_html_escaped
A12: cargo test -p deck-streak-bot --test transport -- --exact the_delivery_marker_tells_a_failed_send_from_a_delivered_one
A13: cargo test -p deck-streak-bot --test commands -- --exact export_sends_the_owners_data_as_a_json_document
A14: cargo test -p deck-streak-bot --test commands -- --exact delete_erases_only_after_the_owner_confirms
A15: cargo test -p deck-streak-bot --test commands -- --exact sync_runs_a_cycle_now_and_forces_one_recompute
A16: python3 -m unittest discover -s scripts/tests -p test_bot_messages.py -k every_golden_message_parses_as_the_bot_api_would
```

The Rust tests run a fake Bot API (an axum server on a loopback port that records every call and
answers as each test scripts it), reached through frankenstein's `Bot::builder().api_url`; waits
run on tokio's paused time. Test tokens never have the Bot API token's shape and test user ids have
fewer than seven digits (SPEC-024's R11). A16 calls the telegram-platform probe's published
`check_message` on each golden file, reporting how many it examined. The probe is box-run
(ADR-069), so A16's test takes it from the maintainer's checkout.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/bot/Cargo.toml` | `deck-streak-bot` | changed: frankenstein (async reqwest client), kernel, identity, coordination, tokio, thiserror, tracing, serde_json; dev: axum, tempfile, serde |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | added: the client, escaping, the 429 and 5xx waits, the delivery counts |
| `crates/bot/src/chunk.rs` | `deck-streak-bot` | added: UTF-16 chunking that closes and reopens tags |
| `crates/bot/src/poll.rs` | `deck-streak-bot` | added: drain, long poll, offset, backoff |
| `crates/bot/src/gate.rs` | `deck-streak-bot` | added: the owner gate and the inbound caps |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | added: the command table, the menu, the handlers |
| `crates/bot/tests/gate.rs`, `transport.rs`, `chunking.rs`, `commands.rs`, `poll.rs` | `deck-streak-bot` | added: A1 to A15 |
| `crates/bot/tests/support/fake_bot_api.rs` | `deck-streak-bot` | added |
| `crates/bot/tests/messages/*.msg.json` | `deck-streak-bot` | added: the golden messages |
| `crates/daemon/src/role_bot.rs`, `crates/daemon/src/main.rs`, `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | added or changed: the role, the `DeliveryMarker` wiring |
| `scripts/tests/test_bot_messages.py` | repo | added: A16 |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: frankenstein (ADR-003) |
| `deny.toml` | workspace | changed only if the TLS stack's licences need an allow entry compatible with AGPL-3.0-or-later |
| `tools/parity-oracle/registry/spec_026.py`, `tools/parity-oracle/goldens/poll_backoff.json`, `send_retry.json`, `bot.constants.json` | repo | added |
| the box-run packs' private wiring (ADR-069) | the maintainer's | unchanged: SPEC-056 removed the two telegram-platform deferrals |
| `privacy.json`, `PRIVACY.md` | repo | changed: the bot as an entry point, its data-rights commands |
| `.env.example` | repo | changed: the Mini App URL, by name |
| `docs/schematics/bot-update-loop.md` | repo | added |
| `docs/decisions/ADR-026-bot-updates-by-long-polling.md` | repo | added |
| `docs/red-first/SPEC-026.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It implements no port of the notification router: the router and its bot transport arrive in W1
  (#27), and dice, reactions, pins and photos with the celebration ladder in W5
  (#120).
- It registers no feature command; each arrives with its feature (#69,
  #38, #93, #98).
- It reads no channel post; the doomscroll sensor's channel arrives with its feature
  (#110).
- It captures no media to the vault inbox (#154).
- It keeps no last owner message id for reactions (#120).
- It sends no scheduled message; jobs and the router send them (#20,
  #27).

## 6. Risks

- **Two pollers for one token.** Telegram answers a second `getUpdates` with a conflict; the bot unit
  is the only poller, and SPEC-032's unit runs one instance. A conflict is a poll error, backed off
  and logged by reason code.
- **A retry storm on a long outage.** R8 bounds a chunk to three attempts, and the poll's own
  backoff caps its wait (the golden's ceiling); the owner is paged through the unit's failure only
  if the role exits.
- **The TLS stack's licences.** frankenstein's reqwest client brings a TLS implementation whose
  licences must pass `cargo deny` under `deny.toml`; the delivery chooses the one that passes, and
  an allow-list entry, if needed, must be compatible with AGPL-3.0-or-later.
- **A golden message drifts from the renderer.** R12's comparison fails on any difference, so the
  committed files the pack judges are the messages the bot sends.
