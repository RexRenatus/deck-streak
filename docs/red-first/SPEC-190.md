# Red-first record: SPEC-190

The SPEC, in `docs/specs/planned/`, and ADR-190 were committed alone (737d547). Then came the test
of A1 to A4 (5a85979) against the unchanged workflows, so every test ran and each red failed by
assertion. The workflow change (6b5c663) turned them green. The replay ran the whole test file on
5a85979's tree: four tests, two red by assertion.

```red-first
A1: red at 5a85979: ['changelog.yml: 0 concurrency blocks, so a job's own or none', 'changelog.yml: carries no concurrency block', "engine-measure.yml: group is 'engine-measure-${{ github.ref }}'", "engine-measure.yml: cancel-in-progress is 'false', so a superseded pull-request run finishes"] != []
A1: green at 6b5c663
A2: red at 5a85979: 'changelog.yml: two runs of a push to dev share a group' and 'engine-measure.yml: two runs of a push to dev share a group' in a list of 10 that should be empty
A2: green at 6b5c663
A3: not red: release.yml was already tag-only and never cancelled a run in progress; the criterion pins that it stays so
A4: not red: ci.yml's and mutation-weekly.yml's blocks were already the rule's; the criterion pins that they stay so
A5: not red: the five workflows' names were already distinct; the criterion holds a new workflow to it
A6: not red: no workflow cancelled a run that is not a pull request's, and no job set a block; the criterion holds a new workflow to it
```

## Addendum: A7 (ADR-292, #377)

The test of A7 (b51b933) ran against the unchanged `release.yml`: the criterion's test failed by
assertion on the workflow's missing queue, and no other test failed. The workflow change (6696ef6)
turned it green.
Measured again on 2026-09-30 at b51b933 with the final test body, two tests fail: A7's and `test_a_shape_that_replaces_drops_or_runs_two_at_once_is_refused`, through its `planted(queue, queue)` precondition, so "no other test failed" holds for the first body only.

```red-first
A7: red at b51b933: ['release.yml: queue is None, so a third run of a tag replaces the waiting second'] != []
A7: green at 6696ef6
```


## Addendum: A8, the class as GitHub reads it (#456, verification round 2)

The test of A8 (`the_release_class_is_read_as_github_reads_it`, ff6c290) ran with its tables and its planting helper against the class helpers and reader as they
stood: it failed by assertion, naming 33 planted shapes the class did not refuse, the first a create
named alone, and no other test in the module failed. Reading `create`, each workflow's own name and
events, the block's keys with their case and a key held twice (d09c26b) turned it green.

```red-first
A8: red at ff6c290: ["create as a name: ['queue is None, so a third run of a tag replaces the waiting second']", ...] (33 shapes) != []
A8: green at d09c26b
```

## Addendum: A9, the class closed by construction (#456, verification round 3)

The test of A9 (`the_release_class_is_closed_by_construction`, 4835abf) ran with its constants and
helpers and with `closed_by_construction` returning no problem: it failed by assertion, naming 1313
planted members the class did not refuse, the first a group reading `github.action`. It was the only
test of `test_workflow_concurrency.py` that failed (14 ran, one failed). The rule (445354c) turned it
green: 14 ran and all passed.

The advisory of the A7 addendum above, "the final test body", meant the body of
`test_workflow_concurrency.py` (with `test_ci_workflows.py`) at d09c26b, run on b51b933's tree: two
tests of that one file failed, A7's and `test_a_shape_that_replaces_drops_or_runs_two_at_once_is_refused`
(13 ran), and no test of another file was run.

```red-first
A9: red at 4835abf: ['a group reading github.action', ...] (1313 members) != []
A9: green at 445354c
```

## Addendum: A10, the class read as GitHub parses it (#456, verification round 4)

The test of A10 (`the_release_class_is_read_as_github_parses_it`, a36ab6d) ran with the membership
census derived from the rule, and with `calls`, `membership` and `release_class_problems` returning
nothing: it failed by assertion, naming 289 members the class did not refuse, the first a
`cancel-in-progress` of `'false'`, and the census failed by assertion on its empty population. They
were the two tests of `test_workflow_concurrency.py` that failed (15 ran, two failed). The rule and
the reader (3a0eb6d) turned both green: 15 ran and all passed.

```red-first
A10: red at a36ab6d: ["types: cancel 'false' is accepted", ...] (289 members) != []
A10: green at 3a0eb6d
```

DISCLOSURE, a test body changed after its green: test_only_the_tag_or_release_workflows_are_in_the_class gained a planted fixture at 229cf07. A second workflow a tag starts and a workflow the release calls must each be classed as release workflows, and a reacher must not. With membership reduced to the tag-started workflows, or to release.yml alone, the test fails.

