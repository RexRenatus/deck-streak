# SPEC-604 red-first record

The criteria were written as one test module before the queue file and the batch arm of the `fragment` check.
Seven of them read red at the commit that adds the tests alone, each an assertion failure on the missing behaviour:
the queue file's values and counts, the queue conditions over planted pull request states, the gaps against the dev
ruleset file, the schema's keys, and the dependency-only batch that the `fragment` step still refuses. Four are
`not red`, each with its planted refusal inside the test: two guard what dev already holds (the three workflows run
on a batch, and a batch whose members add fragments passes), and two guard the edges of the batch arm (a batch that
changes another path, and every login or branch outside the arm, are refused before and after it). Each line below
was measured by running the module by its acceptance command on a tree byte-identical to the commit it names.

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
```
