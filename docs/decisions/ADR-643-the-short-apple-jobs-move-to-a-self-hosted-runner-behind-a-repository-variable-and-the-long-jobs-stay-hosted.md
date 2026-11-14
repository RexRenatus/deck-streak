# ADR-643: the short Apple jobs move to a self-hosted runner behind a repository variable, and the long jobs stay hosted

- **Status:** accepted with SPEC-621.
- **Decides:** SPEC-621.
- **Amends:** ADR-345 D4 (the macOS runner admitted to `xcframework.yml` by file name) and, for
  three jobs of that file alone, ADR-017's rule that no job of this public repository runs on a
  self-hosted runner.

## Context

Every job of `xcframework.yml`, the one Apple job body, runs on the hosted runner `xcode-27` and
waits for one to be free. Three of its jobs are short: `xcframework` builds the framework,
`harness-wire` runs the codec's tests on the host and `card-isolation` runs the card view's
package tests and its Swift mutant sweep. Two are long: `harness` and `release-and-app`, the halves
of the harness work SPEC-591 split, each boot an iPhone and an iPad simulator, and their second
simulator pass is not yet green on the self-hosted runner. `verdict`, the job SPEC-591 added, reads
the result of every other job.

GitHub's documentation sets the shape this decision works within:

- `runs-on` may be an expression, and `fromJSON` of a string literal gives it a list of labels; a
  job waits for a runner that carries every label in the list (Choosing the runner for a job).
- An unset configuration variable reads as an empty string, and `==` compares strings without
  case (Variables reference, Expressions).
- A called workflow reads its caller's `github` context, so its event, actor and pull request are
  the caller's, and the variables it reads are the caller repository's (Reusing workflow
  configurations, Variables reference).
- A self-hosted runner in a public repository runs whatever a pull request's workflow file says,
  so a fork's pull request must never reach it (Self-hosted runner security).

## Decision

- The `runs-on` of `xcframework`, `harness-wire` and `card-isolation` becomes one expression: the
  labels `self-hosted`, `macOS`, `ARM64` and `ds-mini` when the repository variable `DS_MINI` is
  `on`, the actor is not `dependabot[bot]`, and the event is a push, a `workflow_dispatch` or a
  pull request whose head repository is this repository; `xcode-27` otherwise.
- **The variable gate.** Unset, or any value but `on`, keeps every job where it runs today, so the
  change lands with no effect and is turned on, or off again, outside the tree with no commit.
- **The actor clause.** A Dependabot pull request comes from this repository's own branches, so
  the same-repository clause alone would route it; the actor clause keeps it hosted.
- **The same-repository clause.** A fork's pull request, and one whose head repository is gone,
  reads a head repository other than this one and runs hosted.
- **The three short jobs only.** `harness`, `release-and-app` and `verdict` keep
  `runs-on: xcode-27` byte for byte on every event.
- The workflow hardening test admits the expression, and it alone, to those three jobs by file
  and job, and keeps `xcode-27` the file's runner for every other job; a test evaluates the
  expression over every event, variable value and actor, and refuses planted copies that drop a
  clause or route another job.

## Relation to SPEC-591's split and to the earlier form

SPEC-591 split the harness work into `harness` and `release-and-app` and added `verdict`, each
inside its own bound and each on the admitted runner. This decision keeps all three on that runner:
the split's bounds were measured there, and its two simulator passes are the work that is not yet
green on the self-hosted runner. An earlier form of this change routed the jobs on a push and a
dispatch alone, `harness` among them, with no variable and no actor clause; it predates the split,
so it named neither `release-and-app` nor `verdict`, and it is replaced by this one.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| Every Apple job on the self-hosted runner | the two long jobs' second simulator pass is not yet green there, so every Apple run would read red |
| The short jobs on a push and a dispatch alone | this repository's own pull requests, the most frequent Apple runs, would still wait for a hosted runner |
| Every pull request on the self-hosted runner | a fork's pull request would run its own workflow text on the self-hosted runner, which ADR-017 rules out |
| The expression with no variable gate | the jobs would move the moment the change lands, before a runner carries the labels, and moving them back would need a commit |

## Consequences

- With `DS_MINI` unset, nothing changes: every job of the file runs on `xcode-27`.
- With `DS_MINI` set to `on`, the three short jobs of a push, a dispatch and this repository's own
  pull request run on the self-hosted labels, and the long jobs and the verdict stay hosted, so a
  run's verdict still waits on the hosted harness.
- A short job queued with no runner carrying all four labels waits and is cancelled after a day;
  unsetting the variable sends the next run back to `xcode-27`.
- The routing is not a boundary by itself, since a pull request can edit the workflow file. The
  boundaries are the repository's setting that only collaborators open pull requests and its rule
  that a run from an outside contributor waits for approval, and both must stay as they are.

## What would make this wrong

- A fork's pull request that reaches the self-hosted runner, which the same-repository clause and
  the approval rule together are meant to prevent.
- Short jobs that wait longer on the self-hosted runner than on `xcode-27`, which would make the
  move cost more than it saves.

## Amendment: ADR-708 routes the verdict and the long jobs

ADR-708, deciding SPEC-688, amends this decision: `verdict` now takes the short jobs' expression,
and `harness` and `release-and-app` take one expression of their own that reaches the self-hosted
labels only when `DS_MINI` and a second repository variable, `DS_MINI_LONG`, both read `on`, the
actor is not `dependabot[bot]` and the event is a pull request whose head repository is this
repository, `xcode-27` otherwise. The decision that `harness`, `release-and-app` and `verdict` run
on `xcode-27` on every event now reads with ADR-708, under which the upload lanes' and the tag's
runs keep both long jobs hosted. The rest of this decision stands.
