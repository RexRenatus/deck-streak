# SPEC-405: The release lane `testflight-release.yml` takes a dispatch at the tag's own ref, and `apple-on-tag.yml` needs no second path

- **Issue:** #733. **Context:** the app's release lanes (`.github/workflows/testflight-release.yml`,
  `scripts/ios_lane.py`, `RELEASING.md` section 8), beside the release workflow's second path of
  #475 (SPEC-373).
- **Decided by:** ADR-419 (the release lane `testflight-release.yml` gets a second path at the
  tag's ref, and `apple-on-tag.yml` needs none).
- **Status:** accepted. Mutation band S40500-S40599.

## 1. The problem, measured

Read at dev `32f62172` with `git show 32f62172:<path>` and `gh issue view 733`:

- **Three workflows start on a release tag's push.** `RELEASING.md:159` names them: `release.yml`,
  `testflight-release.yml` and `apple-on-tag.yml`. `release.yml` already takes an input-free
  dispatch at the tag's ref (`release.yml:13-17`, SPEC-373). The other two run on the tag's push and
  on nothing else (`testflight-release.yml:7-10`, `apple-on-tag.yml:7-10`).
- **A tag whose push started no lane run never gets its build.** `.github/rulesets/release-tags.json`
  covers `refs/tags/v*` with `deletion`, `non_fast_forward` and `update` and no bypass actor, so the
  tag can be neither moved nor deleted and pushed again.
- **The lane already holds the release workflow's rules for any event.** Its `plan` job's guard step
  (`testflight-release.yml:36-56`) is `release.yml`'s, byte for byte (SPEC-352 A18), and checks
  that `GITHUB_REF` is `refs/tags/` and the tag (44-47). No job and no step carries an `if:`. Its
  group, `testflight-release-${{ github.ref }}` (18-21), never cancels and queues.
- **The plan refuses the dispatch.** `scripts/ios_lane.py:112-113` refuses every release-lane run
  whose event is not `push` or whose ref type is not `tag`; `scripts/tests/test_ios_lane.py:350-356`
  pins that refusal for a dispatch at `refs/tags/v0.2.0` (SPEC-352 A5), and
  `scripts/tests/test_testflight_workflows.py:189` pins the lane's `on` to the tag push alone
  (SPEC-352 A14).
- **Only the lane's `app` job reads the upload credential** (`testflight-release.yml:65-144`: the
  release environment at 69, three credential steps at 107-135).
- **A workflow's own token can start a dispatch.** `testflight-rerelease-check.yml:19-21` holds
  `actions: write`, the only such grant under `.github/workflows`, and
  `scripts/testflight_age.py:209-215` dispatches a lane with it. A dispatch made with a workflow's
  own token creates a run, where a push made with it creates none.
- **`apple-on-tag.yml` builds nothing anyone waits on.** Its one job, `apple`, calls
  `xcframework.yml` (22-24), with read-only permissions (12-13); it publishes nothing and passes no
  secret (3-6), and SPEC-344 pins that shape (`scripts/tests/test_release_workflow.py:155-171`). The
  lane's `framework` job calls the same body at the tag's ref (`testflight-release.yml:61-63`), and
  `xcframework.yml`'s own dispatch runs it at any ref (22-24).

## 2. Requirements

R1. `testflight-release.yml` runs on exactly two events: the push of a tag its filter admits, the
filter unchanged, and a manual dispatch (`workflow_dispatch`) that declares no input.

R2. Both events run the same three jobs, `plan`, `framework` and `app`, and the guard step once, in
the `plan` job. No job and not the guard step carries an `if:`, so no path skips a check and no
refused run reads as a success.

R3. The release plan admits a dispatch at a tag's own ref exactly as it admits that tag's push: the
same outputs for a good tag, and the same refusal, by the same message, of a lightweight tag, a
commit off `main`, a commit off `main`'s first-parent chain and a version that is not the
workspace's. It refuses a dispatch at a branch, and any event but a push or a dispatch, with
`the release lane runs on a tag's push or dispatch only, not on <event> of <ref>`.

R4. The release plan refuses a run whose `GITHUB_ACTOR` is the workflow token's actor,
`github-actions[bot]`, on either event, with `the release lane takes no run that
github-actions[bot] started`, and writes no output.

R5. A pushed run and a dispatched run of one tag share the group
`testflight-release-${{ github.ref }}`, unchanged, so they take turns and none is cancelled.

R6. The `app` job alone names an environment and reads the upload credential; the change adds no
job, step, permission or environment.

R7. `RELEASING.md` section 8 says how to see that a release tag's push started no lane run, gives
the recovery, a manual dispatch of the lane at the tag's own ref, says that `apple-on-tag.yml`
takes no dispatch and why, and the runbook holds no command that moves a release tag.

R8. `apple-on-tag.yml` is unchanged: its one `apple` job calls `xcframework.yml` on every release
tag's push (SPEC-344).

