# Red-first record: SPEC-122

The SPEC, in `docs/specs/planned/`, and ADR-122 were committed alone (97bb5df). Then came the
tests of A1 to A4 (b802609), with a stub: `parse_document` and `REPEATED_KEY` declared in
`scripts/mutation_rows.py` but reading as `json.loads` reads, so every test ran and each red
failed by assertion. The parser's refusal and the readers' use of it (73b3502) turned them green.
The replay ran the whole test file on b802609's tree in a detached worktree: seven tests, six
red by assertion (seven failures counting the two subtests), the control green.

```red-first
A1: red at b802609: the merge of the two branches completed with no conflict (exit 0) and the reader did not refuse the merged file: 0 != 2
A1: green at 73b3502
A2: red at b802609: a key repeated inside tables, and one repeated at the top, were read as the last value: 0 != 2; at any depth: PopulationRefused not raised
A2: green at 73b3502
A3: red at b802609: retired over a revision whose band file repeats a key exited 0: 0 != 2 : examined 0; a revision header that repeats a key: PopulationRefused not raised
A3: green at 73b3502
A4: red at b802609: a header that repeats a key in the tree was read as the last value: 0 != 2
A4: green at 73b3502
```

The control, `every_committed_band_file_reads_as_plain_json_reads_it`, is not red: it pins that
every committed band file reads as plain `json.load` reads it, which the base already did.
Rows S12201 to S12206 are companions of the killing tests of A1 to A4, each proved KILLED by its
full id.

Fix round 1. The acceptance lines of A2, A3 and A4 now select both tests their rows name, with two
`-k` patterns each (8239dab). The refusal assertions now pin the whole sentence the code forms,
`<file> repeats the key '<key>' in one object`, instead of three fragments of it (f4cd866). That is
a test-only tightening after green, not a new red and green pair, so the pairs above stand.

Addendum, 2026-09-29 (issue #345, the amendment's A5 and A6). The tests of A5 and A6 were
committed alone (3c9dfb6) against the unchanged verdict script; the whole file ran five tests,
three red by assertion, two green (the well-formed controls, which are not red). The refusal in
`plan` and in `load_records` (3af853b) turned them green.

```red-first
A5: red at 3c9dfb6: plan over a band file that repeats a key ended in an uncaught PopulationRefused traceback on stderr: 'Traceback' unexpectedly found
A5: green at 3af853b
A6: red at 3c9dfb6: a record fragment that repeats a key was read as its last value and the census exited 0: 0 != 1 : examined 0 record(s)
A6: green at 3af853b
```
