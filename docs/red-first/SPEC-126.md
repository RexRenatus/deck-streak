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

## Addendum, 2026-09-30 (issue #438, fix round 1): A10 and A11, one reader for the binding and the judge

The red commit adds two tests to `test_mutation_python_shard_binding.py` against the unchanged
verdict. The generated population lays a wrong container, an outcome off the runner's vocabulary
and a mutant under an unread path, per shard, over plans of one to four shards; the verdict refuses
70 of 450, crashes on 90 and accepts 290. The empty-listing population refuses 2 of 22, crashes on
12 and reads 8 otherwise. The green commit adds the shared reader and the binding and the judge
both read its yield; both populations are then refused in full. The later commits add four
families (a byte-readers container of another type, a mutant name of another type, a mutant record
of another type, and a listed mutant filed under an unread path), so the populations read 570 of
570 and 27 of 27.

The green commit, 5810a75, also rewrites one assertion in `scripts/tests/test_mutation_python_shard_binding.py`,
in the earlier test of a mutant record that is not an object:
`self.assertIn("0 missing () and 1 extra (?)", output)` becomes `self.assertIn("a mutant record of", output)`
and `self.assertIn("not an object", output)`. The shared reader now refuses that record by name
before any listing is counted. That sentence was wrong, and round 3 corrects it: the rewritten
assertion pinned less than the one it replaced, because it dropped the record's path and its type
from the reading while still demanding exit 3 naming the shard. Round 3 pins every variable part of
both messages again (see the round 3 addendum below).

```red-first
A10: red at 1c4c0d45: AssertionError: 70 != 450 : {'refused': 70, 'crashed': 90, 'accepted': 290, 'other': 0}
A10: green at 5810a754
A11: red at 1c4c0d45: AssertionError: 2 != 22 : {'refused': 2, 'crashed': 12, 'accepted': 0, 'other': 8}
A11: green at 5810a754
```

The same populations, judged in process with one end-to-end control per family, are red at 8a9295b7 by the same assertion, in prose so that A10 keeps its one red and one green line above: `AssertionError: 150 != 570 : {'refused': 150, 'crashed': 110, 'accepted': 310, 'other': 0}`; green at 473388e3.

The verdict program is correct at the head, so a test of a skipped entry is not red against it; it is red against the one swap that stops a loop early. The CI report of the in-process head had one survivor, `scripts/mutation-verdict.py:1465:13` (`continue` replaced by `break` in `read_python_shard`), which no earlier test laid. Commit ee0c987a adds `ASkippedEntryDoesNotHideALaterOne`, whose census reads the AST of `python_reports`, `read_python_shard` and `judge_python` and finds 12 exits (3 loop skips in the first two, 6 early returns, 3 loop skips in the judge), and puts an entry whose reading changes the verdict behind each. Under `continue` replaced by `break` at 1465:13 it is red with `AssertionError: Tuples differ: (0, set()) != (3, {0})`; at 1411:13 and 1416:13 with `(3, {0}) != (3, {0, 1})`; at 1599:13 with `3 != 1 : void entry`; at 1609:17 with `3 != 1 : timeout`; at 1617:17 with `'UNCOVERED ... not found in 'mutation: scripts: SURVIVED ...'`. Each of the six early returns of the shard reader, swapped for `continue`, is red as well. Green at the head, 44 and 96 members of two and four shards, and row S12620 is KILLED by its full id.

## Addendum, 2026-10-01 (issue #438, fix round 3): every variable part of the report messages is pinned

Round 2's rewrite of the record test pinned less, as the correction above says, and nothing seen by
the CI mutation operators could show it: they generate no mutants inside an f-string's text. The
class rule is that a test that reads a message pins every variable part of it on the same input.
The population is therefore generated from the program's own syntax tree: every interpolating
f-string in `python_reports`, `read_python_shard`, `shard_listing_drift` and `judge_python`, keyed
by function, template and occurrence, with each interpolated field dropped in turn by compiling a
rewritten function into the loaded program; a pin must then fail by assertion.

The red commit, e8d9f538, adds the population test and weakens the drift pin to the head's reading
(exit code and slot only), so the test is red in `test_mutation_python_shard_binding.py` and nowhere
else: it counts 47 interpolated fields of 24 asserted messages and finds 7 with no pin that fails
(`MISS 7`: the drift field of the report message, the record's path and type name, and four fields
of the drift tail). The green commit, a7990e5d, restores the drift pin to the whole line and pins
the record refusal's whole line: the same population then reads 47 fields of 24 messages and
`MISS 0`. The other five mapped modules (`test_mutation_verdict`, `test_mutation_equivalent`,
`test_mutation_python_verdict`, `test_mutation_verdict_python_kills`, `test_memory_cap_verdict`) are
untouched and green at both commits.

Two further changes carry no red of their own: the in-process judge now takes the program it
judges, and the pinned in-process-versus-subprocess test grew to 105 members over 24 families, each
outcome of each family (skipped alone, skipped then bad, slot skipped, survivor, timeout and void
entry among them), reading 0 differ with exit codes 0, 1 and 3. Those members already agree at the
head, so they are green there; their red evidence is one planted routing divergence in the CLI
entry (exit 1 mapped to 2), under which the same test reads 12 differ with exit codes 0, 2 and 3,
and the plant was reverted.

Of the mutants this round names, the drift tail dropping the extra names (M10) is not equivalent: the
refusal then reads a different line, and the pin on the whole line fails. The rows S12621 to S12623
carry it and the record's path and type name, each KILLED by its full id. Two earlier mutants are
EQUIVALENT, with the reason: M7 and M11 change only what `python_shards` (the verdict program, lines
872 to 880) writes, which is dict entries and `str` names, so no reader observes the swap.

```text
round 3: red at e8d9f538 (test_mutation_python_shard_binding: MISS 7 of 47 fields, 24 messages); green at a7990e5d (MISS 0)
```
