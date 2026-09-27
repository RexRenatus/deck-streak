---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The router core: the origin picks the surface, the Mini App pulls a feed, the caller supplies the lapse, and the readings line is a new nudge kind

## Context and Problem Statement

`notifications-policy.json` fixes the policy's values and names the one router
(`crates/notifications/src/router.rs`, `route`). Four things it leaves open must be decided before
the first W1 message is sent: which surface an occasion goes to, how the Mini App receives what is
routed to it, where the router learns that the owner is in a lapse, and which kind the morning
readings line is. The last one changes the policy, so the notifications-policy pack requires an
ADR that names the changed key.

## Decision Drivers

- One router, so an event raised on both surfaces is delivered once (constraint 2).
- The crate graph: notifications depends on the kernel only, so it cannot ask the governor
  (streaks) for the lapse itself.
- No new constant without a source; the predecessor had no Mini App, so it has no presence rule.
- During side by side, DeckStreak sends only kinds the predecessor does not send (ADR-011), and the
  predecessor still sends its own morning brief.

## Considered Options (the alternatives it was chosen against)

- The occasion's origin picks the surface (a celebration raised by a Mini App request renders in the app, everything else goes to the bot), the Mini App pulls an `in_app_feed` the router appends to, the caller passes the open lapse id in each occasion, and the readings line is a new nudge kind `reading_ready` recorded as the deviation `kinds.reading_ready` — chosen: no new constant, no new edge, no second morning message.
- Pick the surface from a presence heartbeat the Mini App sends while it is open — rejected because it needs a heartbeat interval and an expiry with no source in the predecessor or the packs, and a timer in the Telegram webview.
- Deliver every occasion to both surfaces — rejected because it is exactly the double celebration the charter forbids.
- Push in-app occasions over a server-sent event stream or a WebSocket — rejected because a long-lived connection through Caddy and the webview serves one owner who opens the app a few times a day; the pulled feed is enough, and a push can be added behind the same transport call.
- Let the router ask the governor for the lapse — rejected because it needs a notifications-to-streaks edge the context map does not have; coordination already calls both.
- Send the readings line as the predecessor's `morning` kind — rejected because the predecessor still sends its own morning brief during side by side, `morning` is in the holdout, and one `morning` per study day would let the two collide.

## Decision Outcome

Chosen option. The surface is the origin for celebrations raised by a Mini App request and the bot
otherwise. `push_in_app` appends to `in_app_feed` inside the router module, and
`GET /api/notifications/feed` serves it to the owner. Each occasion carries a lapse context that
coordination fills from the governor's lapse episode. A kind whose budget is `comeback` is exempt
from the lapse's suppression, because it exists only inside a lapse. The policy gains
`kinds.reading_ready` (class `nudge`, tier T2, no budget, dedupe per study day, setting
`reading_ready_enabled`), so quiet hours withhold it and a lapse suppresses it like every nudge, and
the holdout does not draw it. W1 also carries the policy's deferral bounds and failed-send hold,
because a W1 celebration that failed to send would otherwise vanish silently; the ladder's tier
budgets and the holdout wait for the engagement wave.

### Consequences

- Good, because the one-router row can hold the bot and the Mini App to one call site from the first
  delivery.
- Good, because the readings line is honest about its kind and can be switched off on its own.
- Bad, because a celebration raised by a background job while the app is open still arrives as a
  bot message; the decision ledger shows which surface was chosen.
- Bad, because the lapse context is only as good as the caller's lapse source; the kinds that
  depend on it name the governor as a prerequisite.

### Confirmation

SPEC-041's tests and the notifications-policy pack's rows (`one-router`, `policy-deviation-has-adr`)
in the gate; `notifications-policy.json`'s `deviations` entry citing this ADR.

## What would make this wrong

- The owner reports celebrations arriving in the bot while the app is open often enough to matter
  (then a presence rule earns its constant).
- The engagement wave's morning brief carries the readings line itself (then `reading_ready` is
  retired and the deviation removed).

## More Information

SPEC-041; SPEC-049, SPEC-050 and SPEC-052 (the W1 kinds routed through it); the notifications-policy
pack's "Changing the policy" section; docs/schematics/streaks-and-governor-state-machine.md for the
lapse id.
