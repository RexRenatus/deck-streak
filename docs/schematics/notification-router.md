# Schematic: the notification router's decision

Kind: data flow and decision order, a held celebration's state machine, and the joins. Read at
DeckStreak `main` ce3683d (`notifications-policy.json`, ADR-011,
docs/schematics/streaks-and-governor-state-machine.md), at the predecessor's `27ee2bc`
(`quiet_hours.py:in_quiet_hours`, `pipeline_layers/celebrations.py:CelebrationsLayer.celebrate` and
`flush_deferred_celebrations`), and at the packs vendored from `19bb0f3` (notifications-policy: the
decision ledger, the withhold reasons, `one-router`). Added by SPEC-041, and brought to the design it
delivered at `dev` f5322b2 (SPEC-041 §7; ADR-041's decisions at delivery).

## The decision

```mermaid
flowchart TD
  occ["occasion: kind, dedupe key, origin, tier, text, study day, lapse context"] --> route{{"route (crates/notifications/src/router.rs), inside one write"}}
  route --> s1{"kind's setting at 0?"}
  s1 -- "yes" --> w1["withhold: nudges_disabled"]
  s1 -- "no" --> s2{"claim the key in the kind's dedupe scope: inserted?"}
  s2 -- "no: a delivery holds it" --> w2["withhold: already_recorded"]
  s2 -- "yes" --> s3{"a nudge in an open lapse, and not a comeback-budget kind?"}
  s3 -- "yes" --> w3["withhold: lapse"]
  s3 -- "no" --> s4{"inside the quiet window (23:00 to 07:30, or the owner's)?"}
  s4 -- "celebration" --> defer["defer: held, reason quiet"]
  s4 -- "nudge or digest" --> w4["withhold: quiet_hours"]
  s4 -- "alert, or outside" --> s5{"a comeback past 3 per lapse, or within 3 study days of the last?"}
  s5 -- "yes" --> w5["withhold: budget_spent"]
  s5 -- "no" --> s6{"surface: a celebration from a Mini App request goes in-app, all else to the bot"}
  s6 -- "mini-app" --> app["send: push_in_app appends the feed item in the same write"]
  s6 -- "bot" --> s7{"a bot transport joined, and the breaker closed?"}
  s7 -- "no, a celebration with the breaker open" --> held["defer: held, reason send"]
  s7 -- "no" --> w6["withhold: no_notifier"]
  s7 -- "yes" --> bot["commit the claim, then push_message with the router's Pass"]
  bot -- "failed, a celebration" --> fail["defer: held, reason send, tries 1; breaker opens"]
  bot -- "failed, anything else" --> release["withhold: no_notifier; the claim released; breaker opens"]
  bot -- "delivered" --> sent["send"]
  w1 --> ledger[("decision ledger: a withhold's kind with :withheld appended")]
  w2 --> ledger
  w3 --> ledger
  w4 --> ledger
  w5 --> ledger
  w6 --> ledger
  defer --> ledger
  held --> ledger
  fail --> ledger
  release --> ledger
  app --> ledger
  sent --> ledger
```

A withhold after rule 2 deletes the claim inside the same write, so a key is claimed exactly when
it was sent or held. Three guards hold every delivery to the router, each on its own path:

- **The port's call, by the compiler (SPEC-041 A2).** Only the router module can make a `Pass`, and
  each bot transport call takes one, so a call of the port outside the module does not compile: the
  pass has a private field, no `Default` and no `Clone`. `push_in_app` is private to the module.
- **A delivery around the port, by a census (A15).** A source that never calls the port could still
  reach the owner through the bot's own `send_html` or a raw request to the Bot API. A census of
  every shipped source refuses both: outside `crates/bot/` nothing names the Bot API's host or a
  send method, SPEC-031's alert path aside, and the bot's sends are called only at named call
  sites: by `OwnerChat`, by the command replies and inside the transport's own requests.
- **A call the policy names, by the box run (§3a B1).** The `one-router` row refuses a call of
  `push_message`, `push_dice`, `push_reaction`, `push_pin` or `push_in_app` outside `router.rs`.

## A held celebration

```mermaid
stateDiagram-v2
  [*] --> held: deferred in quiet hours, or held after a failed send
  held --> held: a flush's render or recap fails, tries at most 2 (the first deferral time kept)
  held --> delivered: a flush renders it in full (at most 2), or its recap line is delivered
  held --> abandoned: older than 720 minutes at a flush
  held --> abandoned: a failed send past 2 retries
  held --> abandoned: the lowest-ranked when a 21st is deferred
  abandoned --> named: a recap line naming it is delivered
  delivered --> [*]
  named --> [*]
```

A flush does nothing without a bot transport, inside the quiet window, or while the breaker is
open; its first failed render ends it. Each render, re-hold and abandonment is a decision in the
ledger; an abandonment is withheld with `quiet_hours` or `no_notifier`, by what held it.

## The joins

```mermaid
flowchart LR
  subgraph notifications["deck-streak-notifications"]
    policy["policy.rs: the file, compiled in and parsed at start"]
    router["router.rs: route, flush, push_in_app, Pass"]
    ledger2["ledger.rs: decisions, deliveries, queue, feed, settings"]
    port["transport.rs: BotTransport"]
  end
  bot["deck-streak-bot: transport.rs implements BotTransport on its Transport"] --> port
  wiring["deck-streak-daemon wiring.rs: the router over the bot's transport"] --> router
  cycle["deck-streak-coordination sync_cycle.rs: flush after a sync that succeeded"] --> router
  api["deck-streak-api notifications_routes.rs: GET /api/notifications/feed, owner only"] --> ledger2
  router --> ledger2
  router --> port
  router --> policy
```

The lapse context comes from the governor, through the caller (ADR-041). The bot role's owner
`/sync` flushes through the router over its transport; the job role's scheduled cycle has no bot
transport, so its flush does nothing until a job that sends joins one.
