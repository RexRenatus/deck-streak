# Schematic: discipline's two clocks, a window's life and the standby notice

Kind: state machine and sequence. Read at DeckStreak `dev` 26263de and at the predecessor's
`27ee2bc` (`windows.py:evaluate`, `pipeline_layers/committed_windows.py`'s
`CommittedWindowsLayer._evaluate_windows` and `_window_reminders`,
`pipeline_layers/discipline.py:DisciplineLayer._settle_hardmode`, and
`pipeline_layers/governor.py:GovernorLayer._update_governor`). Added by the W5 architect turn,
under ADR-105 (a quarter-hourly tick that reads no reviews) and ADR-104 (a verdict settles after
its evidence window). SPEC-105 builds it.

It extends four accepted schematics without changing them:
`docs/schematics/cron-fire-ledger-and-catch-up.md` (the job table and its timers, which gain a
quarter-hourly kind), `docs/schematics/sync-cycle-and-change-gate.md` (the cycle whose recompute
each deadline forces), `docs/schematics/streaks-and-governor-state-machine.md` (the verdict that
enters standby) and `docs/schematics/notification-router.md` (quiet hours withhold a nudge). The
fine's life is `docs/schematics/fine-verdict-and-refund.md`.

## Two clocks

Discipline runs on two clocks. The sync cycle reads reviews and judges; the tick reads none and
only speaks.

```mermaid
sequenceDiagram
  participant S as sync job, once a study day, or the owner's sync
  participant C as coordination sync cycle
  participant D as discipline windows and nights
  participant T as discipline tick, minutes 4 19 34 49
  participant R as notifications router
  S->>C: run the cycle
  C->>C: recompute, the governor's step stores the pending notice
  C->>C: the rail and markets steps record their messages pending, raised now with a router
  C->>D: judge every occurrence and night whose end plus 15 minutes passed
  D-->>C: verdicts, and the kept ones to credit once
  T->>D: windows starting within 15 minutes, and a pending notice
  D-->>T: reminders, the notice and the steps' pending messages, each claimed once
  T->>R: kind discipline, withheld in quiet hours or a lapse
```

- No arrow from the tick reaches ingest: it reads no review, and it never syncs (ADR-037).
- The tick's four minutes are one quarter-hourly schedule of the job table; a zone offset that is a
  multiple of 15 minutes maps the set onto itself.

## One window occurrence

```mermaid
stateDiagram-v2
  [*] --> Upcoming: booked, and its mask holds the study day
  Upcoming --> Reminded: a tick finds its start 0 to 15 minutes ahead
  Upcoming --> Open: its start passes
  Reminded --> Open: its start passes
  Open --> Waiting: its end passes
  Waiting --> Judged: the first cycle after the end plus 15 minutes
  Waiting --> NotJudged: a skip day, an open lapse, or before its booking
  Judged --> Judged: a later cycle within 7 closed study days replaces a verdict that is not kept
  Judged --> Kept: effort meets the floor, 5 coins credited once
  Kept --> [*]
  Judged --> [*]: 7 closed study days pass
  NotJudged --> [*]
```

- **Judged** holds `token_effort`, `amber` or `red`; none of them fines, and `red` only records the
  rail's defections inside the window, each already judged by the rail.
- **Kept** is final: a later cycle never replaces it, and its coins are never taken back.

## A hard-mode night

```mermaid
stateDiagram-v2
  [*] --> Booked: booked before noon, under the weekly cap of 4
  Booked --> Cancelled: the owner cancels before the start
  Booked --> Suppressed: the study day is a skip day
  Booked --> Kept: effort meets the floor, 15 coins credited once
  Booked --> Broken: 3 defections with no review, or the end plus 6 hours passes
  Broken --> Kept: a late review meets the floor within 7 closed study days
  Kept --> [*]
  Cancelled --> [*]
  Suppressed --> [*]
  Broken --> [*]: 7 closed study days pass
```

## The standby notice

```mermaid
stateDiagram-v2
  [*] --> None
  None --> Pending: a settle finds the rule true, quiet hours set aside
  Pending --> Raised: a tick outside quiet hours, still standby, no lapse
  Pending --> Dropped: the verdict leaves standby, or a lapse opens
  Raised --> None: the notice day recorded in the governor's state
  Dropped --> None
```

- The rule is SPEC-076's `standby_notice`: it enters standby from a settled day that was not
  standby, outside a lapse, and not within 7 days of the last notice.
- Raising and recording happen in one write, so two ticks at once raise one notice.
