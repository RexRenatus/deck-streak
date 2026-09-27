---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# main is never merged back, main does not require an up-to-date head, and a hotfix goes through dev

## Context and Problem Statement

ADR-017 chose the branch model: `main` changes only through a release pull request from `dev`. Its
outcome also said that `main` is merged back into `dev` after each release, and RELEASING.md cut
hotfixes from `main`. The owner has since made the dev-to-main release workflow binding for the
whole project (2026-09-27): every change is a pull request into `dev`, `main` changes only through a
release pull request from `dev`, and no agent pushes to `dev` or `main` directly. The rulesets
enforce it (SPEC-033):
- both branches take pull requests only;
- both require `ci` and `fragment`, and both required an up-to-date head;
- `base-is-dev`, which `ci` needs, refuses a pull request into `main` whose head is not `dev`
  (probe pull request #187).

The maintainer's review found that this configuration deadlocks the second release (SPEC-034 §1):
- after the first release merge, `main` holds a merge commit that `dev` lacks, and strict blocks
  the next release pull request;
- the pull request that would carry the merge commit back into `dev` changes no file, so `fragment`
  refuses it;
- once `dev` has moved on, `dev`'s own strict rule refuses it as well.

How do releases and hotfixes flow so that none of them can deadlock?

## Decision Drivers

- One path for every change, through the same gate, exactly once.
- `main` never holds a change `dev` lacks.
- No direct push to `dev` or `main`, by anyone.
- What merges into `main` is exactly what its checks tested.

## Considered Options (the alternatives it was chosen against)

- No back-merge and a non-strict `main`, with hotfixes through dev: chosen, because `main`'s
  strictness adds nothing here. `base-is-dev` admits one head into `main` (this repository's `dev`),
  and GitHub keeps one open pull request per head and base. So `main` cannot move between a release
  pull request's checks and its merge, and those checks already run on the merge ref, which contains
  `main`. `dev` keeps its strictness, because many pull requests land there.
- A back-merge pull request from `main` into `dev` after each release: rejected because it needs
  `fragment` taught to pass an empty diff and adds a step to every release. It must also land before
  anything else merges into `dev`, or `dev`'s strict rule refuses it (`main` cannot contain `dev`'s
  newer commits).
- GitHub's update-branch on the release pull request: rejected, because it merges `main` into `dev`
  without a pull request, which `dev`'s ruleset and the owner's workflow forbid.
- A hotfix branch cut from `main`, with a pull request into `main`: rejected, because `base-is-dev`
  refuses any pull request into `main` whose head is not `dev`.

## Decision Outcome

Chosen option.
- `.github/rulesets/main.json` sets `strict_required_status_checks_policy` to `false`, and
  `dev.json` keeps it `true`.
- RELEASING.md now says:
  - a release is prepared by a pull request into `dev` that sets the version and compiles the
    changelog;
  - it ships by a release pull request from `dev` into `main`, merged with a merge commit and
    tagged `vX.Y.Z` on `main`;
  - nothing is merged back into `dev`;
  - a hotfix is a pull request into `dev`, followed by a patch release.

This supersedes three statements in ADR-017's outcome:
- its back-merge sentence;
- its statement that `base-is-dev` is a required check of its own (it is enforced through `ci`,
  which needs it);
- its statement that a ruleset cannot be applied to the private repository (the rulesets were
  active on it).

### Consequences

- Good, because every change to `main` has passed `dev`'s gate and the release pull request's gate,
  and no release waits on a back-merge.
- Bad, because `main`'s history carries merge commits that `dev`'s history never will. A release's
  diff is read against `main`, never against `dev`'s history.
- Bad, because a hotfix ships whatever else `dev` holds at that moment. `dev` must therefore stay
  releasable: green, with nothing half-built behind a public surface.

### Confirmation

- `scripts/tests/test_rulesets.py` (SPEC-033 A9 to A12; SPEC-034 A1 to A4).
- The probe pull request #187: `base-is-dev` failed, so the required `ci` failed.
- SPEC-034 R6: after the first release merge and before its tag, a probe release pull request from
  `dev` into `main` must not read `BEHIND`, and the update-branch result is recorded.

## What would make this wrong

- A second head can reach `main`, for example a merge queue or a bypass actor. `main` would then
  need strictness again, and a back-merge with it.
- A hotfix is urgent while `dev` holds work that must not ship. The answer is a revert on `dev`
  before the patch release, never a branch from `main`. If that happens often, `dev` is not being
  kept releasable, and that is the thing to fix.

## More Information

ADR-017 (the branch model); ADR-035 (the required checks' source, and forks); RELEASING.md; the
release-ops pack's release and ruleset rows.
