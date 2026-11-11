# Red-first record: SPEC-672

SPEC-672 (R1 to R12). The SPEC, ADR-695, the module `scripts/tests/test_build_output_census.py`
with its four planted tests and its repository test, its three fixtures under
`scripts/tests/fixtures/build-output/` and the listing of today's four sites are committed first
and alone, over stubs that keep every input but the missing behaviour: `site` judges no path a
site, `sites` returns no site and refuses nothing, and `verdict` prints and fails nothing. The
forms, the listing and `LEVEL` are already there, so each red below is its criterion's assertion,
never a missing name.

The red-first criterion is the planted path `web/app/node_modules/vite/package.json` in
`sites.fixture`: the tool-directory arm of R2 reports it in the green commit, and the stub reports
no site at the red commit, where the planted-sites test fails on its first assertion. The planted
tests read red and green by name over planted path lists alone; the repository test reads the
tracked path list, so it runs only in CI.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k a_path_under_a_directory_a_tool_writes_is_a_site
A2: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k a_lookalike_or_a_source_tree_is_no_site
A3: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails
A4: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k an_empty_population_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k every_listed_path_is_a_site_of_its_form_and_the_rest_print_at_warn
```

The repository test is not red first: it pins today's listing, which the first push's own CI job
measures over the whole tracked path list, and the behaviour it reads is red first through the
four planted tests. Each planted test's red and green lines are added below once read, each with
the commit it was read at.

```red-first
A1: red at 28de9b9: AssertionError: {} != {'web/app/node_modules/vite/package.json':[2384 chars]I/')}
A1: green at a159f5d
A2: red at 28de9b9: AssertionError: {} != {'web/app/node_modules/vite/package.json': ('npm or pnpm', 'node_modules/')}
A2: green at a159f5d
A3: red at 28de9b9: AssertionError: Tuples differ: ([], []) != (['WARN build-output web/app/node_modules/v[107 chars]te'])
A3: green at a159f5d
A4: red at 28de9b9: AssertionError: AssertionError not raised
A4: green at a159f5d
A5: not red: it pins today's listing, measured by the first push's own CI job; the behaviour it reads is red first through the four planted tests
```

The listing delta read from the first push's hygiene job 130638505747 is empty, so no listing commit follows; a site in the part of the tracked path list the design did not read stays unlisted and prints at warn (SPEC-672 section 5). The green commit `a159f5d` carries the census, and `git diff --quiet a159f5d HEAD` over the module and its planted files reads rc 0.
