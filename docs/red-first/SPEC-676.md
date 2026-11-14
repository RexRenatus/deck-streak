# SPEC-676 red-first record

SPEC-676 (R1 to R11). The red commit e36fad6 adds `scripts/tests/test_doc_counts.py`
with its eight tests and the six planted files under `scripts/tests/fixtures/doc-counts/`, over
stub bodies that keep every input but the missing behaviour: every line reads as prose, every
document stays living, no claim is found, the census finds no site, no stale entry and no bound
claim, the verdict prints and fails nothing, and each derivation counts 0. `BOUND` already holds
its two entries, `LISTED` is empty and `LEVEL` is `warn`, so each red below is its criterion's
assertion and never a missing name. The green commit 32ad016 replaces the stub bodies only.
Every line below was measured by running the module by its acceptance command, on the box over
planted text, on a tree byte-identical to the commit it names.

```text
A2: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_bound_claim_whose_count_matches_is_clean
A3: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_bound_claim_whose_count_differs_is_a_site
A4: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_planted_unbound_claim_is_a_site_at_warn_and_fails_at_fail
A5: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_count_in_code_a_quote_front_matter_a_table_or_a_measured_run_is_not_read
A6: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_dated_record_leaves_the_population_by_its_rule
A8: python3 -m unittest discover -s scripts/tests -p test_doc_counts.py -k test_a_listing_entry_that_names_no_site_fails_at_both_levels
```

`test_the_repository_holds_todays_listing` reads the whole tree, which only CI reads: its green
is read in the first push's `hygiene` job. `test_an_empty_population_is_void` guards the refusal that
`scripts/tests/_support.py` already holds.

Test edits after the red commit: none in the green commit, which changes no test function,
planted text or expected value. The second push fills `LISTED` from the first push's `hygiene`
job and edits no test function.

```red-first
A1: not red: reads the whole tree, which only CI reads; its green is the first push's hygiene job
A2: red at e36fad6: AssertionError: Tuples differ: ([], [], 0) != ([], [], 1)
A2: green at 32ad016
A3: red at e36fad6: AssertionError: Lists differ: [] != [("bound-differs.md 4 jobs 'four jobs' #1", 'mismatch')]
A3: green at 32ad016
A4: red at e36fad6: AssertionError: Lists differ: [] != [("unbound.md 5 crates 'five crates' #1", 'unbound')]
A4: green at 32ad016
A5: red at e36fad6: AssertionError: Tuples differ: (0, 0, 0, 0, 0) != (7, 1, 3, 3, 1)
A5: green at 32ad016
A6: red at e36fad6: AssertionError: '# A planted record\n\nThe planted record names three rows.\n' is not None : docs/red-first/SPEC-001.md
A6: green at 32ad016
A7: not red: the shared examined helper already refuses an empty population; the test guards that the census keeps it
A8: red at e36fad6: AssertionError: Tuples differ: ([], []) != (["STALE doc-count listed bound-match.md 9 [82 chars]#1"])
A8: green at 32ad016
```

Rows `S67600` to `S67624` sit on the census's arms, each killed by its planted test by full id.

The first push's `hygiene` job `130649809572` ran the module, its repository test included, and every test passed.

The green commit `32ad016` carries the census; the listing commit `814fd6d` changes only `LISTED` in the module, and `git diff --quiet 814fd6d HEAD` over the module and its planted files reads rc 0.
