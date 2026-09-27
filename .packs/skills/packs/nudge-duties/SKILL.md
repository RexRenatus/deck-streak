---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/nudge-duties

The OUTPUT checks for two of DeckStreak's agent duties (SPEC-V2-2217 / ADR-V2-2217): the DAILY
DIGEST, deterministic stats plus AI coaching, and the ONE-PER-LAPSE COMEBACK. The owner chose
"3 packs: study, nudges, vault (Recommended)"; this is the nudges pack. Its checks read the
message the agent is about to send and prove it meets the owner's rules and the practice behind
them.

The owner's decisions this pack encodes:

- After two or more days without study, the daily readings PAUSE, and the owner gets ONE comeback
  reading per lapse, inside v9's three-message comeback cap.
- A comeback invites; it never scolds. No guilt, shame or loss framing.
- No dates, deadlines or countdowns, anywhere.
- The digest's deterministic numbers match its stats input.
- When the AI step fails closed, the digest still goes out, and says plainly that coaching was
  unavailable. A silent skip is a defect.
- Blocking for facts, privacy and safety; advisory for voice and heuristics.

`scripts/nudge-duties-probe.py` is the executable contract. It is standard-library Python and
vendorable, it judges any tree through `--root`, and every row below runs it. Which seats consume
this pack is its catalog row's `consumes`, the one record of that edge, so this body names none.

```
phxd pack probe --pack nudge-duties --root PATH --format json
```

## What this pack judges, and what it does not

It judges MESSAGE TEXT and MESSAGE METADATA. The notification POLICY is the notifications-policy
pack's: the celebration ladder T0 to T5, the weekly budgets, deduplication, deferral in quiet hours,
the comeback cap and its spacing, the 10% holdout, and the one router the bot and the mini app share.
The two packs meet at the envelope's metadata (`kind`, `tier`, `budget_key`, `dedupe_key`,
`lapse_id`), whose shape they agreed. This pack checks that shape and never the policy values.

The engine's BEHAVIOUR is the DeckStreak architect's: pausing the readings, the day rollover,
failing closed, and sending the digest every study day. Section "What a text check cannot prove"
teaches those rules; no row pretends to prove them.

## The rows

Sixteen rows, all `tree`-scoped, one per class of `nudge-duties-probe.py`. Each runs
`python3 {skills}/../scripts/nudge-duties-probe.py --root {root} check <class>` under a 120-second
wall. `{skills}` is the skills directory the catalog was read from, so the script and its data
always come from this pack, whatever tree `--root` names.

Stage `contract`: 1 row (1 blocking, 0 advisory). It holds every message to the envelope v1.

| row | severity | reason | refuses when |
|---|---|---|---|
| `message-contract` | block | `message-invalid` | a claimed file is not UTF-8 JSON or not `phx.duty.message.v1`; a key is unknown (other than `x-` keys) or missing; `duty`, `kind`, `tier` or `coaching` is off its vocabulary; a token key is not a token; a key belongs to the other duty; `stats_source` leaves the tree; a `stats` entry lacks a key or a label, repeats one, or its label holds a digit; a part's role is off its duty's roles or repeats, its text is empty, a required part is missing, or the stats part is not first; `send` carries an unknown key, an `entities` list, a `chat_id` or a parse mode other than HTML; `send.text` is not the parts joined by a blank line |

Stage `telegram`: 3 rows (3 blocking, 0 advisory). They hold the payload to the Bot API, whose
refusal is a failed send. Telegram validation has ONE home, the telegram-platform pack's probe:
these rows call it and keep no Telegram parser of their own.

| row | severity | reason | refuses when |
|---|---|---|---|
| `telegram-length` | block | `message-too-long` | telegram-platform's `check_message` names a length problem with the text: longer than 4096 UTF-16 code units after entity parsing, or empty (whitespace only). A markup problem it also names is left to `telegram-entities` |
| `telegram-entities` | block | `entities-invalid` | telegram-platform's `parse_text` names any problem with the markup: an unsupported tag, an unescaped `<`, `>` or `&`, an unsupported named entity, a missing attribute, an unclosed, stray or overlapping tag, or a nesting Telegram forbids |
| `telegram-keyboard` | block | `keyboard-invalid` | telegram-platform's `check_keyboard` names any problem with `reply_markup`: a button with no text, other than one action field, `callback_data` outside 1 to 64 bytes, a URL scheme other than http, https or tg, a `web_app` URL that is not https, or an unknown style |

