---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The prune pin's token groups are proved in Lean over a port that takes any lexer's tokens, tied to both pins by vectors

## Context and Problem Statement

The prune pin and the verdict pin in `crates/agent/tests` read a Rust source as tokens (SPEC-043
R16c, R12c; ADR-293). Since #414 they remove one byte order mark, refuse a source that then opens
with `#!` and not `#![`, and the prune pin's two counts, the run count and the word count, skip
every `doc` attribute group. Issue #474 owes a Lean proof of the property: for every source text,
after one byte order mark is removed, a source that opens with `#!` and not `#![` is refused;
otherwise the run count and the word count descend into exactly the same token groups, so no count
reads a group the other skips. Generated populations check it today (SPEC-043 section 11). The
repository's formal-check settings are on the development branch (#468, SPEC-295). What does the
entry port, what does it claim, what breaks it, and how do the port and the code stay tied?

All citations are read at `36c4783a29c44d2ffafc987803ccb1742a2c2cfc`.

## Decision Drivers

- The claim is the issue's: one text step (the byte order mark and the shebang refusal) and one
  token step (both counts descend into the same groups).
- A port is the code branch for branch, named after the item it ports, so a change to the code
  stales the entry and the ratchet reads a re-port, not a new claim.
- The tokens come from a third-party lexer the repository does not own; what that lexer does is
  outside the claim, and so is what rustc does after it lexes (#444).
- No pin is weakened and no examined count moves.
- The delivery fits one builder and lands on the development branch after the release pull
  request #822 merges.

## D1. The mechanism, measured

The prune pin's reader, in `crates/agent/tests/runs.rs`:

1. `prune_pin_problems` (`runs.rs:165-199`) first asks `opens_with_a_shebang` (`runs.rs:166`); a
   source it answers true of gets one problem, the shebang sentence, and nothing is lexed
   (`runs.rs:167-170`).
2. `opens_with_a_shebang` (`runs.rs:50-53`) removes one U+FEFF with `strip_prefix` and answers
   whether the rest starts with `#!` and not `#![`.
3. Otherwise `tokens_of(source)` (`runs.rs:172`, defined `runs.rs:38-43`) lexes the source as it
   was handed in, the byte order mark included, and a source that does not lex gets one problem
   (`runs.rs:174`). Whether the lexer itself removes a byte order mark is NOT MEASURED by the
   reads this decision rests on; the claim does not depend on it (D2).
4. The one token list feeds both counts: `statements_run_in(&tokens)` (`runs.rs:177`) and
   `code_texts(&tokens, &mut texts)` (`runs.rs:184`), whose texts the word count splits and
   counts (`runs.rs:185-192`).
5. The run count (`runs.rs:145-160`) skips a group `is_doc_attribute(&tokens[..at], group)` holds
   of (`runs.rs:150`), and otherwise descends into `tokens_in(group)` and counts the group as a run
   when `ends_in_the_query_macro` and `opens_with_the_statement` hold (`runs.rs:151-156`).
6. The word count's walk (`runs.rs:97-108`) pushes each identifier's name and each string-like
   literal's cooked text, and descends into a group only when `is_doc_attribute(&tokens[..at],
   group)` does not hold (`runs.rs:102-104`).
7. `is_doc_attribute` (`runs.rs:85-93`) reads a bracket group after `#`, or after `#` and `!`,
   whose first token is an identifier named `doc`, its raw prefix removed by `name_of`
   (`runs.rs:61-65`).

