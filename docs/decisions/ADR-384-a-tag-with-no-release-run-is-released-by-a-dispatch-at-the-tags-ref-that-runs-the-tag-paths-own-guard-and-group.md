---
status: "accepted"
---

# ADR-384: A tag with no release run is released by a dispatch at the tag's ref that runs the tag path's own guard and group

Decides SPEC-373.

## Context and Problem Statement

A release tag's push can start no run (#475), and the tag rules
(`.github/rulesets/release-tags.json`: `deletion`, `non_fast_forward`, `update`, no bypass actor)
refuse moving the tag or deleting it to push it again. At dev `6f9ef860`, `release.yml:12-15`
admits one event, the tag push, so such a tag is never released. #475 accepts either a second path
that keeps every property of the tag path, or detection with a documented recovery that never
pushes the tag again, and asks for a test that the second path refuses a ref that is not a tag and
a tag off `main`.

The tag path's properties, measured in `release.yml`: the tag name is SemVer (48-51), the tag is
annotated (52-55), its commit is on `main` (56-60), every run of one ref shares one group that
queues and never cancels (23-26, ADR-292), the audit job runs before the release job (30), and
the release job alone holds write scopes (33-36).

## Decision Drivers

- #475's acceptance, unchanged: a second path or a recovery, and a test of the refusals.
- The tag rules stay as they are: no release tag is ever moved, deleted or pushed twice.
- One check, one place: a property both paths need is held by one step both paths run.
- No input reaches a step, so the second path adds no injection surface.
- The tests of the release workflow already run its guard under bash; the new tests extend them.

## D1. Which of the issue's two acceptances (the options it was chosen against)

- Chosen: a second trigger, a manual dispatch at the tag's own ref, with the runbook's detection and recovery beside it, because it releases the very tag that was cut, and the tag rules leave no other way to do that.
- Detection with a documented recovery alone: rejected because, under the tag rules' `update` and `deletion` rules, a recovery that never pushes the tag again can only cut a new patch tag, which spends a version and leaves the first tag unreleased for good.
- A `create` trigger for the tag: rejected because it is, like the push, an event of the tag's creation, so a creation that raised no run leaves the tag as unreleased as before, and it takes no tag filter, so every branch and tag anyone creates would start a release run for the guard to refuse.
- A scheduled sweep that releases every tag with no release: rejected because it is a third event with nobody choosing to release, and it would release a tag that was left unreleased on purpose.

## D2. How the second path keeps each property (the options it was chosen against)

- Chosen: one guard step both events run, unchanged but for one added check that `GITHUB_REF` is `refs/tags/` and the tag, and the workflow-level group reused unchanged, because one step cannot drift from itself, and the ref check closes the one hole a dispatch opens: a branch named like a release tag.
- A copy of the guard for the dispatch only, under `if: github.event_name == 'workflow_dispatch'`: rejected because two copies of one check drift apart, and an `if:` on a guard is a skip.
- A separate job that checks the ref, which both jobs need: rejected because it changes the push path's job graph, which `steps_of` and SPEC-340 pin, for no property #475 asks for.
- A job-level `if:` that skips the release for a ref that is not a tag: rejected because a skipped run reads as a success, so a refused dispatch would look released.
- A dispatch on `main` that takes the tag's name as an input: rejected because the run's ref would be `main`, so its group, the workflow file it runs and its attestation's ref would all differ from the tag path's, and an input a step reads is an injection surface.
- A check that `GITHUB_REF_TYPE` is `tag`: rejected because it says the ref is some tag but not that it is the tag whose name the guard checked, where comparing the whole ref binds both.
- An added check that `GITHUB_SHA` is the tag's commit: rejected because a run at a tag's ref runs the tag's commit by construction, and the tag rules refuse moving the tag, so the check could never fire.
- An early check that refuses a tag that already has a release: rejected because #475 does not ask for it, and the draft step already meets an existing release before anything is attested or uploaded.

## D3. The tests, their runner and where they run (the options it was chosen against)

- Chosen: A1 to A4 in `scripts/tests/test_release_workflow.py` and A5 in `scripts/tests/test_rulesets.py`, run by the CI job that runs every module under `scripts/tests`, each red read in CI, because the release workflow's tests already live there and run its guard under bash in a scratch repository, the runbook's tests already live in `test_rulesets.py`, and the release module imports the workflow reader's module, whose suite is read in CI.
- A new test module for the second path: rejected because it would repeat the release module's scratch repository and its reader import, and add one more module to the read census for no new property.
- A dispatch of the real workflow as the test: rejected because a workflow is dispatched only from the default branch's file, the run needs a release tag on `main`, and a release is the maintainer's act, never a test's.
- Pinning the guard's text instead of running it: rejected because a text pin passes a guard that no longer refuses, and running it under bash observes the refusal itself.

