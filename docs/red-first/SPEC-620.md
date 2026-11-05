# SPEC-620 red-first record

The criteria were written as three tests of one module before the dev ruleset file changed. Two of them
read red at the commit that adds the tests alone, each an assertion failure on the old value in the file:
the empty list of bypass actors, and the strict policy still `true`. One is `not red`: it guards what dev
already holds, since main and the release tags keep no bypass actor and every rule type stays enforceable.
Each line below was measured by running the module by its acceptance command on a tree byte-identical to
the commit it names. The whole module at that commit ran 16 tests and read 2 failures.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_dev_ruleset_exempts_only_the_merge_queue_app
A2: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_no_other_ruleset_has_a_bypass_actor_and_none_has_a_refused_rule
A3: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_dev_does_not_require_an_up_to_date_head
```

```red-first
A2: not red: guards what dev already holds; main and the release tags keep no bypass, and every rule type stays enforceable
A1: red at fe7cee3: AssertionError: Lists differ: [] != [{'actor_id': 10562, 'actor_type': 'Integration', 'bypass_mode': 'exempt'}]
A3: red at fe7cee3: AssertionError: True is not False
```
