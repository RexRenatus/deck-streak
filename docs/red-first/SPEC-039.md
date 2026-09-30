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

## Addendum, 2026-09-29: a missing tool is a refusal (issue #431)

A46 to A49 are the acceptance criteria of section 20, made by issue #431's delivery. A46 to A48
were written red at 932358d, against the runner as dev holds it, where the runner spawns `git`,
`cargo`, the interpreter, `bash` and `sh` with no check of its own; A49's updated test was written
red at 27646b2. Each is green at b1bd758, which adds the check to the runner. At 932358d the file
reads `FAILED (failures=73)` over 75 members: 64 members fail with a traceback (`'Traceback'
unexpectedly found`), 8 with a `bash` or `sh` that left the mutant VOID and exit 3 where the rule
says 2, and the census of spawn sites fails with `['git', 'parses', 'builds'] != []`. The
failures name no path outside the repository. The original lines above stand.

```red-first
A46: red at 932358d: AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
A46: green at b1bd758
A47: red at 932358d: AssertionError: Lists differ: ['git', 'parses', 'builds'] != []
A47: green at b1bd758
A49: red at 27646b2: AssertionError: 3 != 2 : S00032-NO-PARSER: VOID: the mutant is unchecked: bash is not installed
A49: green at b1bd758
A48: not red: count, ids and census spawn nothing, so they already exit 0 at dev; a spawn planted in the count branch reds it
```

**A48 is `not red`, and takes no green line.** `count`, `ids` and `census` spawn nothing, so they
already exit 0 with every tool unrunnable at dev. The plant that turns it red is a spawn in the
`count` branch (`tracked_changes(root)` first in that `try`), which reads `AssertionError: 1 != 0 :
Traceback (most recent call last):` from `test_a_verb_that_spawns_nothing_runs_with_every_tool_unrunnable`.

**Three of the 75 members are `not red` at 932358d**: the `git` site, the `retired` verb, in each of
the three modes. `main` already catches `OSError` around `retired` and prints `mutation_rows:
REFUSED: <error>` with exit 2, and a missing, unexecutable or directory `git` raises
`FileNotFoundError`, `PermissionError` or `NotADirectoryError`, which are all `OSError`. The rule
changes their line to the class's own (`retired: REFUSED: missing tool: git: <why>`). The plant that
turns them red is the `retired` mapping to `EXIT_OK` (row S03962).

**The test was strengthened after 932358d, at 27646b2**, with no change to the runner: the line
must also give the reason for its mode (`not found on path`, `not executable`, `is a directory`,
or `no such file`); a fourth mode, a script whose interpreter line names a missing program, joins
the population (100 members); and the cargo build site's member asserts that its shim served the
control run, so the refusal is the mutant build's. Each closes a plant that stayed green: see the
plants below.

**Supersession, A41's missing-parser clause (SPEC-039 section 12), by A49.** Two assertions of
`test_mutation_rows.py` changed, each recorded old then new, verbatim. Neither was removed, skipped
or weakened: the first reads the same fact with the refusal's code and line, and the second
reads it as the exception the parser now raises. The parser-timeout assertions are unchanged.

```text
old: self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
new: self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
old: self.assertEqual(verdicts(done), {"S00032-NO-PARSER": "VOID"})
new: self.assertEqual(verdicts(done), {})
old: self.assertIn("unchecked: bash is not installed", done.stdout)
new: self.assertIn("REFUSED: missing tool: bash", done.stdout)
old: why = runner.parses("bash", b"echo 1\n")
old: self.assertEqual(why, "the mutant is unchecked: bash is not installed")
new: with self.assertRaises(runner.ToolMissing) as refusal:
new:     runner.parses("bash", b"echo 1\n")
new: self.assertEqual(refusal.exception.tool, "bash")
```

The first test is renamed to `test_a_missing_parser_is_a_refusal_naming_it`, and row S03935 follows:
its anchor is now the parse check's `run_tool` call, its mutant the raw `subprocess.run` around it.