Stage `facts`: 4 rows (3 blocking, 1 advisory). They hold the digest to its data.

| row | severity | reason | refuses when |
|---|---|---|---|
| `digest-numbers` | block | `digest-number-mismatch` | a declared stat's label is missing from the stats part or on more than one line, no number follows it, or the number differs from the stats input; the input holds no number at a stat's key or cannot be read; a number in the stats part belongs to no declared stat |
| `digest-degraded` | block | `degraded-digest-unsaid` | coaching is `unavailable` but a coaching part is present, or no notice part says so plainly, or the notice is hidden in a spoiler or an expandable quote; coaching is `ok` but a notice is present or the coaching part is missing |
| `no-internals` | block | `internals-leaked` | any text, button or button URL holds one of persona-core's public scrubber shapes or one of the 12 internals rows: a traceback, an exception name, an errno, an HTTP status, a host and port, a host path, a crash report, a credential variable name, a secret resource name, an exit status, a JSON error body or a model API error type. A finding names the rule, never the value |
| `coaching-numbers` | advisory | `coaching-number-unsourced` | the coaching quotes a number the stats input does not hold |

Stage `lapse`: 2 rows (2 blocking, 0 advisory). They carry the owner's lapse decisions.

| row | severity | reason | refuses when |
|---|---|---|---|
| `comeback-shape` | block | `comeback-invalid` | the comebacks of one `lapse_id` name more than one `reading_id`; a comeback carries other than exactly one button, or its button uses `callback_data`, or neither a url nor a web_app |
| `readings-paused` | block | `readings-during-lapse` | a digest carries a `readings` part while its `lapse_id` names an open lapse |

Stage `ethics`: 3 rows (3 blocking, 0 advisory). They carry the owner's blocking rules for safety
and privacy.

| row | severity | reason | refuses when |
|---|---|---|---|
| `no-dates-or-countdowns` | block | `date-or-countdown` | any text, button or button URL matches persona-core's date or timeline rows or one of this pack's 7 nudge-scale urgency rows; the HTML holds a `<tg-time>` date-time entity, whatever its visible text; a link or button opens `tg://time` |
| `no-shame-framing` | block | `shame-or-loss-framing` | any text or button matches one of the 16 shame rows: a broken or lost streak, blame for the break, a threatened loss, a streak at risk, last-chance wording, a rescue framed against a loss, all-or-nothing framing, shame words, a character judgement, disappointment, failure or quitting, a guilt trip, falling behind, a missed-day count, or confirmshaming |
| `no-journal-in-messages` | block | `journal-leaked` | any text or button links, embeds or tags into the journal, by vault-duties' `journal_leaks`; with `--vault`, any run of the window's consecutive words repeats a journal note. A finding names the line, never the journal's text |

Stage `voice`: 3 rows (0 blocking, 3 advisory). They report and never refuse.

| row | severity | reason | reports when |
|---|---|---|---|
| `autonomy-language` | advisory | `controlling-language` | the text uses controlling language: should, must, have to, a nagging reminder or a pressuring directive |
| `shouting` | advisory | `shouting` | a run of block capitals of three or more letters that is not a listed acronym, or two or more exclamation marks in one message |
| `near-miss-copy` | advisory | `near-miss-copy` | near-miss copy appears in any part but the deterministic stats part, or in a button |

## The message envelope, v1

A message is ONE JSON file whose name ends `.msg.json`. The envelope is every DeckStreak outbound
message's, a celebration's too, and this pack judges the two duties it owns: an envelope whose
`duty` names another pack's duty is not examined here. The walk skips `.git`, dot-directories,
`node_modules` and `target`. A claimed file that does not parse is a finding in every class, never
a skip. The worked examples are in [examples](examples/), and the vocabularies in
[contract.json](contract.json).

