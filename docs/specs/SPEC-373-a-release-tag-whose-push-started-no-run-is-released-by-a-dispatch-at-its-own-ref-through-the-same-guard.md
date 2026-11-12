# SPEC-373: A release tag whose push started no run is released by a dispatch at its own ref, through the same guard

- **Issue:** #475. **Context:** `release` (`.github/workflows/release.yml`, `RELEASING.md`); the
  release queue of #456 and #377.
- **Decided by:** ADR-384 (a tag with no release run is released by a dispatch at the tag's ref
  that runs the tag path's own guard and group).
- **Status:** accepted. Mutation band S37300-S37399.

## 1. The problem, measured

Read at dev `6f9ef860` with `git show 6f9ef860:<path>` and `gh issue view 475`:

- **One event starts a release.** `.github/workflows/release.yml:12-15` declares a push of a tag
  matching `v[0-9]+.[0-9]+.[0-9]+` and nothing else.
- **A tag's push can start no run, and the tag cannot be pushed again.** #475 names a push of many
  tags at once and a skip instruction in the tagged commit's message; either leaves the tag with no
  release run. `.github/rulesets/release-tags.json` covers `refs/tags/v*` with the `deletion`,
  `non_fast_forward` and `update` rules and no bypass actor, so the tag can be neither moved nor
  deleted and pushed again. The only recovery the tree allows today is a new patch tag, which
  spends a version and never releases the tag that was cut.
- **The guard reads the tag's name, never the ref's kind.** `release.yml:44-60` is one step: it
  takes the tag from `GITHUB_REF_NAME` (47), refuses a name that is not SemVer (48-51), a
  lightweight tag (52-55) and a commit not on `main` (56-60). A run whose ref is a branch named
  like a release tag, with its commit on `main`, passes all three while that tag exists.
- **The group is the ref.** `release.yml:23-26` names `release-${{ github.ref }}` with
  `cancel-in-progress: false` and `queue: max` (ADR-292), so every run of one ref takes its turn.
- **The tests.** `scripts/tests/test_release_workflow.py:77` pins the events as exactly `["push"]`
  (SPEC-062 A8). `:189-232` runs the guard under bash in a scratch repository (SPEC-062 A18) with
  `GITHUB_REF_NAME` and `GITHUB_SHA` set, and `GITHUB_REF` left to the caller's environment. The
  row S19022 (`scripts/mutation-rows.d/S19000-S19099.json`) plants a dispatch key after the tag
  filter's line.

## 2. Requirements

R1. `release.yml` runs on exactly two events: the push of a tag its filter admits, the filter
unchanged, and a manual dispatch (`workflow_dispatch`).

R2. The dispatch declares no input. The tag a dispatched run releases is the ref the run was
dispatched at, and nothing a dispatcher types reaches a step.

R3. Both events run the same two jobs and the same guard step. Neither job and not the guard step
carries an `if:`, so no path skips a check and no refused run reads as a success.

R4. The guard refuses a run whose `GITHUB_REF` is not `refs/tags/` followed by the tag it read from
`GITHUB_REF_NAME`, with a message that names the ref. A run with no `GITHUB_REF` is refused too.
The check comes after the SemVer check and before any check that reads the tag.

R5. Under either event, the guard still refuses a tag name that is not SemVer, a lightweight tag
and a tag whose commit is not on `main`.

R6. A dispatched run and a pushed run of one tag share the group `release-${{ github.ref }}`,
unchanged, so they take turns with every other run of the tag and none is cancelled (ADR-292).

R7. `RELEASING.md` section 3 says how to see that a tag's push started no run, and gives the
recovery: a manual dispatch at the tag's own ref. The runbook holds no command that forces,
deletes, moves or pushes a release tag again.

R8. The release job's permissions, steps and artifacts are unchanged, so a dispatched run builds,
attests and publishes as a pushed run does.

R9. SPEC-062 A8's event pin becomes the two events, exactly. A18's guard runs with the
`GITHUB_REF` a push of its tag has. Row S19022 keeps its id, its mutant and its killer, re-anchored
on the dispatch line this delivery adds.

## 3. Acceptance criteria

| | criterion | decided by |
|---|---|---|
| A1 | `release.yml`'s events are exactly `push` and `workflow_dispatch`, the dispatch's value is null (no input), and neither job and not the guard step carries `if:` (R1, R2, R3) | `test_release_workflow.TheReleaseHasASecondPath.test_the_release_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either` |
| A2 | run under bash with a dispatch's environment, the guard refuses `refs/heads/v1.0.0` (a branch named as an annotated tag on `main`, its commit on `main`), a missing `GITHUB_REF` and `refs/heads/dev`, its message naming the ref for the first, and admits `refs/tags/v1.0.0` (R4) | `test_release_workflow.TheSecondPathRunsTheTagGuard.test_the_guard_refuses_a_ref_that_is_not_the_tag_it_names` |
| A3 | for each of `push` and `workflow_dispatch`, each declared by `release.yml`, the guard run with that event's environment refuses a lightweight tag and an annotated tag off `main`, and admits an annotated tag on `main` (R5) | `test_release_workflow.TheSecondPathRunsTheTagGuard.test_both_paths_refuse_a_lightweight_tag_and_a_tag_off_main` |
| A4 | for each of `push` and `workflow_dispatch`, each declared by `release.yml`, a run of the tag `v1.0.0` renders the group `release-refs/tags/v1.0.0`, one name for both (R6) | `test_release_workflow.TheReleaseHasASecondPath.test_a_push_and_a_dispatch_of_one_tag_render_one_group` |
| A5 | `RELEASING.md` section 3 holds the detection (`gh run list --workflow release.yml --commit`) and the recovery (`gh workflow run release.yml --ref vX.Y.Z`), and the runbook holds none of `git push --force`, `git push -f`, `git push --delete`, `git push origin :refs/tags/`, `git tag -f`, `git tag --force`, `git tag -d` or `git tag --delete` (R7) | `test_rulesets.TheReleaseRunbookRecoversATagWithNoRun.test_the_runbook_releases_a_tag_with_no_run_by_a_dispatch_at_its_own_ref` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_release_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either
A2: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_guard_refuses_a_ref_that_is_not_the_tag_it_names
A3: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_both_paths_refuse_a_lightweight_tag_and_a_tag_off_main
A4: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_a_push_and_a_dispatch_of_one_tag_render_one_group
A5: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_releases_a_tag_with_no_run_by_a_dispatch_at_its_own_ref
```

Red first (`docs/red-first/SPEC-373.md`): A1, A2 and A5 are red at the base for their behaviour.
A3 and A4 are red at the base because the second path is not declared. Their behaviour halves
already hold there, since one guard and one group serve both paths by construction; rows S37308
and S37307 prove each test observes a break of one path alone.

## 4. What only a live release proves

A dispatch of the real workflow is the maintainer's act, never a test's. GitHub dispatches a
workflow only when its file is on the default branch, and the run reads the file at the ref it was
dispatched at, so only a tag whose commit carries this change can take the second path. Who: the
maintainer. When: the first release tag, cut after the release pull request carries this change to
`main`, whose push starts no run. The record: that run's event, `workflow_dispatch`, and its
branch, the tag, in `gh run list --workflow release.yml --json event,headBranch,conclusion`.

## 5. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/release.yml` | release | the dispatch event, the ref check in the guard, the header and group comments |
| `.github/workflows/testflight-release.yml` | app lanes | the ref check, byte for byte as release.yml's, in the ancestry step SPEC-352 A18 pins to it |
| `scripts/tests/test_workflow_concurrency.py` | class tests | the tag-trigger fixture's replaced block now holds the whole `on:` block, `workflow_dispatch:` included |
| `scripts/tests/test_release_workflow.py` | release tests | A1 to A4; A18's environment gains `GITHUB_REF`; A8's event pin becomes the two events |
| `scripts/tests/test_rulesets.py` | runbook tests | A5 |
| `scripts/tests/test_ci_workflows.py` | the read census | entries for the new and changed call sites of `test_release_workflow.py`, nothing else |
| `RELEASING.md` | runbook | section 3: the detection and the recovery |
| `docs/specs/SPEC-373-a-release-tag-whose-push-started-no-run-is-released-by-a-dispatch-at-its-own-ref-through-the-same-guard.md` | spec | new |
| `docs/decisions/ADR-384-a-tag-with-no-release-run-is-released-by-a-dispatch-at-the-tags-ref-that-runs-the-tag-paths-own-guard-and-group.md` | decision | new |
| `docs/schematics/release-from-a-tag-push-or-a-dispatch-at-the-tags-ref.md` | schematic | new |
| `docs/red-first/SPEC-373.md` | red-first record | new |
| `docs/specs/SPEC-062-first-deploy-units-caddy-block-and-https.md` | spec | one insert-only amendment line, appended |
| `scripts/mutation-rows.d/S37300-S37399.json` | rows | new band, S37301 to S37309 |
| `scripts/mutation-rows.d/S19000-S19099.json` | rows | S19022's find and replacement re-anchored; id, killer and description unchanged |
| `changelog.d/release-second-path-373.md` | changelog | fragment |

## 6. What this does NOT cover

- Two runs of one tag in one group, and the queue's bound: the group and its tests own them
  (SPEC-190, ADR-292), after #377 and #456. This delivery adds a second way into the same group and
  changes nothing in it.
- A model of the release queue: owed by #467. The dispatched run is one more actor that model must
  carry.
- Staging the web engine module under `/engine/` in the release tarball: #685. Both paths build the
  same tarball, so that change reaches both.
- The reader remainder #464 that #456 left: untouched.
- The app's workflows that a release tag's push also starts (`RELEASING.md` section 8) keep their
  tag-push triggers, so a tag whose push started no run still starts none of them (#475's scope is
  the release workflow).
- testflight-release.yml's ancestry step is pinned to release.yml's guard step (SPEC-352 A18), so
  the ref check is carried there too (#475).
- A tag cut before `main` carries this change has no dispatch trigger at its commit, so it cannot
  take the second path (#475).
- This delivery makes no release, tag or dispatch; the first dispatched release is the maintainer's
  (#475).

## 7. Risks

- **A dispatch at a ref that is not a release tag.** Anyone who can push a tag can also dispatch,
  at any branch or tag. The guard refuses every ref that is not `refs/tags/<the tag>` (A2).
  Detection: the run fails at the guard step, its message naming the ref.
- **A dispatch for a tag that already has a release.** The guard admits it, since the tag is valid;
  the run builds again and reaches the step that creates the draft, which meets the existing
  release before anything is attested or uploaded. The runbook limits the dispatch to a tag whose
  push started no run. Detection: the run fails at the draft step.
- **The deploy's verification.** `RELEASING.md` says the deploy verifies the tarball's attestation
  by repository and signing workflow. A dispatched run signs with the same workflow at the same tag
  ref. If the verification also pinned the triggering event, a dispatched release would not
  deploy; the build reads the deploy script's verification before it changes anything. Detection:
  the deploy's verification fails, naming the attestation.
- **Two pins change.** A8's exact event pin moves to the new exact set, and A18 gains the ref a
  push has; both stay as strong as before and are disclosed as such.
- **A test that passes because both paths share a step.** A3 and A4 would pass a path-specific
  break they never ran; rows S37307 and S37308 plant one in each and must be killed.

## 8. Amendments

- SPEC-405 (#733): the release lane `testflight-release.yml` now takes a dispatch at the tag's own
  ref too, so section 6's bullet on the app's workflows keeping their tag-push triggers no longer
  holds for that lane; `apple-on-tag.yml` keeps its tag push only, by ADR-419 D2b.
