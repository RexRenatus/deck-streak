---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A requested duty answers as its command's reply, and only a prepared duty raises an occasion

## Context and Problem Statement

W6 adds AI duties of two shapes. Some answer a request the owner just made: the writing tutor's
corrections, the conversation partner's next turn, a leech remedy, `/vaultops`. Others finish on a
schedule and may tell the owner so: a drill is minted, the inbox is filed. The W6 plan binds every
message to the ONE router (SPEC-041), whose rules withhold a nudge during quiet hours and while a
lapse is open, and whose census (`crates/notifications/tests/one_router.rs`) lists every command
reply the bot may send. A run takes up to minutes (SPEC-043 R6), so a requested answer cannot be
the handler's immediate reply. How does each shape reach the owner, without a second delivery path?

## Decision Drivers

- One delivery path: every bot send is the router's or a command reply the census names.
- A reply the owner asked for must not be withheld by quiet hours or an open lapse, which exist to
  stop unrequested pings.
- A bot update loop must not stall for minutes on one command (SPEC-026's loop).
- Bounded memory: each agent run is a process, and requests must not start them without limit.

## Considered Options (the alternatives it was chosen against)

- A requested duty answers as its command's reply: a placeholder at once, then one edit of that
  message with the result; a prepared duty's ping is a router occasion of its own kind: chosen,
  because both stay on the paths the census and the router already hold, and each gets the rules
  its shape needs.
- Deliver a requested answer as a router nudge: rejected because quiet hours and an open lapse
  would withhold a reply the owner is waiting for, and a per-day dedupe would drop a second answer.
- Deliver it as a router alert: rejected because an alert is for the product's failures, and it
  would page the owner with study text as if something had broken.
- Hold the handler until the run ends and reply once: rejected because the update loop would stall
  for minutes and every other command would wait.
- A delivery path of the agent's own that sends the result: rejected because it is the second path
  SPEC-041 forbids, and the census would refuse it.

## Decision Outcome

Chosen option: "a requested duty answers as its command's reply; a prepared duty's ping is a router
occasion", because it keeps one path and gives each shape its own rules.

- **A requested duty.** The command replies at once with a placeholder. The run happens off the
  update loop, and its result, a refusal line or the not-enabled line replaces the placeholder by
  one edit. The placeholder and the edit are command replies the census names (the edit is the
  first call site of the transport's `edit_html`). An API request answers in its response.
- **One at a time.** Each process runs at most one requested run per duty; a second request while
  one runs is answered at once that one is running. The bot and the API are separate processes, so
  at most one run per duty per surface.
- **A prepared duty.** Its ping is an occasion of a declared kind (class `nudge`, tier T2, no
  budget, dedupe per study day, its own setting), so quiet hours, a lapse and the setting apply.
  Each new kind's `deviations` entry names this ADR.
- **With the route absent** a requested duty's placeholder is skipped: the reply is the
  not-enabled line (ADR-054), and a prepared duty raises nothing, because nothing was prepared. A
  requested pass that holds a duty needing no model (the vault pass, SPEC-117) still runs, and its
  model duties record their absent route.

### Consequences

- Good, because no reply the owner asked for is lost to quiet hours, and no unrequested ping escapes
  them.
- Good, because the census still names every send, and a new duty's reply is a census change a
  reviewer sees.
- Bad, because an edit of an old placeholder can fail (the message was deleted). The run's result
  is then sent once as a fresh command reply, which the census also names.

### Confirmation

SPEC-113's A10 to A13, SPEC-117's placeholder and edit criteria, and SPEC-111's and SPEC-116's
occasion criteria.

## What would make this wrong

- A requested duty whose answer takes longer than the owner will wait: a ping when it is ready would
  be needed, and would be a router occasion under this ADR's second arm.
- A surface other than the bot and the Mini App: its reply path would need its own census entry.

## More Information

SPEC-041 (the router and its census), SPEC-026 (the update loop), SPEC-043 (the runner), ADR-054,
and the W6 schematic `docs/schematics/w6-duty-run-and-its-degradation.md`.
