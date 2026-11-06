---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-477: the engine's refusal of a stale current state is pinned at the core's answer door, seen red by two rows, and modelled

Decides SPEC-460 (issue #760). Found while deciding #715 (ADR-404), which does not depend on it.

**Amends, by an insert-only amendment section appended to it, with no existing line changed:**
ADR-376 D3, whose reason "The engine already refuses a stale current state" now names the tests,
the rows and the model that hold it.

## Context and Problem Statement

ADR-376 D3 rejects reading the next state at answer time, and one of its reasons is that the engine
already refuses a stale current state. ADR-361 D2 relies on the same check ("the engine's own check
(`card was modified`) compares states"), and the schematic `owner-press-to-recorded-grade.md`
section 5 says the collection "also refuses a stale current state". At `dev` `83846ea3` no test
in the tree holds that refusal: a fenced `git grep` for `card was modified` over the tree finds no
test, and no test asserts an `AnswerRefusal::Engine` that `run_answer` returned (the one hit,
`crates/engine-core/tests/answer.rs:350`, is the refusal's own sentence). A change in the engine,
or in the core's door, could drop the refusal with nothing turning red.

Every `path:line` below was read at `dev` `83846ea3`. Engine paths (under `rslib/` and `proto/`) are
read in the engine, the workspace's pinned git dependency, at the rev `Cargo.lock` pins
(`bd4fe2f4`, `Cargo.toml:173-175`, `Cargo.lock:95-97`), read-only and never built.

## Decision Drivers

- The reliance ADR-376 D3 states should fail a test when it stops being true.
- The pin holds behaviour that is already on `dev`, so its red has to be shown some other way than
  at the base, and shown every time.
- A pin claims no more than the engine does.
- The delivery should share no file with an open pull request.

## Decisions, and the alternatives each was chosen against

### D1. The refusal: where it is, what it compares, and what the caller sees

The refusal is the engine's. `answer_card` (`rslib/src/scheduler/answering/mod.rs:317-319`) runs
`answer_card_inner` (`:321`) inside one `transact` of the answer operation. It reads the card
(`:322-325`), computes the card's current scheduling state from its row at answer time
(`updater.current_card_state()`, `:331`; `rslib/src/scheduler/answering/current.rs:19-60`, from
the card's type, due, interval, lapses, ease, remaining steps and memory state and the deck's kind,
`current.rs:62-135`), sets a learning state's elapsed seconds equal on both sides so that time
passing between show and answer does not trip it (`mod.rs:334`, `:583-607`), and then refuses
unless `current_state == answer.current_state`, with a message that opens `card was modified: `
and renders both states for debugging (`require!`, `mod.rs:335-339`).
Nothing is written before the check: the first write is `apply_study_state` (`mod.rs:341`). A
request with no current state is compared as the default state
(`rslib/src/scheduler/service/answering.rs:19`). The refusal is the engine's invalid-input error
(`rslib/src/error/invalid_input.rs:92-104`), encoded as a `BackendError` of kind `INVALID_INPUT`
whose message opens with `card was modified` (`rslib/src/backend/error.rs:11-17`, by the generated
`run_service_method`, `rslib/rust_interface.rs:141-160`).

What the caller sees. The core's door, `Dispatcher::run_answer`
(`crates/engine-core/src/dispatch.rs:496-502`), checks the request (`:497`) and runs it on the
answered row, `SchedulerService.AnswerCard` (13, 4) (`:498-500`; `crates/engine-core/src/table.rs:273-277`),
and hands the engine's error bytes back unchanged as `AnswerRefusal::Engine { error }` (`:501`;
`crates/engine-core/src/answer.rs:67-71`), whose sentence is "the engine refused the answer (N
bytes)" (`answer.rs:90-92`). The core's own check, `OwnerAnswer::checked` (`answer.rs:116-132`),
compares only the card id (`:119`) and the rating (`:125`) and encodes the decoded request again
(`:131`), so the current state reaches the engine as the adapter wrote it, and the core reads
nothing from the collection at answer time.

Both DeckStreak call paths reach it, and only through `run_answer` (2 paths):
- native: `Engine::answer` (`crates/ffi/src/engine.rs:184-206`) decodes the shown states
  (`:192`), writes the request with `answer_request`, its current state `states.current`
  (`:194-201`; `answer.rs:149-166`, `current_state: current` at `:159`), and calls `run_answer`
  (`:203-205`);
  `press_refusal` (`engine.rs:261-268`) maps the refusal to `PressRefusal::Engine` (`:266`);
- web: `rate` (`crates/web-engine/src/wasm.rs:818-856`) takes the kept card's states from `SHOWN`
  (`:821-824`), writes `current_state: states.current` (`:828`) and calls `run_answer`, its
  refusal mapped by `refuse` (`:835-837`).
The core's `run` cannot reach (13, 4): the table holds it as `NeedsAnswer` (`table.rs:355-356`,
`dispatch.rs:243`).

