# Red-first record: SPEC-039

The SPEC (909856d), the schematic (5a41904) and ADR-057 (2bb3c77, 6719cfb) came first. The tests
were then committed (d438d38) beside stub entry points that kept every interface and did nothing:
the runner read no row and refused nothing, the census and the retirement check examined nothing,
and the verdict read every class green without writing a plan's classes or a draft. Each criterion
was run there with its fenced command and failed by assertion for its own reason, not by a compile
error, a missing fixture or an empty selection.

- The runner and the retirement check went green at 61e5e14, and the verdict at 73133a4. A17's
  selection also needs a killer's file located without a Cargo manifest, which went green at
  c404cb2.
- A9 and A11 need rows to judge. At 509bf2c the pack was vendored and wired and no row existed: the
  census found no header, and the probe's `find-differs` was VOID for want of a row manifest. The
  header and 22 rows went in at 3a366fb, where the runner proved all 22 KILLED on the committed
  tree, each file restored byte for byte (714 s); CI's `mutation-rust` proved the same 22 on this
  pull request (run 36370506636).
- A21 to A25 were red at 509bf2c: no tool configuration, no weekly workflow, no mutation job in
  `ci.yml`, no brief section. They went green at 018a6ea.
- A27 was added when the orchestrator asked that each event read its own case: red at fbd772d,
  against a stub that accepted the options and ignored them, and green at fcb226b.
- No assertion changed between a red and its green. A19's fixture changed in 73133a4: its synthetic
  `outcomes.json` gave the kernel survivor the fixture's own file where it meant `clock.rs`, so the
  draft the assertion names could not be written; the assertions are unchanged.
- One existing test line changed: `test_the_gate_runs_in_four_parallel_jobs` pins `ci`'s needs as
  an exact set (SPEC-038 A3). The set grows by this delivery's two required jobs, `mutation-rust`
  and `mutation-web`, and stays an exact equality.

Every sha above was replayed from a `git archive` export into a directory of its own: all 25 of A1
to A25 red at d438d38; A9, A11 and A21 to A25 red at 509bf2c with the rest green; A1 to A8 and A10
green at 61e5e14; the verdict's criteria but A17 green at 73133a4; A17 at c404cb2; A9 and A11 at
3a366fb; A1 to A25 at 018a6ea; A27 red at fbd772d and green at fcb226b. Failures below name no path
outside the repository.

```red-first
A1: red at d438d38: AssertionError: 0 != 2 : examined 0 (a tracked change was not refused)
A1: green at 61e5e14
A2: red at d438d38: AssertionError: 0 != 3 : examined 0 (an anchor occurring twice was not VOID)
A2: green at 61e5e14
A3: red at d438d38: AssertionError: 0 != 3 : examined 0 (a killer selecting three tests was not VOID)
A3: green at 61e5e14
A4: red at d438d38: AssertionError: {} != {'S00004-DOUBLE': 'KILLED'}
A4: green at 61e5e14
A5: red at d438d38: AssertionError: 0 != 1 : examined 0 (a surviving mutant did not fail the run)
A5: green at 61e5e14
A6: red at d438d38: AssertionError: 0 != 3 : examined 0 (a mutant that does not parse was not VOID)
A6: green at 61e5e14
A7: red at d438d38: AssertionError: 0 != 3 : examined 0 (a killer red without its mutant was not VOID)
A7: green at 61e5e14
A8: red at d438d38: AssertionError: 0 != 3 : examined 0 (no cargo killer was built or run)
A8: green at 61e5e14
A9: red at d438d38: AssertionError: 0 not greater than 0 : examined 0 (the census examined no row)
A9: green at 3a366fb
A10: red at d438d38: AssertionError: 0 != 1 : examined 0 (a row that left while its target stayed was admitted)
A10: green at 61e5e14
A11: red at 509bf2c: AssertionError: 3 != 0 : find-differs VOID, no row manifest at scripts/mutation-rows.json
A11: green at 3a366fb
A12: red at d438d38: AssertionError: None != 'other' : crates/fix/build.rs
A12: green at 73133a4
A13: red at d438d38: AssertionError: 0 != 3 : mutation: ok (a production diff that examined nothing read green)
A13: green at 73133a4
A14: red at d438d38: AssertionError: unexpectedly None : the plan judges no rust class
A14: green at 73133a4
A15: red at d438d38: AssertionError: 0 != 1 : mutation: ok (a missed mutant did not fail)
A15: green at 73133a4
A16: red at d438d38: AssertionError: 'examined 0 by cargo-mutants and 1 by rows' not found in 'mutation: ok'
A16: green at 73133a4
A17: red at d438d38: AssertionError: Lists differ: [] != ['S00050-LAST-HOUR', 'S00051-DOUBLE', 'S00053-ADDED']
A17: green at c404cb2
A18: red at d438d38: AssertionError: 0 != 3 : mutation: ok (a missing report read green)
A18: green at 73133a4
A19: red at d438d38: AssertionError: False is not true : no drafts were written
A19: green at 73133a4
A20: red at d438d38: AssertionError: Regex didn't match: 'examined N exclusion' not found in 'mutation: ok'
A20: green at 73133a4
A21: red at 509bf2c: AssertionError: False is not true : tool-config-valid: nothing of its kind is in the tree
A21: green at 018a6ea
A22: red at d438d38: AssertionError: .github/workflows/mutation-weekly.yml does not exist
A22: green at 018a6ea
A23: red at d438d38: AssertionError: .github/workflows/mutation-weekly.yml does not exist
A23: green at 018a6ea
A24: red at d438d38: AssertionError: 'mutation-rust' not found in ci.yml's jobs
A24: green at 018a6ea
A25: red at d438d38: AssertionError: unexpectedly None : docs/BUILDER-BRIEF.md has no Mutation testing section
A25: green at 018a6ea
A26: not red: it pins behaviour the code already had (a time before the epoch reads as negative milliseconds); its red is the mutation-rust job's, which missed `delete -` in UtcMillis::from_system_time until the test landed (R17, below)
A27: red at fbd772d: AssertionError: None != 'diff' : pull_request
A27: green at fcb226b
```

## The gate's own red first (R17)

The first measurement found one survivor in the kernel: `delete -` in `SystemClock::now`, the branch
that reads a system time before the Unix epoch, which no test reaches (SPEC-039 §1). The conversion
moved into a pure function, `UtcMillis::from_system_time`, beside a test that pins a time after the
epoch and leaves the pre-epoch branch alive. The pull request's `mutation-rust` job then went RED on
the survivor; a test that a time before the epoch reads as negative milliseconds (A26) killed it,
and the job went GREEN.
