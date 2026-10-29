# The scheduling port and its FSRS-7 implementation

Read at dev `75d555c4`. Every `path:line` below is at that commit. SPEC-494 and ADR-512 decide what this draws.

## 1. Where the queue and the due days come from, before (data flow)

Every queue and every due day a learner sees comes from the engine. DeckStreak's one due day of its own is the FSRS-7
replay's, which nothing but a test calls.

```mermaid
flowchart TD
  ankiq["engine queue: get_queued_cards / GetQueuedCards"]
  ingestq["ingest new_card_queue engine.rs:544-570"]
  webq["web engine first_queued and current_card wasm.rs:263-270, :725-729"]
  nativeq["native Engine::run 13,3 ffi engine.rs:104"]
  answer["engine answer: next state from the shown states ffi engine.rs:184-205"]
  rows["replay statement dispatch.rs:62-68"]
  sel["history selection convert.rs:52-83"]
  batch["batch replay fsrs7 replay.rs:40-54"]
  proj["stock projection stock.rs:35-40"]
  sched["scheduled: round, clamp, last day plus ivl replay.rs:164-182"]
  dayof["engine day of an instant replay.rs:186-189"]
  caller["caller's engine day dispatch.rs:602-613"]
  ankiq --> ingestq
  ankiq --> webq
  ankiq --> nativeq
  rows --> sel --> batch --> proj --> sched
  caller --> dayof --> sched
```

## 2. After (data flow)

The replay's due day has one source, the port's `interval_days`. The port's module sits in the engine core beside
`Dispatcher::run`, the clients' one door to the engine's queue; no method of the trait gives a queue, so the engine's
queue passes that door unchanged and the route census names it as the engine's boundary.

```mermaid
flowchart TD
  rows2["replay statement, unchanged"]
  sel2["history selection, unchanged"]
  ev["producer: kept reviews as ReviewEvent, key = review id, study_day = engine day of the review"]
  fold["Fsrs7 fold: event check, then append"]
  memo["Fsrs7 batch replay over the folded cards, once"]
  idays["Fsrs7 interval_days: projection at the target, rounded"]
  stockv["Fsrs7 stock values: stability and difficulty"]
  sched2["scheduled: rounded.min(max_ivl).max(1), last day plus ivl"]
  dq["scheduler day_queue: due on or before the day, by due then card, then the sibling rule"]
  ankiq2["engine queue, unchanged, through Dispatcher::run for both clients, five files at the cut"]
  rows2 --> sel2 --> ev --> fold --> memo
  memo --> idays --> sched2
  memo --> stockv
  sched2 --> dq
```

## 3. The trait between them (component)

```mermaid
flowchart LR
  subgraph core["deck-streak-engine-core"]
    port["scheduler.rs: SchedulerPort, ReviewEvent, EventKey, Recall, FoldError, LoadError, check_event, day_queue"]
    impl7["replay.rs port module: Fsrs7 implements SchedulerPort"]
    prod["replay.rs: Dispatcher::replay, the producer"]
    door["dispatch.rs: Dispatcher::run, the clients' door to the engine's queue, unchanged"]
  end
  fsrs["deck-streak-fsrs7, unchanged"]
  clients["web engine call and native run, unchanged"]
  impl7 --> port
  prod --> port
  prod --> impl7
  impl7 --> fsrs
  clients --> door
```

The crate graph is unchanged: the core depends on the engine, its protocol crates and `deck-streak-fsrs7`, and gains
no dependency; the two client adapters depend on the core; no workspace crate depends on anything new.

## 4. Where the study day enters, and where the sibling rule sits (sequence)

```mermaid
sequenceDiagram
  participant Caller
  participant Replay as the replay
  participant Port as the FSRS-7 port
  participant Queue as the day queue
  Caller->>Replay: decks, parameters, retention, max_ivl, engine day
  Replay->>Replay: rows, selection, events with the engine day of each review
  Replay->>Port: fold each event in key order
  Port-->>Replay: refusal leaves the state unchanged
  Replay->>Port: interval_days at the target retention
  Port-->>Replay: whole days, today's rounding
  Replay->>Replay: clamp, due = last day plus ivl
  Replay-->>Caller: CardReplay per card
  Caller->>Queue: card, note, due per card, and a study day
  Queue-->>Caller: cards, moved to the next day, unlinked count
```

The producer is the only reader of a day: the port and the day queue read no clock and no timezone.

## 5. The tests, reds and rows of each step

| step | test (SPEC-494) | red at the stub commit | rows |
|---|---|---|---|
| the port's whole days on the fixtures, and the retention check | A1 `fsrs7_behind_the_port_gives_todays_due_days_on_the_replay_fixtures` | the stub answers 0 days | S49415, S49428 |
| the port's whole days over a grid | A2 `fsrs7_whole_days_equal_todays_rounding_over_a_fixed_grid` | the stub answers 0 days | S49415 |
| the day queue and the sibling rule | A3 `two_cards_of_one_note_never_share_a_study_day`, `the_sibling_rule_holds_over_every_small_queue` | the stub keeps both siblings | S49409 to S49412, S49424 |
| a card with no note id | A4 `a_card_with_no_note_id_turns_the_rule_off_and_is_counted` | the stub counts nothing | S49413, S49414 |
| only through the port | A5 `the_due_days_come_only_through_the_port_and_no_new_file_asks_for_the_engine_queue` | the stub replay keeps its direct path | S49420 |
| no clock in the port | A6 `no_clock_or_timezone_read_sits_inside_the_port` | the stub port reads the clock | S49421 |
| the fold's refusals | A7 `a_fold_refuses_a_key_not_after_the_last_and_leaves_the_state_unchanged` | the stub refuses nothing | S49403 to S49408 |
| checkpoint and restore | A8 `a_checkpoint_restores_to_equal_bytes_and_equal_intervals` | the stub checkpoint is empty | S49416 to S49418, S49423, S49425 to S49427 |
| the probe and the version | A9 `predict_is_a_pure_probe_and_the_version_names_the_model` | the stub answers one constant | S49400 to S49402, S49419, S49422 |

The replay suite and the graph suite run unchanged at every step and hold today's pins through the port.