R9. SPEC-352 A14's release entry becomes the new exact pin, and A5's refused dispatch moves to a
branch named like the tag, `refs/heads/v0.2.0`, with the new message (ADR-419 D6). No assertion is
removed: every refusal is still asserted, and the newly admitted path is A2's.

## 3. Acceptance criteria of SPEC-405

| | criterion | decided by |
|---|---|---|
| A1 | the release lane's events are exactly `push` then `workflow_dispatch`, `push` is the tag filter unchanged, the dispatch's value is null (no input), the jobs are exactly `app`, `framework` and `plan`, none carries `if:`, and the `plan` job holds the guard step exactly once, with no `if:` (R1, R2) | `test_testflight_workflows.TheReleaseLaneHasASecondPath.test_the_release_lane_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either` |
| A2 | run on the fixture, the release plan gives a dispatch at `refs/tags/v0.2.0` the outputs its push gets, refuses a dispatch at each of the fixture's lightweight, off-`main`, off-first-parent and mismatched tags by the message its push gets, and refuses a dispatch at `refs/heads/main`, `refs/heads/dev` and `refs/heads/v0.2.0` and a scheduled run at `refs/tags/v0.2.0` by the event-and-ref message (R3) | `test_ios_lane.LanePlan.test_the_release_plan_admits_a_dispatch_at_the_tags_ref_as_its_push_and_refuses_a_branch` |
| A3 | for each of `push` and `workflow_dispatch`, each declared by the lane, a run of the tag `v1.0.0` renders the group `testflight-release-refs/tags/v1.0.0`, one name for both (R5) | `test_testflight_workflows.TheReleaseLaneHasASecondPath.test_a_push_and_a_dispatch_of_one_tag_render_one_lane_group` |
| A4 | `RELEASING.md` section 8 holds the detection (`gh run list --workflow testflight-release.yml --commit`), the recovery (`gh workflow run testflight-release.yml --ref vX.Y.Z`) and the sentence that `apple-on-tag.yml` takes no dispatch because the lane's `framework` job runs the same build at the tag's ref (R7) | `test_rulesets.TheLaneRunbookRecoversATagWithNoLaneRun.test_the_runbook_builds_a_tag_with_no_lane_run_by_a_dispatch_at_its_own_ref` |
| A5 | run on the fixture, the release plan refuses a push and a dispatch at `refs/tags/v0.2.0` whose actor is `github-actions[bot]`, by its message, with no output (R4) | `test_ios_lane.LanePlan.test_the_release_plan_refuses_a_run_the_workflow_token_started` |
| A6 | the `app` job alone names an environment and reads a credential, in the lane's three credential steps (R6) | `test_testflight_workflows.TheTestflightLanes.test_only_the_app_job_names_an_environment_and_reads_a_credential` |
| A7 | `apple-on-tag.yml` calls `xcframework.yml` on every release tag's push, its one job unchanged (R8) | `test_release_workflow.TheAppleBuildRunsOnEveryReleaseTag.test_the_tag_caller_runs_the_build_on_every_release_tag` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_release_lane_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either
A2: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_release_plan_admits_a_dispatch_at_the_tags_ref_as_its_push_and_refuses_a_branch
A3: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_a_push_and_a_dispatch_of_one_tag_render_one_lane_group
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_builds_a_tag_with_no_lane_run_by_a_dispatch_at_its_own_ref
A5: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_release_plan_refuses_a_run_the_workflow_token_started
A6: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_only_the_app_job_names_an_environment_and_reads_a_credential
A7: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_tag_caller_runs_the_build_on_every_release_tag
```

Red first (`docs/red-first/SPEC-405.md`): A1 to A5 are red at the base for their behaviour. A6
and A7 hold properties the change must keep, so they are green at the base and stay green. R9 is
decided by SPEC-352 A14 and A5 at their moved pins; each is red at the red commit and green at the
fix, and the record says so.

## 4. What only a live run proves

A dispatch of the real lane is the maintainer's act, never a test's. A workflow is dispatched only
when its file is on the default branch, and the run reads the file at the ref it was dispatched at,
so only a tag whose commit carries this change can take the second path. Who: the maintainer.
When: the first release tag, cut after the release pull request carries this change to `main`,
whose push starts no lane run. The record: that run's event, `workflow_dispatch`, and its branch,
the tag, in `gh run list --workflow testflight-release.yml --json event,headBranch,conclusion`.
No dispatch run carries the workflow token's actor name yet: the scheduled re-release check
dispatches the internal lane with that token, and a scheduled workflow runs only from the default
branch, which carries the check only after a release does. A5's literal, `github-actions[bot]`, is
the hosting service's documented actor for a run the workflow token starts, and the check passes
that token. Who: the maintainer. When: the first run the check dispatches after a release carries
it to `main`. The record: that run's actor login, read from the internal lane's `workflow_dispatch`
runs; a name other than `github-actions[bot]` is a defect against R4.

## 5. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/testflight-release.yml` | app lanes | the input-free dispatch event; the header and group comments |
| `scripts/ios_lane.py` | app lanes | the release plan's event arm (R3), the workflow token's arm and its constant (R4) |
| `scripts/tests/test_testflight_workflows.py` | lane tests | A1 and A3; SPEC-352 A14's release pin and its comment moved; two imports added |
| `scripts/tests/test_ios_lane.py` | lane tests | A2 and A5; the plan helper takes extra variables; SPEC-352 A5's dispatch case moved |
| `scripts/tests/test_rulesets.py` | runbook tests | A4 |
| `RELEASING.md` | runbook | section 8: the detection and the recovery, inserted |
| `docs/specs/SPEC-405-the-testflight-release-lane-takes-a-dispatch-at-the-tags-own-ref-and-the-apple-build-needs-no-second-path.md` | spec | new |
| `docs/decisions/ADR-419-the-testflight-release-lane-gets-a-second-path-at-the-tags-ref-and-the-apple-build-needs-none.md` | decision | new |
| `docs/schematics/release-from-a-tag-push-or-a-dispatch-at-the-tags-ref.md` | schematic | a new section, appended |
| `docs/red-first/SPEC-405.md` | red-first record | new |
| `docs/specs/SPEC-352-testflight-lane.md` | spec | an insert-only subsection appended under its amendments |
| `docs/decisions/ADR-363-testflight-lane.md` | decision | an insert-only pointer appended |
| `docs/specs/SPEC-373-a-release-tag-whose-push-started-no-run-is-released-by-a-dispatch-at-its-own-ref-through-the-same-guard.md` | spec | an insert-only amendments section appended |
| `docs/decisions/ADR-384-a-tag-with-no-release-run-is-released-by-a-dispatch-at-the-tags-ref-that-runs-the-tag-paths-own-guard-and-group.md` | decision | an insert-only pointer appended |
| `docs/schematics/testflight-lane.md` | schematic | an insert-only pointer row after line 12: release trigger, amended (SPEC-405) |
| `scripts/mutation-rows.d/S40500-S40599.json` | rows | new band, S40500 to S40515 |
| `changelog.d/release-lanes-second-path-405.md` | changelog | fragment |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | threat model | the five citations of lines this delivery moves, re-derived at the tree (T5's three of `test_testflight_workflows.py` and one more of it; the lane's `tags:` line); every other line kept |

