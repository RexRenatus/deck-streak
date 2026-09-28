# Red-first record: SPEC-039

`dev` takes this delivery as one squash commit. Every sha below is a commit of pull request #221,
reachable from its head (`refs/pull/221/head`), not from `dev`'s history.

The SPEC (909856d), the schematic (5a41904) and ADR-057 (2bb3c77, 6719cfb) came first. The tests
were then committed (d438d38) beside stub entry points that kept every interface and did nothing:
the runner read no row and refused nothing, the census and the retirement check examined nothing,
and the verdict read every class green without writing a plan's classes or a draft. Each criterion
was run there with its fenced command and failed by assertion for its own reason, not by a compile
error, a missing fixture or an empty selection.

- The runner and the retirement check went green at 61e5e14, and the verdict at 73133a4. A17's
  selection also needs a killer's file located without a Cargo manifest, which went green at
  c404cb2.
- A9 needs rows to judge. At 509bf2c no row existed and the census found no header. The header
  and 22 rows went in at 3a366fb, where the runner proved all 22 KILLED on the committed tree, each
  file restored byte for byte; CI's `mutation-rust` proved the same 22 on this pull request (run
  36370506636).
- A22 to A25 were red at 509bf2c: no weekly workflow, no mutation job in `ci.yml`, no brief
  section. They went green at 018a6ea.
- A11 and A21 first judged the rows and the configurations through a probe vendored from the
  mutation-rows pack (red at 509bf2c; green at 3a366fb and 018a6ea). The owner's decision that the
  packs stay box-only dropped every vendored file, so both were rewritten against DeckStreak's own
  census and configuration check, and red and green again: the lines below are the rewrites'.
- The verifier's round (F2 to F4) and the owner's decision added A29 to A33 and rewrote A11 and A21.
  All seven were committed red at 4146198 beside stubs that accepted the new options and did
  nothing: `battery` and `configs` examined nothing, the census had no duplicate-mutant refusal, the
  judge read a partial report whole, and the lexer read a comment opener inside a string as a
  comment. Every one went green at d1dde73. Rows S03906 to S03910 then pinned the new guards, each
  proved KILLED by the runner with its file restored.
- A27 was added when the orchestrator asked that each event read its own case: red at fbd772d,
  against a stub that accepted the options and ignored them, and green at fcb226b. Its release
  case changed by the owner's directive ("deckstreak will need per merge diff mutation testing",
  "dev to main"): the assertion that a release pull request into `main` reads `not-applicable`
  became one that it is judged on its merge diff, committed red at 7cc64e3 against the old scope
  and green at 6e722dd. The line below is the change's; fbd772d and fcb226b replay as before.
- A28 was added when the first dispatch of the whole battery stopped two shards at the baseline
  under `cargo test` (SPEC-039 section 8): red at 5238623, green at e82b796.
- The release's shards (R18) added A34 to A38, each committed red beside a stub, then green, one
  at a time: A34 at 1350ba8, beside a `shards` verb that sized nothing, and green at eca251d; A36
  at db78aaa, beside a `--shard-reports` option the judge ignored, and green at d0e18ca; A35 at
  b86fbe7, where the judge counted the shards and proved no partition, and green at d72c523; A38 at
  c506ffb, where a shard of no mutant owed a report, and green at 0db24d2. A38 records a finding:
  cargo-mutants exits 0 and writes no report when the diff lists no mutant (measured on a changed
  constant), so the judge read such a diff VOID even when a proved row carried its line; A16's
  synthetic report of zero mutants is one the tool never writes. Rows S03911 to S03924 then
  pinned each new branch of `mutation-verdict.py`, and the runner proved all 24 of the band's rows
  KILLED, S03906 to S03910 among them again, each file restored byte for byte.
- A24 changed with the job layout: it now names the five jobs and allows one job-level `if`, the
  verdict's `always()`. Its first red (d438d38) and green (018a6ea) replay as before; the line
  below is the rewrite's, red at 00a3385 beside A37 and green with the workflow at c954220.
- No assertion changed between a red and its green. A19's fixture changed in 73133a4: its synthetic
  `outcomes.json` gave the kernel survivor the fixture's own file where it meant `clock.rs`, so the
  draft the assertion names could not be written; the assertions are unchanged.
- One existing test line changed: `test_the_gate_runs_in_four_parallel_jobs` pins `ci`'s needs as
  an exact set (SPEC-038 A3). The set grows by this delivery's five required jobs,
  `mutation-plan`, `mutation-rust`, `mutation-rows`, `mutation-verdict` and `mutation-web`, and
  stays an exact equality. It went red at 00a3385 with A24 and A37, and green at c954220.

