# SPEC-026: the bot long-polls Telegram, answers only the owner, sends valid chunked HTML through one transport, and waits out every 429

- **Wave:** W0. **Issue:** #19 (epic #1). **Context(s):** `deck-streak-bot`, `deck-streak-daemon` (the `bot` role and the delivery marker's wiring).
- **Decided by:** ADR-003 (frankenstein, tokio), ADR-006 (the bot's owner gate uses the same configured owner id), ADR-007 (no inbound port), ADR-010 (a `Type=notify` unit per role), ADR-025 (the shared lifecycle), ADR-037 (`/sync` is the owner's explicit sync trigger, with its 5-minute debounce), and this SPEC's ADR-026 (updates by long polling, not a webhook).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-026.md`, and four goldens
  (`poll_backoff`, `send_retry`, `bot.constants`, `bot.timeouts`) generated at the predecessor's
  `27ee2bc`. The delivery made §1, R14, A16, section 3's harness, the manifest and §6 exact where the
  code decided them (§7).

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
  `telegram.py:TelegramNotifier.send_html`, whose `_send_chunk` it drives, with a stub HTTP client
  answering each case's sequence of 429, 5xx, 400, network-error and 200 responses and a recording
  sleep), the constants (`goldens/bot.constants.json`: the inbound caps, the text bound,
  `telegram.py:_DEFAULT_RETRY_AFTER` and `send_html`'s default attempts), and the timeouts
  (`goldens/bot.timeouts.json`: the long-poll and HTTP timeouts a bot the predecessor's own
  constructor built holds).
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
R14. The telegram-platform rows are judged by the box run (ADR-069), and no wiring file is in the
    tree: SPEC-056 removed the vendored probes and the two deferrals the plan named, so every
    telegram-platform bot-api row is enforced, and green over the workspace in the box run.
    `privacy.json` names the bot's command table (`crates/bot/src/commands.rs`, text `privacy`) as a
    policy entry point, and `PRIVACY.md` names the bot's `/export` and `/delete`.

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
| A16 | every committed golden message parses as the Bot API would parse it | `test_bot_messages.py`, DeckStreak's own check, written in the test and importing no pack probe, that each golden parses as the Bot API parses it (its HTML tags and entities, the UTF-16 length bounds); the telegram-platform `payload-*` rows judge the same files in the box run (ADR-069) |

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
answers as each test scripts it), reached through frankenstein's `Bot::builder().api_url`. The
transport's and the poll's waits go through the transport's `Waits`, tokio's timer in the service,
which the tests hand a recorder: tokio's paused time cannot hold them (§7). Test tokens never have
the Bot API token's shape and test user ids have fewer than seven digits (SPEC-024's R11). A16's
test is DeckStreak's own reading of the Bot API's HTML, reporting how many goldens it examined; it
imports no pack probe, and the telegram-platform payload rows judge the same files in the box run
(ADR-069).

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
| `crates/bot/tests/messages/long-sample.source.txt` | `deck-streak-bot` | added (§7): R12's 6000-character sample, which is not itself sent, so it is no `*.msg.json` |
| `crates/daemon/src/role_bot.rs`, `crates/daemon/src/main.rs`, `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | added or changed: the role, the `DeliveryMarker` wiring, the owner's sync |
| `crates/daemon/src/lib.rs`, `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed (§7): the role is a module of the library, where its test reaches it; the job role's doc comment says why it keeps no transport at W0 |
| `crates/daemon/tests/role_bot.rs`, `crates/daemon/tests/roles.rs`, `crates/daemon/Cargo.toml` | `deck-streak-daemon` | added or changed (§7): R13's lifecycle through the built binary against the fake Bot API, included by path, with axum and tokio's `net` as dev-dependencies; the usage line names `bot` |
| `scripts/tests/test_bot_messages.py` | repo | added: A16 |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: frankenstein (ADR-003) |
| `deny.toml` | workspace | changed (§7): one exception admits frankenstein's own licence, WTFPL, for that crate alone, and one advisory exception names RUSTSEC-2024-0436, an unmaintained notice on frankenstein's `paste`; the TLS stack's licences pass as they are |
| `tools/parity-oracle/registry/spec_026.py`, `tools/parity-oracle/goldens/poll_backoff.json`, `send_retry.json`, `bot.constants.json`, `bot.timeouts.json` | repo | added; `bot.timeouts.json` by §7 |
| `scripts/mutation-rows.d/S02600-S02699.json` | repo | added (§7): the owner gate's hand-proved rows (SPEC-039) |
| `stack.json` | repo | changed (§7): frankenstein is in use (ADR-003) |
| the box-run packs' private wiring (ADR-069) | the maintainer's | no telegram-platform deferral (SPEC-056 removed both); notifications-policy's `message-metadata` row is deferred to #257 while it judges a command reply as a notification (§7) |
| `privacy.json`, `PRIVACY.md` | repo | changed: the bot as an entry point, its data-rights commands |
| `.env.example` | repo | changed: the Mini App URL and the Bot API's base URL, by name |
| `deploy/deck-streak.env.example` | repo | changed (§7): the Mini App URL, by name and with no value, since the bot role requires it |
| `scripts/tests/test_privacy_policy.py` | repo | changed (§7): the policy's pinned entry points include the bot's command table (R14) |
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
- **An owner's `/sync` outgrows the bot unit's memory.** R11 runs the sync cycle inside the bot
  role, whose budget is 64M high and 96M at most (`deploy/host-budget.json`), while the sync's own
  budget is 256 MiB (ADR-022) and its job unit's 320M high. A sync near that budget would stop the
  bot, and the drain at its restart would discard the `/sync` update. The owner's sync reaches the
  cycle through one port the daemon implements (`OwnerSync`), so moving that cycle into a unit of
  its own changes the daemon's wiring and not the bot.

