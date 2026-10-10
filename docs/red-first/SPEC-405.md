# Red-first record: SPEC-405

The SPEC, ADR-419, the schematic amendment and the pointers are committed with the red tests. Each
criterion's test is red at the first commit for its own reason, because the release lane still
runs on a tag push alone and its plan holds no actor arm.

A1
(`test_testflight_workflows.TheReleaseLaneHasASecondPath.test_the_release_lane_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either`)
is red because the lane declares the push alone: its events are `["push"]`, not the push and the
dispatch.

A2
(`test_ios_lane.LanePlan.test_the_release_plan_admits_a_dispatch_at_the_tags_ref_as_its_push_and_refuses_a_branch`)
is red because the plan refuses a dispatch even at the tag's own ref, so it writes no output where
the tag's push gets the lane, the build number and the version.

A3
(`test_testflight_workflows.TheReleaseLaneHasASecondPath.test_a_push_and_a_dispatch_of_one_tag_render_one_lane_group`)
is red because the lane declares no dispatch: the group already renders one name for a pushed and a
dispatched run, so the test fails at its last assertion, that the lane declares the
`workflow_dispatch` trigger.

A4
(`test_rulesets.TheLaneRunbookRecoversATagWithNoLaneRun.test_the_runbook_builds_a_tag_with_no_lane_run_by_a_dispatch_at_its_own_ref`)
is red because section 8 of `RELEASING.md` holds neither the detection nor the recovery command.

A5
(`test_ios_lane.LanePlan.test_the_release_plan_refuses_a_run_the_workflow_token_started`) is red
because the plan has no actor arm: it admits a push that the workflow token's actor started.

SPEC-352 A14
(`test_testflight_workflows.TheTestflightLanes.test_each_lane_runs_on_its_one_trigger_and_nothing_a_pull_request_starts`)
and SPEC-352 A5
(`test_ios_lane.LanePlan.test_the_release_plan_admits_only_an_annotated_semver_tag_on_mains_first_parent_chain`)
move by ADR-419 D6, and each is red at the first commit: A14's release pin now expects the dispatch
the lane does not declare yet, and A5's refused dispatch at a branch named like the tag now expects
the new message. Each one's failing line, as CI reads it at the first commit `390d129f`, follows,
and each is green at the fix commit `3652fe61`:

- SPEC-352 A14: `AssertionError: {'push': {'tags': ['v[0-9]+.[0-9]+.[0-9]+']}} != {'push': {'tags': ['v[0-9]+.[0-9]+.[0-9]+']}, 'workflow_dispatch': None}`
- SPEC-352 A5: `AssertionError: "the release lane runs on a tag's push or dispatch only, not on push of refs/heads/main" not found in ['the release lane runs on a tag push only, not on push of refs/heads/main']`

The first commit also reddened one test that is no criterion of this SPEC,
`test_threat_model.TheModelHolds.test_every_control_cites_a_line_that_holds`, which CI reads at
`390d129f` as `AssertionError: Lists differ: ['docs/schematics/the-app-campaigns-surfac[1008 chars]ial'] != []`.
Its red is a moved citation, not a criterion red: the first commit's two added imports and the
longer pin comment moved two test definitions that the threat model cites by line, and the fix's
header moves the lane's `tags:` line, which the model also cites. Commit `364b2e26` is its cure: it
re-derives those five citations to the lines their quotes now sit on and changes only the numbers.
The fence records no line for it.

A6 and A7 hold properties the change must keep, so they are not red: green at the base and after.

```red-first
A6: not red: holds that only the app job reads the credential, a property the change must keep; green at the base and after
A7: not red: holds that apple-on-tag.yml calls the job body on every release tag, unchanged by ADR-419 D2b; green at the base and after
A1: red at 390d129f: AssertionError: Lists differ: ['push'] != ['push', 'workflow_dispatch']
A1: green at 3652fe61
A2: red at 390d129f: AssertionError: {} != {'lane': 'release', 'number': '3', 'version': '0.2.0'}
A2: green at 3652fe61
A3: red at 390d129f: AssertionError: 'workflow_dispatch' not found in ['push'] : the lane declares no workflow_dispatch trigger
A3: green at 3652fe61
A4: red at 390d129f: AssertionError: 'gh run list --workflow testflight-release.yml --commit "$(git rev-parse \'vX.Y.Z^{commit}\')"' not found in ['## 8. TestFlight builds of the iPhone and iPad app'
A4: green at 3652fe61
A5: red at 390d129f: AssertionError: 'the release lane takes no run that github-actions[bot] started' not found in []
A5: green at 3652fe61
```
