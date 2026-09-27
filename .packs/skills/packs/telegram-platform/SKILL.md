---
name: telegram-platform
description: >-
  Checks a Telegram bot and Mini App against the platform's own rules: webhook or long polling,
  flood control and retry_after, MarkdownV2 and HTML escaping, the 4096 and 1024 limits, callback
  answers and callback_data size, deep links, replaced Bot API fields, the Mini App's version gates,
  storage and sendData, and one validator for outbound message payloads. Use when building or
  reviewing a bot or a Mini App, such as DeckStreak's, through scripts/telegram-platform-probe.py.
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/telegram-platform

How a Telegram bot and Mini App meet the platform's own rules, and the check that holds a
repository to them (SPEC-V2-2219 / ADR-V2-2219). It covers:

- the Bot API transport (webhook or long polling);
- flood control;
- the message formats, their limits and chunking;
- inline keyboards and callback answers;
- the Mini App platform;
- the Bot API's replaced fields;
- one validator for outbound message payloads.

Its first user is DeckStreak: a Rust workspace with a frankenstein bot, and a SvelteKit Mini App
behind Caddy.

`scripts/telegram-platform-probe.py` is the check. It is standard-library Python and vendorable,
and it judges any tree through `--root`. Every row below runs it. Which seats consume this pack is
its catalog row's `consumes`, the one record of that edge (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack telegram-platform --root PATH --format json
```

## What this pack composes, and never copies

| practice | the pack that owns the check | its rows |
|---|---|---|
| Mini App init data validated on the server with the `WebAppData` HMAC key, compared in constant time, `auth_date` fresh, `initDataUnsafe` never trusted | `packs/web-security` | `ws.tg-init-data-verified`, `ws.tg-init-data-constant-time`, `ws.tg-init-data-fresh` |
| a webhook passes `secret_token`, and the handler checks `X-Telegram-Bot-Api-Secret-Token` | `packs/web-security` | `ws.tg-webhook-secret` |
| Mini App polish: the theme-change listener, `var(--tg-theme-*)` fallbacks, BackButton and MainButton, haptics, safe areas, `viewportStableHeight` rather than `100vh` | `packs/vibecode-polish` | its six `telegram-*` rows |
| the Mini App's launch readiness: `telegram-web-app.js` first in `<head>`, `ready()`, `expand()`, `initData` sent raw, `openLink` rather than a plain link, the `start` and `startapp` values in its pages and scripts, CloudStorage key syntax, and any `isVersionAtLeast` at all | `packs/web-launch` | `tg-sdk-script`, `tg-ready`, `tg-expand`, `tg-init-data`, `tg-links`, `tg-deep-links`, `tg-startapp`, `tg-cloud-storage-keys`, `tg-version-gate` |
| Telegram login and OIDC for the product's web sign-in | `packs/auth` | its Telegram login rows |
| held SDKs and crates (`@telegram-apps/*`, teloxide) | `packs/stack-selection` | `no-hold-items` |
| the TEXT of a digest, a nudge or a comeback: no guilt, no dates, honest numbers | `packs/nudge-duties` | its message rows, which call this pack's payload API |

Run those packs beside this one. This pack's rows judge what they do not.

## The rows

Nineteen rows, all `tree`-scoped, one per class of `telegram-platform-probe.py`. Each runs
`python3 {skills}/../scripts/telegram-platform-probe.py --root {root} check <class>` under a
60-second wall. `{skills}` is the skills directory the catalog was read from (SPEC-V2-2195), so the
script always comes from this pack, whatever tree `--root` names.

The `bot-api` stage: 13 rows (11 blocking, 2 advisory). They read shipped bot source: Rust,
Python, TypeScript and JavaScript, with comments and docstrings blanked first. A method named as a
string literal or a URL segment (`"sendMessage"`, `/sendMessage`) is the Bot API wherever it
appears. A bare call (`send_message(`, `.sendMessage(`) counts only in a tree that uses a bot
library, named in its code or its dependency manifests: frankenstein, grammY, telegraf, aiogram,
python-telegram-bot and the like.

| row | severity | reason | refuses when |
|---|---|---|---|
| `tg-update-mode` | block | `update-mode-conflict` | the tree calls both `setWebhook` and `getUpdates` and nothing calls `deleteWebhook`: getUpdates answers nothing while a webhook is set |
| `tg-webhook-endpoint` | block | `webhook-endpoint-invalid` | a literal webhook URL is not `https`, names a port other than 443, 80, 88 or 8443, or `max_connections` is outside 1-100 |
| `tg-poll-offset` | block | `poll-offset-missing` | a file polls `getUpdates` with no `offset`, or never recomputes it from `update_id` + 1 |
| `tg-poll-long` | advisory | `short-polling` | `getUpdates` passes no `timeout`, or a literal 0: that is short polling, for testing only |
| `tg-allowed-updates` | advisory | `allowed-updates-implicit` | a poll or a `setWebhook` names no `allowed_updates`, so the previous setting persists |
| `tg-retry-after` | block | `retry-after-ignored` | the tree calls the Bot API and nothing reads `retry_after` (or an auto-retry library): a 429 is answered by waiting that many seconds |
| `tg-parse-mode` | block | `legacy-markdown` | a parse mode is legacy `Markdown`: `ParseMode::Markdown`, `ParseMode.MARKDOWN`, or `parse_mode` set to `"Markdown"` |
| `tg-escape` | block | `markup-unescaped` | MarkdownV2 is used and no escaper covers its 18 special characters and `\`, or an escaper misses some; HTML is used and no escaper covers `<`, `>` and `&`, or a hand-rolled chain replaces `&` after `<` or `>` |
| `tg-length-bound` | block | `length-unbounded` | messages are sent and no bound of at most 4096 is declared, captions are sent and no bound of at most 1024, or a text- or caption-named limit exceeds its bound. A limit's name carries a length word (len, limit, max, size) and no rate, time or queue word |
| `tg-callback-answer` | block | `callback-unanswered` | `callback_query` is handled and `answerCallbackQuery` is never called, or callback buttons are sent and no handler reads `callback_query` |
| `tg-callback-data` | block | `callback-data-size` | a literal `callback_data` is not 1-64 bytes of UTF-8, or a template's fixed part alone exceeds 64 |
| `tg-deep-link` | block | `deep-link-invalid` | a literal `t.me` link in Rust or Python bot source has a `start` parameter over 64 characters, a `startapp` over 512, or either holding a character outside `A-Z a-z 0-9 _ -`. web-launch's `tg-deep-links` and `tg-startapp` judge the links in the Mini App's pages and scripts |
| `tg-replaced-fields` | block | `replaced-bot-api-field` | bot code names a field or method a later Bot API version replaced, such as `reply_to_message_id` or `disable_web_page_preview` |

The `mini-app` stage: 3 rows (3 blocking). They read shipped web source: HTML, Svelte, Vue,
Astro, TypeScript and JavaScript. `tma-send-data` recognises both `Telegram.WebApp` and the raw
bridge (`TelegramWebviewProxy.postEvent` and `web_app_*` events). Each row judges what web-launch's
`tg-*` rows do not.

| row | severity | reason | refuses when |
|---|---|---|---|
| `tma-version-gate` | block | `version-ungated` | a method the official script throws `WebAppMethodUnsupported` on is called with no `isVersionAtLeast` gate at its version, either within three lines before the call or as a negated early-exit gate in the file. web-launch's advisory `tg-version-gate` asks only that some gate exists |
| `tma-storage` | block | `storage-misused` | a literal CloudStorage value exceeds 4096 characters, more than 10 SecureStorage keys are named, or a credential-named key goes to CloudStorage or DeviceStorage rather than SecureStorage. web-launch's `tg-cloud-storage-keys` judges key syntax |
| `tma-send-data` | block | `send-data-unlaunchable` | the Mini App calls `sendData` (or `web_app_data_send`) and no bot code offers a reply-keyboard `web_app` button, the only launch `sendData` works from |

The `payload` stage: 3 rows. They read committed outbound payloads, every `*.msg.json` in the
tree. That includes test directories, where golden messages live. A payload is the file's `send`
object when it has one, otherwise the object itself.

| row | severity | reason | refuses when |
|---|---|---|---|
| `payload-length` | block | `payload-length` | a text is not 1-4096 UTF-16 units after entity parsing or holds only whitespace, or a caption exceeds 1024 |
| `payload-markup` | block | `payload-markup` | the markup does not parse as the Bot API parses it (see "What the payload rows parse"), a link would be dropped (a scheme other than http, https or tg, or text that is not a URL), the parse mode is legacy `Markdown` or unknown, or explicit `entities` fall outside the text or overlap |
| `payload-keyboard` | block | `payload-keyboard` | an inline button carries no action or more than one, `callback_data` is not 1-64 bytes, a URL scheme is not http, https or tg, a `web_app` or `login_url` is not https, a `style` is not danger, success or primary, `copy_text` is not 1-256 characters, rows are not arrays, a reply button carries more than one request field, a button or the markup carries a field the Bot API does not list, or the markup names no keyboard |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`. The
script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when VOID.
- A tree class counts the source files it read. A tree with no source is VOID.
- A class with nothing to judge still counts the files it read, so it is green rather than VOID.
  An example is `tma-storage` in a tree that uses no storage.
- Over `--root`, a payload class counts the files it walked. Over `--subject DIR`, it counts the
  payloads, and a subject with none is VOID.

The pack's card turns any non-zero exit of a `block` row red, and reports an `advisory` row's
failure as `advisory` without reddening the card.

What is not a shipped file: build output (`target`, `node_modules`, `.svelte-kit`, `dist`, `build`
and the like), test directories and test files, a vendored `telegram-web-app.js` or minified
bundle, and a vendored copy of this pack's own probe.

## What the payload rows parse

The payload classes parse a message exactly as the Bot API's formatting options describe it. The
same parser is the Python API below.

- **HTML.**
  - The supported tags are `b`, `strong`, `i`, `em`, `u`, `ins`, `s`, `strike`, `del`, `tg-spoiler`,
    `span class="tg-spoiler"`, `a href`, `tg-emoji emoji-id`, `tg-time unix format`, `code`, `pre`,
    `pre` with `code class="language-…"`, and `blockquote`, optionally `expandable`.
  - Every other tag is refused.
  - A literal `<`, `>` or `&` must be written `&lt;`, `&gt;` or `&amp;`.
  - The named entities are only `&lt;`, `&gt;`, `&amp;` and `&quot;`. Numeric entities are all
    allowed.
- **MarkdownV2.**
  - Outside code, pre and link URLs, the characters `_ * [ ] ( ) ~ > # + - = | { } . !` and the
    backtick, 18 in all, must each be escaped with `\`.
  - Inside code and pre, only `` ` `` and `\` are escaped. Inside a link URL, only `)` and `\`.
  - `>` starts a block quotation at a line start. `**>` starts an expandable one, and a closing `||`
    at a line's end closes it.
  - `![…](tg://emoji?id=…)` is a custom emoji, and `![…](tg://time?unix=…&format=…)` is a
    `date_time`.
- **Nesting.** Two entities that share characters must nest. Beyond that:
  - bold, italic, underline, strikethrough and spoiler may contain, and sit inside, anything except
    `code` and `pre`;
  - a block quotation never nests in a block quotation;
  - all other entities may not contain each other.
- **Date and time.** A `date_time` entity's format must match `r|w?[dD]?[tT]?`. `r` shows the time
  relative to now. nudge-duties refuses it as a countdown by reading the entity's
  `date_time_format`.
- **Links.** A link keeps only an http, https or tg URL, or a bare host that reads as http.
  Telegram drops any other link and keeps its text, so a `javascript:`, `data:`, `mailto:` or `tel:`
  link is refused. An `<a>` with no `href` takes its text as the URL.
- **Attributes and fields.** An attribute that a supported tag does not use, such as `class` on
  `<b>`, is ignored, as Telegram ignores it. A keyboard field the Bot API does not list is refused,
  because Telegram ignores it too, so a misspelled `style` would silently lose its effect.
- **Length.** Length is counted after entity parsing, in UTF-16 code units, the unit the Bot API
  measures entity offsets in. An emoji outside the Basic Multilingual Plane counts as two. A text
  that holds only whitespace is empty, and the Bot API refuses it.

## The Python API

nudge-duties and the DeckStreak engine's own tests call these rather than keep a copy. They are a
published API, and they stay stable. Load the script by path:

```python
import importlib.util
spec = importlib.util.spec_from_file_location("telegram_platform_probe", "scripts/telegram-platform-probe.py")
tpp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tpp)
```

- `utf16_len(text) -> int`: the length in UTF-16 code units.
- `parse_text(text, parse_mode, entities=None) -> Parsed`: `parse_mode` is `"HTML"`,
  `"MarkdownV2"`, `None` (plain, optionally with an explicit `entities` array), or legacy
  `"Markdown"`, which is reported as a problem.
  - `Parsed` is a frozen dataclass:
    - `plain`: the text after entity parsing;
    - `entities`: a tuple of `Entity`;
    - `problems`: a tuple of strings, empty when the markup is sound.
  - `Entity` holds:
    - `type`, `offset` and `length` (in UTF-16 units);
    - `url`, `language`, `custom_emoji_id`, `unix_time` and `date_time_format`, where they apply.
  - `parse_text` never raises on bad markup. It reports it in `problems`.
- `check_message(payload) -> list[str]`: every problem with one send payload. It covers the text
  and caption length, the markup, explicit entities and the keyboard.
- `check_keyboard(reply_markup) -> list[str]`: inline and reply keyboards.
- `check_entities(text, entities) -> list[str]`: an explicit entities array against its text.
- Constants: `TEXT_MAX` (4096), `CAPTION_MAX` (1024), `CALLBACK_DATA_MAX_BYTES` (64).

The command line judges one file with `python3 scripts/telegram-platform-probe.py message FILE`,
and a directory with `--subject DIR check payload-markup` and the other payload classes.

## The platform, taught: what a static read cannot check

- **Webhook or long polling.**
  - DeckStreak runs one VM behind Caddy, so either fits.
  - A webhook needs HTTPS on port 443, 80, 88 or 8443, with no redirects and a certificate whose
    CN matches the domain. It also needs a `secret_token`, which web-security checks.
  - Telegram retries a non-2xx answer, so the handler is idempotent by `update_id`.
  - Answering with a method in the webhook's own response saves a request, but its result cannot
    be read.
  - Long polling needs no public endpoint. Use a positive `timeout` (25-50 seconds), confirm with
    `offset`, and let one process poll.
  - On a deploy, drop or drain stale updates (`drop_pending_updates`, or advance the offset before
    serving) so that old taps are not replayed.
  - Switching from a webhook to polling is `deleteWebhook` first.
- **Flood control.**
  - Send at most about one message a second to a chat, 20 a minute to a group, and about 30 a
    second in bulk. Paid broadcasts raise the bulk rate, and a single-owner bot never needs them.
  - A 429 carries `parameters.retry_after`: wait at least that long, then repeat the same request.
  - Back off exponentially on a 5xx.
  - Pace a fan-out through one queue, never one task per chat.
- **Formatting.**
  - Prefer HTML for generated text: its escaping is three characters, and a missed one is visible.
  - Escape every dynamic value at the point it enters the markup, never after formatting.
  - Never truncate inside a tag or an entity. A cut text is re-parsed, and an unclosed tag is
    refused. Chunk at a paragraph, a line or a word, in UTF-16 units, and close and reopen open
    entities across a chunk boundary.
  - `tg://user?id=` mentions work only inside a link or a button.
- **Keyboards.**
  - Answer every callback query, even with no text. The client shows a progress bar until you do,
    and the answer's text is at most 200 characters.
  - Edit the message in place when a button toggles a setting or turns a page.
  - Keep `callback_data` short and structured, such as `ch:open:<id>`. Look larger state up on the
    server.
  - A `web_app` inline button works only in a private chat.
- **The Mini App.**
  - Load `telegram-web-app.js` first in `<head>`, and call `ready()` as soon as the essential UI is
    up. web-launch's `tg-sdk-script` and `tg-ready` check both.
  - Read launch parameters once at startup: `tgWebAppData`, `tgWebAppVersion`, `tgWebAppPlatform`,
    `tgWebAppThemeParams` and `tgWebAppStartParam` arrive in the URL hash. Route by path so that a
    SvelteKit router never discards the hash.
  - `start_param` is covered by the init data's signature, but the bare `tgWebAppStartParam` is not.
    Trust it only after the server validates the init data.
  - `themeParams` and the `themeChanged` event, `viewportChanged` with `isStateStable`, the safe
    areas and the fullscreen events are vibecode-polish's to check.
  - Store UI state in CloudStorage (up to 1024 items per user), a larger cache in DeviceStorage
    (5 MB), and anything sensitive only in SecureStorage (10 items, backed by the device keychain).
  - Gate every method newer than the app's minimum version with `isVersionAtLeast`.
  - Calls from a foreign origin are blocked since Bot API 10.2. Never drive `Telegram.WebApp` from
    an embedded third-party frame.
  - A direct link opens full-height, and `mode=compact` opens it half-height.

## The Bot API versions that matter here

- 6.0 introduced Web Apps.
- 6.1 added `secret_token`, BackButton, HapticFeedback and `isVersionAtLeast`.
- 6.2 added popups and closing confirmation.
- 6.4 added the QR scanner and the clipboard.
- 6.9 added CloudStorage, write access and contact requests.
- 7.0 replaced `reply_to_message_id` with `reply_parameters`, `disable_web_page_preview` with
  `link_preview_options`, and the `forward_*` fields with `forward_origin`.
- 7.2 added BiometricManager.
- 7.7 added vertical-swipe control.
- 7.10 added the SecondaryButton.
- 8.0 added fullscreen, safe areas, home-screen shortcuts, emoji status, `shareMessage` and
  `downloadFile`.
- 9.0 added DeviceStorage and SecureStorage.
- 9.4 added button `style`.
- 9.5 added the `date_time` entity.
- 9.6 added `requestChat`, and `correct_option_ids` replaced `correct_option_id`.
- 10.2 blocked foreign-origin calls in a Mini App.
- 10.3 added disabled buttons.

frankenstein, the house bot crate, tracks the current version and marks `ParseMode::Markdown`
deprecated.

## How DeckStreak adopts this

1. **Run it.** From this repository, against DeckStreak's tree:

   ```
   phxd pack probe --pack telegram-platform --root PATH --format json
   ```

   Or vendor `scripts/telegram-platform-probe.py` into DeckStreak and run each class in its own CI:
   `python3 scripts/telegram-platform-probe.py --root . check <class>`.
2. **Keep the transport in one place.** Put the frankenstein client, the 429 wait and the escaper
   in the `bot` crate's transport module, and the text and caption bounds as named constants,
   `MAX_TEXT_UTF16` and `MAX_CAPTION_UTF16`.
3. **Commit golden messages.** The engine's tests write every outbound message it renders as
   `*.msg.json`, in nudge-duties' envelope `phx.duty.message.v1`, under `tests/messages/`. The
   payload rows judge them, and nudge-duties judges their text.
4. **One Mini App wrapper.** Put one typed wrapper around `window.Telegram.WebApp` in
   `src/lib/telegram.svelte.ts`. It calls `ready()` and gates each throwing method with
   `isVersionAtLeast`. `src/app.html` loads `telegram-web-app.js` first in `<head>`. Run
   web-launch over the built site for the Mini App's own launch rows.
5. **Read what refuses.**
   - Both update modes with no `deleteWebhook`.
   - A webhook on a wrong port.
   - A poll that never confirms.
   - No 429 wait.
   - Legacy Markdown.
   - An escaper missing `.` or `!`.
   - No length bound.
   - An unanswered callback.
   - `callback_data` over 64 bytes.
   - A replaced field.
   - An ungated CloudStorage call.
   - `sendData` with no keyboard launch.
   - A golden message that the Bot API would refuse.
   - The advisory rows only report.

The worked example of a green tree is phoenix-v2's own `ops/telegram`: three long-polling bots
and two Mini App pages that use the raw bridge. `scripts/tests/test_telegram_platform_probe.py`
judges a copy of it on every tree row.

## References

Primary sources; access dates and the Context7 ids that answered are recorded in SPEC-V2-2219.

- Bot API: https://core.telegram.org/bots/api · https://core.telegram.org/bots/api-changelog ·
  https://core.telegram.org/bots/api#formatting-options · https://core.telegram.org/bots/webhooks
- Bots FAQ and features: https://core.telegram.org/bots/faq · https://core.telegram.org/bots/features
- Mini Apps: https://core.telegram.org/bots/webapps · https://telegram.org/js/telegram-web-app.js
- Launch parameters and the start parameter: https://docs.telegram-mini-apps.com/platform/launch-parameters ·
  https://docs.telegram-mini-apps.com/platform/start-parameter
- frankenstein: https://github.com/ayrat555/frankenstein
