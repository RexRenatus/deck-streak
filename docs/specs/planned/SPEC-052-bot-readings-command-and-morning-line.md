# SPEC-052: `/prestudy` lists today's topics as buttons that open or regenerate a reading, and a morning line says how many readings are ready

- **Wave:** W1. **Issue:** #38 (epic #2). **Context(s):** `deck-streak-bot` (the command and its reply); `deck-streak-coordination` (the morning line; the listing is SPEC-051's today view); the `reading_ready` kind routed by `deck-streak-notifications`.
- **Decided by:** ADR-006 (the bot's owner gate), ADR-011 (side by side: DeckStreak sends only kinds
  the predecessor does not), ADR-019 (tap to pick, never a typed topic), and ADR-052 (the command's
  name, replies outside the router, deep links for the buttons, and golden replies).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-052.md` (ADR-016).

## 1. The problem, measured

- **A command one letter from another.** The predecessor's on-demand reading command sat one
  character from `/read`, which logs minutes of book reading (the habit `reading-log`), and it was
  never merged. The readings command is `/prestudy`, which no one can mistake for `/read`.
- **The readings reached no morning.** The predecessor's morning message never mentioned a reading.
  The owner's day starts in Telegram, and the readings are a ritual before study.
- **Side by side.** The predecessor still sends its own morning brief, so DeckStreak's line is its own
  kind, `reading_ready`, recorded as a deviation of the notification policy by SPEC-041.
- **Prerequisites.** SPEC-026 (the bot's transport, owner gate, command table, menu and reply
  path), SPEC-041 (the router and the `reading_ready` kind), SPEC-046 (readings and their minutes),
  SPEC-048 (the pick token and the regenerate callback), and SPEC-051 (the today view the Mini App's
  Today reads, which `/prestudy` reads too, so both surfaces run one code path).
- **Build order: this SPEC lands before SPEC-049.** SPEC-049 then gives the morning use case this
  SPEC adds (`crates/coordination/src/readings/morning.rs`) its lapse branch, the comeback reading
  in place of the line, and takes over the criterion that no line is raised during a lapse. SPEC-053
  schedules the morning use case after both.

## 2. Requirements

R1. `/prestudy` answers the owner, from SPEC-051's today view, with one line per topic of the
    server's study day and its state, and an inline keyboard with, for each topic, a button that
    opens its reading when one is ready (a URL whose `startapp` value is `r_<reading id>`) and a
    Regenerate button (callback `rg:<pick token>`, SPEC-048).
R2. A deep link is `https://t.me/<bot username>/<app short name>?startapp=<token>`, where the bot's
    username and the Mini App's short name are configuration; a `startapp` value holds only
    `A-Z a-z 0-9 _ -` and at most 512 characters, and a callback holds 1 to 64 bytes.
R3. The `/prestudy` answer is a reply to the owner's own message, sent through the bot's reply path;
    it is not a notification and never calls one of the router's transport calls.
R4. Anything typed after `/prestudy` is ignored: the answer is the same listing, and nothing is
    regenerated.
R5. At the morning readings job (SPEC-053), when at least one reading of the study day is ready and
    unread, a `reading_ready` occasion is raised through the router: the line "N readings ready
    (about M min)", N being the ready unread readings and M the sum of their reading minutes, with
    one button opening the Mini App's Today, and a dedupe key per study day. No line is raised when
    none is ready and unread. During a lapse the job takes SPEC-049's comeback branch instead, and
    SPEC-049, which lands after this SPEC, proves that no line is raised then.
R6. The line's envelope is `phx.duty.message.v1` with duty `readings-morning` (not a nudge-duties
    duty), kind `reading_ready`, tier `T2`, no budget key and an opaque dedupe key. A golden envelope is
    committed, and notifications-policy's `message-metadata` and telegram-platform's payload rows are
    green on it.
R7. The `/prestudy` reply's golden is committed as `prestudy.reply.json`, outside the `*.msg.json`
    naming that the message packs read (a reply has no policy kind), and a test validates it with
    telegram-platform's own message and keyboard checks.
R8. `/prestudy` is in the owner's command menu with the description "today's pre-study readings",
    and `/read` keeps its own meaning.