## 6. What this does NOT cover

- `release.yml`'s own dispatch admits a run that a workflow's own token starts. This change does
  not alter `release.yml`; that path is tracked apart from this issue's lanes (#733).
- The release workflow itself, its guard, its group and its second path stay as they are (#475).
- `apple-on-tag.yml` and `xcframework.yml` are unchanged: ADR-419 D2b records that the tag caller
  needs no second path (#733).
- A model of the release queues, two runs of one tag in one group: owed by #467, which must carry
  the dispatched lane run.
- The internal lane, its triggers and its plan arm are unchanged; a separate change owns the
  internal lane's trigger (#772).
- The framework workflow's timing record and the Swift mutation rows change `xcframework.yml` and the
  workflow reader's module, which this change leaves as they are (#752, #770).
- The workflow reader's module, `scripts/tests/test_ci_workflows.py`, is unchanged: no pin of the
  lane's `on` lives there, and the lane's test module imports two names it already exports (#733).
- No repository, ruleset or environment setting changes; the release environment's review is the
  maintainer's to configure (#634).
- This delivery makes no release, tag or dispatch; the first dispatched lane run is the
  maintainer's (#733).

## 7. Risks

- **A dispatch at a ref that is not a release tag.** Anyone who can push a tag can also dispatch,
  at any branch or tag. The guard step refuses every ref that is not `refs/tags/<the tag>`, and the
  plan refuses a branch by event and ref (A2). Detection: the run fails in the `plan` job, its
  message naming the ref.
- **A dispatch for a tag that already has a build.** The guard and the plan admit it, since the
  tag is valid; the `app` job waits for the release review, and the upload refuses a build number
  it already holds. The runbook limits the dispatch to a tag whose push started no lane run.
  Detection: a review request for a tag that already has a build, or a failed upload step.
- **The token's actor name.** A5 holds the plan to the literal `github-actions[bot]`; if a workflow
  token ran under another name, a run it started would pass the plan and wait at the review. The
  name is the hosting service's documented one, and section 4 names the first run that records it.
  Detection: a lane run whose actor is not an account.
- **Two pins move.** SPEC-352 A14's exact pin moves to the new exact set, and A5's dispatch case
  moves to a branch with the new message; both stay as strong as before (ADR-419 D6).
- **A test that passes because both paths share a step.** A3 would pass a group that splits by
  event only if no row planted one; row S40506 plants it and must be killed. Rows S40502 to S40505
  plant an event-gated `if:` on the guard step and on each job, each killed by A1.
- **Landing the plan's change starts an internal run.** `scripts/ios_lane.py` is an input the
  internal lane's push filter watches, so its merge to `dev` starts one internal lane run; the
  internal arm is unchanged. Detection: that run's plan reads as before.
