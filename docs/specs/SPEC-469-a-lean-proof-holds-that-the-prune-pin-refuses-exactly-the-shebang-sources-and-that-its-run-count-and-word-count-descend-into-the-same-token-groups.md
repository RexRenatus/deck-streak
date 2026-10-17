# SPEC-469: a Lean proof holds that the prune pin refuses exactly the shebang sources and that its run count and word count descend into the same token groups

- **Issue:** #474. **Context(s):** `deck-streak-agent` (`crates/agent/tests/`), `formal` (`formal/`), `repo`
  (`scripts/mutation-rows.d/`, `changelog.d/`) and `docs`.
- **Decided by:** ADR-486 (this SPEC's own: the mechanism, the proof, test-only, one delivery, the reds, the
  exclusions, and what each was chosen against). It amends SPEC-043 insert-only (section 13).
- **Status:** in build, as one delivery; it holds `docs/red-first/SPEC-469.md`.

Every `path:line` below is read at `36c4783a29c44d2ffafc987803ccb1742a2c2cfc`.

## 1. The problem, measured

- **The property is sampled, not proved.** The prune and verdict pins (SPEC-043 R16c, R12c) remove one
  byte order mark and refuse a source that then opens with `#!` and not `#![`
  (`crates/agent/tests/runs.rs:50-53`, `crates/agent/tests/verdict.rs:22-25`), and the prune pin's run
  count and word count each skip a `doc` attribute group (`runs.rs:150`, `runs.rs:102`). The tests that
  hold this are generated populations: 16 shebang members in each pin
  (`runs::a_first_line_rustc_removes_as_a_shebang_is_refused`, `runs.rs:859-892`, and its verdict twin,
  `verdict.rs:618-651`) and 32 doc-attribute members
  (`runs::a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word`, `runs.rs:916-965`). No
  Lean entry covers either file: `git grep -n '@phx covers' <dev> -- .` names
  `crates/agent/tests/runs.rs` or `crates/agent/tests/verdict.rs` on 0 of 280 lines.
- **The two counts share one reader; the two pins carry two.** `prune_pin_problems` lexes once
  (`runs.rs:172`) and hands the one token list to both counts (`runs.rs:177`, `runs.rs:184`), which call
  one `is_doc_attribute` (`runs.rs:85-93`) with the same `before` slice. `verdict.rs` keeps its own
  `tokens_of` (`verdict.rs:10-15`) and a byte-identical copy of the shebang check (`verdict.rs:22-25`).
  `crates/agent/tests/support/mod.rs` holds no token helper.
- **The verdict pin has one descending count, and it reads doc groups.** `declarations`
  (`verdict.rs:45-59`) descends into every group, a doc attribute included, which fails closed.
