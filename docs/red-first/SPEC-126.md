# Red-first record: SPEC-126

The SPEC, in `docs/specs/planned/`, and ADR-126 were committed alone (773cca1). Then came the test
of A1 to A4 (781f4a4) against the unchanged `ci.yml`: a pure-Python emulation of the pinned
download action's layout rule over the verdict's steps, so every test ran and each red failed by
assertion. The workflow change (2795033) turned them green. The replay ran the whole test file on
781f4a4's tree: four tests, three red by assertion.

```red-first
A1: red at 781f4a4: 'T/reports/mutation-plan/plan.json' not found in {'T/reports/git.diff', 'T/reports/whole.json', 'T/reports/plan.json'} : 0 shard(s), rows False, web False
A1: green at 2795033
A2: red at 781f4a4: True is not false : a verdict download reaches mutation-web: mutation-*
A2: green at 2795033
A3: red at 781f4a4: 0 != 1 : the rows' report is not downloaded by name
A3: green at 2795033
A4: not red: the judge's command lines and its reports directory were already as the criterion says; it pins that the downloads keep them
```

## Addendum, 2026-09-29 (issue #358): A4 asserts per judge line

The original lines above stand. The head is correct, so the per-line assertion cannot be red on it;
it is proved by plant, in a scratch copy of `ci.yml` that the test module's `CI` constant was
pointed at (the delivery tree's file was never edited). The old test is the module as it stood
before this change.

Plant a: `--plan` removed from the `rust` judge line, the `oracle` line still carrying it.

```text
old test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
new test: AssertionError: None != '"$reports/mutation-plan/plan.json"' : the rust judge line does not read --plan at $reports/mutation-plan/plan.json
```

Plant b: `--rows` removed from the `oracle` judge line, the `rust` line still carrying it.

```text
old test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
new test: AssertionError: None != '"$reports/mutation-rows/rows.json"' : the oracle judge line does not read --rows at $reports/mutation-rows/rows.json
```

The test compares each path as the shell reads it, with its double quotes, and refuses a class
named by two judge lines. Two further plants record that.

Plant k2: the `rust` line's `--whole` path in single quotes, which the shell does not expand. The
old test failed it; the test as first written passed it; the test as fixed fails it.

```text
old test: AssertionError: '--whole "$reports/mutation-plan/whole.json"' not found in the verdict job
first new test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
new test: AssertionError: "'$reports/mutation-plan/whole.json'" != '"$reports/mutation-plan/whole.json"' : the rust judge line does not read --whole at $reports/mutation-plan/whole.json
```

Plant m: a wrong `rust` judge line, with no `--whole`, placed before the right one. The old test and
the test as first written passed it, because the later line replaced the earlier in the table.

```text
old test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
first new test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
new test: AssertionError: two judge lines of class rust: --plan "$reports/mutation-plan/plan.json" --class rust --shard-reports "$reports" --rows "$reports/mutation-rows/rows.json" --whole "$reports/mutation-plan/whole.json"
```

## Addendum, 2026-09-29 (issue #374): A5 and A6, the reader splits words as the shell does

The lines above stand. The fixtures are strings in the test, so the red is committed: the fixture
tests went in first against the reader of the section 8 amendment (2bf4a9a), then the new reader
(11469f9). Each wrong path is also proved against a scratch copy of `ci.yml` with the judge lines
rewritten (the delivery tree's file was never edited).

```red-first
A5: red at 2bf4a9a: '"${reports}/mutation-plan/plan.json"' != '$reports/mutation-plan/plan.json' : the rust judge line does not read --plan at $reports/mutation-plan/plan.json
A5: green at 11469f9
```

The old reader on a scratch `ci.yml` per spelling, and the new reader on the same:

```text
old: braces ............................ '"${reports}/mutation-plan/plan.json"' != '$reports/mutation-plan/plan.json'
old: expansion closed before the slash . '"$reports"' != '$reports/mutation-plan/plan.json'
old: tail in single quotes ............. '"$reports"' != '$reports/mutation-plan/plan.json'
old: quoted flag name .................. a judge line names no --class: "--plan" "$reports/mutation-plan/plan.json" '--class' rust ...
old: backslash continuation ............ None != '$reports/mutation-rows/rows.json'
old: --flag=value, value quoted ........ None != '$reports/mutation-plan/plan.json'
old: --flag=value, quotes close early .. None != '$reports/mutation-plan/plan.json'
new: all seven ......................... test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
```

The new reader on the wrong paths:

```text
'$reports/...' single-quoted ........ '\\$reports/mutation-plan/plan.json' != '$reports/mutation-plan/plan.json'
'$reports'/... single-quoted head ... '\\$reports/mutation-plan/plan.json' != '$reports/mutation-plan/plan.json'
"\$reports/..." escaped dollar ...... '\\$reports/mutation-plan/plan.json' != '$reports/mutation-plan/plan.json'
a different directory ............... '$reports/mutation-plans/plan.json' != '$reports/mutation-plan/plan.json'
a different variable ................ '$report/mutation-plan/plan.json' != '$reports/mutation-plan/plan.json'
an unquoted expansion ............... '(unquoted)$reports/mutation-plan/plan.json' != '$reports/mutation-plan/plan.json'
--whole moved to the oracle line .... None != '$reports/mutation-plan/whole.json' : the rust judge line does not read --whole
--rows dropped from the rust line ... None != '$reports/mutation-rows/rows.json' : the rust judge line does not read --rows
```

## Addendum, 2026-09-29 (issue #374, fix round 1): A6 for the shell's command end

The lines above stand, except that A6's `not red:` line is replaced by the two lines in the fence
below. The judge-line reader was tightened after review: it ends the command at a control operator
or a comment outside every quote and keeps the command's words on one line. Three fixtures joined
the wrong paths (a flag only in a comment, a flag after a control operator, a line break after
`judge` with no backslash); the red commit changes only those fixtures, and the reader commit
changes only the reader (commit 42f81c4 edits the test module because the reader lives there: the
`JUDGE` pattern and `mark_dollars`, and no fixture or test method). The whole test file at the red commit fails only A6, three subtests, each
`AssertionError: AssertionError not raised`.

```red-first
A6: red at 14a6b2a: FAILED (failures=3), AssertionError: AssertionError not raised, for each of "a flag only in a comment", "a flag after a control operator" and "a line break after judge with no backslash"
A6: green at 42f81c4
```

The A5 red line above quotes a failure that came from `check_judge` comparing a word to
`f'"{path}"'`; its comparison changed to `path` between that red (2bf4a9a) and its green (11469f9),
because the reader's representation changed from quoted words to shell-split words. The fixtures
and the test methods stayed byte-identical across the change.

## Addendum, 2026-09-29 (issue #394): A7 for redirections and substitutions

The lines above stand. The red commit changes only the fixtures (four wrong paths); the green commit
00c589f9 changes only the reader's break set and its docstring, in `scripts/tests/test_verdict_download.py`,
because the reader lives in the test module. The whole test file at the red commit fails
only A7, four subtests, each `AssertionError: AssertionError not raised`.

```red-first
A7: red at 27515b29: FAILED (failures=4), AssertionError: AssertionError not raised, for each of "a flag as a redirection's target", "a flag inside a command substitution", "a flag inside backquotes" and "a flag inside a process substitution"
A7: green at 00c589f9
```

## Addendum, 2026-09-29 (issue #394, round 2): A7 inside double quotes

The lines above stand, and A7 keeps its one red and one green line; this replay is quoted below
them. The red commit e0945c6b adds only two fixtures, a flag inside a double-quoted `$( )` and a
flag inside double-quoted backquotes; the green commit cff9f209 changes only the reader and its
docstring, in `scripts/tests/test_verdict_download.py`, because the reader lives in the test module.
The whole test file at the red commit fails only A7, two subtests, each
`AssertionError: AssertionError not raised`.

```text
A7 replay: red at e0945c6b: FAILED (failures=2), AssertionError: AssertionError not raised, for each of "a flag inside a double-quoted command substitution" and "a flag inside double-quoted backquotes"
A7 replay: green at cff9f209: Ran 7 tests, OK, examined 17 wrong paths
```

## Addendum, 2026-09-29 (issue #394, round 3): A7 with an ANSI-C string

The lines above stand, and A7 keeps its one red and one green line; this replay is quoted below
them. The red commit 372f3614 adds only one fixture, a flag inside an ANSI-C string with an escaped
quote; the green commit 950edffb changes only the reader and its docstring, in
`scripts/tests/test_verdict_download.py`, because the reader lives in the test module. The whole
test file at the red commit fails only A7, one subtest, by assertion.

```text
A7 replay: red at 372f3614: FAILED (failures=1), AssertionError: AssertionError not raised, for "a flag inside an ANSI-C string with an escaped quote"
A7 replay: green at 950edffb: Ran 7 tests, OK, examined 18 wrong paths
```

## Addendum, 2026-09-30 (issue #438): A8 and A9, a report bound to its slot and its listing

The red commit adds `test_mutation_python_shard_binding.py` against the unchanged verdict and
changes nothing else: the correct layout passes as the control, and the verdict accepts 129 of the
129 slot-binding members and 302 of the 311 listing-binding members (only the missing-report family
was refused). Both tests fail by assertion. The green commit changes `python_reports` in the
verdict and the fixtures of five earlier tests that laid one report in several slots or listed no
shards. Commit 7fa6ed7 edits those two test files, `test_mutation_python_verdict.py` and
`test_mutation_verdict_python_kills.py`, and each edit gives a plan the shard listing its report
examined and the report its own slot's `shard` field, the layout the rule now requires. One
assertion is rewritten with its fixture: the expected `examined` count of the whole layout is now
the literal 81, the listing's size, where it was three times the mutants of one full report. No
assertion is dropped or weakened. The population then reads 129 refused of 129 and 311 refused of 311, none accepted.

```red-first
A8: red at 11ab33f: AssertionError: 129 != 0 : 129 of 129 members accepted
A8: green at 7fa6ed7
A9: red at 11ab33f: AssertionError: 302 != 0 : 302 of 311 members accepted
A9: green at 7fa6ed7
```
