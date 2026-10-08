---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-389: a grade recorded through the press token is an ordinary answer, pinned by three tests in the answer's own test file

Decides SPEC-378 (issue `#716`). ADR-376 D2 and D5 settled that the owner's answer has a token of
its own and a row of its own, `ANSWERED`, each chosen against a row in `EXEMPT`, and SPEC-365's
section 5 left one sentence open under `#716`: no row joins the exemption list, and the never-list
and the owner-taps exemption stay as they are. This record decides how that sentence is held by a
test. It amends no ADR, and it changes no line of ADR-301, ADR-337, ADR-376 or any ruling.

## Context and Problem Statement

At `dev` `f48a977c` a grade recorded through the press token reaches the engine only through
`Dispatcher::run_answer` (`crates/engine-core/src/dispatch.rs:377-383`), which runs the one
`ANSWERED` row, (13,4) `SchedulerService.AnswerCard`; `decide` classes the pair `NeedsAnswer` on
both transports (`crates/engine-core/src/table.rs:355-356`), and no `EXEMPT` row
(`table.rs:286-343`) names it. Three tests would fail if it joined the exemptions, but each is a
census that a delivery adding an exempt row edits in the same change, as SPEC-371 edited them for
Undo (SPEC-378 section 1c). No test's one subject is that a grade is not an exempt write.

## Decision Drivers

- The pin must survive the edit a joining delivery makes to the table's censuses, so it cannot be
  a census of the whole table.
- The pin must not be satisfiable by emptying a list: a check that only compares `EXEMPT` with
  `ANSWERED` passes when AnswerCard leaves `ANSWERED` for `EXEMPT`.
- This delivery adds a pin and edits nothing it pins: no source file, no table row, no never-list
  entry, no declared write class and no ruling.

## Considered Options (the alternatives each was chosen against)

### D1. Whether `dev` already pins the answer outside the exemptions

- Chosen: not yet pinned, because the three tests that would fail are censuses a joining delivery
  edits, and one of them, the table's decision census, cannot see AnswerCard join `EXEMPT` while it
  stays in `ANSWERED` (`decide` answers `NeedsAnswer` first; the red-first record quotes the run).
- Close `#716` by citing `crates/engine-core/tests/table.rs:83` and `:131`: rejected, because SPEC-365
  wrote those two tests and still left `#716` open in its section 5, and SPEC-371 grew both of them
  for a new exempt row in the same change that added it.

### D2. What the pin names the answer by

- Chosen: the literal pair (13,4) and the literal name `SchedulerService.AnswerCard`, written in the
  test and never read from the core, because a move of the call is then judged against what the
  engine numbers, whatever the core's lists say.
- Read the answer's pair from `ANSWERED`: rejected, because a delivery that moves AnswerCard out of
  `ANSWERED` and into `EXEMPT` leaves `ANSWERED` empty, and "no exempt row holds an answered pair"
  then passes on an empty population.
- Pin `EXEMPT`'s whole row list: rejected, because that is the census
  `each_exempt_write_names_its_engine_call_and_its_target_kind` already holds, which every new
  exempt row edits.

### D3. Where the pin lives

- Chosen: `crates/engine-core/tests/answer.rs`, the token's own test file, after its existing tests,
  because no open pull request changes it and its target already links the engine.
- `crates/engine-core/tests/table.rs`: rejected, because open pull request #737 changes it, and it
  holds the census literals a joining delivery edits, so the pin would sit beside the lines that
  edit rewrites.
- A new test file: rejected, because it adds a further test binary that links the whole engine for
  three tests that need no collection.

### D4. Which side of the client boundary the pin is on

- Chosen: a Rust test in the core, because the core's table is the one place a write is classed,
  and both clients reach the engine's answer only through `run_answer`.
- A web test: rejected, because `web/app` names no exempt write (no file under it holds `exempt` at
  `f48a977c`) and the Worker classes nothing; a web test would pin the page's protocol, not the
  exemption.
- Both: rejected for the web test's reason.

### D5. How each criterion is seen red

- Chosen: each criterion's red is its row's mutant, planted by hand on the tree the tests were
  committed to and restored byte for byte, as SPEC-362's A8 recorded its red, because a pin of
  behaviour `dev` already has cannot be red at its own commit.
- A red commit whose test calls a stub: rejected, because the stub would live in the test itself,
  and the red would be the stub's, not the criterion's.
- A red commit that plants the mutant in `src/table.rs`: rejected, because that edits the exemption
  list, which `#716` forbids.

### D6. How many rows, and where

- Chosen: three rows, one per side of the classing, each killed by one of the three tests: the
  call joins `EXEMPT` (S37800), `decide` classes it as exempt (S37801), and `run` refuses it as
  held for a gesture (S37802), because each is a distinct way the answer could become an exempt
  write.
- The two table rows alone: rejected, because `run`'s refusal arm would then be held only by the
  native census of `tests/dispatch.rs`, which no row names.

## Decision Outcome

D1 to D6 as chosen above. `crates/engine-core/tests/answer.rs` gains
`the_answer_is_decided_for_a_press_on_both_transports_never_for_a_gesture`,
`no_exempt_row_holds_the_answer_and_a_planted_one_is_refused_by_name` and
`run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture`, and
`scripts/mutation-rows.d/S37800-S37899.json` holds their three rows.

## Consequences

- Good: a delivery that would make a grade an exempt write must delete or rewrite a test whose one
  subject is that it is not, which a reviewer reads as a weakening and which needs a signed ruling.
- Good: the never-list, the owner-taps exemption, the exemption list and every declared write
  class stay as they are, byte for byte.
- Bad: the call's pair and name are written twice in the core's tests, in `tests/table.rs` and in
  `tests/answer.rs`; a renumbering by the engine changes both.
- Neutral: the existing censuses are unchanged and keep their own work.

### Confirmation

SPEC-378's A1 to A3, their planted reds in `docs/red-first/SPEC-378.md`, and the rows `S37800` to
`S37802`.

## What would make this wrong

- If the owner rules that a grade is a tap under the owner-taps ruling, these three tests are
  retired by that ruling's own delivery, with the ruling cited.
- If the engine renumbers AnswerCard, the literal pair changes in both test files, and nothing else
  does.

## More Information

- SPEC-365 section 5 (`#716`), ADR-376 D2 and D5, ADR-301 (a), ADR-337, and the owner-taps ruling.
