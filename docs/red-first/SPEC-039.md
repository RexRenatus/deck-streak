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
- Two of the orchestrator's optional items each took a criterion, red then green: A39 at
  f26c24e, where `prove --band` beside `--row` proved the band and dropped the row, and green at
  688ae71; A40 at 24013e7, where `configs` passed a second Stryker configuration, `ignoreStatic`
  without per-test coverage and a `mutate` list short of R2's, and green at a029d19. Rows S03925 to
  S03928 pin them, and the runner proved them and the runner's own four rows (S03903 to S03905,
  S03910) KILLED again after the fix.
- A24 changed with the job layout: it now names the five jobs and allows one job-level `if`, the
  verdict's `always()`. Its first red (d438d38) and green (018a6ea) replay as before, and so does
  the rewrite, red at 00a3385 beside A37 and green with the workflow at c954220. The first `ci` run
  of the layout (36389287255) then went VOID: `mutation-rows` proved all 45 rows KILLED and could
  not write its report into a directory it never made, so the verdict read `45 row(s) selected and
  no rows report`. A24 gained that the rows job makes its report's directory, red at 9853d50 and
  green at 8bce65d; the line below is that change's.
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
red at 00a3385 and green at c954220, A39 red at f26c24e and green at 688ae71, A40 red at 24013e7
and green at a029d19, and A24 red at 9853d50 and green at 8bce65d, each red sha failing its own
criterion alone. Failures below
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
```
```retired
A20: red at d438d38: AssertionError: Regex didn't match: 'examined N exclusion' not found in 'mutation: ok'
A20: green at 73133a4
```
```red-first
A21: red at 4146198: AssertionError: Regex didn't match: 'examined 2 configuration' not found in 'examined 0 configuration(s)'
A21: green at d1dde73
A22: red at d438d38: AssertionError: .github/workflows/mutation-weekly.yml does not exist
A22: green at 018a6ea
A23: red at d438d38: AssertionError: .github/workflows/mutation-weekly.yml does not exist
A23: green at 018a6ea
A24: red at 9853d50: AssertionError: False is not true : the rows job never makes the directory its report goes to
A24: green at 8bce65d
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
A39: red at f26c24e: AssertionError: {'S00010-DOUBLE': 'KILLED'} != {'S00010-DOUBLE': 'KILLED', 'S00110-RETURN': 'KILLED'} : --band S00000-S00099 --row S00110-RETURN
A39: green at 688ae71
A40: red at 24013e7: AssertionError: 0 != 1 : examined 2 configuration(s) (a second configuration, ignoreStatic and a short mutate list all passed)
A40: green at a029d19
A41: red at 6811975: AssertionError: 0 != 3 : S00020-BASH-UNPARSED: KILLED: its killer passed without the mutant and failed with it (a bash, an extensionless bash and a POSIX sh mutant that did not parse all read KILLED)
A41: green at 8bbeac4
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

