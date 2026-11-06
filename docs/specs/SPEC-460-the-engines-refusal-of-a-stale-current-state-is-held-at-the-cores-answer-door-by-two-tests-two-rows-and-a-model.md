# SPEC-460: the engine's refusal of a stale current state is held at the core's answer door by two tests, two rows and a model

Status: ready to build, one delivery (ADR-477). Issue: #760. Decided by ADR-477.
Amends, insert-only: ADR-376 (one appended amendment section) and
`docs/schematics/owner-press-to-recorded-grade.md` (section 6 appended).

## 1. The problem, measured

Every figure was read at `dev` `83846ea3`, by the command named beside it.

- ADR-376 D3 rejects reading the next state at answer time, and gives as one reason that "The
  engine already refuses a stale current state" (`grep -n 'stale'` over the ADR: line 59, its only
  hit). ADR-361 D2 relies on the same check, and the answer's schematic says so in its section 5.
- No test holds the refusal. A fenced `git grep -n -i 'card was modified' 83846ea3 -- .` with the
  drill and stand-in excludes finds 3 lines, all in documents, and 0 under `crates/`. The same
  grep for `AnswerRefusal::Engine` finds 4 lines under `crates/`: 2 in
  `crates/engine-core/src/dispatch.rs`, 1 in `crates/ffi/src/engine.rs` and 1 in
  `crates/engine-core/tests/answer.rs:350`, which is the refusal's own sentence, not a call of
  `run_answer` that the engine refused.
- The refusal is the engine's, not the core's. `OwnerAnswer::checked`
  (`crates/engine-core/src/answer.rs:116-132`) compares only the card id (`:119`) and the rating
  (`:125`), and `Dispatcher::run_answer` (`crates/engine-core/src/dispatch.rs:496-502`) runs the
  checked request and maps the engine's error to `AnswerRefusal::Engine` (`:501`). The engine, at
  the rev `Cargo.lock` pins (`bd4fe2f4`), refuses unless the card's current scheduling state, as
  it computes it at answer time, equals the request's current state
  (`rslib/src/scheduler/answering/mod.rs:335-339`), before any write (`:341`), with an
  invalid-input error whose message opens `card was modified` (ADR-477 D1).
- Two DeckStreak paths reach it, both through `run_answer`: the native `Engine::answer`
  (`crates/ffi/src/engine.rs:184-206`) and the web `rate` (`crates/web-engine/src/wasm.rs:818-856`).
- The rows and the model read `crates/engine-core/src/dispatch.rs` and
  `crates/engine-core/src/answer.rs`, and the tests call `crates/engine-core/tests/support/mod.rs`;
  none of the three changes in this delivery.

## 2. Requirements

R1. The core's own suite holds the refusal at the core's answer door: an answer recorded through
`Dispatcher::run_answer`, with an `OwnerAnswer` whose card and grade are the request's, on the
states its card was shown with, after the card's scheduling state changed between the show and the
answer, returns `AnswerRefusal::Engine`, and its error decodes as a `BackendError` of kind
`INVALID_INPUT` whose message, up to its first colon, is `card was modified`, spelt in the test.

R2. Another answer in between: the same request bytes for the first card are recorded once (`Ok`)
and give R1's refusal the second time.

R3. Another door in between: the owner's gesture sets the first card's due date
(`ExemptWrite::SetDueDate` through `run_exempt`), which changes the card's snapshot, and the press
on the states shown before it gives R1's refusal.

R4. A refused answer leaves the collection as it was: the card's snapshot (all seven columns of
`Read::CardSnapshot`) and the newest review-log row (`Read::NewestReview`) read the same before and
after it.

R5. The refusal is the changed card's alone: the second card, shown at the same show as the first,
is recorded after R2's refusal with the states it was shown with, and its repetitions then read 1.