- **The formal settings are on the development branch** (`config/formal.json`, SPEC-295, #468), and
  the Lean library builds every module under `formal/lean/Formal/` by glob (`formal/lean/lakefile.toml`),
  whose vector writer `formal/lean/Formal/Vectors.lean` holds 14 arms.

## 2. Requirements

R1. The Lean entry `lean/AgentPinGroups`, in `formal/lean/Formal/AgentPinGroups.lean`, ports branch for
branch, over a token tree type and with the lexer taken as any function from text to an optional token
list, these items of `crates/agent/tests/runs.rs`: `opens_with_a_shebang`, `is_punct`, `name_of`,
`tokens_in`, `is_doc_attribute`, `text_of`, `ends_in_the_query_macro`, `opens_with_the_statement`,
`statements_run_in`, `code_texts` and `prune_pin_problems`; and `crates/agent/tests/verdict.rs`'s
`opens_with_a_shebang`. Each is cited by a `covers` line, each port is named after its item, and each walk
returns its count and the groups it descended into from one recursion.

R2. The theorem `a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one` holds: for every lexer
and every source text, the ported pin refuses the source as a shebang exactly when, after one byte order
mark is removed, the text opens with `#!` and not `#![`. Its witness,
`a_check_that_reads_the_byte_order_mark_misses_a_shebang`, proves the claim false for the check with the
byte order mark kept (row S46901's mutant).

R3. The theorem `the_run_count_and_the_word_count_descend_into_the_same_groups` holds: for every token
list, the run count's groups equal the word count's, in order. Its witness,
`a_word_count_that_reads_doc_attributes_parts_from_the_run_count`, proves the claim false for the word
count with its doc guard dropped (row S46902's mutant). R2 and R3 together are #474's claim.

R4. The writer `formal/lean/Formal/AgentPinGroupsVectors.lean` prints a header and 672 vectors to
`formal/vectors/agent-pin-groups.jsonl`, never edited by hand: 96 shebang vectors (a source and whether the
port refuses it) and 576 count vectors (a source rendered from a token tree and the run count and the word
count the port answers on that tree).

R5. The prune pin's code answers every vector: for each shebang vector, `opens_with_a_shebang` and the
shebang problem of `prune_pin_problems` agree with the port; for each count vector, `statements_run_in`
reads the port's run count and `prune_pin_problems` names exactly the sentences the port's two counts
imply. The verdict pin's `opens_with_a_shebang` and `verdict_pin_problems` answer every shebang vector.
Each test prints how many vectors it examined and refuses any number but its population's.

R6. No production code changes, no existing line of `runs.rs` or `verdict.rs` changes, and no existing
population's examined figure moves.

R7. Both theorems enter at `ramp=report`, and no module under `formal/lean` holds an unproved hole, a
native evaluation or a new axiom.

## 3. Acceptance criteria of SPEC-469

| id | criterion | decided by |
|---|---|---|
| A1 | the prune pin answers every vector its Lean port wrote: 96 shebang vectors (the check and the shebang problem) and 576 count vectors (the run count, and the sentences naming both counts), the header naming the entry and 672 vectors; the test prints its examined count and refuses any other | `runs::the_prune_pin_answers_every_vector_of_its_lean_port` |
| A2 | the verdict pin's shebang check and its shebang problem answer every one of the 96 shebang vectors; the test prints its examined count and refuses any other | `verdict::the_verdict_pins_shebang_check_answers_every_vector_of_its_lean_port` |

```acceptance
A1: cargo test -p deck-streak-agent --test runs -- --exact the_prune_pin_answers_every_vector_of_its_lean_port
A2: cargo test -p deck-streak-agent --test verdict -- --exact the_verdict_pins_shebang_check_answers_every_vector_of_its_lean_port
```

Both are MUTATION COVERAGE, not red first: the code at the base already answers every vector. A1's reds
are rows S46902 and S46904, A2's are S46901 and S46903 (section 7).

## 3a. The formal entry, decided by the formal check

R1 to R4 and R7 are decided by the formal check of the one entry, run by hand at the build with the
repository's pinned toolchain: the entry builds, both witnesses build, both theorems are clean, the axiom
audit lists no axiom outside `config/formal.json`'s allow-list, every cover is stamped (no `STALE`), and
the vectors are byte-equal to the writer's output (no `DERIVED_DRIFT`). No CI job runs a formal check in
this repository, so the build quotes the card, and the claim's red is its witnesses, built before the
theorems are proved.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/agent/tests/runs.rs` | `deck-streak-agent` | changed: A1's test added; no existing line changes |
| `crates/agent/tests/verdict.rs` | `deck-streak-agent` | changed: A2's test added; no existing line changes |
| `formal/lean/Formal/AgentPinGroups.lean` | formal | added: the port, the claims, the theorems, the witnesses |
| `formal/lean/Formal/AgentPinGroupsVectors.lean` | formal | added: the vector writer |
| `formal/lean/Formal/Vectors.lean` | formal | changed: one import and one arm |
| `formal/vectors/agent-pin-groups.jsonl` | formal | added: the writer's header and 672 vectors |
| `scripts/mutation-rows.d/S46900-S46999.json` | repo | added: rows S46901 to S46904 |
| `docs/specs/SPEC-469-a-lean-proof-holds-that-the-prune-pin-refuses-exactly-the-shebang-sources-and-that-its-run-count-and-word-count-descend-into-the-same-token-groups.md` | docs | added |
| `docs/decisions/ADR-486-the-prune-pins-token-groups-are-proved-in-lean-over-a-port-that-takes-any-lexers-tokens.md` | docs | added |
| `docs/red-first/SPEC-469.md` | docs | added |
| `docs/schematics/agent-pin-token-groups-proof.md` | docs | added |
| `docs/specs/SPEC-043-agent-core-runner-gate-and-degradation.md` | docs | changed: section 13 appended, insert-only |
| `changelog.d/agent-pins-token-groups-469.md` | repo | added |

## 5. What this does NOT cover

- What rustc does after it lexes: expanding a macro, evaluating a `cfg`, running one path and not another
  (#444).
- Whether the lexer's tokens are the tokens rustc reads: the proof holds for every token list; ADR-293's
  rustc-labelled populations measure the lexer (#414).
- The verdict pin's declaration count, which reads every group and fails closed on a doc attribute; #474's
  claim names the run count and the word count only (#474).
- Other token readers in the tree, the one-router census's among them (#297): none is ported here, and the
  agent-runs pin keeps its own reader, as SPEC-331 says (#444).
- The CI readings of this delivery's rows and red-first class, which are read by name after the release
  pull request #822 merges (#822).

## 6. Risks

- **A port tidier than the code.** A walk ported with another guard than the code's would prove something
  nobody runs. Detected by the vectors (A1 fails on the first vector the two disagree on) and by the covers,
  which stale the entry when a ported item moves.
- **The rendered sources lex otherwise than the port's trees.** A count vector whose rendering proc-macro2
  lexes into another tree fails A1 at that vector; the writer renders every token with one space between
  tokens and uses only string literals whose cooked text equals what is written.
- **No CI job reads the entry.** A later change to a covered item stales the entry without a red; the
  covers make the next hand check report `STALE`.
- **An open sibling adds a census of formal vectors tests or makes Lean warnings errors.** The build reads
  both at its cut and stops if either has landed.

## 7. The mutation rows

Band `scripts/mutation-rows.d/S46900-S46999.json`, table `MUTATIONS`, each row a mutant of an existing pin
line that this SPEC's tests kill:

- S46901: the verdict pin's check keeps the byte order mark (`verdict.rs:23`), killed by A2's test, which
  reads a source of U+FEFF then `#!` as refused; it is R2's witness on the code.
- S46902: the word count reads a doc attribute's tokens (`runs.rs:102`), killed by A1's test, which reads
  a count vector whose doc group neither count may read; it is R3's witness on the code.
- S46903: the verdict pin's check refuses an inner attribute `#![` too (`verdict.rs:24`), killed by A2's
  test.
- S46904: a call whose tokens open a group is not a run (`runs.rs:117`), killed by A1's test.

## 8. References

Issue #474; #414; #444; #468; SPEC-043 sections 9 to 13; ADR-293; ADR-486; SPEC-295; ADR-295.