Amendment (2026-09-28): the lines of A20 moved into a `` ```retired `` fence, by inserted fence
lines, because SPEC-057 retired that criterion when ADR-070 replaced the exclusion it held to its
reason with the equivalence record; SPEC-057 A8 judges that no exclusion hides a mutant.

Amendment (2026-09-29): A41 (SPEC-039 section 12, issue #288). The tests were committed at 6811975
beside the unchanged runner: of the six in `TheRunnerParseChecksAShellMutant`, the three whose mutant
does not parse (a bash script, a bash script known by its shebang alone, a POSIX `sh` script)
failed by assertion, each reading KILLED where VOID was owed, and the other three (the construct
parses under `bash -n` and not under `sh -n`, and a bash and a POSIX mutant that parse and are
caught read KILLED) passed, since they pin behaviour the runner already had and guard the fix
against a checker that always picks `sh -n`. The fix at 270bbaa parse-checks the mutated bytes of a
shell target before the cargo branch, and all six pass. Five rows pin its decisions, S03929 to
S03933, each proved KILLED with `scripts/mutation_rows.py` restoring its target byte for byte.
Fix round 1 added twelve tests to the same class at 96ae492, each pinning a branch of `shell_parser()`, `parses()` and the shell check in `builds()` that a hand-mutation sweep of the runner's new code (26 mutants) had left standing: the production code was already right, so each is red only against its mutant, measured on a scratch copy and recorded below, and green at the head. Ten rows, S03934 to S03943, pin the same decisions and each is proved KILLED. Fix round 2 found `== 0` read as `<= 1` in `parses()` is not equivalent: `bash -n` exits 1 on an array assignment left open at the end of its input, so a test and row S03944 pin it.
The five rows the tree already held on a shell target (S05706, S05806, S05807, S05808, S05809, all on
`scripts/check.sh`) were re-proved at that head: each reads KILLED, so none was a parse failure
passing for a kill. A8's cargo path is proved by CI's run of the module.

Fix round 1's reds, each measured on a scratch copy of 96ae492 with its mutant applied, sit here and not in the fence, which holds one red line and one green line per criterion:

- A41: red at 96ae492: AssertionError: 0 != 3 : S00025-SH-SHEBANG-BASHISM: KILLED: its killer passed without the mutant and failed with it (measured on a scratch copy at 96ae492, mutant M03: a shebang naming sh always read as bash)
- A41: red at 96ae492: AssertionError: 3 != 0 : S00026-BASH-EXT-CAUGHT: VOID: the mutant does not parse: sh: sh: 1: Syntax error: "(" unexpected (measured on a scratch copy at 96ae492, mutant M05: a .bash target read as sh)
- A41: red at 96ae492: AssertionError: 0 != 3 : S00027-BASH-EXT-UNPARSED: KILLED: its killer passed without the mutant and failed with it (measured on a scratch copy at 96ae492, mutant M06: the .bash branch removed)
- A41: red at 96ae492: AssertionError: 0 != 3 : S00028-SH-EXT-BASHISM: KILLED: its killer passed without the mutant and failed with it (measured on a scratch copy at 96ae492, mutants M07 and M08: a .sh target read as bash, and the .sh branch removed)
- A41: red at 96ae492: AssertionError: 0 != 3 : S00030-DASH-UNPARSED: KILLED: its killer passed without the mutant and failed with it (measured on a scratch copy at 96ae492, mutant M11: dash dropped from the shebang pattern)
- A41: red at 96ae492: AssertionError: Lists differ: ['MARK', '3', 'MARK', 'MARK', '2'] != ['MARK', '3', 'MARK', '2'] (measured on a scratch copy at 96ae492, mutant P01: the parse check without -n executes the mutant)
- A41: red at 96ae492: AssertionError: None != 'the mutant is unchecked: bash is not installed' (measured on a scratch copy at 96ae492, mutant P03: a missing parser passing the mutant)
- A41: red at 96ae492: AssertionError: None != 'the mutant is unchecked: bash -n timed out' (measured on a scratch copy at 96ae492, mutants P02 and P04: no bound on the check, and a hung parser passing the mutant)
- A41: red at 96ae492: AssertionError: "the [22 chars] bash: bash: line 1: `if then'" != "the [22 chars] bash: bash: line 1: syntax error near unexpected token `then'" (measured on a scratch copy at 96ae492, mutant P06: the last stderr line in place of the first)
- A41: red at 96ae492: AssertionError: Lists differ: ['cargo'] != ['bash', 'cargo'] (measured on a scratch copy at 96ae492, mutant B03: the shell check skipped for a cargo killer)
- A41: red at 96ae492: AssertionError: Lists differ: ['cargo', 'bash'] != ['bash', 'cargo'] (measured on a scratch copy at 96ae492, mutant B04: the shell check made after the cargo branch)
- A41: red at 96ae492: AssertionError: 'sh' is not None (measured on a scratch copy at 96ae492, mutant M10: a target that is not a shell script given sh)
- A41: red at 8bbeac4: AssertionError: 0 != 3 : S00034-BASH-OPEN-ARRAY: KILLED: its killer passed without the mutant and failed with it (measured on a scratch copy at 8bbeac4, mutant P05: an exit of 1 from the parser read as a parse)

## Addendum, 2026-09-29: the bin killer (issue #352)

A42 to A44 are the acceptance criteria of section 16, made by issue #352's delivery. Each was
written red at b47b9a3, where `bin` is no killer kind and `scripts/mutation_rows.py` refuses it
by name, and each is green at 1adadce, which adds the kind. Each red fails by assertion: the tests
turn the runner's refusal into a failed assertion, and the failures name no path outside the
repository. The original lines above stand.

```red-first
A42: red at b47b9a3: AssertionError: a bin killer is refused: crates/fix has no test target bin
A42: green at 1adadce
A43: red at b47b9a3: AssertionError: {'S00054-BIN-KILLED': 'VOID', 'S00055-BIN-NO-TEST': 'VOID'} != {'S00054-BIN-KILLED': 'KILLED', 'S00055-BIN-NO-TEST': 'VOID'}
A43: green at 1adadce
A44: red at b47b9a3: AssertionError: 'census: S00058-BIN-LIB-TEST: its killer bin::tests::only_in_the_lib names no test: crates/fix/src/main.rs declares only_in_the_lib 0 times' not found in 'census: S00056-BIN-ROOT: its killer bin::tests::three_triples_to_nine crates/fix has no test target bin\ncensus: S00057-BIN-MODULE: its killer bin::helper::tests::four_halves_to_two crates/fix has no test target bin\ncensus: S00058-BIN-LIB-TEST: its killer bin::tests::only_in_the_lib crates/fix has no test target bin\nexamined 3 row(s)\n'
A44: green at 1adadce
A45: red at a4ce513: AssertionError: 'S00059' unexpectedly found in 'census: S00059-BIN-OTHER-ROOT-MODULE: its killer bin::helper::tests::four_halves_to_two names no test: crates/fix/src/other.rs declares four_halves_to_two 0 times\nexamined 1 row(s)\n'
A45: green at 3a3bdd6
```