So the two counts share one reader: one lex, one token list, one doc-attribute predicate called
with the same `before` slice, and one `tokens_in`. The two pins carry two readers: `verdict.rs` has
its own `tokens_of` (`verdict.rs:10-15`, a source that does not lex reads as no tokens) and its own
copy of `opens_with_a_shebang` (`verdict.rs:22-25`, byte for byte the prune pin's body), called
first by `verdict_pin_problems` (`verdict.rs:157-162`). The verdict pin's one count that descends,
`declarations` (`verdict.rs:45-59`), descends into every group, a doc attribute included, and
`attributes_of` (`verdict.rs:96-125`) reads the top level only; no shared helper lives in
`crates/agent/tests/support/mod.rs`, and neither pin declares `mod support`.

## D1. Chosen against

- Chosen: the mechanism above, each step cited, because it is what the code at the base does.
- Rejected, reading the issue's "skip every doc attribute group in every count" as true of both pins, because `verdict.rs:45-59` measures otherwise: the verdict pin's declaration count reads doc groups, which fails closed (a copy of the enum written in a doc attribute is a second declaration and is refused).

## D2. The proof

The entry `lean/AgentPinGroups` ports, branch for branch, the prune pin's reader over a token tree
type: an identifier with its name as written, a punctuation character, a literal with its kind
(string, byte string, C string, other) and its cooked text, and a group with its delimiter and the
tokens it holds. The lexer is a parameter: the pin takes any function from text to an optional
token list. The ports are named after their items (`opensWithAShebang`, `isPunct`, `nameOf`,
`tokensIn`, `isDocAttribute`, `textOf`, `endsInTheQueryMacro`, `opensWithTheStatement`,
`statementsRunIn`, `codeTexts`, `prunePinProblems`). Each walk returns its count and the list of
groups it descended into, each group named by its path of positions; the trace is emitted at the
walk's one recursive call, so the count and the trace come from one recursion.

The claim, in the issue's words, is two theorems, each stated once as a `Prop`:

- `a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one`: for every lexer and every
  source text, the pin refuses the source as a shebang exactly when, after one byte order mark is
  removed, it opens with `#!` and not `#![`. The right side is stated over list equations, apart
  from the port's prefix test.
- `the_run_count_and_the_word_count_descend_into_the_same_groups`: for every token list a lexer can
  hand the pin, the run count's groups equal the word count's, in order.

Their conjunction is the issue's claim. Each has one witness, a variant of one of this delivery's
own rows (D5), proved to violate the claim at one input: `a_check_that_reads_the_byte_order_mark_misses_a_shebang`
(the check with the byte order mark kept, row S46901's mutant, at a source of U+FEFF then `#!`),
and `a_word_count_that_reads_doc_attributes_parts_from_the_run_count` (the word count's guard
dropped, row S46902's mutant, at `#` and a bracket group opening with `doc`). The code at the base
satisfies the claim since #414, so no witness can port the base itself; the witnesses are
mutants the rows prove killed.

Vectors tie the port and the code. The writer prints 96 shebang vectors (a source and whether
`opensWithAShebang` refuses it) and 576 count vectors (a source rendered from a generated token
tree and the run count and the word count the port answers on that tree). The prune pin's test
reads both kinds and the verdict pin's test reads the shebang kind, so both copies of the check are
tied to one port. Both theorems enter at `ramp=report`.

## D2. Chosen against

- Chosen: an abstract lexer and a traced port of the walks, because the claim then holds for every token list any lexer can produce, and each count's value and trace come from one ported recursion.
- Rejected, porting the third-party lexer so the claim is over text alone, because it ports thousands of lines DeckStreak does not own, and both counts read the one token list that lexer returns, so no lexer can make them disagree.
- Rejected, stating the token step through planted probes alone (a call in each group counted by both counts or by neither), because a count that skips a group holding no probe is not seen; the vectors use probes, the theorem does not.
- Rejected, a TLA+ model, because there is no interleaving: one test reads one immutable token list twice, in sequence.
- Rejected, `ramp=block` at entry, because a theorem blocks only after the configured run of clean landings.

## D3. Production or test

No production line changes and no existing test line changes. Two tests are added, one in each
pin's file: `runs.rs` gains `the_prune_pin_answers_every_vector_of_its_lean_port` and `verdict.rs`
gains `the_verdict_pins_shebang_check_answers_every_vector_of_its_lean_port`. They pin that each
pin's code answers every vector its port wrote: the shebang refusal of both copies, the run count
read directly, and both counts read through the sentences `prune_pin_problems` names them in. Each
prints the number of vectors it examined and refuses any other number than the SPEC's. No count's
examined figure moves: the existing populations (16, 32 and the others) are untouched. Rows
S46901 to S46904 guard the new tests.

## D3. Chosen against

- Chosen: two readers kept, two vector tests added, because the vectors tie both copies of the check to one port and no existing anchor moves.
- Rejected, one shared reader in `tests/support/` replacing the two copies, because it moves the anchors of rows S04356 to S04359 into another file and pulls the agent's fakes into both pins, to remove a duplicate the vectors already tie.
- Rejected, the verdict pin's declaration count skipping doc groups as the prune pin's counts do, because it would let a copy of the enum inside a doc attribute stop being a second declaration, a weaker pin; it is left unchanged.

## D4. Shape

One delivery under SPEC-469 and this ADR, rows in the band S46900 to S46999, the red-first record
`docs/red-first/SPEC-469.md`, a new schematic `docs/schematics/agent-pin-token-groups-proof.md`,
and one insert-only section appended to SPEC-043 after its last line, saying that R16c's claim is
proved by this entry and that its two new tests and rows are SPEC-469's. The Lean entry needs no
change to `formal/lean/lakefile.toml` (its glob builds every module under `Formal/`) or to
`formal/lean/Formal.lean`; `formal/lean/Formal/Vectors.lean` gains one import and one arm.

## D4. Chosen against

- Chosen: one delivery, because the port, its vectors and the two tests that read them must land together or the vectors are read by nothing.
- Rejected, two deliveries (the Lean entry first, the Rust tests second), because the first would land vectors no test reads and a port no test ties to the code.
- Rejected, amending ADR-293 as well, because this decision is new and stands alone; ADR-293 decided the reader, this ADR decides its proof.

## D5. Tests and rows

The two Rust tests are MUTATION COVERAGE, not red first: the code at the base already answers
every vector, so the only red at the base would be the vectors file's absence, which is a red for
another reason than the criterion's. Each test's red is its rows, each a mutant of an existing pin
line that the new test kills:

- S46901: the verdict pin's check keeps the byte order mark (`verdict.rs:23`).
- S46902: the word count reads a doc attribute's tokens (`runs.rs:102`).
- S46903: the verdict pin's check refuses an inner attribute `#![` too (`verdict.rs:24`).
- S46904: a call whose tokens open a group is not a run (`runs.rs:117`).

The Lean claim's red is its witnesses: each builds, the claim's negation proved for its variant,
before the theorem is proved. No CI job checks a Lean entry here, so the formal check is run by
hand at the build and its card quoted; it is not a fence line. One push: no red needs a CI reading,
since every criterion is not red and its rows are proved by hand and then read by name in CI.
Model: opus, for a nested-inductive proof whose ports must stay branch for branch, and four rows
each proved by a mutate, run and sha256 restore.

## D5. Chosen against

- Chosen: not-red tests with their rows as reds, the witnesses as the claim's red, and one push, because each red then fails every time for the criterion's reason.
- Rejected, recording the missing vectors file as each test's red at the base, because a red caused by an absent fixture examines nothing.
- Rejected, a fence line naming the formal check, because no gate runs it, so the line would name a verdict nobody earns.
- Rejected, sonnet, because a stop mid-proof can leave a mutated pin file.

## D6. What this does not decide

What rustc does after it lexes (#444) stays outside the claim. Other token readers in the tree keep
their own readers and are not ported (#297, #444). The verdict pin's declaration count is not
ported (#474 names the run count and the word count). The CI readings of the rows and of the
red-first class are read after #822 merges.

## D6. Chosen against

- Chosen: these exclusions, each cited, because each is a different claim or a later measurement.
- Rejected, porting the verdict pin's declaration count in the same entry, because it has no second count to agree with and it fails closed on doc groups.

## Decision Outcome

Chosen: D1 to D6 above. The entry `lean/AgentPinGroups` proves the issue's claim over a faithful
port with an abstract lexer, its two witnesses are mutants rows S46901 and S46902 prove killed, and
its vectors are read by one new test in each pin's file.

### Consequences

- Good, because the property the populations sampled is proved for every token list and every
  source text, and a change to any ported item stales the entry.
- Good, because both copies of the shebang check are tied to one port by the same vectors.
- Bad, because the proof says nothing about whether the lexer's tokens are rustc's; ADR-293's
  measured populations hold that, and what rustc does after it lexes is #444's.
- Bad, because no CI job checks the entry: it is read by hand at each delivery that touches it.

### Confirmation

SPEC-469's A1 and A2, the formal check of `lean/AgentPinGroups` with both witnesses built and both
theorems clean, and rows S46901 to S46904 KILLED by the tests they name.

## What would make this wrong

- A count added to the prune pin that reads a group by another predicate: the port would no longer
  be the code, and the entry goes `STALE` until it is re-ported.
- A lexer that hands the counts two different token lists: the port's pin hands both one list, as
  `runs.rs:172-184` does today.
- The verdict pin gaining a second descending count: its groups would then owe the same claim.

## More Information

Issue #474; #414; #444; #468; SPEC-043 sections 9 to 12; ADR-293; SPEC-295 and ADR-295.
