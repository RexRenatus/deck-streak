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