## D4. The owning SPEC (the options it was chosen against)

- Chosen: a new SPEC-373 for the second path, with one appended amendment line in SPEC-062 that points at it, because the second path is a new property with its own criteria and band, and SPEC-062's R1 and A8 change under it, which a pointer records without rewriting SPEC-062.
- An insert-only amendment of SPEC-062 alone: rejected because SPEC-062 owns the first deploy, its site block and HTTPS as well, and a second release path with five criteria and nine rows would sit in that SPEC with no band of its own.
- An amendment of SPEC-190, which owns the group: rejected because the group does not change.

## D5a. Row S19022 after the change (the options it was chosen against)

- Chosen: re-anchor S19022 on the new dispatch line, its id, killer and description unchanged, its mutant still an undefined `Inputs` key under the dispatch, because its find, the tag filter's line, now sits just above the real dispatch key, so its mutant would add a second `workflow_dispatch` key, which the workflow reader refuses as a duplicate, and the row would die for that and not for the key it plants.
- Leaving S19022 as it is: rejected because a row killed by a duplicate key proves nothing about the key it plants.
- Retiring S19022: rejected because the property it pins, a dispatch holds only the keys the parser defines, is now a property of a real line, and a row is re-anchored, never retired.

## D5b. A18's environment (the options it was chosen against)

- Chosen: A18 runs the guard with `GITHUB_REF` set to `refs/tags/` and its tag, the ref a push of that tag has, because the new check reads it, and A18 took it from the caller's environment, which in CI is the pull request's ref, so every one of A18's tags would be refused for the wrong reason.
- Letting the ref check pass when `GITHUB_REF` is unset: rejected because a guard that admits a missing ref fails open.

## D5c. SPEC-062 A8's event pin (the options it was chosen against)

- Chosen: A8 pins exactly the two events, because it pins the events exactly, the events change, and an exact pin of the new set is as strong as the old one.
- Removing A8's event assertion and leaving A1 to hold it: rejected because it removes a pin, and a removed pin reads as a weakening.

## D5d. The live proof (the options it was chosen against)

- Chosen: the first dispatched release is the maintainer's, after the release pull request carries the change to `main`, because GitHub dispatches a workflow only from the default branch's file and runs the file at the dispatched ref, so only a tag whose commit carries the change can take the second path.
- A rehearsal tag cut to prove the path: rejected because a release tag cannot be deleted under the tag rules, so a rehearsal spends a version for good.

## Decision Outcome

`release.yml` gains `workflow_dispatch` with no input. Its guard gains one check, after the SemVer
check: `GITHUB_REF` must be `refs/tags/` and the tag, or the run is refused with a message naming
the ref. Nothing else in the workflow changes, so a dispatch made at a tag's own ref runs the same
audit, the same guard and the same build, in the same group, and attests under the same ref as the
tag's push would have. `RELEASING.md` says how to see that a tag's push started no run and gives
the one command that releases it, and names no command that moves the tag.

### Consequences

- Good: a tag whose push started no run is released as cut, with no version spent.
- Good: one guard and one group serve both paths, so no check can hold on one path and not the
  other.
- Good: no input, so nothing a dispatcher types reaches a step.
- Bad: only a tag whose own commit carries the change can take the second path; older tags cannot.
- Bad: the dispatch is one more way to start a release run. It is open to the same accounts that
  can push a tag, and the guard decides what releases.
- Neutral: the app's workflows that a release tag's push also starts keep their tag-push triggers
  (#475's scope is the release workflow).

### Confirmation

SPEC-373 A1 to A5, each red first and read in CI; rows S37301 to S37309 in
`scripts/mutation-rows.d/S37300-S37399.json`, each killed by its named test; S19022 re-anchored and
killed by its own killer for its own key; SPEC-062 A8 and A18 green on the changed workflow.

## More Information

- Issues: #475 (the requirement), #456 and #377 (the queue), #467 (the queue's model, which must
  carry the dispatched run), #685 (the engine module in the tarball), #464.
- SPEC-062 (the release workflow's R1, A8, A18), SPEC-190 and ADR-292 (the group), SPEC-340 (the
  audit job).
- This is wrong if a dispatch at a tag's ref ran with a `GITHUB_REF` other than `refs/tags/` and
  the tag: every dispatch would be refused at the guard, which its message would show on the first
  one. It is also wrong if the deploy's verification pinned the triggering event; the build reads
  that verification before it changes anything.
- Amended by ADR-419 (#733): the release lane `testflight-release.yml` now takes a dispatch at
  the tag's own ref as well, so the Neutral consequence above holds only for `apple-on-tag.yml`.
