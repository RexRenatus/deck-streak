# ADR-492: the test log capture's floor is held by a model of the callsite interest cache

- **Status:** accepted
- **Decides:** SPEC-475 (issue #499, which follows #461)
- **Path:** `docs/decisions/ADR-492-the-log-capture-floor-is-held-by-a-model-of-the-callsite-interest-cache.md`

## Context

Every capturing test in the workspace goes through `tools/log-capture/capture.rs` (SPEC-024
section 8). Its floor is a subscriber installed once as the global default before any capture
registers its own dispatcher, so that a thread holding no capture never caches a callsite as
"never" while a capture on another thread wants it. The capture tests in
`crates/kernel/tests/log_capture_class.rs` drive chosen orders of two threads in child processes.
Issue #499 asks for a model whose checker explores every interleaving of a thread with no capture
and a thread that installs one, over one callsite, with a witness that removes the floor.

Every citation below was read at dev `36c4783a`. Citations into `tracing-core` are into the
version `Cargo.lock` locks, by its own `src/` file and line.

## Decision

### D1. The mechanism, as the code runs it

1. **The level gate comes first.** The global level filter starts at `OFF`
   (`tracing-core` `src/metadata.rs:245`) and is set in one place, the end of a full rebuild
   (`src/callsite.rs:407-422`, `set_max` at `:421`), which runs at every dispatcher registration
   (`src/callsite.rs:484-488`, from `Dispatch::new` at `src/dispatcher.rs:472-481`) and at the
   public `rebuild_interest_cache` (`src/callsite.rs:222-224`). A full rebuild takes the largest
   hint of the dispatchers it reads, and a missing hint counts as `TRACE` (`src/callsite.rs:412`).
   A `tracing` macro checks the level before it reads or registers its callsite: the tree states
   it (`crates/kernel/tests/log_capture_class.rs:207-216` emulates a macro as "the level gate, the
   cached interest, then the current default's `enabled`"; `tools/log-capture/capture.rs:43-44`).
   This decision did not read the `tracing` crate's own macro source; the delivery re-reads it at
   its cut, and another order is a stop.
2. **A first registration** is `DefaultCallsite::register` (`src/callsite.rs:308-342`): a
   compare-and-swap from unregistered to registering (`:310-315`), a push onto the callsite list
   (`:318`, `:437-463`), the rebuilder's choice (`:319`, `:544-549`), the answer
   (`:490-507`), its store (`:506`, `:358-365`), the registered mark (`:320`) and a load of the
   cached value that this emission then obeys (`:337-341`). A thread that finds the callsite
   registering answers "sometimes" for that one emission and caches nothing (`:326-333`). Every
   later emission reads the cached value (`:347-354`), which starts as none (`:281-288`).
3. **One dispatcher or several.** `has_just_one` starts true (`src/callsite.rs:540`) and is
   recomputed only when a dispatcher registers, as "at most one live registrar after the dead ones
   are dropped" (`:551-557`); a dispatcher's drop recomputes nothing. While it is true the
   rebuilder is `JustOne`, which takes no lock and asks the reaching thread's own default
   (`:564-566`). Otherwise it is `Read`: the read lock is held from the choice through the store,
   and the answer folds every live registrar's answer (`:497-503`) by `Interest::and`, which keeps
   an equal pair and answers "sometimes" for an unequal one (`src/subscriber.rs:658-664`); with no
   live registrar it is "never" (`src/callsite.rs:505`). A dispatcher's registration holds the
   write lock across its push, the switch's store, the rebuild of every listed callsite and the
   level's store (`:484-488`, `:551-557`).
4. **The default a thread reads** (`src/dispatcher.rs:379-397`): the global default while no
   thread holds a scoped default, else the thread's scoped default or the global one
   (`:869-876`), and, while any thread holds one, none inside a dispatcher's own call (`:855-862`). Before the global default is
   set, the global reads as none (`:446-449`). None answers "never" (`src/subscriber.rs:674-678`)
   and gives no hint (`src/subscriber.rs:227-229`).
5. **The floor** (`tools/log-capture/capture.rs:46-74`) answers every callsite "sometimes"
   (`:49-51`), enables nothing (`:53-55`) and hints `OFF` (`:57-59`). `register_floor`
   (`:80-88`) runs once, behind `FLOOR` (`:77`): `Dispatch::new(Floor)` registers it first, so one
   dispatcher is registered and the level stays `OFF`, then `set_global_default` makes it the
   global default and leaks it, so its registrar stays live for the binary's life
   (`src/dispatcher.rs:299-331`, `:805-808`).
6. **A capture.** `with_capture` (`capture.rs:153-162`) and `hold_capture` (`:165-174`) each run
   `register_floor`, then `refuse_nested_capture` (`:117-128`), which refuses a thread whose own
   count `HELD` (`:93`) is not 0 and a thread whose default is not the floor; then the capture's
   dispatcher registers and becomes the thread's scoped default (`src/dispatcher.rs:254-262`,
   `:276-281`, `:841-852`). `tracing`'s `subscriber::with_default` and `subscriber::set_default`
   build that dispatcher from the subscriber; this decision did not read them, and the only owned
   constructors `tracing-core` offers both register (`Dispatch::new`, `:472-481`; `From`,
   `:751-757`). The build re-reads them at its cut. `HELD` is raised before the scoped entry's
   body and after the held entry's swap; it decides nesting only. A capture ends when its guard
   drops (`capture.rs:133-150`, `src/dispatcher.rs:889-899`) and its dispatcher is dropped, so its
   registrar no longer upgrades (`src/dispatcher.rs:805-808`).
7. **Where the two threads interleave.** A thread with no capture takes, at its first emission,
   its level read, the compare-and-swap with the push, the rebuilder's choice, and then either
   one locked fold-and-store or, on `JustOne`, an ask, a store and a load, each its own step. The
   capturing thread takes the floor's registration, the global default's store, the refusal with
   the capture's registration (one step under the write lock), the default swap, its emissions and
   the end. Any of the first thread's steps may fall between any two of the second's.
8. **What follows.** While the floor hints `OFF`, the level stays `OFF` until the first capture's
   registration sets it, and that same locked step makes `has_just_one` false for good, since the
   leaked floor keeps a second registrar live. A macro therefore registers a callsite only after
   the floor is the global default and only on the `Read` path, which folds the floor's
   "sometimes" into every answer. The lock-free `JustOne` path, which SPEC-024 section 8 names as
   the defect, is not reached by a macro while the floor hints `OFF`; the floor's "sometimes" as the
   global default is what answers a registration that skips the level gate, which the census
   refuses outside its own killer.
9. **Not measured here.** The `tracing` crate's macro order and its two entry functions (items 1
   and 6), which the build re-reads; and the memory ordering of the relaxed loads at
   `src/metadata.rs:705`, `src/callsite.rs:337` and `:348`, which an interleaving model does not
   represent (SPEC-475 section 6).