Not measured, and not claimed: how the engine's backend serialises two callers on one open
collection (its lock was not read); the web path at run time, which runs the same engine rev (no
target gate in the five engine files on the path: 0 `target_arch` or `wasm` cfg lines) but which
no test here executes; and each adapter's own lines between the press and `run_answer`.

Chosen against:
- Reading the refusal as the core's: rejected, because the core's check compares only the card id
  and the rating (`answer.rs:119`, `:125`); the comparison of states is the engine's alone.
- Treating any change of the card's row as stale: rejected, because the engine compares the
  computed scheduling state, not the row. For a new or review card the state reads no queue and no
  flag (`current.rs:62-135` reads the queue only for a learning card's elapsed seconds, `:68-90`),
  so a bury or a flag between show and answer is not refused, and this record claims no more.

### D2. The pin: two tests at the core's answer door

Two integration tests in a new target, `crates/engine-core/tests/stale_answer.rs`, on a native
dispatcher over the support module's synthetic collection (two new cards,
`crates/engine-core/tests/support/mod.rs:63-98`). Each drives the door the way both adapters do:
the queue's states (`SchedulerService.GetQueuedCards` (13, 3)), the native codec's
`answer_request` from the shown states, `OwnerAnswer::from_press` and `run_answer`.

- A1, another answer in between: the same request bytes for the first card are recorded once and
  refused the second time; the second card, shown at the same show, is then recorded with the
  states it was shown with, so the refusal is the changed card's alone.
- A2, another door in between: the owner's gesture sets the first card's due date
  (`run_exempt`, `ExemptWrite::SetDueDate`), and the press with the states shown before it is
  refused.

Each asserts the exact refusal: `AnswerRefusal::Engine`, whose bytes decode as a `BackendError` of
kind `INVALID_INPUT` with the message head `card was modified`, spelt in the test. Each asserts
that the refusal left the collection as it was: the card's snapshot (`Read::CardSnapshot`, all
seven columns) and the newest review-log row (`Read::NewestReview`) read the same before and after
it. A2 also asserts that the due-date write changed the card's snapshot, its positive control.

Chosen against:
- The native adapter's `Engine::answer` under `crates/ffi/tests`: rejected, because D3's reliance
  is the core's (it forwards the shown state and reads nothing), both adapters reach the engine
  only through `run_answer`, and an open pull request changes `crates/ffi/src/engine.rs`.
- The web `rate` under a wasm32 runner: rejected, because the engine-core package suite cannot run
  it, and its path to the refusal is the same core door on the same engine rev.
- A test in the engine's own tree: rejected, because the engine is a pinned dependency DeckStreak
  does not test, and the reliance to hold is that DeckStreak's door still reaches the refusal.