**The plants**, each a scratch mutant of `scripts/mutation_rows.py` at b1bd758 run against
`test_mutation_rows_missing_tool.py`, restored after; the first red line of each:

```text
a  refusal mapped to EXIT_SURVIVED:           FAILED (failures=72)  AssertionError: 1 != 2 : prove: REFUSED: missing tool: git: not found on PATH
b  git spawns raw:                            FAILED (failures=13)  AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
b  parse check spawns raw:                    FAILED (failures=25)  AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
b  mutant build spawns raw:                   FAILED (failures=13)  AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
b  killer group unresolved (final test):      FAILED (failures=16)  AssertionError: False is not true : prove: REFUSED: missing tool: cargo: Permission denied
c  restore skipped on the refusal path:       FAILED (failures=36)  AssertionError: 4 != 2 : prove: RESTORE FAILED: crates/fix/src/lib.rs was not restored byte for byte after S00002-CARGO
d  named line drops the tool:                 FAILED (failures=72)  AssertionError: 'git' not found in 'prove: REFUSED' : prove: REFUSED
e  refusal mapped to EXIT_VOID:               FAILED (failures=72)  AssertionError: 3 != 2 : prove: REFUSED: missing tool: git: not found on PATH
e  retired refusal mapped to EXIT_OK:         FAILED (failures=3)   AssertionError: 0 != 2 : retired: REFUSED: missing tool: git: not found on PATH
e  unexecutable file taken for a tool:        FAILED (failures=25)  AssertionError: False is not true : prove: REFUSED: missing tool: git: Permission denied
e  directory taken for a non-file:            FAILED (failures=25)  AssertionError: False is not true : prove: REFUSED: missing tool: git: not found on PATH
e  the PATH search ignores the child's PATH:  FAILED (failures=12)  AssertionError: False is not true : the control never reached the cargo shim
e  the spawn-error backstop removed:          FAILED (failures=24)  AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
```

Four plants stayed green at 932358d's test and were closed by the strengthening at 27646b2: the
unexecutable file, the directory, the PATH search and the killer group's resolution (four rows
of the block above, which are not its last four rows: the last row is the backstop's; the
resolution's spawn-error backstop alone still mapped the refusal, but with the
system's `Permission denied` as its reason). The backstop alone removed reads red only because the
interpreter-line mode joined the population. One plant is EQUIVALENT: the slash in a name searched
along `PATH` (`if False:` for the path branch of `resolve_tool`). The runner spawns a bare name or
`sys.executable`, an absolute path, and `pathlib` discards the `PATH` part when the name it joins
is absolute, so the search reaches the same file; measured green, `OK` over 100 members. A relative
name with a slash is never spawned by the runner.

**Two existing tests gained a patch, and lost no assertion.**
`test_a_bin_killer_runs_cargo_test_on_the_binary_by_its_exact_path` and
`test_a_shell_target_with_a_cargo_killer_is_parsed_first_and_built_second` replace
`runner.subprocess.run` with a recorder so that no real cargo runs, and they hold with the
recorder alone only while a `cargo` is on the machine's `PATH`: the runner now resolves the
executable before the spawn it records. Each gains `mock.patch.object(runner, "resolve_tool")`
beside the recorder, so that they read the same fact on a machine without cargo, where they
errored at the rule's own refusal (`mutation_rows.ToolMissing: missing tool: cargo: not found on PATH`)
in the failure-set measurement. Every assertion stands, and the measurement without cargo on `PATH`
now differs from dev's by the new tests alone.

## Addendum, 2026-09-30: a tool the runner cannot run is refused for any reason (issue #431, round 1)