Every sha above was replayed from a `git archive` export into a directory of its own: all 25 of A1
to A25 red at d438d38; A9, A11 and A21 to A25 red at 509bf2c with the rest green; A1 to A8 and A10
green at 61e5e14; the verdict's criteria but A17 green at 73133a4; A17 at c404cb2; A9 and A11 at
3a366fb; A1 to A25 at 018a6ea; A27 red at fbd772d and green at fcb226b; A11, A21 and A29 to A33 red
at 4146198 with the other 25 green, and all 32 green at d1dde73; then A27 red at 7cc64e3 and
green at 6e722dd, A34 red at 1350ba8 and green at eca251d, A36 red at db78aaa and green at d0e18ca,
A35 red at b86fbe7 and green at d72c523, A38 red at c506ffb and green at 0db24d2, and A24 and A37
red at 00a3385 and green at c954220, each red sha failing its own criterion alone. Failures below
name no path outside the repository.

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
A11: red at 4146198: AssertionError: 'census: S00042-AGAIN: installs the mutant S00041-FIRST installs, for the same killer' not found (the census named only S00040-SAME)
A11: green at d1dde73
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
A21: red at 4146198: AssertionError: Regex didn't match: 'examined 2 configuration' not found in 'examined 0 configuration(s)'
A21: green at d1dde73
A22: red at d438d38: AssertionError: .github/workflows/mutation-weekly.yml does not exist
A22: green at 018a6ea
A23: red at d438d38: AssertionError: .github/workflows/mutation-weekly.yml does not exist
A23: green at 018a6ea
A24: red at 00a3385: AssertionError: 'mutation-plan' not found in ci.yml's jobs : ci.yml has no mutation-plan job
A24: green at c954220
A25: red at d438d38: AssertionError: unexpectedly None : docs/BUILDER-BRIEF.md has no Mutation testing section
A25: green at 018a6ea
A26: not red: it pins behaviour the code already had (a time before the epoch reads as negative milliseconds); its red is the mutation-rust job's, which missed `delete -` in UtcMillis::from_system_time until the test landed (R17, below)
A27: red at 7cc64e3: AssertionError: 'not-applicable' != 'diff' : pull_request main
A27: green at 6e722dd
A28: red at 5238623: AssertionError: Regex didn't match: '(?m)^test_tool = "nextest"$' not found in .cargo/mutants.toml
A28: green at e82b796
A29: red at 4146198: AssertionError: 0 != 1 : examined 0 report(s) (shard 2 missing, shards 1, 3 and 4 partial, and no rows report, yet the battery passed)
A29: green at d1dde73
A30: red at 4146198: AssertionError: 0 != 3 : the judge read a report of 1 of its 3 mutants, under exit 137, as green
A30: green at d1dde73
A31: red at 4146198: AssertionError: Lists differ: [] != [6, 11] (the line inside a string and the comparison after "crates/*" read as comments)
A31: green at d1dde73
A32: red at 4146198: AssertionError: Regex didn't match: '--build-timeout \d+' not found in ci.yml's cargo mutants command
A32: green at d1dde73
A33: red at 4146198: AssertionError: 0 != 1 : survivors never counts the battery's reports
A33: green at d1dde73
A34: red at 1350ba8: AssertionError: unexpectedly None : the plan holds no shards
A34: green at eca251d
A35: red at b86fbe7: AssertionError: 0 != 1 : a mutant tested in two shards, and one the listing never named, read green
A35: green at d72c523
A36: red at db78aaa: AssertionError: 3 != 1 : VOID no report: no --outcomes holds no cargo-mutants outcomes (the judge read no shard's report)
A36: green at d0e18ca
A37: red at 00a3385: AssertionError: Regex didn't match: 'cargo mutants [^\n]*--list --json --in-diff ' not found in '' (ci.yml has no mutation-plan job)
A37: green at c954220
A38: red at c506ffb: AssertionError: 3 != 0 : VOID mutation-rust-shard-0: no report (a shard of no mutant owed one)
A38: green at 0db24d2
```

## The gate's own red first (R17)

The first measurement found one survivor in the kernel: `delete -` in `SystemClock::now`, the branch
that reads a system time before the Unix epoch, which no test reaches (SPEC-039 §1). The conversion
moved into a pure function, `UtcMillis::from_system_time`, beside a test that pins a time after the
epoch and leaves the pre-epoch branch alive. The pull request's `mutation-rust` job then went RED on
the survivor; a test that a time before the epoch reads as negative milliseconds (A26) killed it,
and the job went GREEN.

- **RED at 8fb5cd0:** `ci` run 36371069680, `mutation-rust` job 108767357218. cargo-mutants found 3
  mutants on the diff: 1 missed (`crates/kernel/src/clock.rs:39:88: delete - in
  UtcMillis::from_system_time`), 2 unviable, 0 caught; the verdict read `examined 1 by
  cargo-mutants and 0 by rows` and `FAIL: 1 finding(s)`. The 22 rows were all KILLED in the same
  job.
- **GREEN at 9503a63:** `ci` run 36371649918, `mutation-rust` job 108769091236. The same 3 mutants:
  1 caught, 2 unviable; the verdict read `examined 1` and ok, the 22 rows KILLED, and `ci` green.
- The same diff was run locally first, in place on the committed tree: 1 missed at 8fb5cd0, and 1
  caught once the test was committed. The first local attempt passed `-j 1` beside `--in-place`,
  which cargo-mutants 27.1.0 refuses (exit 1), and the verdict read it as VOID (R7's amendment).
- The weekly battery's `rehearsal` ran on this pull request (run 36370506694 at fcb226b):
  cargo-mutants over `clock.rs` examined 6 (5 caught, 1 missed, 3 unviable), row S02001 KILLED,
  StrykerJS ran 20 mutants over `startapp.ts`, and the drafts for the two files with survivors
  passed the scrub; nothing was filed.
