# Schematic: the celebration ladder on the one router

Kind: data flow and decision order. Read at DeckStreak `dev` c3d769b
(`notifications-policy.json`) and at the predecessor's `27ee2bc` (`gamification/ladder.py`,
`pipeline_layers/celebrations.py:CelebrationsLayer.celebrate`, `_streak_broke_today`,
`_react_to_owner` and `flush_deferred_celebrations`,
`database.py:GamifyStore.celebrations_at_or_above`). Added by SPEC-084.

It extends `docs/schematics/notification-router.md`, which draws SPEC-041's decision order. The ladder
sits between that order's third rule (the lapse) and its fourth (quiet hours), so a celebration held
in quiet hours is held at its final tier and counts toward its week.

## The tier of one celebration

```mermaid
flowchart TD
  occ["celebration occasion: event type, rarity, study day, streak facts"] --> pre{{"route, rules 1 to 3: the setting, dedupe, the lapse"}}
  pre --> req["requested tier: the rarity's tier, else the event's, else the unknown event's"]
  req --> ex{"a band-up?"}
  ex -- "yes, budget-exempt" --> cap
  ex -- "no" --> b5{"T5, and the week's T5 slots used?"}
  b5 -- "yes" --> to4["T4"]
  b5 -- "no" --> b4
  to4 --> b4{"T4, and the week's T4 slots used?"}
  b4 -- "yes" --> to3["T3"]
  b4 -- "no" --> fl
  to3 --> fl{"Epic or Legendary below the rare floor?"}
  fl -- "yes" --> floor["the rare floor, T2"]
  fl -- "no" --> cap
  floor --> cap{"did the streak break on the occasion's study day?"}
  cap -- "yes" --> t1["at most T1, every rarity included"]
  cap -- "no" --> quiet
  t1 --> quiet{"route, rule 4: inside quiet hours?"}
  quiet -- "yes" --> held[("queue: held at its tier, counted in its week")]
  quiet -- "no" --> surface{"route: the surface"}
  surface -- "raised by a Mini App request" --> feed["in-app feed item with its tier"]
  surface -- "otherwise" --> bot{"the bot, by tier"}
```

The week is the Monday-start week of the occasion's study day, and it counts the celebrations
delivered and the ones still held; an abandoned hold releases its slot.

## Each tier's render

```mermaid
flowchart LR
  bot{"tier"} -- "T0" --> none["record only"]
  bot -- "T1" --> fresh{"owner's latest message within its age, and the reaction breaker closed?"}
  fresh -- "yes" --> react["reaction on that message"]
  fresh -- "no" --> hold1["held, no attempt, no retry count"]
  react -- "refused" --> rb["reaction breaker opens for the cooldown, T2 to T5 unaffected"]
  bot -- "T2" --> line["message"]
  bot -- "T3" --> ph["placeholder"] --> pause["the golden's pause"] --> edit["edit into the message, or send it anew when the edit fails"]
  bot -- "T4" --> d4["dice"] --> m4["message"]
  bot -- "T5" --> d5["dice"] --> m5["message"] --> pin["pin"]
  feed["in-app feed item"] --> anim["Mini App animation of the tier, the same content without motion when reduced motion is asked"]
```

## The flush of held celebrations

```mermaid
flowchart TD
  q[("held celebrations")] --> age{"older than the age cap, or beyond the queue bound?"}
  age -- "yes" --> drop["abandoned, named in the rollup, its week slot released"]
  age -- "no" --> rank["rank by held tier, the older first on a tie"]
  rank --> top["the first two: the smaller of the held tier and the flush day's cap, rendered in the order held"]
  rank --> rest["every other: one rollup line, each at most T2"]
```

The flush runs after each successful sync, outside quiet hours (SPEC-041 R7); the flush day's cap is
the streak-break cap of the study day the flush runs on.
