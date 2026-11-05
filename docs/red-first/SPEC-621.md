# SPEC-621 red-first record

The criteria were written as one test class before the workflow change. A1 and A3 are expected red
at the commit that adds the tests alone, each an assertion failure on the missing behaviour: the
three short jobs still resolve to `xcode-27` in the scenarios that route them, and A3's control,
the live jobs routed as SPEC-621 wants, fails before any plant is read. Two are `not red`, each
with its refusals inside the test: A2 guards what dev already holds, and A4 guards the expression
reader on planted text. Each red and green line below is read by the test's name in this pull
request's own `ci` run, never on a local tree.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_short_apple_jobs_reach_the_mini_only_on_this_repositorys_own_events_with_the_variable_on
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_long_apple_jobs_and_the_verdict_stay_hosted_on_every_event
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_planted_route_is_refused_by_its_scenario
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_reader_reads_fromjson_of_a_string_literal_only
```

```red-first
A2: not red: guards what dev already holds; harness, release-and-app and verdict resolve to xcode-27 in every scenario, before and after the change
A4: not red: the reader reads planted text only; fromJSON of a string literal is read and every other fromJSON form is refused by name, before and after the change
A1: red at f88a6c7: AssertionError: 'xcode-27' != ['self-hosted', 'macOS', 'ARM64', 'ds-mini']
A3: red at f88a6c7: AssertionError: Lists differ: ["xcframework: a push to dev, DS_MINI on, [2005 chars]els"] != []
A1: green at 01f7eee
A3: green at 01f7eee
```

The runner admission changes in the same commit as the tests: `admitted_runner` admits the routing
expression alone to the three short jobs, and each job of the file is held to its one runner, the
card view's host job included. Those pins read red at the commit that adds the tests, beside A1 and
A3, and green once the workflow holds the expression.

The reds were read in CI run 41949535005, the `hygiene` job 126989913004, at f88a6c7 (the commit
that adds the tests alone), by test name from the job's `python.log`: five tests failed, each by
an assertion and no error, and no other test of the stage failed. A1 failed in 18 subtests, the
three short jobs in each of the six scenarios that route them, each `'xcode-27' != ['self-hosted',
'macOS', 'ARM64', 'ds-mini']`. A3 failed on its control, the live jobs' routing problems, before
any plant was read. The three runner pins failed beside them:
`test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger` (the admission refuses
`xcode-27` on the three short jobs), `test_one_job_body_serves_the_change_and_the_tag`
(`'xcode-27' != ` the expression) and `test_the_card_probe_suite_runs_on_both_simulators`
(`card-isolation: it runs on 'xcode-27', not the admitted runner`). A2 and A4 passed in the same
run. The workflow change follows in the next commit, with the tests unchanged.

The workflow change landed in 01f7eee with the tests unchanged: it gives the three short jobs the
routing expression and adds the changelog fragment and the two red lines above. The commit after
01f7eee changes only this record, so `git diff --quiet 01f7eee HEAD --
scripts/tests/test_ci_workflows.py .github/workflows/xcframework.yml` exits 0 at the head, and A1,
A3 and the three runner pins are read green by name in the `ci` run of the head that carries these
lines, with A2 and A4 still passing.
