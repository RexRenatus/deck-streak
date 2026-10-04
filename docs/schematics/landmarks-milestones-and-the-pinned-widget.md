# Schematic: the landmarks, the milestone pings and the pinned widget

Kind: data flow and state machine. Read at DeckStreak `dev` af0693f and at the predecessor's
`27ee2bc` (`landmarks.py:run_landmarks`, `pipeline.py:GamifyPipeline._notify_milestones` and
`_run_sync_cycle_impl`, `pipeline_layers/showcase.py:ShowcaseLayer._update_widget` and
`_repin_widget`, `telegram.py:render_milestone`, `render_widget` and `widget_mood`). Added by the W5
architect turn, under ADR-109 (the widget is one silent pinned message a study day, edited in place
through the router). SPEC-102 builds it. Part b (#127, ADR-322) redraws the landmarks as offers
between the fold's writes, read at `96f6fe31`.

It extends five accepted schematics without changing them: `docs/schematics/notification-router.md`
(the decision each message goes through), `docs/schematics/celebration-ladder-on-the-router.md` (the
tiers, the dice and the T5 pin), `docs/schematics/recompute-settles-each-study-day.md` (the fold
whose awards phase the two new steps join), `docs/schematics/sync-cycle-and-change-gate.md` (the
cycle whose last step refreshes the widget) and `docs/schematics/cron-fire-ledger-and-catch-up.md`
(the hourly job's fires).

## The awards phase: queue zero

Queue zero runs in the seventh phase of the fold (ADR-071), after the badges, for each study day the
fold settles and then for the current one. It raises its occasion through the router, whose
once-ever dedupe is the guard against a second raise.

```mermaid
flowchart TD
  D{next day the fold settles or evaluates} -- none left --> E[done]
  D -- a day --> B[badges and records, with the milestone text]
  B --> Q{the day holds a study review and a backlog_zero grant above 0}
  Q -- yes --> Z[raise queue_zero, epic, with its dice]
  Q -- no --> D
  Z --> D
```

## The landmarks' offers, between the fold's writes

Redrawn for #127 part b (SPEC-102 section 11, ADR-322), read at `96f6fe31`: the landmarks are no
longer raised inside the awards phase. The sync cycle reads the study days of the whole scoped log
once and hands the fold its offers in turn, the awards' and then the landmarks'. The fold runs them
before each settled day's write, before the current day's write, and once more after its last write
(ADR-303). Each landmark offer call, with today the current study day:

```mermaid
flowchart TD
  C[sync cycle reads the study days of the whole scoped log] -- the read fails --> N[no landmark is offered and no cursor moves]
  C -- read --> O[an offer call]
  O --> F{this recompute's seed has run}
  F -- no --> SEED[one write: the mark where none is stored, the cursor at yesterday where none is; the run is first when the mark was absent]
  F -- yes --> R
  SEED --> R[read the cursor X and the settle cursor S on one reader]
  R --> W{first run}
  W -- yes --> ONE[owe the first landmark due today, and no other]
  W -- no --> OWE[owe each landmark after X and at or before S, and each due today, less the keys answered in this recompute]
  ONE --> H[hand each owed landmark, oldest first, to the router]
  OWE --> H
  H --> G{first run, or no day settled}
  G -- yes --> K[the cursor stays]
  G -- no --> A{the router answered every owed landmark}
  A -- yes --> T1[target is S]
  A -- no --> T2[target is the day before the oldest unanswered, at most S]
  T1 --> V{target after X}
  T2 --> V
  V -- yes --> ADV[move X to the target in a write of its own, never backwards]
  V -- no --> K
```

- The router answers on a send, a hold and a withhold alike; only a refusal leaves a landmark
  unanswered, and the next offer call owes it again. A key offered twice reads the router's
  once-ever dedupe as already recorded, so it is sent once.
- The mark is the predecessor's bytes, `{"anniversary": A, "seeded": true, "study_day": N}`, with A
  and N the highest ordinals among the landmarks up to the day evaluated. It is stored once and
  never overwritten; a mark imported from the predecessor makes no run first.
- An anniversary carries the honest variant when the window holds no language study on its day.
- A celebration raised at the morning's recompute falls in quiet hours: the router holds it, and the
  flush at the window's end delivers it (#291).

## A widget refresh

The owner's sync cycle's last step, and the hourly job at minute 44, route one occasion of the kind
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
