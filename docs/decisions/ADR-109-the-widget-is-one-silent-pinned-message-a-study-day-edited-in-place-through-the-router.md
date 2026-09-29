---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# The widget is one silent pinned message a study day, edited in place through the router

## Context and Problem Statement

The predecessor keeps one pinned message a day in the owner's chat: the streak or the strength, a
mood, the day's progress bar, the quest lines and a footer of the day's buffs, tokens, wager and
weekly quest (`pipeline_layers/showcase.py:ShowcaseLayer._update_widget`, predecessor `27ee2bc`). It
sends it silently at the day's first refresh, pins it, unpins the day before's, and edits it in
place whenever its text changes; a failed edit sends and pins it anew. The bot sends, edits and
pins it by itself, beside the celebration path, and a T5 pin is followed by a re-pin so the widget
stays the newest pin. #121 keeps the pinned message and asks that its button open the Mini App.

In DeckStreak every message goes through one router (SPEC-041): `route` is the one entry point, the
policy's delivery calls are made only in the router module, and a census refuses the bot's own
send or edit anywhere else. The notifications policy admits four classes, celebration, nudge, digest
and alert, and quiet hours hold every class but the alert. The widget is no celebration and no
nudge, it edits a message the router sent, and DeckStreak reads the collection once a study day, at
04:07, inside the quiet window (ADR-037). How is the widget sent, kept and refreshed?

## Decision Drivers

- One router, and one place that sends, edits or pins a message (SPEC-041).
- The owner is never woken by the widget, and no second message a day is posted for it.
- The texts and the calls equal goldens of the predecessor's own functions (ADR-012).
- The policy's classes and its quiet-hours exemption are the pack's, and no new one is invented.
- A refresh must move the mood with the owner's day, not only with the one scheduled sync.

## Considered Options (the alternatives it was chosen against)

- One message a study day, the kind `widget` of class `digest`, sent silently with an Open app button, pinned, the day before's unpinned, edited in place when its text changes, every call in the router module, refreshed after each sync cycle and hourly — chosen, because it keeps the predecessor's one pinned message and its calls, and adds no delivery path, class or exemption.
- A new message at every refresh — rejected because the chat would gain a message an hour and a pin notice with each, where the predecessor edits one message in place.
- The bot sends, edits and pins the widget by itself, as the predecessor does — rejected because SPEC-041 keeps every delivery call in the router module and its census refuses the bot's own send or edit elsewhere, and the widget would escape the switch, quiet hours and the ledger.
- A new class for ambient messages, exempt from quiet hours — rejected because the policy admits four classes and exempts only the alert, so the pack would refuse both the class and the exemption.
- Refresh only after a sync — rejected because the one scheduled sync runs inside the quiet window (ADR-037), so the widget would never show the evening's mood or the hour's footer.
- A slim widget of the streak and the button alone — rejected because #121's criteria assert every mood branch, the quest lines and the footer.
- No widget, the Mini App's home screen only — rejected because #121 keeps a pinned message whose button opens the app.

## Decision Outcome

Chosen option: "one message a study day, the kind `widget`, edited in place through the router".

- The kind `widget` is class `digest`, tier T2, one a study day, behind its own switch
  `widget_enabled`, recorded in the policy's deviations with this ADR.
- A refresh is one occasion of the kind `widget`, routed by `route`, which stays the one entry point
  (SPEC-041 R1). Its arm for the kind compares the text's sha256 with the stored one: an unchanged
  text pushes and records nothing. Otherwise the switch, quiet hours and the transport decide as
  they do for every kind, and a withhold is recorded.
- The day's first send claims the kind for the study day, sends silently with the Open app row (the
  token `today`), pins the message, unpins the day before's widget and stores the message id and the
  sha256 in `widget_messages`. A later change edits that message with the same row; "message is not
  modified" counts as edited, and a failed edit sends and pins anew.
- After a T5 pins its message, the router re-pins the day's widget, as the predecessor's
  `_repin_widget` does.
- The sync cycle refreshes the widget after its recompute, and the job `widget_refresh` refreshes
  it hourly at minute 44, without catch-up.
- The transport gains `push_widget`, `push_edit` and `push_unpin`, named in the policy's
  `router.transport` and called only in the router module.

### Consequences

- Good, because the chat holds one widget a day, edited in place and never announced by a sound.
- Good, because the switch, quiet hours, the outage breaker and the decision ledger apply to the
  widget as to every message.
- Good, because the mood follows the owner's evening, hour by hour, without a second sync.
- Bad, because quiet hours hold the widget: the new day's widget first appears after the window
  ends, and an owner's sync late at night leaves the widget as it was until the morning.
- Bad, because an hourly refresh inside quiet hours or while the switch is off records a withheld
  decision each time its text changes.
- Bad, because the policy's delivery calls grow from the five SPEC-041 R1 counts to eight, and
  `route` gains an arm for one kind.

### Confirmation

SPEC-102's A11 to A19, A22, A23 and A34.

## What would make this wrong

- The owner wants the widget to move at night: that needs a silent class the pack exempts from quiet
  hours, which is a pack change before it is a code change.
- Telegram stops letting a bot edit or pin its own messages in a private chat: the widget would then
  become a message a day without edits, or the Mini App's home screen alone.
- DeckStreak reads the collection more than once a study day: the hourly job would then add little
  and could go.

## More Information

Cites SPEC-041 (the router core and its census), ADR-041 (the router core), ADR-037 (one scheduled
sync a study day), ADR-052 (deep links are URL buttons), ADR-011 and ADR-027 (the job minutes),
ADR-012 (the parity oracle), SPEC-084 (the ladder's T5 pin) and SPEC-100 (the button rows). SPEC-102
builds it.
