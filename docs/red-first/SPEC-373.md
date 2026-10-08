# Red-first record: SPEC-373

The SPEC, ADR-384 and the schematic are committed with the red tests. Each criterion's test is red
at the first commit for its own reason, because `release.yml` still runs on a tag push alone and its
guard checks no ref.

A1 (`test_release_workflow.TheReleaseHasASecondPath.test_the_release_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either`)
is red because the workflow's events are `["push"]`, not the push and the dispatch. A2
(`test_release_workflow.TheSecondPathRunsTheTagGuard.test_the_guard_refuses_a_ref_that_is_not_the_tag_it_names`)
is red because the guard admits a run whose ref is the branch `refs/heads/v1.0.0`, its commit being
on `main`, where it must refuse it and name the ref. A3
(`test_release_workflow.TheSecondPathRunsTheTagGuard.test_both_paths_refuse_a_lightweight_tag_and_a_tag_off_main`)
and A4
(`test_release_workflow.TheReleaseHasASecondPath.test_a_push_and_a_dispatch_of_one_tag_render_one_group`)
are red because the second path is not declared: the guard already refuses a lightweight tag and a
tag off `main` and the group already renders one name for a pushed and a dispatched run, so each
fails at its last assertion, that `release.yml` declares the `workflow_dispatch` trigger. Rows S37307
and S37308 prove they observe a one-path break: the first splits the group by event, the second lets
the dispatch path skip the guard. A5
(`test_rulesets.TheReleaseRunbookRecoversATagWithNoRun.test_the_runbook_releases_a_tag_with_no_run_by_a_dispatch_at_its_own_ref`)
is red because section 3 of `RELEASING.md` holds neither the detection nor the recovery command.

The second commit greens A1 to A5.
