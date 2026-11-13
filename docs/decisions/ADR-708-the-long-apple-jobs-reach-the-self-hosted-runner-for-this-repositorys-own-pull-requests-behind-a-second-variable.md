# ADR-708: the long Apple jobs reach the self-hosted runner for this repository's own pull requests behind a second variable

- **Status:** accepted with SPEC-688.
- **Decides:** SPEC-688.
- **Amends:** ADR-643 (its decision that `harness`, `release-and-app` and `verdict` run on
  `xcode-27` on every event).

## Context

ADR-643 moved the three short Apple jobs of `xcframework.yml` to the self-hosted runner behind the
repository variable `DS_MINI`, and kept `harness`, `release-and-app` and `verdict` on `xcode-27` on
every event. The two long jobs are the file's longest (SPEC-688 section 1). Four callers run the
same file: the pull request lane, the release tag, and the two upload lanes on a push or a
dispatch. A called workflow reads its caller's event, so one expression for every job would move
the upload lanes' and the tag's long jobs onto the self-hosted runner before they were ever tried
there.

This decision keeps ADR-017's rule that no job of this public repository runs on a self-hosted
runner, except, as ADR-643 already admits for its three jobs, the jobs of `xcframework.yml` named
here, each behind its variables; and it keeps ADR-345 D4's admission of `xcode-27` to the file as
the fallback of every job. SPEC-591 R3's and R8's admitted runner for `release-and-app` and
`verdict` now reads as each job's route in SPEC-688.

## Decision

- The `runs-on` of `verdict` becomes the short jobs' expression unchanged. It needs no Apple tool,
  only the results of the jobs it needs, so it follows them.
- The `runs-on` of `harness` and `release-and-app` becomes one new expression: the labels
  `self-hosted`, `macOS`, `ARM64`, `ds-mini` and `ds-mini-sim` when `DS_MINI` and a second repository variable,
  `DS_MINI_LONG`, both read `on`, the actor is not `dependabot[bot]` and the event is a pull
  request whose head repository is this repository; `xcode-27` otherwise.
- `DS_MINI_LONG` is the trial switch, set outside the tree. Until it reads `on`, both long jobs run
  where they run today, and the upload lanes' and the tag's runs keep both long jobs hosted
  whatever the variables read.
- Nothing else of the file changes: its environment lines, steps and comments stay byte for byte,
  so both routes name the same toolchain path, simulator names and simulator OS.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| the short expression for every job | the upload lanes and the tag call the same file, so their long jobs would move onto the self-hosted runner untried |
| the long jobs behind `DS_MINI` alone | turning `DS_MINI` on for the short jobs would move the long jobs too, with no way to try them apart |
| the verdict left on `xcode-27` | a hosted machine would start only to read the other jobs' results while every other job could run on the self-hosted runner |
| the simulator OS routed with the runner | the two routes would then test at different simulator OS versions, so the trial could not compare the runners |

## Consequences

- With both variables off, every job runs where it runs today. With `DS_MINI` on alone, the short
  jobs and the verdict run on the self-hosted runner and the long jobs stay hosted, linking a
  framework the self-hosted runner built.
- The trial is this change's own pull request run with both variables on (SPEC-688 section 7). A
  red simulator pass there blocks no land: `DS_MINI_LONG` stays off, and both long jobs stay hosted
  with no commit.
- The hardening test admits the short expression to four jobs and the long expression to two, each
  by file and job.

## What would make this wrong

- A run of an upload lane or of the tag whose long jobs reach the self-hosted runner, which
  SPEC-688 R3 is meant to prevent.
- Long jobs that take longer on the self-hosted runner, one after the other, than hosted side by
  side, so that the move costs more than it saves.
- A toolchain or simulator runtime that changes under a running job on the self-hosted runner.
