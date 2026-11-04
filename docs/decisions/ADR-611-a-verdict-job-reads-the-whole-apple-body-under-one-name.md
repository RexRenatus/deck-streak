---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-611: a `verdict` job reads the whole Apple body under one name and fails unless every job succeeded

Decides SPEC-591 (R8).

## Context and Problem Statement

After the split (ADR-610), a reader that reads only `harness` would miss a red `release-and-app`, and a reader that
reads both by name must carry the body's job list. The body needs one name that is green only when every one of
its jobs succeeded, while `harness` stays the name the planted suite's evidence is read from.

## Decision Drivers

- A cancelled job is never green, and a skipped job must not read as success.
- `harness` keeps its referent: the job that holds the planted suite.
- No context joins a required set; the Apple build stays advisory (ADR-355).
- The verdict fetches nothing, so it cannot hang on a download, and a test can pair each need with its entry.

## Considered Options (the alternatives it was chosen against)

- **Chosen: a job `verdict` that needs every other job of the body, runs under `if: ${{ always() }}` on the
  admitted runner with `timeout-minutes: 10`, reads each need's result through one environment entry per need
  and exits non-zero unless every result is `success`.**
- A verdict under `if: ${{ !cancelled() }}`: rejected because a cancelled run then skips it, and a skipped job does
  not read as a failure.
- A verdict that fails only on `failure`: rejected because a cancelled or skipped need would pass it.
- One entry holding the whole needs context as text: rejected because the step then needs a parser, and a test can
  no longer pair each need with the entry that carries its result.
- No verdict, with every reader reading every job by name: rejected because each reader then carries the job list,
  and a job added later is missed by any reader not updated with it.
- A verdict named `harness`: rejected because it would move where the planted suite's evidence is read.
- Making `verdict` a required context: rejected because the Apple build is advisory (ADR-355), and a caller filtered
  by paths reports no check on a pull request that does not touch them.

## Decision Outcome

Chosen: the `verdict` job under `always()` that requires every result to equal `success`, because it reports on
every outcome, fails on a failed, cancelled or skipped need, keeps `harness` as the evidence's referent and adds
no required context:

- its needs are `xcframework`, `harness-wire`, `card-isolation`, `harness` and `release-and-app`, every other job of
  the body, and a test holds the list equal to the file's jobs;
- each result reaches the script through an environment entry, never through an expression inside `run:`;
- its bound, `timeout-minutes: 10`, ends a hung step; it reads a context and runs a few shell lines.

### Consequences

- Good: one check reads the whole body, and a job added later fails the census until the verdict needs it.
- Good: re-running any body job re-runs the verdict, which needs it.
- Bad: the verdict waits for an admitted runner like every other job of the body.
- Neutral: a cancelled run's verdict reads red, as intended.

### Confirmation

SPEC-591 A4, red at the base where there is no `verdict` job, with rows S59100 to S59104.

## More Information

- ADR-355 (one body, advisory), ADR-610 (the split it reads), SPEC-591 R8.
- What would make this wrong: a job whose skip is intended (a job filtered by its own condition), which would then
  need its own reading rule; or a reader that needs the verdict as a required context, which is its own decision.
