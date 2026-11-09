---
status: "accepted"
---

# ADR-497: a runaway mutant of the release run is killed by a progress bound that is the identity on every admitted input

Decides SPEC-480. It orders its own cure before ADR-451's kill parts 2 to 6, and it reads the
Rust finding through SPEC-196's memory scope.

## Context and Problem Statement

The release pull request (#822) ran its mutation legs in run 39033655190, whose six legs below died 755 s to 1327 s into their run step.
Each of them ran `python3 scripts/mutation_python.py run --plan ... --shard <k>/<n> --failfast --report ...`; each
log ends with the runner's shutdown signal and exit 143, with no line after the step starts, and no
leg left a report or an artifact. Both attempts died at about the same point:

| leg | its third dealt mutant, in `rust_tokens` | attempt 1: job and run-step seconds | attempt 2: job and run-step seconds |
|---|---|---|---|
| `mutation-python (150)` | `scripts/mutation-verdict.py:532:25`, `and` to `or` | job 117399178683, 781 s | job 117533719194, 846 s |
| `mutation-python (152)` | `scripts/mutation-verdict.py:532:53`, `==` to `!=` | job 117399178676, 822 s | job 117533719226, 837 s |
| `mutation-python (154)` | `scripts/mutation-verdict.py:534:24`, `<` to `>=` | job 117399178643, 817 s | job 117533719211, 788 s |
| `mutation-python (156)` | `scripts/mutation-verdict.py:534:62`, `or` to `and` | job 117399178597, 896 s | job 117533719241, 755 s |
| `mutation-python (157)` | `scripts/mutation-verdict.py:534:76`, `==` to `!=` | job 117399178680, 1271 s | job 117533719224, 1327 s |
| `mutation-python (158)` | `scripts/mutation-verdict.py:534:79`, `"_"` to `""` | job 117399178691, 1025 s | job 117533719259, 1280 s |

All 78 mutants dealt to these legs are in `scripts/mutation-verdict.py`, 13 per leg, dealt by the
interleave of SPEC-437 (`indices[cut::cuts]`, mutation-verdict.py:1236). The run's mutation verdict
read the scripts class FAIL with 804 unexplained survivors of 6617 examined
(`scripts/mutation-verdict.py` 681, `scripts/mutation_rows.py` 107, `scripts/public-scrub.py` 16),
the oracle class VOID for the six missing reports, and the rust class FAIL 1: shard 140's mutant
`crates/ingest/src/memory_state.rs:85:16`, `+=` to `*=` in `leniently`, which the memory scope of
SPEC-196 stopped, neither caught nor timed out. A class FAIL outranks the class's VOID lines
(`Verdict.close`, mutation-verdict.py:1495-1505), so the six scripts VOID legs read only under the
scripts FAIL.

`rust_tokens` (mutation-verdict.py:513-539) reads Rust text as `c_code_lines` does. Under each of
the six mutants a token start (punctuation for line 532; a word character, or a `_`, for line 534)
leaves `stop == at`, and `tokens.append((at, stop)); at = stop` (:537-538) then appends one token
per pass without end. The child's memory grows until the runner stops it.

The harness (`scripts/mutation_python.py`, unchanged since the earlier release reading at
`b25ed04a`) runs each mutant in a child of its own session (:1039-1046), its output to a temporary
file, under a wall-clock bound derived from the control run (:1138); it kills the child's process
group when the bound passes (:1047-1077), and a survivor of the scoped run takes a second, whole
run (:1186-1195). It bounds neither the child's memory nor the leg's total. Its outcome words
(:67) are killed, survived, uncovered, unviable, timeout and void; timeout and void exit 3, VOID.

## Decision Drivers

- A route that closes a finding without killing its mutant narrows the gate.
- A bound that ends a runaway mutant reads a classified outcome, never KILLED; a new outcome word
  would fall through `judge_python` (:2181-2204) without a line.
- The release plan's Python figure stays within `PYTHON_RELEASE_CEILING` (mutation-verdict.py:131),
  305 legs against the 304 the run dealt.
- A change to the verdict script joins the release Python population, so its own mutants are dealt
  and must be killed there.
- Each land on dev starts one push-triggered run of #822, and a newer one cancels the older
  (ci.yml:25-27).

## D1, the six runaway mutants: what it was chosen against

Each leg's runaway is its third dealt mutant, all six in `rust_tokens`, and the mechanism is
memory: the `tokens` list grows without end. The code is measured; the moment each leg died is
inferred from the siblings below.

- Chosen: name the six as each leg's third dealt mutant, because each leaves `stop == at` on a token start every Rust text holds, and `tokens.append((at, stop)); at = stop` then appends forever.
- A leg's second mutant as its runaway: rejected because each of the six second mutants sits in `rust_literal` (lines 383 to 386), whose hashes loop cannot return a position at or before its start, and sibling legs 155 and 159 ran that class whole to a report (run 39033655190, 350.1 s and 285.9 s).
- A time bound firing first: rejected because the siblings' bounds were 763 s to 1138 s against a third-mutant start of about 440 s to 880 s in run 39033655190, so memory runs out before the bound passes.
- A disk or process exhaustion: rejected because `rust_tokens` writes nothing and starts nothing; it only appends to a list in its own process.
- Any of the other 72 dealt mutants: rejected because no function they sit in holds a `while` loop except `rust_literal`'s hashes loop, where none of them is dealt.
- The sibling legs' own `rust_tokens` mutants as evidence against the reading: rejected because each of them advances and reported (legs 151, 153, 155 and 159 of run 39033655190, the slowest 302.2 s).

## D2, the cure for the six legs: what it was chosen against

A progress guard in `rust_tokens`, right after `stop = at + 1 if stop is None else stop`:

```python
        if stop <= at:
            raise RuntimeError(at)
```

Every pass of the loop then either moves `at` forward by at least one character or raises naming
the position. On every admitted input each branch already advances, so the guard is the identity
there; under each of the six mutants the guard raises at the stalled token, and any test that reads
such a token fails: the mutant reads KILLED. The guard adds one listed mutant, `<=` to `>`, which
raises on every advancing token. The verdict file's dealt figure moves from 2843 to 2844, within
the four the plan holds before its 219 runs grow. Four conditions hold the guard to that claim, each
a requirement of SPEC-480: a differential test that reads every Rust source of the release
population with and without the guard and finds byte-equal output, with its examined count; the
Lean entry `lean/TokenProgress`; a named killer of the guard's own mutant with its non-vacuity
control; and, after the land, the next push-triggered run of #822, whose six legs above each report
with their examined count. That last one is the delivery's measure after the land, never a
criterion of the pull request that lands it.

- Chosen: the progress guard, because it kills the six by construction, is the identity on every admitted input, changes no harness file, and adds one mutant the plan already has room for.
- The per-mutant memory bound in the harness now: rejected because it kills nothing (a runaway would read a classified memory outcome, never killed), and the mutants it adds to `scripts/mutation_python.py` take the release plan past `PYTHON_RELEASE_CEILING`; D7 holds it.
- A lower wall-clock bound: rejected because memory runs out before any bound the control runs admit, and a timeout reads VOID, never killed.
- An equivalence record for each of the six: rejected because each mutant changes the tokens of a text a test can read, so each record would be false.
- Excluding the six from the plan: rejected because it narrows the gate.
- A test-side alarm that ends a slow test: rejected because it launders a timeout into a kill.
- A guard on every branch of the loop: rejected because it adds mutants and moves anchors, and the branches that append nothing loop on time, which the wall-clock bound already ends.
- Deriving the unguarded side of the differential from the function's own source: rejected because a test that reads its target file's bytes fails under the runner's sentinel and is dropped from that file's mutant runs; the test holds a verbatim copy of the base function instead.

## D3, the scripts class: what it was chosen against

The 804 unexplained survivors are killed by ADR-451's parts 2 to 6, test-only, each re-derived from
the cure's measure after the land, never from an earlier part's build. By file:
`scripts/mutation-verdict.py` read 560 at `b25ed04a` and 681 at `36c4783a`: 551 are common; 130 are
new to the reading (the survivors of legs ADR-451 found cut at the job bound, now read after
SPEC-437's interleave: `rust_literal` 50, `c_code_lines` 20, `web_literal` 16, `survivors_in` 14,
`partial_reason` 7, `item_keyword` 7, `item_end` 6, `row_span` 4, `selected_rows` 3, `block_end`
1, `rust_tokens` 1, `rust_test_spans` 1); and 9 read at `b25ed04a` sit among the six dead legs
(`stryker_findings` 6, `survivors` 2, `draft_body` 1). Between those commits the file changed 24
lines (the interleave and the class filter of `python_excuses`); `scripts/mutation_rows.py` (107)
and `scripts/public-scrub.py` (16) did not change, and `scripts/write-back-issues.py`'s three
UNCOVERED mutants were closed by SPEC-437. Part 2 takes `scripts/public-scrub.py` and
`scripts/mutation_rows.py`; parts 3 to 6 take the verdict file's readers, judging functions, tool
readers and records, and module constants, as ADR-451 grouped them.

- Chosen: ADR-451's parts 2 to 6, re-derived from the cure's measure, because that run is the first to read every Python leg, and the cure moves the verdict file's listing.
- A route that closes the class without kills: rejected because it narrows the gate.
- Reusing an earlier kill-part build: rejected because its survivor counts were read before the six legs reported and before the guard moved the listing.
- Starting the kill parts before the cure lands: rejected because six legs' survivors stay unread until the cure lets them report.

## D4, the Rust memory finding: what it was chosen against

`leniently` (crates/ingest/src/memory_state.rs:41-89) replaces `while at < bytes.len() {` with a
loop bounded by the text's length:

```rust
    for _ in 0..bytes.len() {
        if at >= bytes.len() {
            break;
        }
        // the old loop body, unchanged
    }
```

Every correct pass advances at least one byte, so the bound never ends a reading early: it is the
identity on every admitted input. Under shard 140's mutant, `at *= width` with `at` at 0 on a text
that opens with `{` never advances; with the bound the reading ends after its length's passes with
a wrong text, `parse` answers none, and the existing test
`memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field` fails at its `expect`.
The listing loses one Rust mutant: the two of the old `<` go, and the new `>=` to `<` breaks at the
first pass, an empty output every test that expects a state kills. A differential test over a
seeded population against a verbatim copy of the base reader proves the identity, with its examined
count and the count that parsed to a state.

- Chosen: the pass bound, because it ends the runaway inside the reader, changes no output on any admitted input, and lets an existing test kill the mutant.
- A record for the finding: rejected because no record exists for a memory-scope reading (SPEC-196 R8 to R11), and the mutant is killable.
- A slow-test terminate in the test runner's configuration: rejected because it would read a killed mutant the tests never caught.
- A test-only kill: rejected because no test can catch a reading that never returns before the memory scope stops it.
- An iterator rewrite of the reader: rejected because it changes more lines and moves more anchors for the same bound.

## D5, the landing order: what it was chosen against

#969, the lock-only advisory bump, lands first and the cure right after it. The two lands start two
runs of #822, and the second cancels the first (ci.yml:25-27); that second run, R1, is the cure's
measure. ADR-451's parts 2 to 6 follow, re-derived from R1, each its own land; the last run they
start, R2, is the closing run. In all at least seven runs of #822 start, one per land, and at least
two, R1 and R2, are read whole; any other land on dev starts one more.

- Chosen: #969 and the cure back to back, because one run then measures both, and nothing else lands between them.
- A manual full run: rejected because each push-triggered run already deals the whole release population, and a manual one adds a full run that no land asked for.
- The cure first and #969 after R1: rejected because #969's land would start a run of its own after R1, one more full run with nothing new to measure.
- The kill parts before R1: rejected because their survivors move with the cure's listing and six legs have not yet reported.

## D6, the shape of the delivery: what it was chosen against

One pull request into dev carries the guard and its tests in
`scripts/tests/test_mutation_verdict_python_kills.py`, the differential census in a new module,
`scripts/tests/test_rust_tokens_identity.py`, the reader's bound and its differential test
`crates/ingest/tests/memory_state_lenient_bound.rs`, rows S48001 to S48007, the Lean entry and the
documents, in one push. It carries no harness bound, no kill part and nothing for the rust shards'
red baselines.
ADR-509 now bounds those baselines: each Rust leg runs its own unmutated baseline under a time budget before cargo-mutants, which skips its own.

- Chosen: one pull request with one push, because each push starts a run on the pull request, the one red-first criterion reads its red from a bounded stand-in, and the two cures share one measure.
- Two pushes, a red one and a green one: rejected because the red is measured without a push, and a second push starts a second run.
- The Rust bound in a pull request of its own: rejected because its land would start one more run of #822, and both cures are read by the same R1.
- The census in the kills module: rejected because that module runs under every mutant of the verdict file it is mapped to, so a census of every Rust source would cost each surviving mutant's whole run; outside the map it runs once per check.

## D7, the per-mutant memory bound in the harness: what it was chosen against

Held until #822 merges. The bound then runs each Python leg under SPEC-196's memory scope; a child
stopped while the scope's out-of-memory count rose reads a new outcome, `memory`, which the verdict
scores through `score_memory_cap`, never as killed and never as examined.

- Chosen: hold the bound until #822 merges, because the guard of D2 already ends the six, and the bound kills nothing.
- The bound in this delivery with a raised Python ceiling: rejected because it needs a new ceiling and still kills nothing.
- The bound as a new outcome read as killed: rejected because a memory stop is not a test's catch.

## Decision Outcome

D2 and D4 are built by SPEC-480 in one pull request; D1 names what they cure; D3, D5 and D7 order
the work after it.

### Consequences

- `rust_tokens` raises `RuntimeError` naming the position where a reading does not advance; no
  admitted text reaches it, so the mutation plan reads every Rust line as before.
- Every line of `scripts/mutation-verdict.py` after the guard moves down two lines; rows anchor by
  text, so none moves.
- The stride deal places the verdict file's mutants on legs by listing position, so the six may sit
  on other legs after the cure.
- The rust class still reads its shards' red baselines as VOID under the class line; this decision
  does not change them (#822).
- ADR-509 now bounds those red baselines: a Rust leg whose own baseline is red, past its time budget or unrecorded runs no mutant and reads VOID with its reason, and cargo-mutants skips its own baseline.

### Confirmation

SPEC-480's criteria in the pull request's checks, its rows read KILLED by id, the Lean entry checked
with its witnesses caught, and R1 reading the six legs with a report each.

## What would make this wrong

- A Python leg of R1, any of the release run's legs, reading VOID with no report.
- The differential census finding one Rust source whose tokens differ with the guard.
- A test that reads a text whose reading stays: the guard would raise where the base looped.
- The shard holding the Rust mutant reading a memory stop again with the bound in place.

## More Information

Built by SPEC-480; drawn in section 11 of `docs/schematics/mutation-testing-python.md`. Refers #322
and #825.
