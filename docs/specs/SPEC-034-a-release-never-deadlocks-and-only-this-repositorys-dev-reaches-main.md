# SPEC-034: a release never deadlocks, only this repository's dev reaches main, and only GitHub Actions satisfies a required check

- **Wave:** W0. **Issue:** #23 (epic #1). **Context(s):** `repo` (`.github/`, `RELEASING.md`, the
  contributor and agent documents).
- **Decided by:**
  - ADR-034, this SPEC's own: main does not require an up-to-date head, nothing is merged back,
    and a hotfix goes through dev.
  - ADR-035, this SPEC's own: the required checks are pinned to GitHub Actions, and a fork never
    reaches main.
  - ADR-017 (the branch model) and ADR-033 (the committed rulesets are the enforced ones).
- **Status:** judged: delivered with its tests before the first release is tagged.

## 1. The problem, measured

The maintainer's review after the repository went public found three defects in the enforced
rulesets and the `ci` workflow (`gh api repos/RexRenatus/deck-streak/rulesets/<id>`, and
`.github/workflows/ci.yml` at `dev` c1f53c1).

- **The second release deadlocks.**
  - `main`'s ruleset requires an up-to-date head (`strict_required_status_checks_policy: true`),
    and both branches take pull requests only.
  - After the first release merge, `main` holds a merge commit that `dev` lacks, so the next
    release pull request from `dev` is out of date and strict blocks it.
  - Bringing `dev` up to date needs a pull request from `main` into `dev`. It changes no file, so
    `fragment` refuses it (`.github/workflows/changelog.yml`). Once `dev` has moved on, `dev`'s own
    strict rule blocks it too, because `main` cannot contain `dev`'s newer commits.
  - GitHub's "update branch" on the release pull request would merge `main` into `dev` without a
    pull request, which `dev`'s ruleset exists to refuse.
- **A fork's branch named `dev` passes `base-is-dev`.** The job compares `github.head_ref` with
  `dev` and reads nothing about the head's repository. The fork approval policy
  (`all_external_contributors`, set when the repository went public) holds workflow runs from forks
  for a maintainer's approval. That is a human step, not a check.
- **Any source can satisfy a required check.** Both branch rulesets list `ci` and `fragment` with
  `integration_id` unset, which means any app or any commit status with that context name. Measured
  on `dev`'s check runs: every `ci`, `fragment` and `base-is-dev` run carries `app.id` 15368
  (`github-actions`).

The fork rule the maintainer set at the flip also lives only in a message: agents never approve a
workflow run from a fork, and never merge a pull request whose head repository is not
`RexRenatus/deck-streak`.

## 2. Requirements

