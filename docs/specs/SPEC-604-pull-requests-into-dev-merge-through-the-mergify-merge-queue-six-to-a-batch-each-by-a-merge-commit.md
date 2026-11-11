# SPEC-604: pull requests into dev merge through the Mergify merge queue, six to a batch, each by a merge commit

- **Issue:** none. **Context(s):** the repository's continuous integration and its merge path into
  dev (`.github/workflows/`, `.mergify.yml`).
- **Decided by:** ADR-625 (dev merges through the Mergify merge queue, chosen over hand-built union
  branches and GitHub's own merge queue).
- **Revised by:** ADR-689 (R2: a pull request also enters the queue on the READY status a dedicated
  GitHub App posts on its head; R1, R3 to R9 and ADR-625 stand).
- **Schematic:** `docs/schematics/merge-queue.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and implementation.
- **Mutation band:** S60400-S60499.

## 1. The problem, measured

Pull requests into dev merge one at a time, by hand, each after its own run of the three checks the
dev ruleset requires. A queue that tests several ready pull requests together, on the code it is
about to merge, needs a configuration in the tree and needs the three checks to run on its batch
pull request. Measured on dev before this change:

- No Mergify configuration exists: `git ls-tree --name-only dev -- .mergify.yml .mergify.yaml
  .mergify .github/mergify.yml .github/mergify.yaml` prints nothing.
- The dev ruleset file requires three checks and resolved review threads:
  `grep -n -E '"context"|required_review_thread_resolution' .github/rulesets/dev.json` prints
  `ci`, `fragment` and `dependency-review` (lines 43, 47 and 51) and
  `"required_review_thread_resolution": true` (line 33).
- Each of the three workflows runs on every pull request into dev, a draft included:
  `grep -n -A2 'pull_request:' .github/workflows/ci.yml .github/workflows/changelog.yml
  .github/workflows/dependency-review.yml` prints, for each, `branches` naming `dev` and
  `types: [opened, synchronize, reopened]`; `grep -c -E 'paths(-ignore)?:'` over the three prints
  `0` for each. No job-level `if:` in them reads the draft state, the head ref, the author or the
  queue: `grep -n -E '^    if:'` over the three prints only `ci.yml`'s conditions on the mutation
  plan's outputs and `always()`.
- A batch whose members each add a fragment passes `fragment`: line 35 of `changelog.yml` exits 0
  when the pull request adds any file under `changelog.d/` other than its README.
- A batch of dependency updates alone does not: it adds no fragment, and line 36 opens the
  dependency-file rule only when the author is exactly `dependabot[bot]`, while the queue opens its
  batch as `mergify[bot]` from a branch named `mergify/merge-queue/` and ten hexadecimal
  characters (Mergify's page on queue pull requests). Such a batch reads
  `Add a fragment under changelog.d/` and exits 1.

## 2. Requirements

R1. `.mergify.yml` at the root is the one Mergify configuration. It holds one queue rule, with
`batch_size: 6` and `merge_method: merge`, and nothing that comments, labels, closes, assigns,
deletes a branch or posts anything. The queue checks one batch at a time
(`merge_queue.max_parallel_checks: 1`), gives the checks of a batch a fixed `checks_timeout` of six
hours in place of the computed `auto`, and holds no `priority_rules`, since no pull request into dev
is marked release-blocking.

R2. A pull request enters the queue only when its base is dev, it is not a draft, it has no
unresolved review thread, `ci`, `fragment` and `dependency-review` each succeeded on it, and it
either carries the label `queue` or carries the status `queue-ready` that the `deckstreak-ready`
GitHub App posted on its head with the state `success`, read in the qualified form
`@deckstreak-ready/queue-ready`, so a status of that name from any other poster never matches. Two
`pull_request_rules` entries, each with the `queue` action alone, send a pull request to the queue:
one on the label, one on that status. A queue command, or the queue checkbox, admits only a pull
request that one of the two rules would admit. While that App's status on a pull request's head
reads `failure`, the pull request does not enter, and a queued one leaves the queue: the queue
conditions refuse it. The App never posts on a queue batch pull request, so the merge conditions,
which repeat the queue conditions, hold there.

R3. A batch merges only after `ci`, `fragment` and `dependency-review` each succeeded on the batch
pull request; each member then merges by a merge commit. The queue rule's `merge_conditions` are the
same list as its `queue_conditions`, so the queue is single-step: a lone queued pull request that
already contains the tip of dev merges directly, on its own green checks, with no batch pull request
(Mergify's Direct Merge page).

R4. The queue requires every check the committed dev ruleset file requires, and its review-thread
rule, so a pull request the queue merges meets the bar of one merged by hand. A check added to the
ruleset file without the queue turns a test red.

R5. The file validates against Mergify's published configuration schema, fetched once with its
sha256 recorded in the red-first record and never vendored, and every key it uses is one that
schema lists.

R6. The three required workflows run on a queue batch pull request into dev, shown from the
workflow files: the trigger, the branches, the event types, no path filter, and no job condition
that skips a draft, a queue branch or a bot author.

R7. `fragment` passes a batch whose members each add a fragment, with no change to that rule.

R8. `fragment` passes a pull request opened by exactly `mergify[bot]` from a head ref that starts
with exactly `mergify/merge-queue/` when every path it changes passes the dependency-file rule of
the Dependabot arm, unchanged. It reads the head ref from the runner's own `GITHUB_HEAD_REF`, so
the step's declared env stays `BASE` and `AUTHOR`. Every other pull request, a person's on a queue
branch and the queue's on any other branch included, reads exactly as before. Every member of a
batch already passed `fragment` on its own pull request (R2), so this admits nothing a member could
not.

R9. Nothing else changes: no ruleset file, no other workflow, no Dependabot configuration and no
existing test.

## 3. Acceptance criteria of SPEC-604

Each criterion is one test of `scripts/tests/test_merge_queue.py`, named in the block below.

| criterion | what it decides | test |
|---|---|---|
| A1 | the one queue rule tests up to six together, one batch at a time under a fixed checks timeout, and merges each member by a merge commit; no rule ranks pull requests, and every rule only queues (R1) | `test_merge_queue.py` |
| A2 | a pull request enters the queue only into dev, labelled `queue` or carrying the App's `queue-ready` status at `success`, ready for review, with no unresolved thread and the three checks green; the same status name from another poster, the App's status at `failure`, and a queue command on a pull request with neither the label nor the status are refused, and a queued pull request whose App status turns to `failure` no longer meets the queue conditions (R2) | `test_merge_queue.py` |
| A3 | a batch merges only after `ci`, `fragment` and `dependency-review` pass on it; the merge conditions repeat the queue conditions, so a lone pull request that holds the tip of dev merges directly; a batch whose members entered by the App's status merges on the batch's checks alone; planted two-step and `merge-batch` copies are refused (R3) | `test_merge_queue.py` |
| A4 | the queue requires every check and rule the dev ruleset file requires; a planted ruleset copy holding a fourth check is refused (R4) | `test_merge_queue.py` |
| A5 | the three required workflows run on a queue batch pull request into dev; planted copies with a `paths` filter and with a job-level draft condition are refused (R6) | `test_merge_queue.py` |
| A6 | a batch whose members each add a fragment passes `fragment` (R7) | `test_merge_queue.py` |
| A8 | a queue batch that changes another path needs a fragment (R8) | `test_merge_queue.py` |
| A9 | only the queue login on a queue branch takes the batch arm; a person on a queue branch, the queue login on another branch, and logins and branches that only resemble them are refused (R8) | `test_merge_queue.py` |
| A10 | the queue file is the only Mergify configuration and names one queue (R1) | `test_merge_queue.py` |
| A11 | every key the queue file uses is one the published schema lists (R5) | `test_merge_queue.py` |

The seventh criterion, named with the others in the block below, decides that a queue batch of
dependency updates alone needs no fragment (R8).

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_queue_tests_six_together_and_merges_each_by_a_merge_commit
A2: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_pull_request_enters_the_queue_only_into_dev_labelled_ready_resolved_and_green
A3: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_batch_merges_only_after_ci_fragment_and_dependency_review_pass_on_it
A4: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_queue_requires_every_check_and_rule_the_dev_ruleset_file_requires
A5: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_three_required_workflows_run_on_a_queue_batch_pull_request_into_dev
A6: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_queue_batch_whose_members_add_fragments_passes_the_fragment_check
A7: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_queue_batch_of_dependency_updates_alone_needs_no_fragment
A8: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_a_queue_batch_that_changes_another_path_needs_a_fragment
A9: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_only_the_queue_login_on_a_queue_branch_takes_the_batch_arm
A10: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_the_queue_file_is_the_only_mergify_configuration_and_names_one_queue
A11: python3 -m unittest discover -s scripts/tests -p test_merge_queue.py -k test_every_key_the_queue_file_uses_is_one_the_published_schema_lists
```

## 4. File manifest

| path | change |
|---|---|
| `docs/specs/SPEC-604-pull-requests-into-dev-merge-through-the-mergify-merge-queue-six-to-a-batch-each-by-a-merge-commit.md` | new (C1) |
| `docs/schematics/merge-queue.md` | new (C1) |
| `docs/decisions/ADR-625-dev-merges-through-the-mergify-merge-queue-chosen-over-hand-built-union-branches-and-the-github-merge-queue.md` | new (C1) |
| `changelog.d/ci-merge-queue-604.md` | new (C1) |
| `scripts/tests/test_merge_queue.py` | new (C2): A1 to A11 |
| `docs/red-first/SPEC-604.md` | new (C2), appended at C4 |
| `.mergify.yml` | new (C3) |
| `.github/workflows/changelog.yml` | amended insert-only (C3): one contiguous block between cut lines 35 and 36, `git diff --numstat` deleted count `0` |
| `scripts/mutation-rows.d/S60400-S60499.json` | new (C3) |
| `scripts/tests/test_ci_workflows.py` | ONLY IF CI's python.log at push 1 shows a census there refusing the new module by name: insert-only, the census line alone, at a line no open pull request's hunk holds (measured by hunk header); otherwise untouched |
| `docs/decisions/ADR-689-pull-requests-into-dev-also-enter-the-merge-queue-on-a-ready-status-a-dedicated-github-app-posts.md` | new (the revision named in the header) |
| `changelog.d/ci-merge-queue-ready-status.md` | new (the revision named in the header) |

## 5. What this does NOT cover

- The `queue` label itself: it is made outside the tree after this change merges, and nothing here
  creates, edits or applies a label.
- The dev ruleset file and its test: `.github/rulesets/dev.json` and `scripts/tests/test_rulesets.py`
  are not edited here (R9).
- Merges into main: the release pull request from dev into main is merged as before, and the queue
  acts only on pull requests into dev.
- The GitHub App and its key: the App is made and installed outside the tree, and nothing here
  creates, installs, holds or reads it or its key.

## 6. Risks

- **The queue acts only from the default branch.** Mergify reads its configuration from the
  repository's default branch, not from the base of the pull request, so this file takes effect only
  once it reaches that branch through a release. Until then the queue holds no rule, and a pull
  request labelled `queue` waits. Detected by the queue's own summary check on a labelled pull
  request.
- **A batch's `ci` run is a full run.** A batch pull request runs every `ci` job over the union of
  its members, so one batch costs about one full run, not one per member. A red batch is split and
  retried by the queue, which costs further runs; detected in the run list of the batch branch.
- **A batch that outruns its timeout.** A batch whose checks are still pending at `checks_timeout`
  is dequeued with the reason `checks-timeout`; detected in the queue's own summary check.
- **A dependency-only batch.** The batch arm trusts the queue's own login and branch prefix; a
  person cannot open a pull request as `mergify[bot]`, and A9 refuses every near miss of either.
- **A ruleset that drifts from the queue.** A check added to the ruleset file without the queue
  reds A4 before it can merge.
