# SPEC-480: each runaway mutant of the release run ends at a progress bound and an existing test kills it

- **Issue:** #822, the release whose run measured six Python legs dead with no report and one Rust
  mutant stopped by the memory scope; refers, closes nothing. Refers #322 (every survivor killed or
  recorded equivalent) and #825 (the Python legs' fit).
  **Context(s):** the mutation scripts (`scripts/`), the ingest crate's memory-state reader
  (`crates/ingest`) and `formal/`; no workflow and no file of the mutation harness changes.
- **Decided by:** ADR-497 (a runaway mutant of the release run is killed by a progress bound that is
  the identity on every admitted input).
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 11, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-480.md`. **Mutation band:** S48000-S48099. **Models:** the new Lean entry
  `lean/TokenProgress`.

## 1. The problem, measured

Every figure below was read from run 39033655190 (its six dead legs ran 755 s to 1327 s), its jobs' logs and artifacts,
or from `git show 36c4783a:<path>` at #822's head `36c4783a29c44d2ffafc987803ccb1742a2c2cfc`.

- **Six Python legs left no report.** `mutation-python (150)`, `(152)`, `(154)`, `(156)`, `(157)`
  and `(158)` each died in the step that runs `scripts/mutation_python.py run ... --failfast`, the
  log ending with the runner's shutdown signal and exit 143 and no line after the step's start, in
  both attempts. Attempt 1's jobs 117399178683, 117399178676, 117399178643, 117399178597, 117399178680, 117399178691 ran 781 s, 822 s, 817 s, 896 s, 1271 s and 1025 s;
  attempt 2's jobs 117533719194, 117533719226, 117533719211, 117533719241, 117533719224, 117533719259 ran 846 s, 837 s, 788 s, 755 s, 1327 s and 1280 s (each the run step's seconds, from its log).
- **Each leg's third dealt mutant never advances its loop.** All 78 mutants dealt to the six are in
  `scripts/mutation-verdict.py`, 13 per leg. The third of each is in `rust_tokens` (:513-539):
  `532:25` `and` to `or`, `532:53` `==` to `!=`, `534:24` `<` to `>=`, `534:62` `or` to `and`,
  `534:76` `==` to `!=`, `534:79` `"_"` to `""`. Under each, a token start (punctuation for line
  532, a word character for `534:24` and `534:62`, a word opening with `_` for `534:76` and
  `534:79`) leaves `stop == at`, and `tokens.append((at, stop)); at = stop` (:537-538) appends
  without end.
- **The harness bounds time, not memory.** `scripts/mutation_python.py` runs each mutant in a child
  of its own session (:1039-1046) under a wall-clock bound derived from its control run (:1138),
  and kills the child's process group when it passes (:1047-1077). Nothing bounds the child's
  memory. The sibling legs 151, 153, 155 and 159 of run 39033655190 had bounds of 1138 s, 800 s, 932 s and 763 s.
  Their own `rust_tokens` mutants advance and reported, the slowest whole in 302.2 s (run 39033655190, leg 153);
  by their pace the third mutant of a dead leg started near 440 s to 880 s, before any bound could pass (run 39033655190).
- **The verdict hid the six under the class FAIL.** The run's mutation verdict read the scripts
  class FAIL with 804 unexplained survivors of 6617 examined, the oracle class VOID for the six
  missing reports, and six scripts VOID lines below the FAIL: a class FAIL outranks its VOID lines
  (`Verdict.close`, mutation-verdict.py:1495-1505).
- **One Rust mutant ran until the memory scope stopped it.** Rust shard 140 (job 117399177077) read `crates/ingest/src/memory_state.rs:85:16`, `+=` to `*=` in `leniently`, stopped 754.018 s in;
  the class read it as a memory finding, neither caught nor timed out. In `leniently` (:41-89),
  `at *= width` with `at` at 0 on a text that opens with `{` never advances, and `out` grows.
- **The plan has room for one mutant.** The release plan dealt `scripts/mutation-verdict.py`'s
  2843 mutants over 219 runs of 13 (215) and 12 (4), so four more fit before a run is added; it
  dealt 304 Python legs against `PYTHON_RELEASE_CEILING` (mutation-verdict.py:131), 305.
- **The Rust population the plan tokenizes.** At `36c4783a`, 340 paths match the `rust` class's
  pattern (`RUST`, mutation-verdict.py:115); the plan tokenizes each changed one whole through
  `rust_test_lines` (:633-637), called at :866.

## 2. Requirements

R1. `rust_tokens` refuses a reading that does not advance. Right after
`stop = at + 1 if stop is None else stop` it holds `if stop <= at:` followed by
`raise RuntimeError(at)`, so every pass of its loop moves `at` forward by at least one character or
raises naming the position. Nothing else in `rust_tokens` changes.

R2. The guard is the identity on the release population. For every path of the tree that the
verdict's own `classify` reads as `rust` (the pattern `RUST`, `crates/<crate>/src/**/*.rs`), read
whole at the head under test, the JSON text of the guarded function's token list, encoded as UTF-8,
is byte-equal to that of a verbatim copy of the base function (`36c4783a`, mutation-verdict.py:513-539)
held in the test module. At #822's head the population is the 340 sources of section 1, which hold
every Rust file the release plan tokenizes. The test prints its examined count and refuses 0. A
local run may add exclude pathspecs at its invocation through `RUST_CENSUS_PATHSPECS`, one per line;
the test prints how many it applied, and refuses any when `CI` is set, so CI reads the population
whole.

R3. Each of the six runaway mutants is killed by a test that reads a literal text holding its token
class and asserts the whole token list: a punctuation text for S48001 and S48002, a word text for
S48003 and S48004, a word opening with `_` for S48005 and S48006. Under each mutant the guard
raises at the stalled token and the test fails.

R4. The guard's own listed mutant, `<=` to `>`, is killed by the word-text test of R3, which then
raises at its first token, and a non-vacuity control shows that test's text meets the guard once per
token: a spy wrapping the real `rust_literal` counts one reading per token, and every token
advances, so each reading reaches the guard with its stop past its start. A bounded stand-in reader
that returns its start shows the guard refuses at the position where a reading stays.

R5. `leniently` in `crates/ingest/src/memory_state.rs` runs at most `bytes.len()` passes: its
`while at < bytes.len() {` becomes `for _ in 0..bytes.len() {` whose body opens with
`if at >= bytes.len() { break; }`, the old body unchanged after it. The bound is the identity on
every admitted input: over 4096 texts drawn from a fixed seed, `parse` returns exactly what a
verbatim copy of the base reader returns (`36c4783a`, memory_state.rs:41-124), and the test prints
its examined count and the count that parsed to a state, refusing 0 for either.

R6. Row S48007 mutates `at += width;` to `at *= width;` in `src/memory_state.rs`, and the existing
`memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field` kills it: with the
bound the reading ends with a wrong text and `parse` answers none.

R7. The Lean entry `lean/TokenProgress` (`formal/lean/Formal/TokenProgress.lean`) covers
`rust_tokens` and `block_end` and proves that, when a block comment's end lies past its start, the
guarded loop ends within the text's length plus one passes and returns at most the text's length of
tokens, whatever the literal reader answers. Its witnesses are the base loop under row S48001's
mutant on `{` and under row S48003's on `a`, and its vectors hold `rust_tokens` to the proof.

R8. No file of the mutation harness changes: `scripts/mutation_python.py` and
`scripts/mutation-python.json` stay as at the base, and `PYTHON_RELEASE_CEILING` stays 305. The
census of R2 lives in a module outside the verdict file's map, so no mutant run executes it; the
verdict file's dealt figure becomes 2844, within its 219 runs.

R9. After the land, the next push-triggered run of #822 reads each of the six legs with a report
and its examined count (section 7). This is the delivery's measure after the land, decided there,
and no criterion of the pull request that lands it.

## 3. Acceptance criteria of SPEC-480

| id | criterion | decided by |
|---|---|---|
| A1 | a reading that does not advance is refused at its start: over `" {"` with a stand-in `rust_literal` that returns its start, `rust_tokens` raises `RuntimeError(1)` after one reading (R1, R4) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_reading_that_does_not_advance_is_refused_at_its_start` |
| A2 | a punctuation character is one token: `"{};"` reads `[(0, 1), (1, 2), (2, 3)]` (R3; kills S48001, S48002) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_punctuation_character_is_one_token` |
| A3 | a word is one token to its end: `"ab c1"` reads `[(0, 2), (3, 5)]` (R3, R4; kills S48003, S48004 and the guard's own mutant) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_is_one_token_to_its_end` |
| A4 | a word that opens with an underscore is one token: `"_a b_"` reads `[(0, 2), (3, 5)]` (R3; kills S48005, S48006) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_that_opens_with_an_underscore_is_one_token` |
| A5 | A3's text meets the guard once per token: a spy wrapping the real `rust_literal` counts 2 readings, and both tokens advance (R4, the non-vacuity control of A3's kill) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_each_token_of_the_word_text_meets_the_guard_once` |
| A6 | every Rust source of the release population reads byte-equal tokens with the guard and with the base copy, its examined count printed and 0 refused (R2) | `python3 -m unittest discover -s scripts/tests -p test_rust_tokens_identity.py -k test_every_rust_source_reads_the_same_tokens_with_the_guard` |
| A7 | the bounded reader parses every seeded text as the base reader does, its examined and parsed counts printed and 0 refused (R5) | `cargo test -p deck-streak-ingest --test memory_state_lenient_bound -- --exact the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does` |
| A8 | `rust_tokens` agrees with every vector of `lean/TokenProgress`, each vector's tokens no more than its text's length (R7) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_tokens_agree_with_every_vector_of_their_proof` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_reading_that_does_not_advance_is_refused_at_its_start
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_punctuation_character_is_one_token
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_is_one_token_to_its_end
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_that_opens_with_an_underscore_is_one_token
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_each_token_of_the_word_text_meets_the_guard_once
A6: python3 -m unittest discover -s scripts/tests -p test_rust_tokens_identity.py -k test_every_rust_source_reads_the_same_tokens_with_the_guard
A7: cargo test -p deck-streak-ingest --test memory_state_lenient_bound -- --exact the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_tokens_agree_with_every_vector_of_their_proof
```

A1 is red first: at the base `rust_tokens` holds no guard, so the stand-in's reading at position 1
repeats until the stand-in's own call limit fails the test with its sentinel, an assertion bounded
in memory, never a missing name or an empty selection. The other seven pin behaviour the base
already has and are recorded `not red`, each with its reason: A2, A3 and A4 are the killers of rows
S48001 to S48006 (A3 also of the guard's own mutant), whose mutants the base cannot run to an end;
A5 is A3's non-vacuity control; A6 and A7 are the identity proofs, equal at the base by
construction, the second with row S48007 as its red; and A8 holds the code to the Lean entry's
vectors. A6, A7 and A8 enumerate what they read, print the count they examined and refuse zero.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | the verdict | the progress guard in `rust_tokens`: two lines after `stop = at + 1 if stop is None else stop` (R1) |
| `scripts/tests/test_mutation_verdict_python_kills.py` | tests | class `RustTokensEndOnEveryInput`: A1 to A5 and A8 |
| `scripts/tests/test_rust_tokens_identity.py` | tests | new: A6, the census over the release population, outside the map |
| `crates/ingest/src/memory_state.rs` | ingest | the pass bound in `leniently` (R5) |
| `crates/ingest/tests/memory_state_lenient_bound.rs` | tests | new: A7, with the verbatim base reader and the seeded population |
| `scripts/mutation-rows.d/S48000-S48099.json` | rows | new: S48001 to S48006 in `SCRIPT_MUTATIONS`, S48007 in `MUTATIONS` |
| `formal/lean/Formal/TokenProgress.lean` | formal | new: the entry, its claim, its theorem and its two witnesses |
| `formal/lean/Formal/TokenProgressVectors.lean` | formal | new: the entry's vectors writer |
| `formal/lean/Formal/Vectors.lean` | formal | one import and one arm for the writer |
| `formal/vectors/token-progress.jsonl` | formal | new: the vectors, written by the writer |
| `docs/specs/SPEC-480-each-runaway-mutant-of-the-release-run-ends-at-a-progress-bound-and-an-existing-test-kills-it.md` | docs | new: this SPEC |
| `docs/decisions/ADR-497-a-runaway-mutant-of-the-release-run-is-killed-by-a-progress-bound-that-is-the-identity-on-every-admitted-input.md` | docs | new: the decision |
| `docs/schematics/mutation-testing-python.md` | docs | section 11 appended, insert-only |
| `docs/red-first/SPEC-480.md` | docs | new: the red-first record |
| `changelog.d/release-mutation-legs-480.md` | docs | new: the changelog fragment |

## 5. What this does NOT cover

- The per-mutant memory bound in `scripts/mutation_python.py` (ADR-497 D7), held until the release pull request merges (#822).
- The scripts class's unexplained survivors: ADR-451's kill parts 2 to 6 (SPEC-438 to SPEC-442), each re-derived from the cure's measure after this land and never from an earlier part's build (#822).
- The 107 survivors in `scripts/mutation_rows.py`, which an open change to that file and its test may move before part 2 reads them (#954).
- The rust shards' red baselines read VOID under the class FAIL (#822).
- The open change to the verdict script and the Python schematic, which shares two of this manifest's files; whichever lands second re-reads the other's lines (#873).
- Every survivor killed or recorded equivalent, which is the closing run's (#322), and the Python legs' fit to their job (#825).

## 6. Risks

- A6 reads every Rust source of the tree, so a source added later in which a reading stays would
  raise in the guarded function where the base copy loops; no base reading stays on any text,
  since each branch advances, and A6 runs in the job that runs `scripts/tests`, which reads it.
- The guard moves every later line of the verdict file down two lines: rows anchor by text, and the
  pull request's own mutation verdict reads any record or row it moves as STALE.
- The stride deal places mutants on legs by listing position, so the six may sit on other legs
  after the cure; section 7 reads the six named legs, and any Python leg with no report reads VOID
  in the same verdict.
- A census over the test modules may refuse a module outside the verdict file's map; the job that
  runs `scripts/tests` reads it.
- `lean/TokenProgress` covers `rust_tokens` and `block_end`, so an edit of either, #873's included,
  reads the entry STALE until it is re-ported and stamped.

## 7. The cure's measure after the land

Who: the next push-triggered run of #822 after this pull request lands, the run ADR-497 D5 reads
as the cure's measure, read by the legs' names. When: once that run completes. Record: each of `mutation-python (150)`, `(152)`,
`(154)`, `(156)`, `(157)` and `(158)` uploads its report, and the run's mutation verdict prints each
leg's examined count with no `no report` line for any of them. It is no acceptance criterion: the
pull request's own checks cannot run it, and it is read after the land.

## 8. Mutation rows S48001 to S48007

Each find occurs once in its target at `36c4783a`, and each killer is one test.

| id | table | target | find | replace | killer |
|---|---|---|---|---|---|
| S48001 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `if stop is None and (char.isalnum() or char == "_"):` | `if stop is None or (char.isalnum() or char == "_"):` | A2 |
| S48002 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `if stop is None and (char.isalnum() or char == "_"):` | `if stop is None and (char.isalnum() or char != "_"):` | A2 |
| S48003 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop >= len(text) and (text[stop].isalnum() or text[stop] == "_"):` | A3 |
| S48004 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop < len(text) and (text[stop].isalnum() and text[stop] == "_"):` | A3 |
| S48005 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop < len(text) and (text[stop].isalnum() or text[stop] != "_"):` | A4 |
| S48006 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == ""):` | A4 |
| S48007 | `MUTATIONS` | `ingest`, `src/memory_state.rs` | `at += width;` | `at *= width;` | `memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field` |

The six Python rows mirror the release run's six runaway mutants. Under the guard each ends at its
first stalled token; without it each runs until memory ends it, so none of them is installed where
the guard is absent. S48007 is installed only where the bound of R5 holds.
