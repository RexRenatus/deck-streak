# Red-first record: SPEC-480

SPEC-480 (R1 to R9, A1 to A8). The SPEC, ADR-497 and section 11 of the Python schematic were
committed first, in aaf5dd0c. The tests of A1 to A7 were committed next and alone, in 54e30834,
over the base's production code unchanged: `rust_tokens` held no progress guard, and `leniently`
looped `while at < bytes.len()`. A1's red is therefore the stand-in's own sentinel, an assertion
bounded in memory, never a missing name or an empty selection. The guard followed in 390a800d,
the pass bound in 84bcb25e, the rows in c0b0c340, and the Lean entry with A8 in 18f3893b, its
covers stamped in f9a8fb93 and its vectors written in d8609712.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_reading_that_does_not_advance_is_refused_at_its_start
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_punctuation_character_is_one_token
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_is_one_token_to_its_end
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_that_opens_with_an_underscore_is_one_token
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_each_token_of_the_word_text_meets_the_guard_once
A6: python3 -m unittest discover -s scripts/tests -p test_rust_tokens_identity.py -k test_every_rust_source_reads_the_same_tokens_with_the_guard
A7: cargo test -p deck-streak-ingest --test memory_state_lenient_bound -- --exact the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_tokens_agree_with_every_vector_of_their_proof
```

```red-first
A1: red at 54e308341cc8136dfd44af18aa31e3ce0191956a: FAIL: test_a_reading_that_does_not_advance_is_refused_at_its_start (test_mutation_verdict_python_kills.RustTokensEndOnEveryInput.test_a_reading_that_does_not_advance_is_refused_at_its_start) AssertionError: rust_tokens read position 1 64 times: its loop does not end
A1: green at 390a800d0cdc09ab1eee51b964dabc54fe9d077e
A2: not red: pins the token list the base already reads; its reds are rows S48001 and S48002, killed by A2
A3: not red: pins the token list the base already reads; its reds are rows S48003 and S48004 and the guard's own listed mutant, killed by A3
A4: not red: pins the token list the base already reads; its reds are rows S48005 and S48006, killed by A4
A5: not red: A3's non-vacuity control: its text meets the guard once per token, each reading past its start
A6: not red: the identity census, equal at the base by construction; it holds the guard to the release population
A7: not red: the identity of the pass bound, equal at the base by construction; its red is row S48007, killed by memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field
A8: not red: holds rust_tokens to the vectors of lean/TokenProgress
```

Each line above was read by running its one test by name, in a checkout of the commit it names,
before the pull request's one push; the pull request's own checks read every criterion again at
its head. A1 ran `Ran 1 test` at 54e30834 and failed with the stand-in's assertion above, after
one reading at position 1 had repeated until the stand-in's limit of 64 readings: the base's loop
appended the token `(1, 1)` and moved to 1 on each pass. At 390a800d it ran `Ran 1 test` and passed:
the guard raised `RuntimeError(1)` after one reading. A2, A3, A4 and A5 each ran by name at
54e30834, `Ran 1 test`, and passed; at 390a800d, 84bcb25e, c0b0c340 and 18f3893b they passed
inside the whole run of their module's classes. A6 and A7 passed at each of 54e30834, 390a800d,
84bcb25e, c0b0c340 and 18f3893b, and A8 passed at 18f3893b and at d8609712.

The counts each census printed (round protocol P8):

- A5, the non-vacuity control of A3's kill: a spy wrapping the real `rust_literal` counted 2
  readings over `"ab c1"`, and both tokens end past their start.
- A6: the positive control first, a guarded side that drops one token, read the one fixture path
  as differing over 6 base tokens; then the census printed `examined 332 rust sources` and
  `compared 360180 tokens over 332 sources` at 84bcb25e, c0b0c340 and 18f3893b, with no source
  differing (`360164` at 54e30834 and 390a800d, before the pass bound added its lines). That run
  passed four exclude pathspecs at its invocation (`added pathspecs 4`), which left 332 of the 340
  sources the `rust` class reads; under `CI` the census refuses any pathspec, so CI reads all 340.
- A7: `examined 4096 texts, 410 parsed to a state`, with no text whose state differs from the
  base reader's.
- A8: `examined 12 token progress vectors`, each text's tokens equal to its vector's and no more
  of them than the text's length.

Row S48007 was proved by hand at 84bcb25e, the head that holds the pass bound, its target's
sha256 recorded first. `at *= width;` was installed once by script in
`crates/ingest/src/memory_state.rs`, in which `at += width;` occurs once. The killer,
`memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field`, then ran
`running 1 test` and failed at its `expect`, which found no state (`a state`, at
`crates/ingest/tests/memory_state_boundaries.rs:137:80`): the reading ended after its text's
length of passes with a wrong text, and `parse` answered none. A7 ran `running 1 test` under the
same mutant and failed too, by a panic in the mutated reader, which sliced inside a multibyte
character (`crates/ingest/src/memory_state.rs:71:30`) before the comparison ran. Both runs ended.
The target was restored by writing its saved bytes back, its sha256 compared equal, the file was
touched and rebuilt, and both targets then passed, 14 tests and 1. Rows S48001 to S48006 mutate
`scripts/mutation-verdict.py`, whose mutants are never installed outside CI: their proof is the
anchor census, each find occurring once in its target, the band's duplicate-key check, and CI's
`mutation-rows` line for each, read by its stem.

A1's green names the commit that adds the guard, and HEAD agrees with it for A1:
`git diff --quiet 390a800d HEAD -- scripts/mutation-verdict.py` exits 0, and
`git diff --numstat 390a800d HEAD -- scripts/tests/test_mutation_verdict_python_kills.py` reads
`20 0`, the 20 added lines being A8 alone. No commit between the red and the green changes a test.
