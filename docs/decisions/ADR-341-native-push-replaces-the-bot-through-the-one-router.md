---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Native push replaces the bot, through the one router: opt-in, and at most one nudge a day

## Context and Problem Statement

The owner chose to retire the Mini App and the bot once the web client and the iPhone and iPad
client exist (the owner's answer SCP-03), and to reach the owner outside the app by opt-in push,
with at most one nudge a day (NOTIF-01). Today the one router (ADR-041) has one outbound port,
`BotTransport` (`crates/notifications/src/transport.rs:84`), and the celebration reveal (ADR-084),
the evening check-in (ADR-100) and the widget (ADR-109) are all shaped for Telegram. CHARTER 2
names two surfaces, and the design system's first two principles are "Native to Telegram" and
"Calm by default". The owner's signed ruling,
`docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`, amends all three. How do nudges and
celebrations reach the owner once Telegram retires, without a second policy?

## Decision Drivers

- One router, one policy: every celebration and nudge passes through ONE router, so an event
  never celebrates twice (CHARTER 2 as amended).
- Push is opt-in on every device, and the owner can revoke any one device.
- At most one nudge a day, across every transport.
- Vivid in a session, calm outside it; every CHARTER 10 anti-goal holds.
- The Telegram surfaces keep working until the new transports carry the router, and the cutover
  from the predecessor runs on them.

## Considered Options (the alternatives it was chosen against)

- APNs and web push as the one router's transports, opt-in per device, at most one nudge a day, with the Telegram surfaces retired only after they carry the router — chosen because the owner chose it (SCP-03, NOTIF-01), and the router keeps its one decision for every surface.
- Keep both the Mini App and the bot beside the new clients — rejected because the owner chose to retire both.
- Retire the Mini App and keep the bot as the push channel — rejected because the owner chose native push over the bot.
- No push, celebrations and nudges inside the app only — rejected because a nudge must reach the owner outside the app to matter, and the owner chose opt-in push.

## Decision Outcome

Proposed option: native push through the one router.

- **One router, more transports.** The router's outbound port generalises from the bot's to a
  transport per surface: Telegram while it exists, APNs for iPhone and iPad, and web push for the
  web client. The router decides once per event, with its dedupe, budgets, quiet hours and lapse
  rule unchanged, and then hands the decision to the transports the owner has enabled.
- **Opt-in and revocable.** Each device asks for push permission only after the owner turns
  nudges or celebrations on, and registers its device token with the service. A token is stored
  per device, removable by the owner from any signed-in surface, and exported and erased with the
  rest of the owner's data (CHARTER 13).
- **At most one nudge a day.** The nudge budget counts across every transport, so a nudge that
  reaches the phone is the day's nudge on the web as well.
- **In a session, and outside it.** In a session, celebrations render in the client: haptics,
  motion, sound, instant XP and the ladder. Outside a session, nothing pressures, shames or
  fabricates.
- **The credentials.** The APNs key and the web push keys are the owner's to place; they reach
  the service as credentials and never enter this repository.
- **The retirement.** The Mini App and the bot retire only after native push carries the router,
  and after the cutover from the predecessor, which runs on them. The retirement is its own
  delivery, with its own ADR.
- **What this ADR carries.** The app-surfaces ruling's amendments of CHARTER 2 (one router, one
  policy) and of the design system's first two principles (an identity of its own; vivid in
  session, calm out of session).

### Consequences

- Good, because the owner keeps nudges and celebrations after Telegram retires, still under one
  policy.
- Good, because a nudge budget across transports means more surfaces never mean more nudges.
- Bad, because the router gains two transports, each with its own failure modes and its own
  credentials to rotate.
- Bad, because web push on the iPhone and iPad depends on the web app being installed to the
  Home Screen.

### Confirmation

- The router's tests: one decision per event across every enabled transport, and the nudge budget
  counted across them.
- The push spike's record: a push received from the service by the app and by the web client,
  against fakes in CI and on a device in the owner's session.
- A device token's export and erase in the data-rights census.

## What would make this wrong

- A platform refuses a push the router sent without telling the sender, so a nudge would count
  against the budget without arriving; the transports must report failure for the router's ledger.
- The owner wants a second channel per event, which would need a ruling on CHARTER 2's one
  router.

## More Information

- SPEC-334 (R12, R14; stretch row 2.3).
- `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md` (CHARTER 2; the design system's
  principles 1 and 2).
- ADR-041, ADR-084, ADR-100, ADR-109.
