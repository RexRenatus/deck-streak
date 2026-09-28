# Schematic: seasons at the study day, the node track and the one ceremony

Kind: state machine and data flow. Read at DeckStreak `dev` c3d769b and at the predecessor's
`27ee2bc` (`gamification/seasons.py`, `pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes`
and `_month_ceremony`). Added by SPEC-074. It extends `docs/schematics/seasons-state-machine.md`,
which stays as it is: that machine names "the first sync of a calendar month"; here the month is the
study day's (ADR-074), and the fold that reaches each study day once is ADR-071's.

## A chapter, by the study day

A chapter is the calendar month of the study day; its number is `year × 12 + month − 1`. The fold
(ADR-071) reaches every study day once, in order: a closed day at its settle, then the current day.

```mermaid
stateDiagram-v2
  [*] --> Open: the fold reaches the first study day of a month
  Open --> Open: a study day of the chapter is reached - season XP is summed, nodes cross
  Open --> Closing: the fold reaches the first study day of the next month
  Closing --> Closed: one ceremony occasion, key chapter plus its number, event ceremony
  Closed --> [*]
  note right of Open: the node target is computed once, at the first evaluation, then frozen
  note right of Closing: the router defers it in quiet hours and delivers it once
```

## The boundary on a month's first day

The predecessor's `season_period` turns at local midnight; the study day turns at the rollover.
DeckStreak reads the study day on every screen (CHARTER 7), so the two differ only in this window.

```mermaid
flowchart LR
  a["local midnight on the 1st"] --> b["the rollover on the 1st"]
  a -- "predecessor view: the new month" --> v1["season_period: the new month"]
  a -- "DeckStreak: the study day is still the last day of the old month" --> v2["chapter: the old month"]
  b -- "both: the new month" --> v3["chapter: the new month"]
```

The golden's cases inside that window carry the class `midnight-to-rollover`; the test asserts the
study day's month there and the predecessor's month everywhere else.

## The node track, at each day the fold reaches

The track and the ceremony are one step in phase 7 (awards) of SPEC-071's fold, after the day's
derived bonuses and its coin mint: the track reads the day's whole XP and pays coins only.

```mermaid
flowchart TD
  day["a study day the fold reaches"] --> ch["its chapter"]
  ch --> tgt{"a target stored for the chapter?"}
  tgt -- "no" --> comp["node_target over the three previous chapters' season XP, floored, clamped by the previous target"]
  comp --> store["store it once in season_targets"]
  tgt -- "yes" --> read["read the frozen target"]
  store --> xp
  read --> xp["season XP: both XP tables, the chapter's study days, both tracks"]
  xp --> crossed["nodes_crossed: at most ten"]
  crossed --> loop{"a crossed node not yet claimed?"}
  loop -- "yes" --> claim["claim it in season_nodes, once"]
  claim --> coins["coins through economy's credit port, once per source and ref"]
  claim --> five{"node five, and both freeze caps allow?"}
  five -- "yes" --> freeze["one freeze through streaks, reason season"]
  coins --> msg
  freeze --> msg["one collapsed celebration naming the top node"]
  loop -- "no" --> done["nothing more: nodes never un-cross"]
```

A node claimed on an earlier recompute whose coins were never written is paid at the next
evaluation, and a node whose coins exist is never paid again: the predecessor's claim-then-grant
self-heal (`database.py:GamifyStore.claim_season_node`, `coin_ref_exists`).
