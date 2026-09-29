# Schematic: the landmarks, the milestone pings and the pinned widget

Kind: data flow and state machine. Read at DeckStreak `dev` af0693f and at the predecessor's
`27ee2bc` (`landmarks.py:run_landmarks`, `pipeline.py:GamifyPipeline._notify_milestones` and
`_run_sync_cycle_impl`, `pipeline_layers/showcase.py:ShowcaseLayer._update_widget` and
`_repin_widget`, `telegram.py:render_milestone`, `render_widget` and `widget_mood`). Added by the W5
architect turn, under ADR-109 (the widget is one silent pinned message a study day, edited in place
through the router). SPEC-102 builds it.

It extends five accepted schematics without changing them: `docs/schematics/notification-router.md`
(the decision each message goes through), `docs/schematics/celebration-ladder-on-the-router.md` (the
tiers, the dice and the T5 pin), `docs/schematics/recompute-settles-each-study-day.md` (the fold
whose awards phase the two new steps join), `docs/schematics/sync-cycle-and-change-gate.md` (the
cycle whose last step refreshes the widget) and `docs/schematics/cron-fire-ledger-and-catch-up.md`
(the hourly job's fires).

## The awards phase: landmarks and queue zero

Both steps run in the seventh phase of the fold (ADR-071), after the badges, for each study day the
fold settles and then for the current one. Each raises its occasions through the router, whose
once-ever dedupe is the guard against a second raise.

```mermaid
flowchart TD
  R[recompute after a sync] --> S[read the study days of the whole scoped log once]
  S --> D{next day the fold settles or evaluates}
  D -- none left --> E[done]
  D -- a day --> B[badges and records, with the milestone text]
  B --> M{landmark_high_water stored}
  M -- no --> SEED[store the mark, raise at most the first due landmark of the run]
  M -- yes --> L[raise each landmark due that day]
  SEED --> Q
  L --> Q{the day holds a study review and a backlog_zero grant above 0}
  Q -- yes --> Z[raise queue_zero, epic, with its dice]
  Q -- no --> D
  Z --> D
```

- An anniversary carries the honest variant when the language streak's last study day, as of the day
  evaluated, is not that day.
- A celebration raised at the morning's recompute falls in quiet hours: the router holds it, and the
  flush at the window's end delivers it (#291).

## A widget refresh

The sync cycle's last step, and the hourly job at minute 44, route one occasion of the kind
`widget`; its arm in `route` decides.

```mermaid
flowchart TD
  O[occasion of kind widget, the day's text] --> H{stored row for the day with the same sha256}
  H -- yes --> U[unchanged, nothing pushed or recorded]
  H -- no --> SW{widget_enabled is 0}
  SW -- yes --> W1[withhold, nudges_disabled]
  SW -- no --> QH{inside the quiet window}
  QH -- yes --> W2[withhold, quiet_hours]
  QH -- no --> T{bot transport joined and the breaker closed}
  T -- no --> W3[withhold, no_notifier]
  T -- yes --> ROW{a row for the day}
  ROW -- no --> C[claim the kind for the day]
  C --> SEND[send silently with the Open app row]
  SEND -- delivered --> PIN[pin it, unpin the day before's widget, store id and sha256]
  SEND -- failed --> F1[release the claim, no_notifier, open the breaker]
  ROW -- yes --> ED[edit the stored message with the same row]
  ED -- edited or not modified --> UP[store the new sha256]
  ED -- failed --> RS[send anew silently and pin, store the new id and sha256]
  RS -- failed --> F2[no_notifier, open the breaker, keep the row]
  PIN --> REC[record one send]
  UP --> REC
  RS -- delivered --> REC
```

## The widget message across a study day

```mermaid
stateDiagram-v2
  [*] --> None: the study day turns over
  None --> Pinned: first refresh sent and pinned
  None --> None: refresh withheld or its send failed
  Pinned --> Pinned: text changed and edited
  Pinned --> Pinned: text unchanged
  Pinned --> Pinned: edit failed, sent anew and pinned
  Pinned --> Pinned: a T5 pinned, the widget re-pinned
  Pinned --> Unpinned: the next day's first widget pinned
  Unpinned --> [*]
```

- The day before's widget is unpinned only when the next day's widget is sent; it is never edited
  again.
- A T5 in `route` or in the flush unpins and pins the day's widget after its own pin, so the widget
  stays the newest pin.
