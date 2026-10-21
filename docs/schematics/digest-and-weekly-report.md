# Schematic: the digest, the weekly report and the debrief

Kind: data flow and sequence. Read at DeckStreak `dev` c8d8a30 and at the predecessor's `27ee2bc`
(`pipeline_layers/digests.py:DigestsLayer`'s `run_daily_digest` and `run_weekly_report`,
`telegram.py:render_daily_digest` and `render_weekly`, `bot.py:CommandBot._callback_debrief` and
`database.py:GamifyStore.session_debrief_readout_line`). Added by the W5 architect turn, under
ADR-108 (the weekly report names this week's stored instrument reports and splices none). SPEC-101
builds it.

It extends five accepted schematics without changing them: `docs/schematics/notification-router.md`
(the decision each message goes through), `docs/schematics/cron-fire-ledger-and-catch-up.md` (the
two jobs' fires and the digest's catch-up), `docs/schematics/bot-update-loop.md` (the loop that
answers a debrief tap), `docs/schematics/insights-instrument-frame.md` (the stored reports the
weekly report names) and `docs/schematics/nudge-coordinator-and-holdout.md` (the holdout the weekly
report settles and reads).

## The digest

The job `daily_digest` runs at the digest hour, minute 6. Coordination gathers each context's
read and passes plain values; notifications renders the text and raises one occasion.

```mermaid
flowchart TD
  J[daily_digest job] --> T[target is the study day before the current one]
  T --> R{a stored rollup for the target}
  R -- no --> N[raise nothing]
  R -- yes --> L{the governor holds an open lapse}
  L -- yes --> LP[the lapse line with the relight count]
  LP --> LK[the debrief row and the Deal row]
  L -- no --> G[gather the ten reads of R15]
  G --> C[law block, body, Road to C2, habits, focus, Oracle, windows line, readout]
  C --> O[omit each empty block]
  O --> DK[the debrief row for the target]
  LK --> RT[router, kind digest, key digest]
  DK --> RT
  RT -- claimed today --> W[withhold, already_recorded]
  RT -- sent --> S[one push]
  RT -- push failed --> F[claim released, a second run can send it]
```

- The body, the Oracle block, the windows line and the debrief row read the target; the law block,
  Road to C2, the habits, the focus, the streak and the level read the current study day, as the
  predecessor's digest does.
- A day never recorded renders no card or queue line, never a zero.

## The weekly report

The job `weekly_report` runs on Sundays at the digest hour, minute 11, without catch-up.

```mermaid
flowchart TD
  J[weekly_report job] --> H[settle the holdout]
  H --> B1[law block]
  H --> B2[weekly body over 8 rollups]
  H --> B3[Road to C2]
  H --> B4[habits]
  H --> B5[focus]
  H --> B6[Oracle weekly line]
  H --> B7[holdout readout]
  H --> B8[research line from the stored reports]
  B1 --> C[compose in that order, each empty or failed block omitted]
  B2 --> C
  B3 --> C
  B4 --> C
  B5 --> C
  B6 --> C
  B7 --> C
  B8 --> C
  C --> K[row: Road to C2, and Research when the line shows]
  K --> RT[router, kind weekly, key weekly]
  RT -- sent --> S[one push]
  RT -- push failed --> F[claim released, the week stays unsent]
```

- Each block's read runs on its own: a failed read omits its block, is logged by the block's name,
  and never stops the report.
- The research line names this week's stored reports by registry id, marking a failed read; the
  report runs no instrument.

## A debrief tap

```mermaid
sequenceDiagram
  participant O as owner
  participant B as bot update loop
  participant D as notifications debrief rule
  participant S as debrief_ratings
  O->>B: de rating and day
  B->>B: owner only, else nothing is written
  B->>D: accept the rating and the day at the current study day
  alt malformed, ahead of today or over 3 days back
    D-->>B: refused
    B-->>O: the stale answer
  else accepted
    D-->>B: accepted
    B->>S: record the rating for its day, a second tap replacing the first
    B-->>O: the logged answer
  end
```

- The readout counts the rated days, and below 42 names the days still needed; it never names a
  percentage.
