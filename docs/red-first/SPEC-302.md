# Red-first record: SPEC-302

The SPEC, its schematic and the two pointer edits in SPEC-090 and SPEC-095 were committed first.
The goldens and their registry followed, then the four tests against stubs of `pynum` that compile
and return a wrong value, so each test fails by assertion. The implementation turns them green.
