# Schematic: the notification router's decision

Kind: data flow and decision order. Read at DeckStreak `main` ce3683d (`notifications-policy.json`,
ADR-011, docs/schematics/streaks-and-governor-state-machine.md), at the predecessor's `27ee2bc`
(`quiet_hours.py:in_quiet_hours`, `pipeline_layers/celebrations.py:CelebrationsLayer.celebrate` and
`flush_deferred_celebrations`), and at the packs vendored from `19bb0f3` (notifications-policy: the
decision ledger, the withhold reasons, `one-router`). Added by SPEC-041.

```mermaid
flowchart TD
  occ["occasion: kind, dedupe key, origin, tier, payload, study day, lapse context"] --> route{{"route (crates/notifications/src/router.rs)"}}
  route --> s1{"kind's setting off?"}
  s1 -- "yes" --> w1["withhold: nudges_disabled"]
  s1 -- "no" --> s2{"delivered already in the kind's dedupe scope?"}
  s2 -- "yes" --> w2["withhold: already_recorded"]
  s2 -- "no" --> s3{"a nudge in an open lapse, and not a comeback-budget kind?"}
  s3 -- "yes" --> w3["withhold: lapse"]
  s3 -- "no" --> s4{"inside 23:00 to 07:30?"}
  s4 -- "celebration" --> defer["defer: queue of at most 20"]
  s4 -- "nudge" --> w4["withhold: quiet_hours"]
  s4 -- "alert, or outside" --> s5{"a comeback past 3 per lapse, or within 3 study days of the last?"}
  s5 -- "yes" --> w5["withhold: budget_spent"]
  s5 -- "no" --> s6{"surface: a celebration from a Mini App request goes in-app, all else to the bot"}
  s6 --> s7{"transport answers?"}
  s7 -- "no" --> w6["withhold: no_notifier"]
  s7 -- "yes" --> send["send: push_message and the other bot calls, or push_in_app"]
  send -- "failed" --> hold["hold: first time kept, 2 retries, 60-second breaker"]
  send -- "sent" --> delivery["record the delivery"]
  w1 --> ledger[("decision ledger: kind with :withheld appended")]
  w2 --> ledger
  w3 --> ledger
  w4 --> ledger
  w5 --> ledger
  w6 --> ledger
  defer --> ledger
  hold --> ledger
  delivery --> ledger
  flush["flush after each successful sync, outside quiet hours"] --> defer
  flush -- "at most 2 in full, the rest one rollup line, older than 720 minutes abandoned by name" --> send
```

The five delivery calls the policy's `router.transport` names are called only inside the router
module; the bot implements its four in `crates/bot/src/transport.rs`, and `push_in_app` appends to the
in-app feed that `GET /api/notifications/feed` serves to the owner. The lapse context comes from the
governor, through the caller (ADR-041).