- The answer's own test file, `crates/engine-core/tests/answer.rs`: rejected, because an open pull
  request (#932) changes it; a new target shares no file with any open pull request, at the cost
  of one more test binary that links the engine.
- Asserting the whole message: rejected, because after its head the message is the engine's debug
  rendering of two states, with values the engine computes at answer time; the head names the
  refusal.
- A core-side comparison of the current state: rejected, because it is ADR-376 D3's rejected read
  at answer time, and this delivery pins behaviour without changing it.

### D3. Seen red: two rows of the core's door

The refusal is on `dev` and it is a third-party engine's, so neither test can be red at the base
for its criterion's reason. Each criterion is recorded `not red` in `docs/red-first/SPEC-460.md`,
and each test is seen red by a hand-written mutation row of the core's door, in the band
`S46000-S46099`, proved by hand before the push and decided by CI's rows job by id:

- `S46001`: `let request = answer.checked(input)?;` (`dispatch.rs:497`) becomes a block that keeps
  the check, reads the card's states from the engine at answer time
  (`SchedulerService.GetSchedulingStates`, (13, 23)) and sends the engine's current state in place
  of the shown one: ADR-376 D3's rejected option. Its killer is A1's test. The second press then
  passes the engine's comparison, because the engine's own state always equals itself (a learning
  state's elapsed seconds are set equal), so the assertion on the refusal fails, every time; what
  follows the comparison, a recorded answer or the engine's later queue check
  (`rslib/src/scheduler/queue/mod.rs:164-177`), is another outcome than the pinned refusal.
- `S46002`: `.map_err(|error| AnswerRefusal::Engine { error })` (`dispatch.rs:501`) becomes
  `.or_else(|_| Ok(Vec::new()))`: the engine's refusal reads as a recorded answer. Its killer is
  A2's test, every time.

Each mutant is also run against the other test, which fails too, and against the whole engine-core
package suite at the tree before the tests are added, where no test fails: the gap #760 names,
measured. The index 23 is derived, and the row's proof measures it: the engine numbers a service's
methods after its backend-only methods (three for the scheduler, the engine's `scheduler.proto:71-78`),
which gives GetQueuedCards 3, AnswerCard 4 and SchedTimingToday 5, as the core and its tests already
use, and GetSchedulingStates (index 20 in `SchedulerService`, `scheduler.proto:42`) 23. A mutant
under which the stale press is still refused as stale is a stop, never a weaker row.

Chosen against:
- A red at the base: rejected, because the behaviour is already on `dev`, in a third-party engine,
  and no edit of the base could be red for the criterion's reason without changing what is pinned.
- A stub engine behind a seam the test controls: rejected, because the core holds the engine
  privately (`dispatch.rs:99-100`), and adding a seam changes production code to test a pin.
- cargo-mutants' generated mutants: rejected, because it generates neither mutant above, and its
  replacement of `run_answer`'s whole body is already killed by the existing refusal tests.
- A mutant of the codec's `current_state: current` (`answer.rs:159`): rejected, because under it
  the stale press is still refused (the engine compares the default state), so it cannot be the
  stale test's red.

### D4. Shape and split: one delivery, with its model

One delivery: the test target, the band, a TLA+ entry, SPEC-460, this record, an insert-only
amendment of ADR-376 and an insert-only section 6 of `docs/schematics/owner-press-to-recorded-grade.md`.
The claim D3 relies on is a check followed by an act on a card that another caller can change in
between, so the model rides with the test.

Chosen against:
- Two deliveries, the tests and then the model: rejected, because the surface is present now, and
  the model is small (two presses, one other writer, one card), so a second round would cost a whole
  cut and gate for a few small files.
- No model, declined here: rejected, because the claim has the form "no answer is recorded on a
  state that changed between the show and the answer", which is what a model holds.
- A new schematic: rejected, because `owner-press-to-recorded-grade.md` already draws the answer's
  path and states the reliance (its section 5), and a second schematic would draw one path twice.
- No amendment of ADR-376: rejected, because D3 states the reliance with no citation, and its
  reader should find the tests that now hold it.

### D5. Tests and rows

- Red-first per criterion: A1 and A2 each `not red`, with its row named as its red; the record's
  prose carries each row's proof and the base-suite run of each mutant.
- Rows: `S46001` and `S46002` in `scripts/mutation-rows.d/S46000-S46099.json`, `MUTATIONS` table,
  crate `engine-core`, file `src/dispatch.rs`, each find occurring once.
- FORMAL, TLA+, REQUIRED: `formal/tla/AnswerOnTheShownState/`, two presses and one other writer over
  one card's state; the engine's check and write are one action. Covers name
  `crates/engine-core/src/dispatch.rs` `run_answer`, `crates/engine-core/src/answer.rs` `checked`
  and `answer_request`. Properties `AnAnswerCarriesTheStateItWasShown` (witness: the core reads the
  engine's state at answer time) and `NoAnswerIsRecordedOnAChangedCard` (witness: the engine records
  without comparing), each `ramp=report`. Lean: not applicable, because no pure function of
  DeckStreak's decides the refusal; the comparison is the engine's equality of two states.
- Pushes: ONE.

Chosen against:
- Rows on the adapters' lines: rejected, because these tests do not run the adapters, so no row
  there could be killed by them, and open pull requests change both entry files.
- Two pushes, a red one and then the green: rejected, because no test is red at the base, so a red
  push measures nothing; the rows are proved by hand before the one push and CI decides them by id.
- Covers on `Engine::answer` and `rate`: rejected, because open pull requests change both entry
  files, `rate` is already covered by another entry, and the model's show abstracts both adapters.

### D6. Not in this delivery

- Reading the next state at answer time: ADR-376 D3 rejects it (#711).
- #715 and ADR-404: a grade that arrives by sync (#715).
- The scheduler's clock tests: a review card shown before the day's rollover and answered after
  it, whose computed state then differs, among them (#747).
- What only this delivery's own CI run can measure: the rows' verdicts by id and the CI timing,
  read after the push (#760).

Chosen against:
- Folding the clock cases in: rejected, because the clock tests are their own issue (#747), and a
  rollover case needs a pinned clock this delivery does not add.
- Pinning each adapter's mapping of the refusal here: rejected, because these tests run the core
  door only; the adapter layer is a question for its own item.

## Decision Outcome

D1 to D6 as chosen above. The core gains no line. Its test suite gains `tests/stale_answer.rs`
with A1 and A2, the band gains `S46001` and `S46002`, `formal/tla` gains `AnswerOnTheShownState`,
and ADR-376 and the answer's schematic gain one appended section each.

## Consequences

- Good: ADR-376 D3's reliance fails two tests when the engine stops refusing a stale current state
  at this door, or when the core stops forwarding the shown state, or stops handing the refusal back.
- Good: the refusal's identity, its kind and its message head, is spelt in the tree once.
- Bad: one more test binary links the engine.
- Bad: an engine bump that rewords the message head turns both tests red; the bump's delivery
  re-reads the head from the engine's source and re-pins it.
- Neutral: the engine refuses a changed scheduling state, not a changed row: a bury or a flag
  between show and answer is answered (D1).

### Confirmation

SPEC-460's A1 and A2, the rows `S46001` and `S46002` proved by hand and by CI's rows job, and the
TLA+ entry's two properties clean with each witness caught.

## What would make this wrong

- If the engine's backend lets another caller's write land between the engine's check and its
  write (not measured), the model's one-action engine is wrong, and the entry gains the two steps.
- If index 23 is not `GetSchedulingStates` on the pinned rev, `S46001`'s proof shows it (the stale
  press still refused as stale), and the row is rewritten before the push.
- If a native or web adapter is found to build its current state from anything but the shown
  states, the adapter needs its own pin, and the model's show action splits by adapter.
