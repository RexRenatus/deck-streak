# Schematic: the skip day's switch and the monthly cap of bridges

Kind: data flow. Read at DeckStreak `dev` af0693f and at the predecessor's `27ee2bc`
(`config.py:Settings.skip_enabled`, `constants.SKIP_BRIDGE_MONTHLY_CAP`,
`pipeline_layers/skip.py:SkipDaysLayer.take_skip_day` and `skip.py:real_misses`, which check
neither). Added by the W5 architect turn, by the owner's decision at #269. SPEC-109 builds it.

It extends `docs/schematics/skip-day-record-and-effects.md` without changing it: the take there
gains a first refusal, and its readers' port answers a second set.

## A take and the switch

The switch is read in the same write that checks the day, so a switch turned off before that
write refuses the take, and one turned off after it finds the day's row already written.

```mermaid
flowchart TD
  P[the owner opens the skip day] --> SW{the switch on}
  SW -- no --> OFF[the preview answers off, no due count, cards or tariff]
  OFF --> NO[no confirm offered and no Cheat day row on the stakes preview]
  SW -- yes --> PV[the preview with bridges true or false]
  PV --> CF[the owner confirms]
  CF --> W[begin the take's first write]
  W --> SW2{the switch on, read in this write}
  SW2 -- no --> R0[refused skip_disabled, no row, no request, no purchase]
  SW2 -- yes --> AS{a skip pending or applied and not undone on the day}
  AS -- yes --> R1[refused already_skipped]
  AS -- no --> TAKE[the take as SPEC-083 specifies, then its tariff]
```

- The undo, its refund, the summary and the skip set never read the switch.
- A change of the switch bumps the settings generation in its own write; writing the same value
  writes nothing.

## Two sets from one record

```mermaid
flowchart LR
  rec[("skip_days, owned by ingest")] --> skip{{"the skip set: study days with an applied skip not undone"}}
  skip --> cap{{"the bridged set: the first days of each calendar month, up to the cap from economy.json"}}
  cap --> ls["language streak: bridged, no freeze consumed"]
  cap --> law["law streak: bridged"]
  cap --> gov["governor: the silent run neither counts nor ends at the day"]
  cap --> run["consistency run: left unchanged"]
  cap --> view["streak view: a skip marker"]
  cap --> pv["preview: bridges true while the month holds fewer than the cap"]
  skip --> qs["quests voided, no chest, a race week counts it"]
  skip --> asc["Ascendant: never armed on the day"]
  skip --> dis["no fine booked and no window judged on the day"]
  skip --> nud["the nudges and the comeback decline on the day"]
  skip --> tar["the tariff counts every skip of the month"]
```

- A skip day in the skip set and not in the bridged set is a missed day to every rule fed by the
  bridged set, and still a declared day off to every rule fed by the skip set.
- The tariff's month and the cap's month are one function of the study day, so the price and the
  bridge never disagree at a month's edge.
- An undo removes its day from the skip set, so the month's next skip joins the bridged set.