## 7. Amendments at delivery

- **R14 and A16 (the architect, 2026-09-28).** SPEC-056 moved every pack verdict to the box run, and
  no wiring file is in the tree, so R14 says the telegram-platform rows are judged there. A16's
  public test is DeckStreak's own check, written in the test and importing no pack probe; the
  payload rows judge the same files in the box run, and no run-time pack artifact is needed.
- **§3: the waits are recorded, not paused.** Under tokio's paused time every request to the fake
  failed after exactly 60 virtual seconds, while the same request completed in about a millisecond
  in real time: the runtime moves its clock to the next timer whenever it parks, a loopback request
  in flight included, so the HTTP client's own timeout fired every time. The transport and the poll
  therefore wait through the transport's `Waits`, tokio's timer (`TokioTimer`) in the service, which
  the tests hand a recorder that notes each wait and returns at once; the goldens' waits are read
  from what it noted, and a network error is an answer withheld past a short client timeout. A test
  on paused time, with no socket, holds `TokioTimer` to its duration (ADR-026).
- **§1, R8: the golden drives `send_html`.** Its adapter drives `telegram.py:TelegramNotifier.send_html`
  rather than `_send_chunk` alone, so the golden also records `send_html`'s default attempts and the
  (attempted, delivered) marker, and its cases add a 400 and a network error. It shows the
  predecessor's rule, which the port keeps: a 429 waits its `retry_after` (1 s without one) before
  the same request, except after the last attempt; a 5xx, a 400 or a network error is followed by
  the next attempt at once. The schematic's doubling wait after a 5xx was not the predecessor's, and
  it is corrected.
- **Goldens: `bot.timeouts`.** The long-poll and HTTP timeouts are `CommandBot.__init__`'s keyword
  defaults, not module constants, and its `__kwdefaults__` also holds the injected clock and sleep,
  which JSON cannot carry; so a fourth golden reads them from a bot the predecessor's own
  constructor built, and `bot.constants` holds the rest.
- **R2: each update is read on its own.** `getUpdates` is decoded as JSON values, and each update is
  read by frankenstein in turn, so one it cannot read is still confirmed by its `update_id` and never
  holds the poll on a batch that would fail forever; it is dropped by its kind and a reason code.
- **R5: the caps' units.** A text is counted in characters, as the predecessor's `len(text)` counted
  it; callback data in bytes, the unit the Bot API bounds it in.
- **R6: attributes.** A value that enters an attribute, such as the privacy policy's link, is
  escaped by `escape_attribute`, which also writes `"` as `&quot;`.
- **R7: the chunker's edges.** The whitespace at a cut stays at the end of the chunk before it, so
  the chunks' visible texts, joined, are the text's; a cut never leaves a tag opened last or closed
  first, so no chunk carries an empty entity; a text with nothing visible is not sent. R12's sample
  is committed as `long-sample.source.txt` beside its two chunk goldens: its first cut falls inside a
  quotation and the bold within it, closed there and opened again.
- **R10: the job role keeps no transport at W0.** No job of the W0 table sends a message, so the job
  role keeps `NoNotifier`; wiring's `TransportMarker` hands the transport's counts to the first job
  that sends (#20, #27). A delivery is a message whose every chunk came back with a message id, and
  each message counts once however many chunks it took.
- **R11: `/export`, `/delete` and `/sync`.** frankenstein uploads a document only from a path, so the
  export is uploaded from memory, through frankenstein's own client and its re-export of reqwest:
  the owner's data is never written to the host's disk to be sent. A `/delete` confirmation erases
  only when it is the latest question's button, and only once; any other tap is answered and told it
  expired. `/sync` marks the owner's rescore before it reads the sync's settings, so the next cycle
  serves it even when this one cannot run, and the answer names the reason code of a sync that could
  not run.
- **R11: the Bot API's base URL is a setting.** `DECKSTREAK_BOT_API_URL`, `https:` or `http:` to a
  loopback host, defaults to Telegram's own; it lets the role's test run the built binary against
  the fake, and a deployment use a local Bot API server.
- **R12: the envelope.** Each golden carries `schema` (`phx.duty.message.v1`), `duty`
  (`bot-commands`, no nudge duty, so nudge-duties passes over it), `x-message` and `send`, and no
  `kind`: a command reply has no notification kind. notifications-policy's `message-metadata` row
  judges every envelope as a notification, so the box run defers that row to #257 until it judges
  only notifications. Every message the bot renders is a golden: 15 replies, the export's caption
  and the sample's two chunks.
- **R13: readiness and the stop.** The database opens before the loop; `READY=1` is sent as the first
  long poll is issued, after the drain, and the heartbeat starts with it. A SIGTERM during a long
  poll abandons it, and nothing it held is handled. An offset stands confirmed only once the server
  has answered a request that carried it, and an abandoned poll's request may never have left the
  process, so the stop sends its confirming request whenever the last poll went unanswered
  (ADR-026); an update the abandoned poll was about to return is drained at the next start, never
  replayed.
- **Manifest: the deploy template and SPEC-021's test.** The bot role requires the Mini App's URL, so
  the deploy template names it, with no value, as it names the sync's endpoint. SPEC-021's policy
  test pins the policy's entry points, and R14 adds the bot's command table to them.
- **The TLS stack and the licences.** frankenstein's reqwest client turns on reqwest's rustls with
  its aws-lc-rs provider, whose licences deny.toml admits as it stands. frankenstein's own licence,
  WTFPL, is admitted by one exception for that crate alone, never for the workspace. frankenstein
  depends on `paste`, whose RustSec notice RUSTSEC-2024-0436 says it is unmaintained, not
  vulnerable; deny.toml ignores it by its id and why, as it does the engine's six such notices.
