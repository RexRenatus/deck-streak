---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A share is a prepared inline message that the owner sends from Telegram's own share sheet

## Context and Problem Statement

#125 asks that the owner can share a streak milestone's card. The card is a photo the bot has
already sent to the owner's chat, so Telegram holds it by a file id. Telegram offers several ways
for a Mini App to hand content onward, and DeckStreak sends only to the owner's own chat through
the one router (ADR-041). How does a card leave that chat, and who chooses where it goes?

## Decision Drivers

- The owner chooses every recipient; DeckStreak never posts to a chat on its own.
- No image is served outside the owner's session or from a public path (SPEC-136).
- The transport keeps one port with refusing defaults for what a transport cannot do (ADR-041).
- A client too old for the call shows nothing it cannot do.

## Considered Options (the alternatives it was chosen against)

- The Bot API's `savePreparedInlineMessage` with the card's file id, then the Mini App's
  `shareMessage`: chosen, because Telegram's own sheet asks the owner for each recipient, and the
  photo is referenced by the id Telegram already holds.
- `shareToStory`: rejected because it needs the image at a public URL, and no image is served from
  a public path.
- `downloadFile`: rejected because it saves a file to the device rather than sharing it, and it also
  needs a URL the client fetches.
- The Web Share API from the page: rejected because its support varies across Telegram's webviews,
  and it would carry the bytes through the page instead of the id Telegram holds.
- The bot posts the card to a chat the owner names: rejected because the bot would need rights in
  that chat, and the owner's own chat is the only chat the router sends to.

## Decision Outcome

Chosen option: "a prepared inline message sent through `shareMessage`", because it is the only
option where the owner picks every recipient in Telegram's own interface and nothing becomes
public.

- **The transport.** `BotTransport` gains `prepare_share`, additive with a refusing default
  (ADR-041), answering a prepared message's id, unsupported, or failed; the bot's transport makes
  one `savePreparedInlineMessage` call.
- **The route.** `POST /api/images/{key}/share` behind the owner's session and the CSRF bound, at
  most 10 a minute, answers the id or `share_unavailable` by name.
- **The client.** The share control appears only on Bot API 8.0 or later, and only for a card with
  a file id.

### Consequences

- Good, because a share needs no public URL and adds no chat the bot writes to.
- Bad, because an older Telegram client shows no share control at all.

### Confirmation

SPEC-132 §3 (the transport's two additive calls and their defaults) and SPEC-136 §3 (the share
route and the control), with their rows in `S13200-S13299` and `S13600-S13699`.

## What would make this wrong

- Telegram retires prepared inline messages, or makes them expire before the owner taps: the route
  would prepare on each tap, as it already does.
- The owner wants a public link to a card: that is publishing's decision (ADR-137), with its allow
  list and its withdrawal.

## More Information

SPEC-132, SPEC-136, ADR-041, ADR-135, and the W7 schematic
`docs/schematics/w7-image-pipeline-and-its-no-provider-path.md`.