Review of this pull request found that a root file not named `main.rs`
was walked as if its modules sat under a directory, and that a crate holding `tests/bin.rs` had a
`bin::` killer rerouted to the binary's own test. A45, the criterion of section 16 for both, is
the fix round's: its two tests were written red at a4ce513 against the head's runner, and are green
at 3a3bdd6, which changes the runner alone. Each red fails by assertion. A42 to A44 are unchanged:
the fence line for A44 above is now the full failure message, where the first version cut its end.
A45 has two tests, and the fence holds one line for the criterion; the other test's red, verbatim:

```text
test_a_bin_killer_beside_a_tests_bin_rs_is_refused: AssertionError: 'census: S00060-BIN-SHADOW: its killer bin::tests::three_triples_to_nine crates/fix has a test target bin, which the bin kind shadows' not found in 'examined 1 row(s)\n'
```

## Addendum: the `bin` kind reads what the compiler builds (section 19, issue #405)

The tests of A46 to A49 were committed first, alone, at a3d4ff9b, against the runner as it stood
on `dev`. They fail by assertion, not by error: the generated population agreed with the oracle on
75 of its 165 members, and each planted shape of the issue failed
for its own reason. The runner's fix went green at 9595183f. The population was then widened (over
`cfg`, lexemes and blocks at e43c60ef, over target dedupe, dotted paths and unpathed tables at
349d2423) and A50's refusals and A51's predicate test were added with them (A51 at da82cca3), each green on arrival against the fixed
reader and proved by the mutants listed in the pull request.

```red-first
A46: red at a3d4ff9b: AssertionError: 75 != 165 : a member was neither agreed nor refused
A46: green at 9595183f
A47: red at a3d4ff9b: AssertionError: Lists differ: ['src/main.rs', 'src/x.rs'] != ['src/main.rs']
A47: green at 9595183f
A48: red at a3d4ff9b: AssertionError: KillerUnresolved not raised
A48: green at 9595183f
A49: red at a3d4ff9b: AssertionError: "holds 2 binaries," does not match "crates/fix holds 3 binaries, and a bin killer names none of them"
A49: green at 9595183f
A50: not red: its first cases passed against the unchanged runner, and the cases added with the fix's widening (a file module in a block, a malformed declaration, a missing binary file) test code the fix introduced
A51: not red: it tests `cfg_value`, which the fix introduced and `dev` does not have, so there is no runner to fail; mutants of the function prove it instead
```

The examined lines: at the fix (9595183f) the population read `examined 165 generated crate
layouts` and `agreed with the oracle 162, refused by name 3`; at da82cca3 it reads `examined 268
generated crate layouts` and `agreed with the oracle 247, refused by name 21`, and A51 reads
`examined 2406 predicates` with `329 decided, every one equal to rustc's value`. At 630c25db and at
bc6ee994 it reads `examined 316 generated crate layouts` and `agreed with the oracle 292, refused
by name 24`. Those two commits pin the token reader's nested block comments and then bound that
scan by the input's length; the test added at 630c25db reads `nested block comment shapes examined
5` and passes at both, so the bound changed no reading the test sees.

## Addendum: the five classes the population did not hold (section 21, issue #405, 2026-10-01)

The families of section 21 were committed first, alone, at 24ec6c76, against the reader as it
stood at 6d3a9638. They fail by assertion, and the failing members are those of the five classes:
a `mod` inside a macro invocation, a path through `..`, a dotfile, an absolute path, and a manifest
with no `edition` key. The reader's fix went green at 87d385df. A53 pins a bound the reader already
had, so it is not red.

```red-first
A52: red at 24ec6c76: AssertionError: 1186 != 1610 : a member was neither agreed nor refused
A52: green at 87d385df
A53: not red: it pins the block-comment bound the reader already had; under the mutant that shortens the bound by three it fails with AssertionError: '/*x' reads ['x'], where [] is due
```

The planted refusals of A50 for an edition the reader cannot decide were red at 24ec6c76 for their
own reason:

```text
AssertionError: KillerUnresolved not raised
```

The examined lines at 87d385df read `examined 1610 generated crate layouts` and `agreed with the
oracle 1523, refused by name 87`, and the block-comment test reads `examined 13344 block comments
left open`.