### D2. The model

`formal/tla/LogCaptureFloor/`, module `LogCaptureFloor`. Two threads, one with no capture and one
that captures `Captures` times in turn, and one callsite.

- **State.** The callsite's cached interest (none, never, sometimes, always), its registration
  state and whether it is listed; whether the floor's registrar is live and whether the floor is
  the global default; whether a capture's registrar is live and its answer; `has_just_one`; the
  level (off, on); the capturing thread's default and its place in its run (idle, floor, global,
  register, swap, held); each thread's step in a first registration (none, choose, ask, store,
  load) and its asked answer; each thread's emissions; and three history booleans (a line lost
  under a capture, a line reaching a capture, an interest asked while the floor is not the global
  default).
- **Actions**, in D1's order: `Emit(t)` (the level gate, the cached read, the registering arm or
  the start of a registration); `Choose(t)` (the rebuilder's choice; on `Read`, one locked
  fold-and-store); `Ask(t)`, `Store(t)` and `Load(t)` (the `JustOne` path and the emission's
  load); `InstallFloor` and `SetGlobal` (`register_floor`'s two calls); `RegisterCapture` (the
  refusal's two arms, then the capture's registration with its rebuild, its switch and its
  level); `Swap`; `EndCapture`; and, only where the census switch is off, `Rebuild(t)` (the
  public rebuild) and `DirectRegister(t)` (a registration that skips the level gate). `Finished`
  stutters only at the runs' ends.
- **Switches**, each true in the configuration that models the code: `Floor` (the helper installs
  the floor and refuses a capture where the floor is not the default), `FloorHintsOff` (the
  floor's hint is `OFF`) and `CensusHolds` (nothing outside the helper rebuilds the cache or
  registers the callsite directly).
- **Properties.** `NoCachedNeverWhileACaptureIsLive`: while a capture's dispatcher is registered
  and live, the callsite's cached interest is not "never". It is the issue's property: once the
  floor is installed, no cached interest is "never" while a capture may be live; with `Floor`
  true no capture registers before the floor is installed (`capture.rs:157`, `:169`), and with the
  floor removed the property says what the floor exists to guarantee. `ALineUnderACaptureReachesIt`:
  no line emitted on a thread that holds a capture is dropped at the level gate or by a cached
  "never". `NoInterestIsAskedBeforeTheFloor`: no first registration asks a thread's default while
  the floor is not the global default.