## Addendum: A10 and A11, the reader default-deny over its grammar (#456, round 6)

The tests of A11 (`the_reader_reads_only_its_named_forms`) and of A10's grammar members (b2fa82a)
ran against the reader as it stood: A11 failed by assertion, naming 498 of its 2262 generated forms
the reader read, or refused, otherwise than R12 part 1 requires, the first an item `#x` read as
text, and A10 failed by assertion, naming 120 of its 616 members the class did not refuse, the
first a release workflow's name starting with `@`. They were the only tests of the two files that
failed. The named-form reader (c78bbc8) turned both green.

```red-first
A11: red at b2fa82a: [("item '#x'", ('read', {'k': ['#x']}), ...), ...] (498 forms) != []
A11: green at c78bbc8
```

A10's replay over the same grammar members, quoted as text because A10 is recorded above:

```text
A10: red at b2fa82a: ["grammar: release.yml line 1 starting '@' is accepted", ...] (120 members) != []
A10: green at c78bbc8
```

Round 7 (R12 part 1's line end and tab refusals). The tests of A11 and of A10's population (81b3204)
ran against the reader as it stood: A11 failed by assertion, naming 297 of its 2573 generated forms
the reader read, or refused, otherwise than R12 part 1 requires, the first a carriage return that
does not end a line read as a line end, and A10 failed by assertion, naming 500 of its 1081 members
the class did not refuse, the first a tab-indented comment line. They were the only tests of the two
files that failed (A11's file ran 31 tests with 1 failure, A10's file ran 15 with 1). The reader that
ends a line only at a line feed or a carriage return and a line feed, and refuses a tab in every
line's indentation (a2899a2), turned both green. Both are quoted as text, because A10 and A11 are
recorded above:

```text
A11: red at 81b3204: [('a carriage return that does not end a line at the end of a value', ...), ...] (297 forms) != []
A11: green at a2899a2
A10: red at 81b3204: ['tabs: a tab-indented comment line is accepted', ...] (500 members) != []
A10: green at a2899a2
```

The tests read a workflow file's bytes as GitHub does (ee89d8e): a workflow file planted with a lone
carriage return in the concurrency block, or inside a block scalar's text, was read by four of the
tests' loaders as a file without it, and the census of the test modules' file reads named no loader.
Two of the four new tests failed by assertion, 9 checks in all (8 loader reads of a planted file
and the census); the CRLF and the planted-census tests passed from the start, as controls. The one
loader that reads the bytes (81ff76b) turned all four green. Quoted as text:

```text
loaders: red at ee89d8e: the file was read, not refused (8 reads: 2 shapes x 4 loaders)
census: red at ee89d8e: 'def workflow_file_text(' not found in the test module
loaders: green at 81ff76b
census: green at 81ff76b
```

## Addendum: A12, the census default-deny over the loader's population, and A11's tab after a dash (#456, round 8)

The killer was committed first, alone, at 5bbe692, with A11's members for a tab after a sequence
item's `-` and the refusal's words for a tab in the indentation. Against the head's census, which
read `read_text`, `read_bytes` and `open` by name in the modules whose source names
`test_ci_workflows`, 56 of the 90 plants stayed green, each failing the
killer by assertion; the three controls and the plants the head's census already refused passed.
A11 failed by assertion on 97 members. The census over the loader's population and
the reader's refusal of a tab after a dash (610a658) turned both green. A12 is recorded in the
fence below and A11, recorded above, is quoted as text:

```red-first
A12: red at 5bbe692: 56 plants: the census was green
A12: green at 610a658
```

```text
A11: red at 5bbe692: First list contains 97 additional elements.
A11: green at 610a658
```

## Addendum: A12, the census places the interpreter's exception classes by their value (#456, round 10)

With dev merged, the census read two new test modules and refused six sites: a builtin exception
class its hand list lacked, and the dynamic sites of the formal settings tests. The killer was
committed first, alone, at 70ccd8c: over the 69 exception classes the interpreter's builtins
define, each planted in an `except`, a `raise` and an `isinstance`, 56 were refused as a name the
census cannot place (`ArithmeticError`, `AttributeError`, `BaseExceptionGroup` among them); the
second population, 25 other builtin names and 10 read or dynamic ones, stayed red as before and
passed as a pin. The census's derivation of the classes by their value, and the five listings
(5d471ab), turned the first green; A12 is recorded above, so it is quoted as text:

```text
A12: red at 70ccd8c: 56 builtin exception classes the census cannot place
A12: green at 5d471ab
```
