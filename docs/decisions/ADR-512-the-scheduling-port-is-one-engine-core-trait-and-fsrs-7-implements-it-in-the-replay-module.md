# ADR-512: The scheduling port is one engine-core trait, and FSRS-7 implements it in the replay module

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-494 |
| builds on | ADR-338, ADR-356, ADR-400 (none amended) |
| measured at | dev `75d555c4` |

## Context

DeckStreak is to gain one scheduling interface, the scheduling port: a trait in the crate where DeckStreak consumes
the engine's queue, with today's FSRS-7 path as its first and only implementation, every due day byte-equal to
today's, and a census that DeckStreak's due days come only through it and that no new file asks the engine for its
queue. A later delivery plugs a second implementation in behind the same trait, with a switch whose default the owner
sets (#997). This ADR decides where the trait lives, what routes through it, how its first implementation keeps
every due day, the same-note sibling rule, the censuses, and the shape of the change. Every fact below was read at
dev `75d555c4` by `git show` with line ranges; a fact the tree cannot show is named as not measured.

## D1. The scheduling model, measured, and what it was chosen against

Where each queue and each due day is computed today:

- **The server's queue is the engine's**: `AnkiEngine::new_card_queue` (`crates/ingest/src/engine.rs:141-152`) opens
  the collection and, for each top-level deck, sets it current and asks the engine for its queued cards
  (`queue_of_every_root`, `:544-570`, the engine call at `:553-555`, at most `QUEUE_FETCH_LIMIT` = 1000 cards, `:36`),
  keeping the new cards only. It computes no due day: a skip's due cards are read back as the engine stored them
  (`:206-229`, `:442-506`).
- **The clients' queue is the engine's**: `SchedulerService.GetQueuedCards` (13, 3) is an ordinary pair admitted on
  both transports (`crates/engine-core/src/table.rs:186-192`). The web engine asks for the queue's head at
  `crates/web-engine/src/wasm.rs:263-270` and `:725-729`; the native client reaches the same pair through
  `Engine::run` (`crates/ffi/src/engine.rs:104`). The engine builds that queue with its own order rules and its own
  sibling-bury options.
- **Both clients consume the engine's queue at one door in the core**: The web engine's `call`
  (`crates/web-engine/src/wasm.rs:103-105`, behind `first_queued` and `current_card` above and `run_method` at
  `:1073-1076`) and the native adapter's `run` (`crates/ffi/src/engine.rs:104-116`, its allow-list row
  `crates/ffi/src/allow_list.rs:52-56`) each reach the engine's queue only through `Dispatcher::run`
  (`crates/engine-core/src/dispatch.rs:220-245`), which admits the pair by the core's table; the context map names
  the core as the engine for both clients (`docs/CONTEXT-MAP.md:37`). The server's new-card queue is consumed in
  ingest, which has no edge to the core (`crates/ingest/Cargo.toml:14-31`).
- **The engine's scheduler is not FSRS-7**: The engine locks the released FSRS crate (`Cargo.toml:142-145`;
  `Cargo.lock:125`), which carries no FSRS-7; the FSRS-7 revision is pinned for the isolated crate alone
  (`Cargo.toml:137-141`; `Cargo.lock:1270-1275`).
- **A due day after an answer is the engine's**: The native answer picks the next state out of the engine's own
  `SchedulingStates` the card was shown with (`crates/ffi/src/engine.rs:184-205`), and the engine records it.
- **DeckStreak's own due day is the FSRS-7 replay's, and nothing calls it**: `Dispatcher::replay`
  (`crates/engine-core/src/replay.rs:92-146`) reads the review rows by one fixed statement, selects each card's kept
  reviews (`crates/fsrs7/src/convert.rs:52-83`), replays them in one batch (`replay.rs:120`;
  `crates/fsrs7/src/replay.rs:40-54`), projects each state at the desired retention (`crates/fsrs7/src/stock.rs:35-40`)
  and gives a review card `ivl` = the interval rounded, at most the preset's maximum and at least 1, and `due` = the
  engine day of its last kept review plus `ivl` (`replay.rs:164-182`, the engine day of an instant at `:186-189`). No
  client adapter names the replay (`crates/engine-core/tests/graph.rs:582-630`; SPEC-386 R14), and its result is
  written by nothing but a test: the write is the switch's (#997).

