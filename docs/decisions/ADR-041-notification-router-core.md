---
status: accepted
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

- Good, because the bot and the Mini App are held to one router from the first delivery, each path
  by a guard that says what it reads: the compiler holds the port's calls; SPEC-041 A15's census
  holds a delivery around the port in every shipped source of the kinds it reads (the Rust, Python
  and web sources, the shell scripts by extension or `#!` first line, and the systemd units and
  their drop-ins); and the one-router row holds the calls the policy names.
- Good, because the readings line is honest about its kind and can be switched off on its own.
- Bad, because a celebration raised by a background job while the app is open still arrives as a
  bot message; the decision ledger shows which surface was chosen.
- Bad, because the lapse context is only as good as the caller's lapse source; the kinds that
  depend on it name the governor as a prerequisite.
- Bad, because a text census reads names, not requests: a request whose URL or method is
  assembled from parts, so that neither the Bot API's host nor a send method's name appears; a
  write to the held queue from outside the router's modules, which a flush would deliver; and a
  source of a kind the census does not read, all go unread (#297).

### Confirmation

SPEC-041's tests in the gate, A2's compile-fail cases and A15's census among them; the
notifications-policy pack's rows (`one-router`, `policy-deviation-has-adr`) in the box run
(SPEC-041 §3a, ADR-069); and `notifications-policy.json`'s `deviations` entry citing this ADR.

### Decided at delivery (SPEC-041 §7)

The delivery decided each question the SPEC left open against its alternatives:

- **The port's calls by type, and a delivery around the port by a census of what it reads.** Each
  bot transport call takes a `Pass` that no other module can make, not by its field, `Default`, or
  a clone of a borrowed one (SPEC-041 A2, the compiler), and `push_in_app` is private to the router
  module. A delivery that never calls the port is refused by A15's census, which reads every
  shipped source of the kinds it names: outside the bot's sources nothing names the Bot API's host,
  a send method or the bot's `DEFAULT_API_URL` (private to the bot's crate), SPEC-031's alert path
  aside; inside them a send method is named only in the named send that makes its request; the
  bot's `send_html`, `edit_html` and command handler are used only at named call sites (`OwnerChat`,
  the command replies (#257) and the transport's own requests; none for `edit_html`; the long poll
  for the handler); and only the router's modules name the Mini App's feed. The box run's
  `one-router` row refuses a call the policy names (§3a B1). Chosen against holding the rule by that
  row alone, which matches only the names the policy lists and finds a stray call only after it is
  written; against a private trait, which the bot could not implement; and against parsing the
  sources with a Rust parser, a new dependency, since a census that leaves out only
  `#[cfg(test)]` modules fails closed.
- **The policy is compiled into the binary and parsed at start.** Chosen against reading a deployed
  copy of the file, which could drift from the one the box run judged, and against typing the values
  as constants, which would be a second policy the pack never reads.
- **A decision is one write.** The key is claimed by inserting its delivery at the dedupe rule, and a
  later withhold releases it in the same `BEGIN IMMEDIATE` write, so the unique index, not a read
  before the write, is what holds a key to one delivery across both surfaces. A bot send commits
  its claim before the call. Chosen against claiming after the send, which two surfaces raising one
  event at once could both pass.
- **A failed send holds only a celebration.** Anything else that fails is withheld with `no_notifier`
  and its claim released, so the scheduler's catch-up can send it again; the breaker makes every
  bot send wait 60 seconds after a failure. Chosen against queueing nudges beside celebrations,
  which the flush would render as held celebrations, and against keeping a failed nudge's claim,
  which would silence the catch-up's retry.
- **Until the ladder, a celebration renders as a line, or as nothing at T0.** Chosen against
  rendering T1 as nothing, which would drop it silently, and against holding tiers the router
  cannot render yet, which would fill the queue with holds no flush could deliver.
- **The queue's bound is kept at deferral.** Past 20 held celebrations the lowest-ranked is
  abandoned and waits to be named, so the queue never holds more than 20. Chosen against letting the
  table grow until a flush trims it, as the predecessor did, which holds its bound only at the
  flush.
- **An abandonment is a withhold under the policy's reasons:** `quiet_hours` for a quiet hold,
  `no_notifier` for a failed send's. Chosen against new reasons outside the policy's vocabulary,
  which the pack's withhold ledger would not recognise.
- **The owner's quiet window and switches are rows of `notification_settings`:** the predecessor's
  `quiet_start_min` and `quiet_end_min`, and a switch off at `"0"`. Chosen against new names, which
  the import of the predecessor's settings would have to translate.

## What would make this wrong

- The owner reports celebrations arriving in the bot while the app is open often enough to matter
  (then a presence rule earns its constant).
- The engagement wave's morning brief carries the readings line itself (then `reading_ready` is
  retired and the deviation removed).

## More Information

SPEC-041; SPEC-049, SPEC-050 and SPEC-052 (the W1 kinds routed through it); the notifications-policy
pack's "Changing the policy" section; docs/schematics/streaks-and-governor-state-machine.md for the
lapse id.