| key | on | value |
|---|---|---|
| `schema` | both | `phx.duty.message.v1` |
| `duty` | both | `daily-digest` or `comeback` |
| `kind` | both | the duty's kind in the policy's kinds table: `digest` or `comeback` |
| `tier` | both | `T0` to `T5`; the policy pack decides the value (`T2` for both kinds) |
| `dedupe_key` | both | an opaque token, `[a-z0-9][a-z0-9:._-]*`, that spells no calendar date |
| `budget_key` | comeback | an opaque token naming a declared budget (`comeback`); absent on the budget-exempt digest |
| `lapse_id` | comeback, and a digest during a lapse | an opaque token for the lapse |
| `reading_id` | comeback | the lapse's one comeback reading |
| `coaching` | digest | `ok` or `unavailable` |
| `stats_source` | digest | the stats input, a relative path beside the message |
| `stats` | digest | `[{"key": "<dotted path in the input>", "label": "<label without digits>"}]` |
| `parts` | both | `[{"role": ..., "text": ...}]`: the provenance of the text |
| `send` | both | the literal Bot API sendMessage payload: `text`, `parse_mode` (`HTML` or absent), `reply_markup`, `link_preview_options`, `disable_notification`, `protect_content` |
| `x-<slug>` | both | an extension key, ignored |

**Parts.** A digest's roles are `stats` (required, first), `coaching`, `notice`, `readings` and
`invite`. A comeback's roles are `invite` (required) and `reading`. Each role appears once, and
`send.text` is the parts' texts joined by a blank line, so every character sent has a known source.

**What never belongs in a message file.** A chat id (the engine adds it when it sends), an
`entities` list (the contract admits HTML or plain text), and a caption: v1 is a text message.

## What each check reads

- **Visible text.** Every class reads the text as telegram-platform's `parse_text` returns it: the
  plain text after entity parsing, and its entities with UTF-16 offsets. Plain text is read as it
  is.
- **Hidden text.** A `spoiler` or an `expandable_blockquote` entity hides its range until the
  reader acts. The ranges are UTF-16 units, so an emoji outside the basic plane is two.
- **Dates in entities.** A `date_time` entity (Bot API 9.5) is a date whatever its text says, and a
  `text_link` to `tg://time` renders one.
- **The stats part.** Each stat is one line, `<label>: <number>`. The number is the FIRST number
  after the label on its line, and it is compared with the input's value at the stat's key, as a
  decimal: `1,240` equals `1240`, and `92.5` equals `92.5`. An ordinal such as `2nd` and a token
  such as `T4` are not numbers. Every other number in the stats part is unsourced.
- **The degraded notice.** It must match one of `patterns.json`'s `degraded_notice` rows OUTSIDE any
  hidden range.
- **Patterns.** Every table row is matched per NFKC-normalised line through persona-core's public
  `scan`, and every row carries an `example` the test suite replays. The tables are in
  [patterns.json](patterns.json).
- **Buttons.** Button texts are read by `no-dates-or-countdowns`, `no-shame-framing`, `autonomy-language`,
  `shouting` and `near-miss-copy`; button URLs by `no-dates-or-countdowns` and `no-internals`.

## Reuse: what this pack calls and never copies

persona-core is DeckStreak's shared text core, and this pack loads its probe,
`scripts/persona-core-probe.py`, through importlib:

- `load_patterns()` gives `no-dates-or-countdowns` the calendar-date and timeline rows in six languages;
- `load_deny(None, None)` gives `no-internals` the PUBLIC scrubber shapes. The private list is not
  applied: the digest is the owner's own, and a deck name in it is not a leak;
- `scan(text, rows)` is the one per-line NFKC loop every pattern class uses.

vault-duties owns the rule that the journal never leaks, and this pack calls its probe,
`scripts/vault-duties-probe.py`, the same way: `load_journal()` once a run, then `journal_leaks()`
on every message's visible text and button texts. Without `--vault` it reads the SHAPES that reach
into the journal (links, embeds and tags), which needs nothing private and runs in CI. With
`--vault DIR` it also proves a verbatim quote of a journal note, which needs the private vault and
so runs only where the vault is, on the VM, before a message is sent.

telegram-platform is the one home of Telegram message validation, and this pack calls its probe,
`scripts/telegram-platform-probe.py`: `parse_text()` for every visible text and its entities,
`check_message()` for the length and emptiness of the text, and `check_keyboard()` for every
`reply_markup`. This pack adds no Telegram rule of its own.

