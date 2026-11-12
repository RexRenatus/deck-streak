# SPEC-604 red-first record

The criteria were written as one test module before the queue file and the batch arm of the `fragment` check.
Seven of them read red at the commit that adds the tests alone, each an assertion failure on the missing behaviour:
the queue file's values and counts, the queue conditions over planted pull request states, the gaps against the dev
ruleset file, the schema's keys, and the dependency-only batch that the `fragment` step still refuses. Four are
`not red`, each with its planted refusal inside the test: two guard what dev already holds (the three workflows run
on a batch, and a batch whose members add fragments passes), and two guard the edges of the batch arm (a batch that
changes another path, and every login or branch outside the arm, are refused before and after it). Each line below
was measured by running the module by its acceptance command on a tree byte-identical to the commit it names.
The queue file was also validated against Mergify's published configuration schema, fetched once and never vendored
(its sha256 is 7a41ae7c44c579dc69a807d704db8ff1a30808b65d0c7d405ec59cb6136aa148): a planted copy whose batch size is a
string read 1 error, and the queue file read 0 errors.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_queue_tests_six_together_and_merges_each_by_a_merge_commit
A2: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_pull_request_enters_the_queue_only_into_dev_labelled_ready_resolved_and_green
A3: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_batch_merges_only_after_ci_fragment_and_dependency_review_pass_on_it
A4: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_queue_requires_every_check_and_rule_the_dev_ruleset_file_requires
A5: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_three_required_workflows_run_on_a_queue_batch_pull_request_into_dev
A6: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_queue_batch_whose_members_add_fragments_passes_the_fragment_check
A7: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_queue_batch_of_dependency_updates_alone_needs_no_fragment
A8: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_queue_batch_that_changes_another_path_needs_a_fragment
A9: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_only_the_queue_login_on_a_queue_branch_takes_the_batch_arm
A10: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_queue_file_is_the_only_mergify_configuration_and_names_one_queue
A11: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_every_key_the_queue_file_uses_is_one_the_published_schema_lists
```

```red-first
A5: not red: guards what dev already holds; its planted workflow copies, one with a paths filter and one with a job-level draft condition, are refused on every run
A6: not red: guards what dev already holds; the fragment rule passes a batch whose members each add a fragment, before and after the batch arm
A8: not red: guards an edge of the batch arm; a batch that changes a path outside the dependency files is refused before and after the arm
A9: not red: guards an edge of the batch arm; every login and branch outside the arm, ones that only resemble them included, is refused before and after the arm
A1: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: 0 != 1 : .mergify.yml holds exactly one queue rule
A1: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
A2: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: False is not true : a ready, labelled, green pull request enters
A2: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
A3: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: False is not true : a draft batch with the three checks green merges
A3: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
A4: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: Lists differ: ['queue: #review-threads-unresolved = 0', [243 chars]iew'] != []
A4: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
A7: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: 1 != 0 : Add a fragment under changelog.d/ (see changelog.d/README.md).
A7: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
A10: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: Lists differ: [] != ['.mergify.yml']
A10: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
A11: red at 4a4acbf66d4aa22d3121d9ac0bc56e0d799f331e: AssertionError: examined 0 keys of .mergify.yml: the population is empty, so nothing was judged
A11: green at 1ecc86c1d4a0d808dbf5ab94c28a8a5aa55a1cc7
```

The criteria were committed with the tests alone (4a4acbf6) and read red by name in the pull
request's own run. The queue file and the batch arm of the fragment check landed in 1ecc86c1 with
the tests unchanged, and the run at this delivery's head reads them green. The commit after
1ecc86c1 changes only this record.

After dev was merged into this branch, one commit tuned the queue file and changed A1, A3 and A11
with it, together with this paragraph: the queue checks one batch at a time, gives the checks of a
batch a fixed timeout, and repeats its queue conditions as its merge conditions, so a lone pull
request that holds the tip of dev merges directly. Run by their acceptance commands against the
previous queue file, the three changed tests each read red by assertion (A1: `None != 1 : the queue
checks one batch at a time`; A3: the merge conditions differ from the queue conditions; A11:
`examined 0 merge queue settings`), and against the tuned file all eleven read green. The tuned file
read 0 errors against the same schema, and a planted copy whose parallel check count is a string
read 1 error.

The revision named in the specification's header changed two tests together with the queue file:
the entry test, `test_a_pull_request_enters_the_queue_only_into_dev_labelled_ready_resolved_and_green`,
and the batch test, `test_a_batch_merges_only_after_ci_fragment_and_dependency_review_pass_on_it`.
The revised tests were committed alone first and run by their acceptance commands against the queue
file as it stood before the revision, and each read red by assertion. The entry test read
`AssertionError: False is not true : a ready, green pull request with the READY status enters`
and the batch test read
`AssertionError: False is not true : a batch whose members entered by the READY status merges on the batch's checks alone`.
Against the revised queue file all eleven read green.
The revised queue file read 0 errors against Mergify's published configuration schema, fetched
once more and never vendored (its sha256 is now
f6f924c812112063e81b42f087200935e317e66f64942e001545fea2effba1cb), and a planted copy whose
batch size is a string read 1 error.
