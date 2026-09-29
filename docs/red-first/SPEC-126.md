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
A6: not red: the earlier reader compared each word as written, so it already refused every wrong path; the test pins that the new reader still does
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