R6. Each test is seen red by a mutation row of the core's door that it kills every time:
`S46001` (the core reads the card's states from the engine at answer time and sends the engine's
current state in place of the shown one, ADR-376 D3's rejected option), killed by A1's test, and
`S46002` (the engine's refusal read as a recorded answer), killed by A2's test. Each row's mutant is
also run against the other test, which fails, and against the whole engine-core package suite on
the tree before the tests are added, where no test fails, and the red-first record says so.

R7. A TLA+ entry, `formal/tla/AnswerOnTheShownState/`, models two presses and one other writer
over one card's state, with the engine's check and write as one action.
`AnAnswerCarriesTheStateItWasShown` and `NoAnswerIsRecordedOnAChangedCard` are clean in its `MC`
configuration, and each property's witness is caught.

R8. ADR-376 and `docs/schematics/owner-press-to-recorded-grade.md` each gain one appended section,
and no existing line of either changes.

R9. No file under `crates/*/src` changes, and no test is removed, skipped or narrowed.

## 3. Acceptance criteria of SPEC-460

| A | criterion | decided by |
|---|---|---|
| A1 | R1, R2, R4 and R5: a second press on the states shown before the first is refused by the engine, the collection is unchanged by it, and the other card is then recorded | `stale_answer::a_second_press_on_the_states_shown_before_the_first_is_refused_by_the_engine` |
| A2 | R1, R3 and R4: a press on the states shown before a due date was set is refused by the engine, and the collection is unchanged by it | `stale_answer::a_press_on_the_states_shown_before_a_due_date_was_set_is_refused_by_the_engine` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test stale_answer -- --exact a_second_press_on_the_states_shown_before_the_first_is_refused_by_the_engine
A2: cargo test -p deck-streak-engine-core --test stale_answer -- --exact a_press_on_the_states_shown_before_a_due_date_was_set_is_refused_by_the_engine
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/engine-core/tests/stale_answer.rs` | 1 | added: the tests of A1 and A2 |
| `scripts/mutation-rows.d/S46000-S46099.json` | 1 | added: rows S46001 and S46002 (section 8) |
| `formal/tla/AnswerOnTheShownState/AnswerOnTheShownState.tla` | 1 | added: the model (section 9) |
| `formal/tla/AnswerOnTheShownState/MCAnswerOnTheShownState.cfg` | 1 | added: the model as the code is |
| `formal/tla/AnswerOnTheShownState/witness/an-answer-carries-the-state-it-was-shown.cfg` | 1 | added: the witness of AnAnswerCarriesTheStateItWasShown |
| `formal/tla/AnswerOnTheShownState/witness/no-answer-is-recorded-on-a-changed-card.cfg` | 1 | added: the witness of NoAnswerIsRecordedOnAChangedCard |
| `docs/specs/SPEC-460-the-engines-refusal-of-a-stale-current-state-is-held-at-the-cores-answer-door-by-two-tests-two-rows-and-a-model.md` | 1 | added: this SPEC |
| `docs/decisions/ADR-477-the-engines-refusal-of-a-stale-current-state-is-pinned-at-the-cores-answer-door-seen-red-by-two-rows-and-modelled.md` | 1 | added: the decision |
| `docs/decisions/ADR-376-the-owner-answer-token-holds-a-grade-to-a-press-and-the-engine-records-again-and-good.md` | 1 | amended: one section appended, no line changed |
| `docs/schematics/owner-press-to-recorded-grade.md` | 1 | amended: section 6 appended, no line changed |
| `docs/red-first/SPEC-460.md` | 1 | added: the red-first record (section 10) |
| `changelog.d/engine-stale-state-refusal-460.md` | 1 | added: the changelog fragment |

## 5. What this does NOT cover

- Reading the next state at answer time, which ADR-376 D3 rejects (#711).
- A grade that arrives by sync, #715 and ADR-404 (#715).
- The scheduler's clock tests, among them a review card shown before the day's rollover and
  answered after it, whose computed state then differs (#747).
- What only this delivery's own CI run can measure: the rows' verdicts by id and the CI timing,
  read after its push (#760).

## 6. Risks

- An engine bump that rewords the message head turns A1 and A2 red. That is the pin working: the
  bump's delivery re-reads the head from the engine's source and re-pins it, never loosens it.
- An open pull request changes `crates/engine-core/tests/support/mod.rs`, whose `synthetic` and
  `open_request` the new target calls: a rename there breaks this target at its merge. The build
  re-measures both names at its cut and before its push.
- An open pull request changes `crates/engine-core/src/dispatch.rs`, where both rows' finds and the
  `run_answer` cover sit: a moved find VOIDs its row and a moved span stales the entry. The build
  re-measures each find (once) at its cut and before its push.
- `S46001`'s method index, 23, is derived from the engine's numbering (ADR-477 D3), not read from
  generated code: its proof measures it, and a mutant under which the stale press is still refused as
  stale stops the build.
- A census that counts test targets could red on the added one: the whole engine-core package
  suite runs at every committed step, and CI's whole run decides the rest.
- The engine's backend lock was not read: the model takes the engine's check and write as one
  action, and ADR-477 names what would make that wrong.

## 7. What decides R6 to R9

| R | decided by |
|---|---|
| R6 | each row's hand proof (its mutant installed once, its killer run by `--exact`, the failing assertion read, the file restored by sha256), recorded in section 10's record, and CI's rows job by id |
| R7 | the repository's formal check of the one entry: both properties clean, each witness caught |
| R8 | `git diff --numstat` of each amended file reads `<n> 0` |
| R9 | `git diff --name-only` of the delivery names no `crates/*/src` path, and the weakening table is empty |

## 8. Mutation rows, band `S46000-S46099`

Both rows are in the `MUTATIONS` table, crate `engine-core`, file `src/dispatch.rs`, and each find
occurs once in that file at `dev` `83846ea3` (`grep -c -F`: 1 and 1).

| id | find | replacement | killer |
|---|---|---|---|
| `S46001-THE-CORE-SENDS-THE-SHOWN-STATE` | `let request = answer.checked(input)?;` (`:497`) | a block that keeps the check, reads `SchedulerService.GetSchedulingStates` (13, 23) for the request's card and sends that reply's current state in place of the shown one | `stale_answer::a_second_press_on_the_states_shown_before_the_first_is_refused_by_the_engine` |
| `S46002-THE-ENGINES-REFUSAL-REACHES-THE-PRESS` | `.map_err(\|error\| AnswerRefusal::Engine { error })` (`:501`) | `.or_else(\|_\| Ok(Vec::new()))` | `stale_answer::a_press_on_the_states_shown_before_a_due_date_was_set_is_refused_by_the_engine` |

## 9. Formal

TLA+, REQUIRED: `formal/tla/AnswerOnTheShownState/`, citing #760.

- Covers: `crates/engine-core/src/dispatch.rs` `run_answer`, `crates/engine-core/src/answer.rs`
  `checked` and `crates/engine-core/src/answer.rs` `answer_request`.
- Actors: two presses, each showing, sending and being answered once, and one other writer that
  may change the card's state; the engine's comparison and write are one action.
- `AnAnswerCarriesTheStateItWasShown`: every recorded answer's current state is the state its press
  was shown. Its witness sets `ReadAtAnswer = TRUE`, the core reading the engine's state at answer
  time.
- `NoAnswerIsRecordedOnAChangedCard`: every recorded answer's current state is the card's state at
  its write. Its witness sets `EngineCompares = FALSE`, an engine that writes without comparing.
- Both properties enter at `ramp=report`. Lean is not applicable: no pure function of DeckStreak's
  decides the refusal.

## 10. Red-first

The behaviour is on `dev` in a third-party engine, so no criterion can be red at the base for its
reason. `docs/red-first/SPEC-460.md` carries one line per criterion in its `red-first` fence, A1
and A2 each `not red`, naming its row as its red, and in prose below the fence each row's proof
(the failing assertion under its mutant, on both tests) and each mutant's run of the whole
engine-core package suite on the tree before the tests (no failure).
