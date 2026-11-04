# ADR-625: dev merges through the Mergify merge queue, chosen over hand-built union branches and GitHub's own merge queue

- **Status:** accepted with SPEC-604.
- **Decides:** SPEC-604.

## Context

Pull requests into dev merge one at a time, each after its own run of `ci`, `fragment` and
`dependency-review`, the three checks `.github/rulesets/dev.json` requires. When several pull
requests are ready together, each waits for the one before it, and each merged pull request was
tested against a dev that the next merge changes. A merge queue tests ready pull requests together,
on the code it is about to merge, and merges them only when that code is green.

Mergify's documentation sets the shape this decision works within:

- Mergify reads its configuration from the repository's default branch, from the first of six
  file names it finds, and refuses a pull request that would leave two of them in place
  (Configuration File Format).
- A queue rule holds `queue_conditions` (when a pull request may enter), `merge_conditions` (when a
  batch may merge), `batch_size` and `merge_method` (Merge Queue Rules, Merge Strategies).
- With a `batch_size` above 1 the queue tests a batch on a draft pull request from a branch named
  by `queue_branch_prefix`, `mergify/merge-queue/` by default, followed by ten hexadecimal
  characters, opened by `mergify[bot]` unless `draft_bot_account` is set (Draft Pull Requests).
- A pull request enters the queue by the `queue` action of a `pull_request_rules` entry, by a
  command, or by `auto_merge_conditions` under `merge_protections_settings`, which works only once
  Merge Protections is turned on in Mergify's dashboard and then posts its own check and a summary
  comment on every pull request (Queue action, Auto-Merge, Merge Protections setup).
- A ruleset on the queued branch lists Mergify as a bypass actor in the `exempt` mode, and Mergify
  applies the ruleset's required checks and reviews as merge conditions (GitHub Rulesets).

## Decision

Pull requests into dev merge through the Mergify merge queue, configured in one file at the root,
`.mergify.yml`:

- one queue rule, `batch_size: 6` and `merge_method: merge`, so each member keeps its own commits
  and merges by a merge commit, as a pull request merged by hand does;
- its `queue_conditions` admit a pull request only when its base is dev, it carries the label
  `queue`, it is not a draft, it has no unresolved review thread, and `ci`, `fragment` and
  `dependency-review` each succeeded on it;
- its `merge_conditions` require the same three checks on the batch pull request;
- one `pull_request_rules` entry with the `queue` action puts a labelled pull request into dev in
  the queue. This form needs nothing outside the file, posts nothing of its own, and its conditions
  are read beside the queue's, so the label alone never admits a pull request the queue conditions
  refuse.

The `fragment` check gains one arm: a pull request opened by exactly `mergify[bot]` from a head ref
that starts with exactly `mergify/merge-queue/` is read by the dependency-file rule the Dependabot
arm already applies. A queue batch of dependency updates alone adds no fragment and is not opened by
Dependabot, so without the arm it could never merge; every member of a batch already passed
`fragment` on its own pull request.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| Batches assembled by hand as union branches of several pull requests | each union is built, tested and merged by hand, so a batch costs a person's time per merge, and a member that changes after the union was cut is merged untested unless the union is rebuilt |
| GitHub's own merge queue | GitHub offers it only to repositories owned by an organization, so a repository owned by a personal account cannot turn it on ("Managing a merge queue", GitHub Docs) |
| `auto_merge_conditions` under `merge_protections_settings` in place of the `queue` action | it acts only once Merge Protections is turned on in Mergify's dashboard, outside the tree, and then posts a check and a summary comment on every pull request, which R1 rules out |

## Consequences

- A labelled, green pull request into dev waits for the queue rather than a person. The queue keeps
  Mergify's documented defaults for how many batches it checks at once, `max_parallel_checks`
  (default 5), and how long it waits to fill a batch, `batch_max_wait_time` (default thirty seconds, as Mergify's configuration schema
  states it).
- Each batch runs `ci` in full over the union of its members, so a green batch of six costs one
  full run instead of six, and a red batch costs further runs while the queue splits it to find
  the failing member.
- The file takes effect only once it reaches the default branch; until then a pull request labelled
  `queue` waits.
- The committed ruleset file is not edited here, so the queue's conditions are held equal to it by
  a test: a check added to the ruleset file without the queue turns that test red.

## What would make this wrong

- A batch run that is red while each member is green on its own, more often than the merges it
  saves, would make one-at-a-time merging cheaper.
- A Mergify change that opens queue pull requests under another login or branch prefix would leave
  dependency-only batches refused by `fragment` until the arm follows it.