R1. `main`'s ruleset does not require an up-to-date head (`strict_required_status_checks_policy:
    false`). `dev`'s ruleset does (`true`).
    - `base-is-dev` admits one head into `main`: this repository's `dev`. GitHub keeps one open pull
      request per head and base, so `main` cannot move between a release pull request's checks and
      its merge.
    - Those checks run on the pull request's merge ref, which already contains `main`.
R2. Nothing is merged from `main` into `dev`. A hotfix is a pull request into `dev`, followed by a
    patch release. `RELEASING.md` has no back-merge step and says so (ADR-034).
R3. Every required status check in `.github/rulesets/main.json` and `dev.json` names
    `integration_id` 15368, the GitHub Actions app.
R4. `base-is-dev` fails a pull request into `main` unless its head branch is `dev` and its head
    repository is this repository (`github.event.pull_request.head.repo.full_name` equals
    `github.repository`). It reads both through `env`, never by expanding an expression inside the
    script.
R5. `AGENTS.md`, `CLAUDE.md` and `CONTRIBUTING.md` each state the fork rule. The maintainer re-lands
    an accepted outside contribution from a branch of this repository.
R6. After the first release merge, and before its tag, a probe release pull request from `dev` into
    `main` is opened while `dev` lacks `main`'s release merge commit.
    - Its merge state must not be `BEHIND`.
    - GitHub's update-branch is tried on it, and the result is recorded in `RELEASING.md`.
    - The probe is closed unmerged. It is an operational proof, recorded on the release pull request.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | main does not require an up-to-date head | `test_rulesets.py` |
| A2 | dev requires an up-to-date head | `test_rulesets.py` |
| A3 | every required check comes from GitHub Actions | `test_rulesets.py` |
| A4 | the release runbook merges nothing back into dev | `test_rulesets.py` |
| A5 | base-is-dev refuses a fork whose branch is named dev | `test_ci_workflows.py` |
| A6 | base-is-dev admits this repository's dev into main | `test_ci_workflows.py` |
| A7 | base-is-dev refuses this repository's other branches into main | `test_ci_workflows.py` |
| A8 | every agent document states the fork rule | `test_fork_rule.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k main_does_not_require_an_up_to_date_head
A2: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k dev_requires_an_up_to_date_head
A3: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k every_required_check_comes_from_github_actions
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k the_release_runbook_merges_nothing_back_into_dev
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k base_is_dev_refuses_a_fork_whose_branch_is_named_dev
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k base_is_dev_admits_this_repositorys_dev_into_main
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k base_is_dev_refuses_this_repositorys_other_branches_into_main
A8: python3 -m unittest discover -s scripts/tests -p test_fork_rule.py -k every_agent_document_states_the_fork_rule
```

A5 to A7 run `base-is-dev`'s own script, as extracted from `ci.yml`, under `bash -e`.
- Each of its `env` expressions is given a scenario's value.
- A step that reads an expression the scenarios do not model fails the test, rather than running
  with an empty value.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/rulesets/main.json` | `repo` | changed: strict off; checks pinned to 15368 |
| `.github/rulesets/dev.json` | `repo` | changed: checks pinned to 15368 |
| `.github/workflows/ci.yml` | `repo` | changed: `base-is-dev` reads the head repository |
| `scripts/tests/test_rulesets.py` | `repo` | changed: A1 to A4; SPEC-033's A9 no longer asserts main's strictness |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed: A5 to A7 |
| `scripts/tests/test_fork_rule.py` | `repo` | added: A8 |
| `RELEASING.md` | `repo` | changed: the release is a pull request into dev, then a release pull request; no back-merge; hotfixes through dev |
| `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md` | `repo` | changed: the fork rule |
| `docs/decisions/ADR-034-main-is-never-merged-back-and-a-hotfix-goes-through-dev.md` | `repo` | added |
| `docs/decisions/ADR-035-required-checks-come-from-actions-and-forks-never-reach-main.md` | `repo` | added |
| `docs/decisions/ADR-017-branch-model-and-hosted-ci.md` | `repo` | changed: a note that ADR-034 supersedes part of its outcome |
| `docs/specs/SPEC-034-a-release-never-deadlocks-and-only-this-repositorys-dev-reaches-main.md` | `repo` | added |
| `docs/red-first/SPEC-034.md` | `repo` | added |
| `changelog.d/fix-release-flow-and-forks.md` | `repo` | added |

## 5. What this does NOT do

- It builds no release workflow: the tag-triggered build and its draft release are the first
  deploy's work (#42).
- It does not tag v0.1.0 or merge the first release. That is the release pull request's work, and
  R6's probe runs between the two (#23).
- It gives no bypass to anyone, including the owner's account: every ruleset keeps an empty bypass
  list (#160).
- It admits no fork contribution automatically. The maintainer re-lands accepted outside work from
  an internal branch (#160).

## 6. Risks

- **Two release pull requests at once.** GitHub refuses a second open pull request with the same
  head and base, and `base-is-dev` refuses any other head. A5 to A7 hold the second half.
- **A strict-off `main` merges a stale head.** Only `dev` reaches `main`, and a pull request's
  checks run on its merge ref, so what merges is what was tested. R6's probe measures it.
- **The integration id changes.** It is GitHub's own app, and A3 pins it. A check from any other
  source is refused, which is the intent. A change would show as every pull request blocked on
  `ci`.
