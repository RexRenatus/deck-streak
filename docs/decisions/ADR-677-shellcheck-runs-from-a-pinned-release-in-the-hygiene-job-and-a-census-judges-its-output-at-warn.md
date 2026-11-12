---
status: "accepted"
---

# ADR-677: ShellCheck runs from a pinned release in the hygiene job, and a census judges its saved output at warn

Decides SPEC-655. Path:
`docs/decisions/ADR-677-shellcheck-runs-from-a-pinned-release-in-the-hygiene-job-and-a-census-judges-its-output-at-warn.md`.
Amends nothing. Cites SPEC-038 R12 (a tool from its release archive against a pinned digest),
SPEC-190 R12 (one loader reads every workflow) and ADR-450 D6 (the kill criteria, the promotion
rule and VOID tooling).

## Context

No job runs ShellCheck over the repository's shell scripts, and nothing lists which tracked files
are scripts: some carry a `.sh` extension and some are test doubles with no extension and a shell
`#!` line. The hygiene job installs `gitleaks` and `protoc` from release archives checked against a
full digest, then runs the python stage, which runs every guard test under `scripts/tests` and
keeps its logs in the job's uploaded log folder.

## Decision Drivers

- A new rule starts at warn and is promoted by its own delivery (ADR-450's Decision Drivers and
  D6), so today's findings never block the pull request that adds the check.
- A tool is downloaded only from its release archive against a pinned digest, never from a package
  index or an unpinned installer (SPEC-038 R12).
- A census counts what it examined and refuses an empty population; a missing tool, or one off its
  pin, is VOID, which blocks, never a pass (ADR-450 D6).
- No new job: the `ci` job's needs list stays as it is.

## Decisions

**D1. One step in the hygiene job.** Before the python stage, one step downloads the pinned
ShellCheck archive (release 0.11.0, the `linux.x86_64` archive), checks its digest by
`sha256sum -c`, lists the population and runs the tool with `--norc --format=json1
--severity=style`, saving the list, the version, the exit status and the output under the job's log
folder. The step fails on the download or the digest only.

**D2. One population rule, applied twice.** SPEC-655 R1 is spelt in the step's shell and in the
census's Python, and the census refuses any difference by name (`DRIFT`), so neither can drop a
script alone.

**D3. The census judges; the tool does not.** The census reads the saved output, refuses it as
VOID when it is missing, off the pin or contradicted by its exit status, and compares each finding
with `LISTED` by path and check code, with a count and a reason per entry, so moving a line within a
script changes nothing. At warn an unlisted finding prints a `WARN` line; a listed entry no longer
found fails as `STALE` at both levels, as does an entry with an empty reason (`UNREASONED`).

**D4. Kill criteria.** ADR-450 D6 applies as written: `LEVEL` moves to `fail` after three rule
reviews at precision of at least 0.8 with at least five hits, by its own delivery; demotion is a
signed ruling; live precision is fixes over fixes plus new listing entries or new `# shellcheck
disable=` directives across the last 30 merged pull requests; `examined 0`, or a tool missing or
off its pin, is VOID.

**D5. Licence.** ShellCheck is GPL-3 and runs only as a CI tool over the tree; it is neither linked
nor shipped, so `deny.toml`'s allow-list, which governs the Cargo graph, does not apply to it.

## What it was chosen against

- **A new job.** It would change the `ci` job's needs list, and the python stage that reads the
  output runs in the hygiene job.
- **ShellCheck at its own exit status.** Today's findings would block this pull request.
- **A key with the line number.** Every edit above a finding would churn the listing.
- **Reading the workflow from the census.** SPEC-190 R12 keeps workflow reads in one loader; the
  pin of D1 lives in `test_ci_workflows.py`.

## Consequences

- Each pull request's hygiene job checks every tracked shell script, and its uploaded logs hold the
  output.
- The repository test reads VOID outside the hygiene job, where no output is saved.
- A change of release changes the step, its digest, `VERSION` and the pin together.

## Confirmation

SPEC-655 A1 to A6, and the rows of SPEC-655 section 6.

## What would make this wrong

- The python stage running before the step, or in another job: A6 refuses it.
- A release whose `json1` shape differs: the census fails reading it, never passes.