A class that needs a sibling's probe and cannot load it, or whose sibling cannot load its own data,
is VOID, never green. `--persona-core FILE`, `--vault-duties FILE` and `--telegram-platform FILE`
name vendored copies.

## The Python API

`nudge-duties-probe.py` loads through importlib whether or not the module is registered.

- `load_message(path) -> Message`: a frozen dataclass with `path`, `envelope` (the decoded object),
  `duty`, `parts` (a tuple of `Part(role, text)`), `send`, `text` and `parse_mode`. A file that is
  unreadable, not UTF-8, not JSON, not an object or not `phx.duty.message.v1` raises
  `ContractError`, a ValueError subclass whose `str()` is the reason.
- `read_text(telegram_platform, text, parse_mode) -> Reading`: the visible and unhidden text, the
  markup problems, the hiding entities, the date-time entities and the link URLs.
- `visible_text(text, parse_mode)` and `numbers(text)`.
- `discover(bases)`, `claimed(path)`, `load_contract(pack_dir=None)`, `load_patterns(pack_dir=None)`,
  `load_persona_core(path=None)`, `load_vault_duties(path=None)` and
  `load_telegram_platform(path=None)`.
- The `parse FILE` command prints one envelope as JSON: its parts, visible text, UTF-16 length,
  markup problems and contract problems.

## What a text check cannot prove

These rules bind the ENGINE. The rows cannot see them, so they are taught here:

- The daily readings pause after two or more days without study, and resume on the next study day.
- The digest goes out every study day. When the coaching step fails closed, the engine removes the
  coaching part, adds the notice from `templates/daily-digest.md`, and sends. The cause goes to the
  logs.
- The engine never lets the model write the stats part, the notice or a number in the stats input.
- A message that fails a blocking row is not sent. The engine falls back to the degraded digest, or
  to the next comeback variant.
- The comeback cap, its spacing, the holdout, quiet hours and the budgets are the policy pack's
  rows, read from the same envelope.

## How DeckStreak's engine uses this pack

1. The stats step writes the stats input. The coaching step writes its paragraph under
   `templates/coaching-rules.md`.
2. The engine renders the message from `templates/daily-digest.md` or `templates/comeback.md` and
   writes the envelope beside its stats input, in the run's output directory.
3. The engine runs `python3 scripts/nudge-duties-probe.py --root <repo> --subject <run dir> check
   <class>` for every blocking class, with `--vault <vault>` for `no-journal-in-messages`, and sends
   `send` only when all are green.
4. The repository keeps synthetic golden envelopes of both duties, so these rows examine something
   in CI: a tree with no `.msg.json` file is VOID for every row.

## References

- Telegram Bot API: sendMessage, Formatting options, MessageEntity, InlineKeyboardButton and the
  changelog, https://core.telegram.org/bots/api and https://core.telegram.org/bots/api-changelog;
  styled text and UTF-16 lengths, https://core.telegram.org/api/entities; limits,
  https://core.telegram.org/bots/faq.
- Notifications: Kushlev, Proulx and Dunn, CHI 2016; Fitz and colleagues, Computers in Human
  Behavior 2019; App Store Review Guideline 4.5.4.
- Dark patterns: Gray and colleagues, CHI 2018 (nagging); Mathur and colleagues, CSCW 2019
  (confirmshaming); the FTC staff report "Bringing Dark Patterns to Light"; the Digital Services
  Act, Article 25.
- Framing and lapses: O'Keefe on guilt appeals; Gallagher and Updegraff's framing meta-analysis;
  Silverman and Barasch on broken streaks; Lally and colleagues on habit formation; Marlatt and
  Gordon's relapse prevention; Cochran and Tesser; Breines and Chen on self-compassion;
  Vansteenkiste and colleagues on autonomy-supportive language; Gollwitzer on implementation
  intentions; Dixon and colleagues on losses disguised as wins; Clark and colleagues on near-misses.
- Faithful numbers: Wiseman, Shieber and Rush, EMNLP 2017. Error hygiene: the OWASP Error Handling
  Cheat Sheet. Tone: GOV.UK's guidance on block capitals.

The full list, with access dates and the Context7 ids that answered, is in SPEC-V2-2217.
