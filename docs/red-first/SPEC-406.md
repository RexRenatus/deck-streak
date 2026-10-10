# Red-first record: SPEC-406

The delivery takes one push. Every criterion is decided by `scripts/tests/test_dynamic_imports_check.py`,
which runs before the push, so each red was read where it was written. The first commit (027ce1b9)
carries SPEC-406, ADR-420, the new test module, a stub `scripts/dynamic-imports-check.py` that
keeps every name and signature and judges nothing, the three register lines in
`scripts/tests/test_ci_workflows.py` and the map entry. The second commit (95f0a305) carries the
script, the SPEC template's paragraph and the schematic's section; no test file changes in it.

Each red is the first assertion that failed against the stub, by assertion and not by an error.
The module was run by its file path, alone, as `python3 scripts/tests/test_dynamic_imports_check.py`.

MUTATION COVERAGE: the python mutation run over the script found eight survivors at the second
commit (107 killed). Two were behaviours no case observed, and the cases that kill them are named
here as such: `exec` after an unrelated `import` and after an unrelated `from` import, and `exec`
bound by an `import ... as exec` and by a `from ... import ... as exec`. Six sat on redundant text
(a `text=True` that `encoding` implies, a default for an absent module, a star-import guard that a
star never needs, and three help strings) and were removed from the script with no change in
behaviour. A second run (105 killed) left one survivor, which a case with a sibling module named
`runpy` imported by a relative `from` kills; the last run read 106 killed, 0 survived. The cases
were added to the table of A3 in 73f34e58 and 86d03e0d.

```red-first
A1: red at 027ce1b9: AssertionError: 0 != 1
A1: green at 95f0a305
A2: red at 027ce1b9: AssertionError: Lists differ: [] != ['dynamic-imports: register: 2 module(s) i[124 chars] OK']
A2: green at 95f0a305
A3: red at 027ce1b9: AssertionError: Lists differ: [] != [(2, 'importlib')]
A3: green at 95f0a305
A4: red at 027ce1b9: AssertionError: '_support' not found in set()
A4: green at 95f0a305
A5: red at 027ce1b9: AssertionError: 0 != 2
A5: green at 95f0a305
A6: red at 027ce1b9: AssertionError: 0 != 1 : []
A6: green at 95f0a305
A7: red at 027ce1b9: AssertionError: Lists differ: [] != ['dynamic-imports: register: 1 module(s) i[136 chars]BLE']
A7: green at 95f0a305
A8: red at 027ce1b9: AssertionError: 'scripts/tests/test_ci_workflows.py' not found in '\n\n| file | context | change |
A8: green at 95f0a305
```

The green lines above name C2 `95f0a305`, which CI never ran. The module and the script changed after it (`73f34e58`, `86d03e0d`). CI ran the criteria green at the verified head `84b3c0af` (hygiene job 114249800504).
