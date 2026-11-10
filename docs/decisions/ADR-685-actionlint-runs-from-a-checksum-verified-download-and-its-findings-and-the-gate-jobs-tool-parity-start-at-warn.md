---
status: "accepted"
---

# ADR-685: actionlint runs from a checksum-verified download, and its findings and the gate jobs' tool parity start at warn

Decides SPEC-663. Cites ADR-450 D6 (promotion and kill criteria) and SPEC-428 (the workflow lint
job and its pins).

## Context

The workflows are linted by zizmor alone, and nothing compares the tools the gate jobs install.
Today's actionlint findings are not known until a run measures them.

## Decision

**D1.** actionlint joins the `workflow-lint` job at one pinned release, downloaded and checked by
its digest, run once with its external linters off, its output and exit saved.
**D2.** A census in `scripts/tests/test_workflow_lint.py` reads the saved run and the gate jobs'
installs at warn; an empty population or an unread run is VOID, which fails.
**D3.** Today's findings are listed from the delivery's first run, each with its reason and that
run's id; today's parity sites are listed the same way.
**D4.** Promotion to fail follows ADR-450 D6, by its own delivery; demotion is a weakening.

## What it was chosen AGAINST

- The install action's `tool` input: rejected here because the request names a checked download.
- A configuration file or `-ignore` patterns: rejected because they hide a finding where the listing names it with its reason.
- Running shellcheck and pyflakes inside actionlint: rejected because their versions move with the runner image.
- A new CI job: rejected because the aggregate job's needs are pinned and `workflow-lint` already holds the workflow lint.

## Consequences

The first run prints today's findings at warn; the listing commit names them. Each later finding
prints until it is cured or listed.

## What would make this wrong

A finding class that is false at least two times in its first five hits proposes a demotion under
ADR-450 D6; a census that cannot read actionlint's output reads VOID and blocks.