The addendum above stands as history; where it counts modes or members, this one is the reading at
the head. The head's population is 180 members, over six modes (absent, not executable, a directory,
a script whose interpreter line names a missing program, an empty file with the execute bit, and a
wrapper whose program is missing), and every searched tool again behind a `PATH` entry the runner
cannot look at, plus 3 quiet verbs, 4 spawn sites and 127 spawner spellings. A50 to A52 are the
criteria this round adds to section 20.

The killer file was committed alone at adf4fbf8, against the runner as the earlier commits left it,
and read `FAILED (failures=80)` over `Ran 5 tests`, `examined 180 missing-tool member(s)`, by
assertion in every case. The rule and the census extension are green at 8574e9f4, with `Ran 6 tests`
and `OK`. A50 is red at adf4fbf8; A51 passes there because the census the file already held reads
the two spellings it named. A52 is the census's own test and was added at 8574e9f4.

```red-first
A50: red at adf4fbf8: AssertionError: 1 != 0 : Traceback (most recent call last):
A50: green at 8574e9f4
A51: not red: the census at adf4fbf8 reads the spellings it holds, so it exits OK there; a spawn by another stdlib name planted in `git` reds it (S03981)
A52: not red: it is new at 8574e9f4, beside the census it tests; against the census without `os.startfile`, the loop's two methods and `from os import *` it reads `FAILED (failures=28)`
```

**A51 and A52 are `not red`, and take no green line.** A52's red is a measurement, not a commit:
the test was run against the census as it stood before its extension and failed by assertion
(`AssertionError: [] == [] : from os import *`, and the `os.startfile` spellings), then the census
was extended and the test passed.

**The plants at the head.** Each row of S03960 to S03982 is one plant, a scratch mutant of
`scripts/mutation_rows.py` at the head run against `test_mutation_rows_missing_tool.py` with no
`cargo` on `PATH`, restored after, with the first assertion line and the failure count it reads.
The counts of the first addendum's plants a to e were made at b1bd758 and do not reproduce at the
head, where the population is larger (plant a reads 174 here, not 72); this table is what
reproduces.

