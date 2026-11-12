# Red-first record: SPEC-378

The SPEC and ADR-389 were committed first (f3aa14c0), and the criteria's three tests next, alone
(597bee73): its tree is the base's code with those tests and no other change, because this delivery
adds a pin and changes no code path. A pin of behaviour the base already has cannot be red at its
own commit, so every criterion is recorded `not red` below, and each one's red is its row's mutant,
planted by hand on the tree at 597bee73, run through the criterion's own fenced command, and
restored byte for byte, as SPEC-362's A8 recorded its red. On the restored tree each fenced command
selected one test (`running 1 test`) and passed: A1 and A3 print `examined 2 of 2 transport(s)`, A2
prints `examined 8 exempt row(s)`.

## The fence, line by line

Each of the 3 lines of SPEC-378 section 3's fence resolves to a test this delivery adds.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/answer.rs` `the_answer_is_decided_for_a_press_on_both_transports_never_for_a_gesture` | added (597bee73) |
| 2 | A2 | `crates/engine-core/tests/answer.rs` `no_exempt_row_holds_the_answer_and_a_planted_one_is_refused_by_name` | added (597bee73) |
| 3 | A3 | `crates/engine-core/tests/answer.rs` `run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture` | added (597bee73) |

```red-first
A1: not red: it pins the base's classing of the call that records a grade, which the base already answers NeedsAnswer; row S37801's planted mutant reads it red (below)
A2: not red: it pins the base's exempt table, which already names no such call; row S37800's planted mutant reads it red (below)
A3: not red: it pins the base's refusal of that call through run, which already reads NeedsAnswer; row S37802's planted mutant reads it red (below)
```

**A1, by its planted mutant.** A1 is mutation coverage, not red-first. Row S37801's mutant, which
makes `decide`'s answered arm read `Decision::NeedsGesture`, was planted by hand in
`crates/engine-core/src/table.rs` and read A1 red at `crates/engine-core/tests/answer.rs:410`:
`assertion left == right failed: the call that records a grade is the one answered row, held for an
owner's press on both transports`, with `left: ([(Native, NeedsGesture), (Web, NeedsGesture)], [(13,
4, "SchedulerService.AnswerCard")])` and `right: ([(Native, NeedsAnswer), (Web, NeedsAnswer)], [(13,
4, "SchedulerService.AnswerCard")])`. `table.rs` read sha256 `a7af4a57…92147667` before the plant
and after its restore, byte for byte.

**A2, by its planted mutant.** A2 is mutation coverage, not red-first. Row S37800's mutant, which
adds a ninth `EXEMPT` row naming (13,4) `SchedulerService.AnswerCard`, was planted by hand in
`crates/engine-core/src/table.rs` and read A2 red at `crates/engine-core/tests/answer.rs:461`:
`assertion left == right failed: no exempt row names the call that records a grade; a planted row
that joins, takes its pair or takes its name is refused by name`, with the table's own reading
`["SchedulerService.AnswerCard"]` where the criterion holds `[]`. `table.rs` read sha256
`a7af4a57…92147667` before the plant and after its restore, byte for byte.

With the same mutant planted, the table's two censuses were run as well.
`crates/engine-core/tests/table.rs` `every_pair_is_admitted_held_or_refused_by_its_transport`
passed (`ok`): `decide` answers `NeedsAnswer` before it reads `EXEMPT`, so the census's decision
reading cannot see the call join the exemptions while it stays answered, which is SPEC-378 section
1c's claim.
`each_exempt_write_names_its_engine_call_and_its_target_kind` failed at `tests/table.rs:144`
(`assertion left == right failed`), because it holds `EXEMPT` equal to its eight literal rows: that
census is the one a joining delivery grows in the same change, as SPEC-371 grew it.

**A3, by its planted mutant.** A3 is mutation coverage, not red-first. Row S37802's mutant, which
makes `run`'s answered arm refuse with `Refusal::NeedsGesture`, was planted by hand in
`crates/engine-core/src/dispatch.rs` and read A3 red at `crates/engine-core/tests/answer.rs:494`:
`assertion left == right failed: run holds the call that records a grade for an owner's press on
both transports, never for a gesture`, with `left: [(Native, Err(NeedsGesture { service: 13, method:
4 })), (Web, Err(NeedsGesture { service: 13, method: 4 }))]` and `right: [(Native, Err(NeedsAnswer {
service: 13, method: 4 })), (Web, Err(NeedsAnswer { service: 13, method: 4 }))]`. `dispatch.rs` read
sha256 `5c783d20…7f755b3c` before the plant and after its restore, byte for byte.

Each red above came from the behaviour's own assertion, the first in its test, and never from a
compile error or an examined count.