So at the cut DeckStreak computes no queue of its own, and exactly one due day: the replay's. Every queue and every
due day a learner sees comes from the engine.

Where the study day enters, and every clock on the path:

- **Two day frames**: The kernel's study day is an epoch day, `floor((t + m*60000 - h*3600000) / 86400000)` for the
  instant `t`, the offset `m` and the rollover hour `h` (`crates/kernel/src/study_day.rs:104-114`; default hour
  `:25`, default offset `:52`). The engine's day counts whole days since the collection was created, with its next
  rollover (`crates/engine-core/src/late.rs:17-26`), read from the engine's own timing of today
  (`crates/engine-core/src/dispatch.rs:43-46`, `:602-613`; on the server `crates/ingest/src/engine.rs:373-383`, from
  the collection's own offset and hour, `:185-192`). A card's due counts in the engine's frame.
- **The replay reads no clock**: Its caller passes the engine day (`replay.rs:82-99`; SPEC-386 R8), and
  `crates/fsrs7/src/replay.rs`, `convert.rs` and `stock.rs` read none; the crate's timing harness takes its clock as an
  argument (`crates/fsrs7/src/measure/mod.rs:35-67`).
- **The clocks on the answer path are the producers'**: The native adapter stamps an answer by `SystemTime`
  (`crates/ffi/src/engine.rs:243-249`), the web engine by the page's `Date.now()` (`wasm.rs:253-261`), and the
  engine computes its own day.

What the fixtures pin: `crates/engine-core/tests/replay.rs:400-457` pins a first Easy at 54 days, a first Again held
at 1 and the maximum at 3, each due its last review's engine day plus that; `:459-482` pins a review three engine days
back as due on day 1051. The four first-review intervals at 0.9 (`crates/fsrs7/tests/stock.rs:10-15`) round and
round up to the same whole day, so no fixture tells rounding from rounding up. The formal vectors
(`formal/vectors/replay-history.jsonl`, 2321 lines) pin the history selection only, read by
`crates/fsrs7/tests/formal_vectors_replay_history.rs:41-90`. The single and batch replays agree only within a relative
1e-6 (`crates/fsrs7/tests/replay.rs:15-17`), so bit-equal values need the batch method the replay uses.

Not measured: which order and which sibling-bury options each learner's presets set in the engine, and whether any
learner has them on; the collections' real sizes; and any deployed setting. The tree cannot show them.

Chosen against:
- Reading the engine's queue as DeckStreak's own computation: the engine builds it from its own stored dues, so a trait in front of it would hold no due day DeckStreak computes.
- Taking the kernel's study day as the frame of the replay's due: the due counts in the engine's frame, and the two differ by the collection's creation day.

## D2. The seam, and what it was chosen against

**The home.** A new module `crates/engine-core/src/scheduler.rs` holds the trait `SchedulerPort`, `ReviewEvent`,
`EventKey`, `Recall`, `FoldError`, `LoadError`, the shared event check and the day queue. The engine core is where
DeckStreak consumes the engine's queue for both clients, at `Dispatcher::run` (D1), and it already holds the one due
day DeckStreak computes; its one workspace edge is the FSRS-7 crate (ADR-400 D1). The crate graph is unchanged: the
core depends on the engine, its protocol crates and `deck-streak-fsrs7`, the two client adapters depend on the core,
and no cycle is possible because the trait's module names no other workspace crate.

**The seven methods, with no rename:** `version_id`, `new_state`, `fold`, `predict`, `interval_days`, `checkpoint`
and `restore`, with the associated type `State`. A hash of a folded state is not a method here: the later
implementation brings it with its own crate (#997). `version_id` returns a fixed identifier per implementation.
`interval_days`'s doc names each implementation's own whole-day rule; FSRS-7's is its interval rounded to the nearest
whole day, so every due day stays byte-equal, not only the fixtures'. `ReviewEvent` keeps its
eight fields (`card`, `note`, `deck`, `preset`, `grade`, `answer_ms`, `key`, `study_day`) and `Recall` keeps its one
`f32`, always within 2^-13 and 1 - 2^-13 and never NaN, built only by `Recall::new`, which refuses a value outside
that range, or `Recall::clamped`, which holds a finite value to it.

**`EventKey`** is the event's instant in milliseconds, as the review log's id stores it: a total order, strictly
increasing over one learner's events. A key not after the last folded key is refused.

**The errors** are plain enums with `Display` and no payload of user text. `FoldError` names a key not after the last,
a grade outside 1 to 4, an answer time above the cap of 3,600,000 milliseconds, an unknown card, a day before the
card's last review, a value that is not finite, a state made under other parameters and the model's refusal;
`LoadError` names truncated bytes, bytes that are no checkpoint, a checkpoint of another implementation and a refused
event. No input panics: a malformed event is an error, a checkpoint's lengths are checked arithmetic, and the state is
unchanged after any error.

**FSRS-7 is the first implementation,** `Fsrs7`, in an inline module of `crates/engine-core/src/replay.rs`, the one
core source that may name the FSRS-7 crate (ADR-400 D1; `graph.rs:537-567`). Its state holds the folded events in key
order and the bits of the parameters it was made under; its memory states are the batch replay over every folded
card, computed once after the last fold.

**The call sites.** `Dispatcher::replay` folds each card's kept reviews into a fresh `Fsrs7` state and takes a review
card's whole-day interval only from `interval_days`; its stability and difficulty come from the same state through
the implementation's own stock projection. The day queue takes due days as input. No client adapter, no server crate
and no screen gains a call. No method of the trait returns a queue, so the engine's own queue, and every due day
after an answer, still pass the core's doors (`dispatch.rs:220-245` and `:496-502`) unchanged, and the route census
names the five files that ask the engine for its queue at the cut; moving them behind the port is the later
delivery's (SPEC-494 section 5, #997).

**The queue from due days.** `scheduler::day_queue` builds a study day's queue from per-card due days: the cards due on
or before the day, in the order of their due day and then their card id, then the sibling rule (D4). DeckStreak has no
queue order rule of its own at the cut (D1), so this order is the replay's own card order refined by the due day.

Chosen against:
- A trait per client, one in the native adapter and one in the web engine: two copies of one rule, and the web copy cannot be tested natively against the engine.
- A direct call kept beside the trait for the due day: the census could then not tell a route through the port from one around it.
- A new dependency-free crate for the trait: a new crate and a new map edge for one module, which ADR-400 D1 refused as a bridge crate; a later delivery that needs the trait outside the core moves it then (What would make this wrong).
- The trait in the FSRS-7 crate: a second implementation would then depend on FSRS-7's crate to implement the trait.
- The trait in the kernel: the core would gain an edge to a server crate that the web build does not carry today.
- The engine's own queue behind the trait now: no method of the trait returns a queue, and a queue built from the trait's due days by the port's order would change the order a learner sees.
- An engine-backed first implementation: the engine's answer needs the states a card was shown with, which no event carries, and the engine's scheduler is not FSRS-7 (D1), so its due days are not the replay's.

## D3. Byte-equal due days, and what it was chosen against

Every due day stays byte-equal because the arithmetic moves without change: `Fsrs7::interval_days` returns the
pinned revision's interval at the target rounded to the nearest whole day (the cast `replay.rs:170-177` makes today),
and `Dispatcher::replay` keeps today's composition, `rounded.min(max_ivl).max(1)` and the engine day of the last
kept review plus `ivl`, on the lines rows S38609, S38610 and S38614 anchor. The memory states come from the same batch
call over the same items in the same card order, so stability and difficulty keep their bits too. A retention that is
no `Recall` (NaN, or outside 2^-13 to 1 - 2^-13) is refused by the replay; at the cut it was replayed, and no caller
passes one.

Two tests prove it, each with today's direct path as the control, built in the test from the review rows by the
FSRS-7 crate's own functions and never by the core:

- On the replay fixtures (`tests/replay.rs:400-482`, built the same way, with the same seed inputs): for every
  replayed card, the port's whole-day interval and stock values equal the control's bit for bit, and
  `Dispatcher::replay`'s schedule equals the control's due and interval; the pinned 54, 1, 3 and 1051 hold.
- Over a fixed grid of two-review histories: the port's whole-day interval equals the control's rounded interval at
  every cell, and the grid holds cells whose interval's fraction is below one half, which round and round up apart.

Chosen against:
- The least-whole-day rule for FSRS-7 now: it rounds up, and moves every due day whose interval's fraction is below one half; that rule is the later implementation's own, never the trait's contract.
- A tolerance on the stock values: a tolerance hides the difference the single and the batch method already show.

## D4. The sibling rule, and what it was chosen against

When the queue is built from a per-card schedule, two cards of one note never fall on the same study day. In
`day_queue`, after the order rule, the first card of a note in the day's order enters the day's queue, and each later
card of that note moves to the next day. A card whose note id is missing, 0 or less, takes no part in the rule: it
enters the queue, and each such card is counted in the queue's `unlinked` count. The rule holds in the day queue the
port builds; no screen reads that queue yet, so no learner sees a change when this lands. Whether the engine's own
sibling-bury options are on for any learner is not measured (#138).

Its tests: two cards of one note due on one day, the later in the day's order moved to the next day, a card of
another note kept, and an overdue sibling first; cards with no note id all kept and each counted; and every queue of
four cards over three note values and three due days, judged by an independent property check.

Chosen against:
- Holding the rule behind a setting whose default keeps the old queue: the port's queue has no old behaviour to keep, and a setting nobody reads is a branch with no test that matters.
- Applying the rule to the engine's own queue now: it changes what a learner sees, which is the switch's to decide.
- Moving a later sibling by its own interval instead of to the next day: the rule names the next day.

## D5. Census and purity, and what it was chosen against

- **The route census**: it reads the production sources of the engine core, the two client adapters, ingest and the
  FSRS-7 crate, reports how many it examined, and refuses: a model call (the batch or single replay, the stock
  projection, the interval and the retrievability functions) outside the FSRS-7 crate and the port module of
  `replay.rs`; a replay whose schedule does not come from `interval_days`; and an engine queue call outside the five
  files that make one at the cut. Its planted controls are a producer that rounds the stock projection's interval
  itself, a sixth file asking the engine for its queue, and a model call in an adapter, each refused by name.
- **The purity census**: it refuses a clock or timezone read (`SystemTime`, `Instant`, `UNIX_EPOCH`, `Date`, `chrono`,
  the engine's timing of today, a UTC offset or a rollover hour) in `scheduler.rs` or the port module, with a planted
  port that reads the clock refused by name.
- **Fold refusal**: a key equal to or before the last, a grade of 0 or 5 and an answer time above the cap are each
  refused, and the state's checkpoint is unchanged after each.
- **Checkpoint and restore**: a restored checkpoint gives equal bytes and equal intervals; two different folded sets
  give different checkpoints; truncated bytes, bytes that are no checkpoint and a checkpoint that names another
  implementation are refused. A checkpoint holds the implementation's fixed identifier and the folded events, so the
  engine core gains no dependency.

Chosen against:
- A census by file only: the direct path today sits in the same file as the port, so a file census reads green over a bypass.
- A hash of the folded state in this delivery: the engine core would gain a hashing dependency that the later implementation's own crate brings, and equal checkpoint bytes already prove the round trip.

## D6. Production or test, and the shape, and what it was chosen against

Production: `crates/engine-core/src/scheduler.rs` (new), `crates/engine-core/src/replay.rs` and
`crates/engine-core/src/lib.rs` (one module line and its doc line). Test: `crates/engine-core/tests/scheduler_seam.rs` and
`crates/engine-core/tests/scheduler_census.rs` (new). Byte-unchanged: the FSRS-7 crate and its pin, its arithmetic
and parameters, every due day, the replay's one statement (`dispatch.rs:62-68`), the core's doors to the engine
(`dispatch.rs:220-245`, `:496-502`), every route, every client screen, both client adapters, ingest, the root
manifest, the engine core's manifest and the lockfile, every formal entry and vector, every existing test and every
existing row. No kept check is loosened, and no SPEC, ADR or schematic states a changed rule (SPEC-386 R8 states no
range for the retention the replay refuses), so none is amended. One delivery for one builder, landing on dev after
the release pull request.

Chosen against:
- Two deliveries, the server path and the client path: neither path computes a due day of DeckStreak's, so a split leaves one part with nothing to route.
- The port in a child module of the replay (`src/replay/scheduler.rs`): it spares `lib.rs` but names the port after its first implementation.

## D7. Tests and rows, and what it was chosen against

Every criterion is red first at a stub commit that keeps every input and lacks only the behaviour: the port answers
0 days, a constant recall and empty checkpoints, the day queue keeps every card, the fold checks nothing, the producer
keeps its direct path, and the stub port holds one planted clock read. Every red is an assertion read locally by
a named test target; no red is read only in CI, so the delivery takes one push. Rows S49400 to S49428 hold each new
literal constant, the comparisons, the rounding, the replay's retention check, the checkpoint's header and length,
and the two censuses' planted refusals.

Formal: Lean NOT APPLICABLE (the day queue and the fold's check decide a study queue and a refusal, no gate verdict,
ledger, seal, ratchet, money or release; the day queue is closed by its exhaustive population and an independent
property check). TLA+ NOT APPLICABLE (one actor folds under exclusive access; no interleaving). No cover moves: the
entry over the history selection covers `crates/fsrs7/src/convert.rs`, which is unchanged, and no entry covers the
replay module; every vector stays byte-unchanged.

Chosen against:
- Reds that fail to compile at the base: a missing type is not the criterion's reason.
- A Lean port of the day queue: a pure queue rule that decides no verdict is closed by its population, as the formal rule allows.

## D8. Not in this delivery, and what it was chosen against

Not here: the second implementation and its switch, the owner's default, where the port runs for the web client, the
engine's own queue and the server's new-card queue through the port, a hash of a folded state, note ids in the
replay's statement, any change to a due day, and anything only a later CI run can measure. Each is listed with its
issue in SPEC-494 section 5.

Chosen against:
- Plugging the second implementation in now: its crates do not exist yet, and its switch's default is the owner's.

## Consequences

- The replay's due day has one source, `interval_days`, held by a census; a second implementation plugs in by
  implementing the trait, with no change to the producer's composition.
- The day queue and its sibling rule exist and are tested before any screen reads them.
- The engine core's dependency list, the root manifest and the lockfile are unchanged.

## Confirmation

SPEC-494's acceptance fence, run at the head, with the replay and graph suites unchanged and green.

## What would make this wrong

- A later delivery runs the port outside the engine core (the server, or a crate the core cannot reach): the trait
  then moves to a dependency-free crate in that delivery.
- A second implementation's whole-day rule is the least whole day with recall at or below the target: the two
  implementations then differ by rule, which the switch's evaluation measures, never this delivery.
- A learner's engine preset buries siblings already: the port's queue and the engine's agree on siblings, and the rule
  costs nothing; measured by #138, not here.
- The batch replay's per-card values depend on which other cards share the batch: the port computes over the same
  cards in the same order, so the values hold here, but a producer that folds a different card set would see them move.
- The engine's own queue must come through the trait before a second implementation lands: no method gives a queue,
  so the trait would then need one, which this delivery does not add.