```text
S03960-A46-MISSING-TOOL-READS-AS-A-SURVIVOR                  (failures=174)   AssertionError: 1 != 2 : prove: REFUSED: missing tool: git: not found on PATH
S03961-A46-MISSING-TOOL-READS-AS-VOID                        (failures=174)   AssertionError: 3 != 2 : prove: REFUSED: missing tool: git: not found on PATH
S03962-A46-RETIRED-MISSING-TOOL-READS-AS-OK                  (failures=6)     AssertionError: 0 != 2 : retired: REFUSED: missing tool: git: not found on PATH
S03963-A47-GIT-SPAWNS-WITHOUT-THE-CHECK                      (failures=35)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03964-A47-PARSER-SPAWNS-WITHOUT-THE-CHECK                   (failures=62)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03965-A47-BUILD-SPAWNS-WITHOUT-THE-CHECK                    (failures=32)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03966-A46-KILLER-GROUP-SPAWNS-WITHOUT-RESOLVING             (failures=19)    AssertionError: False is not true : prove: REFUSED: missing tool: cargo: Permission denied
S03967-A46-A-REFUSED-PROOF-KEEPS-ITS-MUTANT                  (failures=90)    AssertionError: 4 != 2 : prove: RESTORE FAILED: crates/fix/src/lib.rs was not restored byte for byte after S00002-CARGO
S03968-A46-THE-REFUSAL-LINE-DROPS-THE-TOOL                   (failures=174)   AssertionError: 'git' not found in 'prove: REFUSED' : prove: REFUSED
S03969-A46-A-FILE-WITHOUT-THE-EXECUTE-BIT-RESOLVES           (failures=30)    AssertionError: False is not true : prove: REFUSED: missing tool: git: Permission denied
S03970-A46-A-DIRECTORY-READS-AS-ABSENT                       (failures=30)    AssertionError: False is not true : prove: REFUSED: missing tool: git: not found on PATH
S03971-A46-THE-PATH-SEARCH-IGNORES-THE-CHILDS-PATH           (failures=90)    AssertionError: False is not true : prove: REFUSED: missing tool: git: Permission denied
S03972-A46-A-SPAWN-THAT-FAILS-AFTER-RESOLUTION-IS-A-TRACEBAC (failures=58)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03973-A46-A-PATH-ENTRY-THE-RUNNER-CANNOT-LOOK-AT-STOPS-THE- (failures=55)    AssertionError: 1 != 0 : Traceback (most recent call last):
S03974-A46-RUN-TOOL-CATCHES-ONLY-THREE-SPAWN-ERRORS          (failures=24)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03975-A46-THE-PROCESS-GROUP-SPAWN-CATCHES-ONLY-THREE-ERRORS (failures=10)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03976-A46-A-CHECKED-EXIT-OF-A-MISSING-PROGRAM-IS-A-TRACEBAC (failures=6)     AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03977-A46-RUN-TOOL-READS-EXIT-126-127-AS-A-RESULT           (failures=15)    AssertionError: 3 != 2 : S00002-CARGO: VOID: the mutant does not build
S03978-A46-THE-PROCESS-GROUP-READS-EXIT-126-127-AS-A-RESULT  (failures=9)     AssertionError: 3 != 2 : S00002-CARGO: VOID: its killer selected 0 tests, not one
S03979-A46-EXIT-126-IS-NOT-A-REFUSAL                         (failures=5)     AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03980-A46-EXIT-127-IS-NOT-A-REFUSAL                         (failures=25)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03981-A47-A-SPAWN-BY-ANOTHER-STDLIB-NAME-SKIPS-THE-TOOL-CHE (failures=35)    AssertionError: 'Traceback' unexpectedly found in 'Traceback (most recent call last):
S03982-A47-AN-ALIASING-IMPORT-OF-A-SPAWNER-IS-NOT-SEEN       (failures=1)     AssertionError: Lists differ: ['import subprocess as sp'] != []
```

Two more plants were run and have no row of their own. Removing the spawn-error backstop reads
`FAILED (failures=58)`, and is the same plant as S03972's `if False:` in another spelling. Reading
the search's `PATH` from this process's environment instead of the child's reads `OK`: the tests'
parent and child agree on the tools they name, so the two are not told apart by this file, and
S03971's plant (`path = os.defpath`) is the one that is.

Plants that are not expressible as rows: none. All ten of this round's plants are rows S03973 to
S03982, and S03971 and S03972 were the round's first two.

## Addendum, 2026-09-30: every refusal is read whole (issue #431, round 2)