R9. An update from anyone but the owner gets no answer (SPEC-026's owner gate).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `/prestudy` answers with a button per topic, deep links carrying `r_` startapp tokens for ready readings and `rg:` callbacks, equal to the golden reply | `prestudy_answers_with_a_button_per_topic_and_startapp_links` |
| A2 | the golden reply passes telegram-platform's message and keyboard checks | telegram-platform payload checks; `test_the_prestudy_golden_reply_passes_the_telegram_checks` |
| A3 | a topic typed after `/prestudy` regenerates nothing and changes the answer in no way | `a_typed_topic_after_prestudy_regenerates_nothing` |
| A4 | the morning line counts the ready unread readings and their minutes | `the_morning_line_counts_ready_unread_readings_and_their_minutes` |
| A5 | no morning line is raised when no reading is ready and unread | `no_morning_line_when_nothing_is_ready_and_unread` |
| A6 | the line's golden envelope is green under notifications-policy's `message-metadata` and telegram-platform's payload rows, and `reading_ready` is a recorded deviation | notifications-policy `message-metadata`, `policy-deviation-has-adr`; `test_the_morning_line_envelope_is_green_under_the_message_rows` |
| A7 | after this delivery the `one-router` row is still green: the reply path calls no transport call | notifications-policy `one-router`; `test_the_bot_calls_no_transport_outside_the_router` |
| A8 | `/prestudy` is in the owner's menu, and `/read` is untouched | `prestudy_is_in_the_menu_and_distinct_from_read` |
| A9 | a stranger's `/prestudy` gets no answer | `a_stranger_gets_no_prestudy_answer` |

```acceptance
A1: cargo test -p deck-streak-bot --test prestudy -- --exact prestudy_answers_with_a_button_per_topic_and_startapp_links
A2: python3 -m unittest discover -s scripts/tests -p test_prestudy_messages.py -k test_the_prestudy_golden_reply_passes_the_telegram_checks
A3: cargo test -p deck-streak-bot --test prestudy -- --exact a_typed_topic_after_prestudy_regenerates_nothing
A4: cargo test -p deck-streak-coordination --test readings_morning -- --exact the_morning_line_counts_ready_unread_readings_and_their_minutes
A5: cargo test -p deck-streak-coordination --test readings_morning -- --exact no_morning_line_when_nothing_is_ready_and_unread
A6: python3 -m unittest discover -s scripts/tests -p test_prestudy_messages.py -k test_the_morning_line_envelope_is_green_under_the_message_rows
A7: python3 -m unittest discover -s scripts/tests -p test_prestudy_messages.py -k test_the_bot_calls_no_transport_outside_the_router
A8: cargo test -p deck-streak-bot --test prestudy -- --exact prestudy_is_in_the_menu_and_distinct_from_read
A9: cargo test -p deck-streak-bot --test prestudy -- --exact a_stranger_gets_no_prestudy_answer
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/bot/src/commands/prestudy.rs` | `deck-streak-bot` | added: the command and its reply |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: registers the prestudy command in the command table and the owner's menu |
| `crates/bot/tests/prestudy.rs` | `deck-streak-bot` | added |
| `crates/bot/tests/replies/prestudy.reply.json` | `deck-streak-bot` | added: the golden reply, synthetic |
| `crates/bot/src/links.rs` | `deck-streak-bot` | added: deep links from the configured bot username and short name |
| `crates/coordination/src/readings/morning.rs` | `deck-streak-coordination` | added: the morning job's ready line |
| `crates/coordination/tests/readings_morning.rs` | `deck-streak-coordination` | added |
| `crates/coordination/tests/messages/reading-ready.msg.json` | `deck-streak-coordination` | added: the golden envelope, synthetic |
| `scripts/tests/test_prestudy_messages.py` | repo | added |
| `docs/specs/SPEC-052-bot-readings-command-and-morning-line.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-052-prestudy-replies-deep-links-and-the-morning-line.md` | docs | added |
| `docs/red-first/SPEC-052.md` | docs | added |

## 5. What this does NOT do

- It changes no morning brief; the readings line may join the brief when the engagement wave builds
  it (#122).
- It touches nothing of the minutes-of-reading habit and its `/read` (#93).
- It registers no Mini App with BotFather; the short name is the owner's action (#171).
- It sends no comeback message and takes no lapse branch: in a lapse the morning job offers the
  comeback reading instead, a branch SPEC-049 adds with its criterion that no line is raised then
  (#35).
- It draws no Today screen (#37).

## 6. Risks

- **A reply written with a transport call** breaks the one-router rule. Detected by A7 and the
  `one-router` row in the gate.
- **A deep link names the bot's real username in the repository.** The username and short name are
  configuration; the goldens use a synthetic example host and name, and the public scrub runs over them.
- **A morning line during a lapse, before SPEC-049 lands.** Until SPEC-049 gives the morning use case
  its lapse branch, the use case knows no lapse. No timer runs it in that interval: SPEC-053 schedules
  the morning job only after this SPEC and then SPEC-049 (the build order above).
- **Too many topics overflow one message.** The listing is one line and one button row per topic, far
  under Telegram's limits; telegram-platform's length check runs on the golden (A2).
