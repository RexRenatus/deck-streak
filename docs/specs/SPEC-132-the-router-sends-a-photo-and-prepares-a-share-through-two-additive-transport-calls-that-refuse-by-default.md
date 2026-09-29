# SPEC-132: the router sends a photo and prepares a share through two additive transport calls that refuse by default

- **Wave:** W7. **Issues:** #125 (streak milestone share cards) and #126 (AI keepsake art for chapter
  ceremonies) (epic #8), for the delivery both need. **Context(s):** `deck-streak-notifications`
  (the photo occasion, `route_photo`, `prepare_share`, the two transport calls and their default
  bodies, the policy's transport list); `deck-streak-bot` (the two calls on the Bot API). No daemon
  role file changes: the bot's transport is already joined wherever a role sends.
- **Decided by:** ADR-041 (the router and its transport port), ADR-136 (this wave's: a share is a
  prepared inline message the owner sends), ADR-054 (no-AI mode is the default) and ADR-135 (this
  wave's: the image pipeline).
- **Prerequisites:** SPEC-026 (the bot's notifier), SPEC-041 (the one router, its order and its
  ledger) and SPEC-084 (the ladder, which adds the dice, reaction and pin calls). SPEC-084 has landed.
  **Mutation band:** `S13200-S13299`.
- **Status:** delivered with its tests, its hand-proved rows and `docs/red-first/SPEC-132.md`
  (ADR-016).

## 1. The problem, measured

- **The router sends text only.** `crates/notifications/src/transport.rs` declares one bot call,
  `push_message`, and `Occasion` (`crates/notifications/src/occasion.rs`) carries a text. A share card
  and a keepsake are photos (#125, #126).
- **A photo sent around the router is refused by design.** The one-router census
  (`crates/notifications/tests/one_router.rs`) holds `sendPhoto` and `savePreparedInlineMessage` among
  the calls no source outside the router's transport may make, so both must become transport calls.
- **The predecessor sent both photos outside its ladder.** `showcase.py:ShowcaseLayer._month_ceremony`
  sends the keepsake after the ceremony's text, and only outside quiet hours;
  `ghost_race.py:GhostRaceLayer._milestone_share_card` sends the share card at once (at `27ee2bc`).
- **A transport that cannot send a photo must say so.** A test double, or a transport built before
  this SPEC, has no photo call. A silent success would record a delivery that never happened.

## 2. Requirements

The two calls (the additive-port pattern)

R1. `BotTransport` gains `push_photo(&Pass, &Photo, caption: &str) -> PhotoFuture`, whose output is
    `PhotoPushed`: `Delivered { file_id }` (the Bot API's file id of the largest size it answers),
    `Failed`, or `Unsupported`. Its DEFAULT body answers `Unsupported` and makes no call. This is the
    house pattern for widening a port (ADR-041's transport): **an additive method with a refusing
    default**, so every existing implementation compiles unchanged and none can answer a success it
    did not earn.
R2. `BotTransport` gains `prepare_share(&Pass, &FileId, caption: &str) -> ShareFuture`, whose output
    is `Prepared`: `Ready { id }` (the prepared message's id), `Failed`, or `Unsupported`, with the same
    refusing default.
R3. A `Photo` holds PNG or JPEG bytes of at most 10 MB (10,000,000 bytes), whose width and height
    total at most
    10,000 pixels with a ratio of at most 20 (Bot API `sendPhoto`), and a caption of at most 1,024
    characters after escaping; anything else is refused at construction
    `photo_invalid`, before any call. It is never written to a log line (SPEC-041's decision ledger
    records the key, never the bytes).

The photo occasion

R4. `Router::route_photo(&Occasion, &Photo)` is the one entry point for a photo. It runs SPEC-041 R4's
    rules in their order, with two differences: a rule that would DEFER (quiet hours) answers
    `NotNow { quiet_hours }` and holds nothing, so no photo's bytes enter the deferral queue; and a
    `Failed` push answers `NotNow { send_failed }` and records nothing. The caller raises it again on a
    later run (SPEC-135 R9 bounds how often and how long).
R5. A photo occasion asks tier T2 and is rendered by one `push_photo`, whatever tier the ladder names
    for its event: the predecessor sent both photos outside its ladder (§1), so a photo never spends
    the week's T4 or T5 budget (SPEC-084). The ladder's `share_card` entry is left as it is; no text
    occasion raises it. A share photo is rendered at T2 outside the ladder's tier, budget and break
    cap, as the predecessor ships its keepsake photo outside the ladder (`showcase.py`), and it
    stays on the one router (R4).
R6. `Delivered` records the delivery under SPEC-041 R6's shape and answers `Sent { file_id }`.
    `Unsupported` records the occasion withheld with the new reason `photo_unsupported` (its kind with
    `:withheld` appended, SPEC-041 R6), final, and sends nothing else: no caption as text, because
    the keepsake's ceremony already stands as text (SPEC-074 R4) and a share card without its photo is
    silent (#125).
R7. `Reason` gains `PhotoUnsupported` (`photo_unsupported`), and `Reason::ALL` names it, so the
    ledger's census of reasons holds it.

The prepared share

R8. `Router::prepare_share(&FileId, caption)` passes the owner's request to `prepare_share` and
    answers its `Prepared`. It raises no occasion and records no delivery: the owner, not the bot,
    sends the result, from Telegram's own share sheet (ADR-136). `Unsupported` and `Failed` are
    answered by name to the caller.

The Bot API

R9. The bot's transport (`crates/bot/src/transport.rs`) implements `push_photo` as one `sendPhoto` to
    the owner's chat with the caption in the bot's HTML, and `prepare_share` as one
    `savePreparedInlineMessage` for the owner's user id with an `InlineQueryResultCachedPhoto` that
    names the file id and the caption, allowing user, bot, group and channel chats. A refusal or an
    unreachable Bot API answers `Failed`; neither call retries inside the transport.
R10. `notifications-policy.json`'s `router.transport.bot` lists `push_photo` and `prepare_share`
    beside the four calls it lists, and the one-router census holds both names.
R11. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are one router
    for every message (R4, R8), no silent success (R1, R2, R6) and no unbounded work (R3, R4).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a transport that does not implement `push_photo` answers `Unsupported` and makes no call | `a_transport_without_push_photo_answers_unsupported` |
| A2 | a transport that does not implement `prepare_share` answers `Unsupported` and makes no call | `a_transport_without_prepare_share_answers_unsupported` |
| A3 | an `Unsupported` photo is recorded withheld `photo_unsupported` and nothing else is sent | `an_unsupported_photo_is_withheld_by_name_and_sends_nothing` |
| A4 | a photo in quiet hours answers `NotNow { quiet_hours }` and holds nothing | `a_photo_in_quiet_hours_is_not_held` |
| A5 | a failed photo answers `NotNow { send_failed }` and records nothing | `a_failed_photo_records_nothing` |
| A6 | a delivered photo is recorded once and a second raise of its key is `already_recorded` | `a_delivered_photo_is_recorded_once` |
| A7 | a photo asks T2 and spends no T4 or T5 budget | `a_photo_spends_no_celebration_budget` |
| A8 | a photo with its kind's setting off is withheld `nudges_disabled` and never pushed | `a_photo_with_its_kind_off_is_never_pushed` |
| A9 | a photo of 10,000,001 bytes, of width plus height 10,001 pixels, of a ratio over 20, or with a caption of 1,025 characters is refused `photo_invalid` | `an_oversized_photo_is_refused_before_any_call` |
| A10 | `prepare_share` records no delivery and answers the transport's outcome | `a_prepared_share_records_no_delivery` |
| A11 | `Reason::ALL` holds `photo_unsupported` | `every_reason_is_named` |
| A12 | the bot's `push_photo` is one `sendPhoto` to the owner's chat and answers the largest size's file id | `push_photo_is_one_send_photo_to_the_owner` |
| A13 | the bot's `prepare_share` is one `savePreparedInlineMessage` naming the file id | `prepare_share_is_one_prepared_inline_message` |
| A14 | a refused or unreachable Bot API answers `Failed` from both calls, with no retry | `a_refused_photo_or_share_answers_failed` |
| A15 | the policy's bot transport list and the census hold `push_photo` and `prepare_share` | `the_policy_names_every_bot_call` |
| A16 | a caption is counted in UTF-16 units: 512 characters outside the BMP (1,024 units) are taken, 513 (1,026) are refused `photo_invalid`, and a combining sequence counts each of its units | `a_caption_is_counted_in_utf16_units` |

```acceptance
A1: cargo test -p deck-streak-notifications --test photo_render -- --exact a_transport_without_push_photo_answers_unsupported
A2: cargo test -p deck-streak-notifications --test photo_render -- --exact a_transport_without_prepare_share_answers_unsupported
A3: cargo test -p deck-streak-notifications --test photo_render -- --exact an_unsupported_photo_is_withheld_by_name_and_sends_nothing
A4: cargo test -p deck-streak-notifications --test photo_render -- --exact a_photo_in_quiet_hours_is_not_held
A5: cargo test -p deck-streak-notifications --test photo_render -- --exact a_failed_photo_records_nothing
A6: cargo test -p deck-streak-notifications --test photo_render -- --exact a_delivered_photo_is_recorded_once
A7: cargo test -p deck-streak-notifications --test photo_render -- --exact a_photo_spends_no_celebration_budget
A8: cargo test -p deck-streak-notifications --test photo_render -- --exact a_photo_with_its_kind_off_is_never_pushed
A9: cargo test -p deck-streak-notifications --test photo_render -- --exact an_oversized_photo_is_refused_before_any_call
A10: cargo test -p deck-streak-notifications --test photo_render -- --exact a_prepared_share_records_no_delivery
A11: cargo test -p deck-streak-notifications --test photo_render -- --exact every_reason_is_named
A12: cargo test -p deck-streak-bot --test photo_transport -- --exact push_photo_is_one_send_photo_to_the_owner
A13: cargo test -p deck-streak-bot --test photo_transport -- --exact prepare_share_is_one_prepared_inline_message
A14: cargo test -p deck-streak-bot --test photo_transport -- --exact a_refused_photo_or_share_answers_failed
A15: cargo test -p deck-streak-notifications --test one_router -- --exact the_policy_names_every_bot_call
A16: cargo test -p deck-streak-notifications --test photo_render -- --exact a_caption_is_counted_in_utf16_units
```

## 3a. What the box run judges

The private-wiring change that enforces these rows is none: the notifications-policy and
telegram-platform packs are already enforced for DeckStreak, and this delivery widens the population
each judges. The delivery hands back an empty JSON diff and says so.

| id | criterion | decided by |
|---|---|---|
| B1 | over `notifications-policy.json` and `crates/notifications/src/router.rs`: every bot call the policy lists is made from the router module alone, `push_photo` and `prepare_share` among them | the notifications-policy pack |
| B2 | over `crates/bot/src/transport.rs`: the photo and the prepared message stay within the Bot API's limits and name the owner's chat and user only | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/notifications/src/transport.rs` | `deck-streak-notifications` | changed: `push_photo` and `prepare_share` with their refusing defaults, `PhotoPushed`, `Prepared` |
| `crates/notifications/src/photo.rs` | `deck-streak-notifications` | added: `Photo`, `FileId` and their bounds |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: `route_photo`, `prepare_share`, `photo_unsupported` |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the module |
| `crates/notifications/tests/photo_render.rs` | `deck-streak-notifications` | added: A1 to A11 |
| `crates/notifications/tests/photo_jpeg.rs` | `deck-streak-notifications` | added: the JPEG size reader's paths, for the mutation gate |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: A15 |
| `crates/notifications/tests/ladder_policy.rs` | `deck-streak-notifications` | changed: the port's call list includes the two new calls |
| `crates/notifications/tests/policy.rs` | `deck-streak-notifications` | changed: the required withhold reasons include `photo_unsupported` |
| `notifications-policy.json` | repo | changed: `push_photo` and `prepare_share` |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: the two calls on the Bot API |
| `crates/bot/tests/photo_transport.rs` | `deck-streak-bot` | added: A12 to A14 |
| `crates/bot/tests/support/fake_bot_api.rs` | `deck-streak-bot` | changed: answers the two methods, the photo's sizes ordered by area and by sides differently |
| `docs/specs/SPEC-132-the-router-sends-a-photo-and-prepares-a-share-through-two-additive-transport-calls-that-refuse-by-default.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/notification-router.md` | docs | changed: the census line names the two new calls |
| `docs/red-first/SPEC-132.md` | docs | added |
| `docs/decisions/ADR-135-images-are-drawn-through-a-port-with-no-provider-wired-by-a-sending-job-capped-cached-and-gated.md` | docs | changed: the withhold reason, its rejected option and a delivered-so-far line; it stays proposed until SPEC-135 delivers the rest |
| `docs/decisions/ADR-136-a-share-is-a-prepared-inline-message-the-owner-sends-from-telegrams-share-sheet.md` | docs | changed: a delivered-so-far line; it stays proposed until SPEC-136 delivers the route and the client |
| `scripts/mutation-rows.d/S13200-S13299.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It generates no image and raises no photo occasion: the keepsake and the share card do (#126,
  #125).
- It holds no photo in the deferral queue: a caller raises it again (#126).
- It adds no photo to the Mini App's in-app feed; the gallery shows images (#125).
- It changes no tier of the ladder's table (#120).
- It does not route a share photo through the ladder's tier (#125).

## 6. Risks

- **A success that never happened.** R1's refusing default and R6's named withhold; detected by A1
  to A3.
- **A photo that bypasses quiet hours.** R4 runs the router's order; detected by A4.
- **A photo that spends a fanfare budget.** R5; detected by A7.
- **A caller that raises a photo forever.** SPEC-135 R9 bounds the caller; this SPEC records nothing
  for `NotNow`, detected by A4 and A5.

## 7. Parity goldens

None. The predecessor's photo send is a Bot API call with no number to prove; the tiers are
SPEC-084's goldens.

## 8. Tables and the v9 import

None. The deliveries go into SPEC-041's decision ledger.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13201-PHOTO-DEFAULT-REFUSES` | `crates/notifications/src/transport.rs` | `push_photo`'s default answers `Unsupported` | `photo_render::a_transport_without_push_photo_answers_unsupported` |
| `S13202-SHARE-DEFAULT-REFUSES` | `crates/notifications/src/transport.rs` | `prepare_share`'s default answers `Unsupported` | `photo_render::a_transport_without_prepare_share_answers_unsupported` |
| `S13203-UNSUPPORTED-WITHHELD` | `crates/notifications/src/router.rs` | `Unsupported` is recorded withheld by name | `photo_render::an_unsupported_photo_is_withheld_by_name_and_sends_nothing` |
| `S13204-QUIET-NOT-HELD` | `crates/notifications/src/router.rs` | quiet hours answer `NotNow` without a hold | `photo_render::a_photo_in_quiet_hours_is_not_held` |
| `S13205-FAILED-NOT-RECORDED` | `crates/notifications/src/router.rs` | a failed push records nothing | `photo_render::a_failed_photo_records_nothing` |
| `S13206-DEDUPE` | `crates/notifications/src/router.rs` | a delivered key is `already_recorded` | `photo_render::a_delivered_photo_is_recorded_once` |
| `S13207-T2` | `crates/notifications/src/router.rs` | a photo asks T2 | `photo_render::a_photo_spends_no_celebration_budget` |
| `S13208-SIZE-BOUND` | `crates/notifications/src/photo.rs` | 10,000,000 bytes; the test names 10,000,000 and 10,000,001 | `photo_render::an_oversized_photo_is_refused_before_any_call` |
| `S13209-SHARE-NO-RECORD` | `crates/notifications/src/router.rs` | a prepared share records no delivery | `photo_render::a_prepared_share_records_no_delivery` |
| `S13210-LARGEST-SIZE` | `crates/bot/src/transport.rs` | the file id of the largest size | `photo_transport::push_photo_is_one_send_photo_to_the_owner` |