A53 and A54 are the criteria of section 22. Round 1 pushed with `mutation-verdict` red: 14 generated
mutants of the refusal's own lines survived. The whole-value tests are green at 1361a265 (`Ran 9
tests`, `OK`, 5 examined member lists: 16 reasons by route, 260 errno members, 768 exit members) and
are `not red` at the head they test, because the code already refuses correctly; the plants are
the measurement. Applied to a scratch copy of `scripts/mutation_rows.py`, 26 of the 31 mutants
of `resolve_tool`, `_backstop`, `_exit_refusal` and `run_tool` failed the module by assertion, 4 failed by
error only, and the one that survived, `replace "." with "" in resolve_tool`, could not be told from the original
(`Path("") == Path(".")`), so the runner now reads `Path(part)` and the mutant is gone; round 3 closes the 4. The
census test was committed alone at edb24a2c and reads `FAILED (failures=551)` over 578 members; the
census that refuses them is green at 23b303b9.

```red-first
A53: not red: the code at dev + round 1 already refuses correctly; the generated mutants of the refusal and spawn helpers are measured at round 3 (136 examined: 135 red by assertion, 1 equivalent, 0 error-only)
A54: red at edb24a2c: AssertionError: [] == [] : import os
A54: green at 23b303b9
```

## Addendum, 2026-09-30: the judged tool is the tool the spawn runs (issue #431, round 3)

A55 to A57 are the criteria of section 24. The tests were committed alone at 3c8a340a: the child
directory test with the refusal module, and the two census tests over a census that read neither a
reference nor an import it had not been given. Each fails by assertion and by no error: the child
directory test reads `FAILED (failures=180)` over 384 members, the reference and other-module test
398 of 442, and the unread-import test 558 of 568. The resolver that reads each relative candidate in
the child's directory and the census that refuses a reference and an unread import went green at
fecfe85e: the refusal module `Ran 11 tests`, `OK`, and the missing-tool module `Ran 10 tests`, `OK`.

```text
examined 384 child working directory member(s), 768 whole outcome member(s), 768 exit member(s), 260 spawn errno member(s), 16 refusal member(s), 7 path entry member(s), 5 name-shape member(s)
examined 442 spawner reference member(s), 568 unread import member(s), 578 dynamic spelling(s), refused all; 5 benign source(s), refused none
```

The mutants of the round were generated from the code and run against the modules with no `cargo`
on `PATH`: the 136 generated mutants of `resolve_tool`, `_backstop`,
`_exit_refusal`, `run_tool` and `run_in_own_group`, with the six unrunnable exits, read `red 135,
error-only 0, survived 1` (120 by the refusal module, 10 by the missing-tool module and 5 by the
row census; the one survivor replaces `return None` in `_backstop` with `pass`, which is the same
function). The 88 generated mutants of the census read `red 85, error-only 0, survived 3`; the three
survivors change a fallback name that no source reads, `names[0]` to `names[-1]` over a list of
one, and a `return None` that ends the function anyway.

```red-first
A55: red at 3c8a340a: AssertionError: Tuples differ: ('refused', 'missing tool: refusal-probe-tool: not found on PATH') != ('ran', 'CHILD')
A55: green at fecfe85e
A56: red at 3c8a340a: AssertionError: [] == [] : import subprocess
A56: green at fecfe85e
A57: red at 3c8a340a: AssertionError: [] == [] : import _abc
A57: green at fecfe85e
```

## Addendum, 2026-09-30: the spawn runs the file the runner judged (issue #431, round 4)

A58 to A60 are the criteria of section 26. The tests were committed alone at 146eb84c: the judged-file
test with the refusal module, and the two census tests over a census that reads neither what a read
module reaches nor an annotation that holds code. Each fails by assertion and by no error: the
judged-file test reads `FAILED (failures=40)` over 40 members, the unread-reach test 603 of 679 and
the annotation test 5 of 19. The spawn that executes the judged file and the census that refuses an
unread reach went green at d5420d4b: the refusal module `Ran 12 tests`, `OK`, and the missing-tool
module `Ran 12 tests`, `OK`.

One more test was added at cc0dc969 and is not red: it reads the judged file from a relative working
directory, where the code at d5420d4b already returns an absolute path. It exists for the one mutant
of `resolve_tool` that returns the relative path, which the other tests did not tell from the
original; with that mutant it fails by assertion.

```text
examined 384 child working directory member(s), 768 whole outcome member(s), 768 exit member(s), 260 spawn errno member(s), 16 refusal member(s), 7 path entry member(s), 5 name-shape member(s), 40 judged-file member(s)
examined 442 spawner reference member(s), 568 unread import member(s), 679 unread reach member(s), 578 dynamic spelling(s), refused all; 5 benign source(s), refused none
```

```red-first
A58: red at 146eb84c: AssertionError: Tuples differ: ('ran', 'LATER') != ('refused', 'missing tool: refusal-probe-tool: No such file or directory')
A58: green at d5420d4b
A59: red at 146eb84c: AssertionError: [] == [] : import argparse
A59: green at d5420d4b
A60: red at 146eb84c: AssertionError: [] == [] : import subprocess
A60: green at d5420d4b
```
