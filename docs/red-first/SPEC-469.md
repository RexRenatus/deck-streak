# Red-first record: SPEC-469

SPEC-469 (R1 to R7, A1 and A2). Both criteria are MUTATION COVERAGE, not red first: the prune
pin's and the verdict pin's code at the base already answers every vector the Lean port
`lean/AgentPinGroups` writes, so neither test can fail at the base for its criterion's reason. The
only red the base could show is the vectors file's absence, which examines nothing (ADR-486 D5).
Each test's reds are its rows, each a mutant of an existing pin line in
`crates/agent/tests/runs.rs` or `crates/agent/tests/verdict.rs`, proved killed by hand below and
read by id in CI.

```text
A1: cargo test -p deck-streak-agent --test runs -- --exact the_prune_pin_answers_every_vector_of_its_lean_port
A2: cargo test -p deck-streak-agent --test verdict -- --exact the_verdict_pins_shebang_check_answers_every_vector_of_its_lean_port
```

```red-first
A1: not red: pins the base's counts against the Lean port's vectors; its reds are hand rows S46902 and S46904, killed by A1
A2: not red: pins the verdict pin's shebang check against the Lean port's vectors; its reds are hand rows S46901 and S46903, killed by A2
```

Both fence lines pass at the commit that adds the tests: each selects `running 1 test` and reads
`1 passed`, and run with `--nocapture` A1 prints `examined 672 vectors` and A2 prints
`examined 96 shebang vectors`. Each refuses any population but its own: A1 asserts 96 shebang
vectors and 576 count vectors, and A2 asserts 96 shebang vectors, 32 of them refused, beside the
576 count vectors it counts by their rule and does not read.

## The Lean claim's red

The Lean claim's red is its two witnesses, each a variant of one of this delivery's rows proved to
break its theorem at one input. The entry, its writer and its first vectors were committed
together at 09bd6d3eebae848bac0070baa59a29fae06a74c6, every cover's digest still unstamped. The
formal check of that commit, run by hand with the repository's pinned toolchain, built both
witnesses, `a_check_that_reads_the_byte_order_mark_misses_a_shebang` (row S46901's mutant, at the
source U+FEFF then `#!`) and `a_word_count_that_reads_doc_attributes_parts_from_the_run_count`
(row S46902's mutant, at `#` then a bracket group opening with `doc`), and found no hole and no
vectors drift; its only findings were the twelve unstamped covers, read `STALE` on both theorems.
The theorems' proofs were read clean only after the stamp. No CI job checks a Lean entry in this
repository, so this reading is the build's own and is not a fence line.

## The rows' proofs

Rows S46901 to S46904 (`scripts/mutation-rows.d/S46900-S46999.json`) were proved by hand over the
tree that adds the two tests, one row at a time: the target's sha256 recorded, the row's
replacement installed once by script (its find occurring once), the killer run by `--exact`
(`running 1 test`) and its failing assertion read, the other new test run under the same mutant,
then the saved bytes written back, the sha256 compared equal, the file touched, and the test
targets rebuilt before the next row. Each killer failed on the assertion its row names, never on
a count assertion.

- S46901, `tests/verdict.rs` (sha256 `ab894b60814ffa6e5b465efb77a465b3ff67de1bd778f2e97a9f157440dad5e6`
  before and after): A2 failed at the first one-mark `#!` vector, on the check's own assertion:
  `assertion 'left == right' failed: the shebang check parted from its port: "\u{feff}#!"`,
  `left: false`, `right: true`. A1, which does not read `tests/verdict.rs`, passed under it.
- S46902, `tests/runs.rs` (sha256 `ce83ab9005b252fd0a685fad11b98f873895c045f56e03fd52e79a52579a032f`
  before and after): A1 failed at a skipped doc group with no call beside it, on the sentences
  `prune_pin_problems` names: `the counts parted from their port: # [ doc = sqlx : : query ! (
  "DELETE FROM agent_runs WHERE created_at < ?1" ) ]`, the word count reading the group's word
  where the port reads none. A2 passed under it.
- S46903, `tests/verdict.rs` (same sha256 before and after): A2 failed at the first `#![` vector,
  no mark and no tail: `the shebang check parted from its port: "#!["`, `left: true`,
  `right: false`. A1 passed under it.
- S46904, `tests/runs.rs` (same sha256 before and after): A1 failed at the first count vector, no
  prefix, a parenthesis group, no head, depth 1 and no call beside, on the run count's own
  assertion: `the run count parted from its port: ( sqlx : : query ! (
  "DELETE FROM agent_runs WHERE created_at < ?1" ) )`, `left: 0`, `right: 1`. A2 passed under it.
