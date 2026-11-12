---
status: "accepted"
---

# ADR-419: The release lane `testflight-release.yml` gets a second path at the tag's ref, and `apple-on-tag.yml` needs none

Decides SPEC-405.

## Context and Problem Statement

A release tag's push can start no run (#475). SPEC-373 and ADR-384 gave `release.yml` a second
path, a manual dispatch at the tag's own ref, and left out of scope the other workflows a release
tag's push starts. #733 asks, for each of them, for a second path held to the release workflow's
rules (no input, a run at the tag's own ref, one guard that both paths run, one concurrency group),
or a recorded decision that the lane needs none.

Measured at dev `32f62172` with `git show 32f62172:<path>`:

- `RELEASING.md:159` names the workflows a release tag's push starts: `release.yml`,
  `testflight-release.yml` and `apple-on-tag.yml`.
- `release.yml` runs on the tag push and an input-free dispatch (`on` at 13-17, the dispatch at
  17), in the group `release-${{ github.ref }}` (26-29, never cancelled, `queue: max`), through one
  guard step (47-67) that checks SemVer, that `GITHUB_REF` is `refs/tags/` and the tag, an annotated
  tag and a commit on `main`.
- `testflight-release.yml` runs on the tag push only (`on` at 7-10). Its group is
  `testflight-release-${{ github.ref }}` (18-21, never cancelled, `queue: max`). Its `plan` job
  runs a guard step (36-56) that is `release.yml`'s, byte for byte (SPEC-352 A18), the ref check
  included (44-47), and then `python3 scripts/ios_lane.py plan --lane release` (57-59). Its
  `framework` job (61-63) calls `xcframework.yml`. Its `app` job (65-144) alone names an
  environment (69) and reads the upload credential, in three steps (107-135), each from its own
  step's environment. No job and no step of the lane carries an `if:`.
- `scripts/ios_lane.py:112-113` refuses every release-lane run whose event is not `push` or whose
  ref type is not `tag`, so a dispatch of the lane is refused even at the tag's own ref.
- `apple-on-tag.yml` (24 lines) runs on the tag push only (`on` at 7-10), with read-only
  permissions (12-13), the group `apple-on-tag-${{ github.ref }}` (17-20) and one job, `apple`,
  that calls `xcframework.yml` (22-24). It has no guard; its header (3-6) says it builds and
  publishes nothing and passes no secret.
- `xcframework.yml` runs on a call and on its own input-free dispatch (`on` at 22-24); no tag's push
  starts it.
- `.github/rulesets/release-tags.json` covers `refs/tags/v*` with `deletion`, `non_fast_forward` and
  `update` and no bypass actor, and has no creation rule. A release tag is never moved, deleted or
  pushed again, so a tag whose push started no lane run never gets its build.
- `testflight-rerelease-check.yml:19-21` holds `actions: write`, the only such grant under
  `.github/workflows`, and `scripts/testflight_age.py:209-215` dispatches the internal lane with that
  workflow's own token. A dispatch made with a workflow's own token creates a run, where a push made
  with it creates none.
- No model or proof under `formal/` names a release workflow, `workflow_dispatch` or `github.ref`
  (0 hits).

The population D1 decides, and what this change does to each member:

| workflow | trigger at the base | guard | group | after this change |
|---|---|---|---|---|
| `release.yml` | tag push and dispatch (13-17) | one step (47-67) | `release-${{ github.ref }}` (26-29) | unchanged; its second path is SPEC-373's |
| `testflight-release.yml` | tag push only (7-10) | `plan`'s step (36-56) and the release plan | `testflight-release-${{ github.ref }}` (18-21) | gains an input-free dispatch (D2a) |
| `apple-on-tag.yml` | tag push only (7-10) | none: it builds and publishes nothing | `apple-on-tag-${{ github.ref }}` (17-20) | unchanged: needs none (D2b) |

## Decision Drivers

- #733's acceptance, unchanged: each lane gets a second path held to the release workflow's rules,
  or a recorded decision that it needs none.
- The tag rules stay as they are: no release tag is moved, deleted or pushed twice.
- One check, one place: a rule both paths need is held by one step both paths run.
- No input reaches a step, so a second path adds no injection surface.
- A second path never widens who or what can start a run that reads the upload credential.

## D1. The population (the options it was chosen against)

- Chosen: the three workflows whose `on` holds a release tag's push, `release.yml`, `testflight-release.yml` and `apple-on-tag.yml`, because `RELEASING.md:159` names exactly these and each declares the tag push, so these are the runs a tag whose push started no run is missing.
- Counting `xcframework.yml`: rejected because its `on` (22-24) is a call and a dispatch, so a tag's push never starts it; it runs only as the callee of `apple-on-tag.yml` and of the lane's `framework` job.
- Counting `ci.yml`: rejected because its push trigger names branches only (`ci.yml:12-17`), so a tag's push starts nothing there.

## D2a. The release lane `testflight-release.yml` gets the second path (the options it was chosen against)

- Chosen: `workflow_dispatch:` with no input, after the tag filter as in `release.yml:17`, the guard step and the group unchanged, and the release plan in `scripts/ios_lane.py` admitting `workflow_dispatch` beside `push` when the ref type is a tag, because the guard step already runs on both events with no `if:` and already binds `GITHUB_REF` to `refs/tags/` and the tag, the group renders one name for both events, and the plan's event check is the one refusal that keeps the dispatch out.
- One dispatcher that fans out to every lane: rejected because a fan-out by call puts the `app` job, which names an environment, under a `workflow_call` trigger, which ADR-363 D2 rejected, a fan-out by command needs a token that can write actions, and either way it adds a third copy of the guard and a group of its own.
- A dispatch that takes the tag as an input: rejected because the run's ref would be the default branch, so its group, the workflow file it reads, its checkout and the environment's tag rule would all differ from the push's, and an input a step reads is an injection surface (ADR-384 D2).
- Leaving the lane on a tag push only: rejected because a tag whose push started no lane run then never gets its build, and the tag rules refuse pushing it again.
- A job-level `if:` that skips a run whose ref is not a tag: rejected because a skipped run reads as a success, so a refused dispatch would look built.
- A second file that holds only the dispatch: rejected because two files of one lane drift apart, and the second would take a group of its own.
- A check in the plan that binds the whole ref again: rejected because the guard step binds `GITHUB_REF` to `refs/tags/` and the tag one step earlier in the same job, and the plan reads the ref type only to keep a branch out of the release arm.

The new event arm, one line of 98 characters, with the refusal wrapped as the internal arm's is:

```python
    elif event not in ("push", "workflow_dispatch") or os.environ.get("GITHUB_REF_TYPE") != "tag":
        raise Refused(
            f"the release lane runs on a tag's push or dispatch only, not on {event} of {ref}"
        )
```

## D2b. `apple-on-tag.yml` needs no second path (the options it was chosen against)

- Chosen: a recorded decision that it needs none, because the job body it calls, `xcframework.yml`, already runs at the tag's ref on both of the lane's paths as the lane's `framework` job, after the lane's guard and in the lane's group, it is advisory (it publishes nothing, reads no credential, and no ruleset requires its checks, ADR-355), and `xcframework.yml`'s own dispatch already runs that body at any ref.
- A dispatch plus a guard job: rejected because SPEC-344 pins its one `apple` job (`scripts/tests/test_release_workflow.py:155-171`), and a guard job would change that shape for a build nobody waits on.
- A dispatch with no guard: rejected because it would be a path held to none of #733's rules, and it would repeat `xcframework.yml`'s own dispatch.
- A job-level `if:`: rejected because a skipped run reads as a success.

## D3. What a second path may start (the options it was chosen against)

Only the lane's `app` job reads the upload credential: it names the release environment, and three
of its steps read the credential's parts, each from its own step's environment. Who can start a
dispatch: an account with write access, the same accounts that can create a release tag, since the
tag rules hold no creation rule and no bypass actor. What can start one: a dispatch made with a
workflow's own token creates a run where a push made with it creates none, and one workflow holds
`actions: write`. Left open, the second path would admit a run that a workflow started.

- Chosen: the release plan refuses a run whose `GITHUB_ACTOR` is the workflow token's actor, `WORKFLOW_TOKEN = "github-actions[bot]"`, a module constant, on either event, as the arm after the event arm, with the message `the release lane takes no run that github-actions[bot] started`, and the release environment's required review (ADR-363 D7) stays, because then the accounts and the tokens that can start a lane run reading the credential are exactly the ones that can start it by a push.
- Relying on the environment's review alone: rejected because the run still starts and builds and waits at the review, a request a reviewer must refuse by hand.
- Narrowing the token: rejected because `actions: write` is granted for the whole repository, so a narrower grant cannot name one workflow.
- A census of the workflows the actions-write job dispatches: rejected because code can build a workflow's name at run time, so a census of literals proves nothing.

`GITHUB_ACTOR` is the actor of a run's first attempt, and a re-run keeps it, so a re-run of a
refused run is refused too. The token's actor is the hosting service's documented name for a run
that token starts; no dispatch run records it yet, and SPEC-405 section 4 names the first that
will. `release.yml`'s own dispatch admits a run a workflow's token starts; that is outside #733
and is an exclusion in SPEC-405.

## D4. FORMAL, decided by surface (the options it was chosen against)

- Chosen: no model and no proof, because the second path adds one actor to a queue the hosting service holds, not repository state, each run reads only immutable or monotone inputs (a tag the rules forbid moving, and `main`'s ancestry, which only grows), the upload refuses a build number it already holds, and no model or proof under `formal/` cites a release workflow (0 hits).
- A new TLA+ model of the two triggers: rejected because the state it would model sits outside the repository, and the queue's model is owed by #467, which must carry the dispatched lane run.
- A Lean proof of the guard: rejected because the guard is shell, and the tests hold it by running it under bash.

## D5. The tests, their runner and the push count (the options it was chosen against)

- Chosen: A1 and A3 in `scripts/tests/test_testflight_workflows.py`, A2 and A5 in `scripts/tests/test_ios_lane.py`, and A4 in `scripts/tests/test_rulesets.py`, run by CI's `hygiene` job, which runs every module under `scripts/tests`, each red read in CI, with TWO pushes (the red tests and the documents alone, then the fix), because the lane's workflow tests import the workflow reader's module, whose suite is read in CI, so a red exists at a pre-fix commit only if that commit is pushed alone.
- One push: rejected because no CI run would ever see the new tests red at a commit without the fix.
- A new test module for the second path: rejected because it would repeat the reader import and add a module to the read census for no new property.
- A dispatch of the real lane as the test: rejected because a dispatch runs the default branch's file, needs a release tag on `main`, and is the maintainer's act, never a test's.

## D6. SPEC-352's two pins (the options it was chosen against)

- Chosen: SPEC-352 A14's release entry moves from one exact pin to the new exact pin, `{"push": {"tags": TAGS}, "workflow_dispatch": None}`, and A5's refused start `("workflow_dispatch", "refs/tags/v0.2.0")` becomes a dispatch at `refs/heads/v0.2.0` with the new message, because the admitted path changed, every refusal is still asserted, and the newly admitted path has its own red-first criterion (SPEC-405 A2).
- Dropping A14's release assertion and leaving SPEC-405 A1 to hold it: rejected because a removed pin is a weakening.
- Deleting A5's dispatch case: rejected because a dispatch at a branch is a refusal the lane still makes, and a deleted case is a weakening.
- Keeping A5's case as it is: rejected because a dispatch at the tag's own ref is now admitted, so the case cannot stay refused.

## D7. The owning SPEC (the options it was chosen against)

- Chosen: a new SPEC-405 with insert-only pointers appended to SPEC-352, ADR-363, SPEC-373 and ADR-384, because the second path is a new property with its own criteria and band, and each of those four documents states the rule it replaces.
- An amendment of SPEC-352 alone: rejected because five new criteria and sixteen rows would sit inside the lane's SPEC with no band of their own.

## Decision Outcome

`testflight-release.yml` gains `workflow_dispatch` with no input. Its guard step, its group and its
jobs are unchanged, so a dispatch made at a tag's own ref runs the same guard, the same plan, the
same framework build and the same `app` job, waits for the same review, and takes its turn in the
tag's group. The release plan admits that dispatch, refuses a dispatch at any branch with a message
naming the event and the ref, and refuses a run a workflow's own token started. `apple-on-tag.yml`
keeps its tag push only; the lane's `framework` job runs the same body at the tag's ref.
`RELEASING.md` section 8 says how to see that a tag's push started no lane run and gives the one
command that builds it, and names no command that moves the tag. ADR-363's sentence "the release
lane keeps its one trigger" is superseded by this ADR, through a pointer appended to ADR-363 that
cites #733.

### Consequences

- Good: a tag whose push started no lane run gets its build as cut, with no version spent.
- Good: one guard step and one group serve both paths, so no check can hold on one path only.
- Good: no input, so nothing a dispatcher types reaches a step.
- Good: the starters of a run that can read the credential are the push's, and no workflow's.
- Bad: only a tag whose own commit carries the change can take the second path.
- Bad: the dispatch is one more way to start a lane run; it is open to the accounts that can push
  a tag, and the guard, the plan and the review decide what builds.
- Neutral: `apple-on-tag.yml` keeps its one trigger; a tag whose push started no run still starts
  no run of it, and nothing waits on that run.

### Confirmation

SPEC-405 A1 to A5, each red first and read in CI, and A6 and A7 green on the changed tree; rows
S40500 to S40515 in `scripts/mutation-rows.d/S40500-S40599.json`, each killed by its named test;
SPEC-352 A14 and A5 at their moved pins, red at the red commit and green at the fix; rows S35204,
S35212, S35213 and S37309 unmoved, each find text present once.

## What would make this wrong

- A dispatch at a tag's ref ran with a ref type other than `tag`: every dispatch would be refused
  at the plan, which its message would show on the first one.
- The workflow token's actor were not `github-actions[bot]`: a run it started would pass the
  plan. No dispatch run records the actor yet; SPEC-405 section 4 names the first run that will.
- A run reached the `app` job without the release environment's review: D3 would rest on the plan
  alone. ADR-363 D7 holds that review.
- A ruleset began to require `apple-on-tag.yml`'s checks: D2b would need a second path.

## More Information

- Issues: #733 (the requirement), #475 (the release workflow's second path), #467 (the queue's
  model, which must carry the dispatched lane run), #634 (repository and environment settings).
- SPEC-352 (R1, A5, A14, A18) and ADR-363 (D2, D7); SPEC-373 and ADR-384 (the release workflow's
  second path); SPEC-344 and ADR-355 (one job body, two callers); ADR-292 (queued runs).