- **Witnesses**, each one switch off: `witness/no-cached-never-while-a-capture-is-live.cfg`
  (`Floor = FALSE`, the helper before #461), `witness/a-line-under-a-capture-reaches-it.cfg`
  (`FloorHintsOff = FALSE`, a floor that hints a level), and
  `witness/no-interest-is-asked-before-the-floor.cfg` (`CensusHolds = FALSE`, a rebuild or a direct
  registration outside the helper).
- **Constants and budget.** `MCLogCaptureFloor.cfg`: `Bystanders = {"a"}`, `Captures = 2`,
  `Emits = 2`, the three switches `TRUE`. The repository's default entry budget; no entry in
  `config/formal.json`.

### D3. Today's code meets the property, and nothing is cured

At dev the order of D1 meets the property for every interleaving the model's configuration
explores, and the build's checker run decides it. No change to `capture.rs` is made, no test line
changes and no mutation row is added; the census's routed count (`assert_eq!(routed, 25, ..)`,
`crates/kernel/tests/log_capture_class.rs:1588`) and every other pin keep their figures.

### D4. One delivery

SPEC-475, this ADR, the model with its configuration and three witnesses, a new schematic, a
red-first record and a changelog fragment, in one pull request into dev that closes #499. SPEC-024
is not amended: the delivery changes nothing it states, and it checks section 8's class under
section 8's own preconditions.

### D5. Tests and rows

Each criterion is one property: the checker reports its witness as a violation, then reads the
property clean in the model's configuration. The red-first record carries one `not red` line per
criterion, naming the witness that is its red, and records the witness's violation line, read
before the property's clean line. The model's covers name six items of `capture.rs`:
`register_floor`, `with_capture` and `hold_capture`, which the issue names, and
`refuse_nested_capture`, `register_callsite` and `max_level_hint`, which the witnesses switch. No
production line changes, so no row is written and the claimed band stays empty. One push.

### D6. Not in this delivery

The `tracing` crates' behaviour beyond the interest cache, the level and the default that the
model reads (#499), the capture's line format and the census's other classes (#461), lines emitted
inside a dispatcher's own call (#461), logging outside tests (#461), more threads, callsites or
nested captures (#499), and what only a later run can measure (#499). SPEC-475 section 5 holds
each.

## What it was chosen AGAINST

- D1, a model whose bystander registers without the level gate: the tree's own emulation of a macro gates on the level first, and a gate-less model reports an order no macro reaches, an answer asked before the floor and stored after a capture registers.
- D1, reading the floor's install as one step: `Dispatch::new(Floor)` registers before `set_global_default` stores, and a floor that hints a level opens exactly that gap, which witness two shows.
- D2, one witness alone, the floor removed: it kills one property, and the floor's hint and the census's refusals would then stale nothing the checker reads.
- D2, `_POSSIBLE` predicates for reachability: they are provisional on the pinned checker and paid in every run, while traps in a scratch copy measure the same reachability and commit nothing.
- D2, splitting a dispatcher's registration into a push and a rebuild: the write lock holds both, and a lock-free step between them acts as one before or after, so the split adds states and no behaviour.
- D2, a model of the relaxed loads: the checker explores interleavings of atomic steps, and a weak-memory model needs another tool (SPEC-475 section 6).
- D3, a rebuild of the interest cache after each capture registers: the model finds nothing it would cure, and it would put a name the census refuses inside the helper.
- D3, a harness rule that every test in a capturing binary installs the floor first: nothing the model finds needs it, and it would move the census's routed count.
- D4, an insert-only amendment of SPEC-024 that points at the model: SPEC-024 states nothing the delivery changes, and its section 13 already rejected amending it for every delivery that touches the capture.
- D4, two deliveries, the model and then a cure: there is no cure to deliver.
- D5, a configuration first committed with the floor removed, to read red at one commit and green at the next: it would commit a model of code nobody runs only to flip it, and the witness keeps that configuration.
- D5, a mutation row on the floor's hint: no Rust test observes the hint, so the row would have no killer, while the cover on `max_level_hint` stales the entry when the hint changes.
- D6, folding the excluded items into this model: each adds actors or state the issue does not ask for.

## Consequences

- A change to any of the six covered items reads `STALE` on every property of the entry until the
  model is re-read against it.
- The model shows which of the floor's three parts carries which guarantee: its second live
  registrar keeps every macro-driven registration on the locked path, its `OFF` hint keeps the
  level closed until a capture registers, and its "sometimes" as the global default answers a
  registration that skips the level gate.
- The properties enter at `ramp=report`; a flip to `block` is a later delivery read by the streak.
- The lock-free path's actions are reached only in the witness configurations. The build records
  that by a trap in a scratch copy, and the main configuration's clean reading over them is the
  finding, not a vacuous pass.

## What would make this wrong

- A `tracing` macro that reads or registers its callsite before the level gate: D1 item 8 then
  fails, and the lock-free path is reachable with the floor installed.
- A locked `tracing-core` whose `has_just_one` is recomputed at a drop, whose rebuild skips the
  level, or whose dispatcher registration releases its lock before the rebuild.
- A capture whose subscriber hints no level that admits its line: such a capture does not want
  the line, which the model does not distinguish from a lost one.
- A weak-memory reordering of the relaxed loads that lets a thread read the raised level before
  the floor's global store.
