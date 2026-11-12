---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner, through the maintainer), the DeckStreak architect"
---

# Vendoring is a committed script that excludes at copy time and scrubs before it writes

## Context and Problem Statement

DeckStreak vendors the packs repository's pack probes and pins them to one commit (ADR-004). Some upstream
files must never enter this public repository: the subscription-proxy reference client and its
scanner name a private secret, and one SKILL.md names the maintainer's host paths. `VENDORED.json`
records those exclusions, but only in prose, and the vendoring was a script outside the repository
that copied everything first. At the train-84 re-pin it wrote the two secret-bearing files into the
tree as untracked files, and only a person noticing kept them out (#205). How does vendoring keep an
excluded or private file out by construction (SPEC-037)?

## Decision Drivers

- An excluded file must never exist in the tree, tracked or not.
- A private value in an upstream file nobody excluded must stop the run, not wait for review.
- The rules are the public scrub's own, never a copy (CHARTER).
- The tool is reviewed and tested like any other code.

## Considered Options (the alternatives it was chosen against)

- A committed script that excludes at copy time and scrubs before writing: chosen, because an
  excluded path is dropped before it is read, and any other file that carries a private shape stops
  the run before anything is written. The exclusions are machine-readable, and the tree changes all
  at once or not at all.
- A person copies the files and quarantines what should not be there, as at the re-pin: rejected
  because it depends on someone noticing untracked files, which is exactly the near-miss.
- Scrub after writing: rejected because the file already sits in the tree, where a `git add -A` or
  a tarball can carry it before any scan runs.
- A `.gitignore` for the excluded paths: rejected because the file is still written to disk, and
  `git add -f` or any tool that ignores `.gitignore` would still carry it.
- A git subtree or sparse checkout of the packs repository: rejected because it brings the private
  repository's history, or its paths, into view, and still needs the same exclusions applied by
  hand.

## Decision Outcome

Chosen option.
- **The script.** `scripts/vendor-packs.py --source <packs checkout>` reads `VENDORED.json`.
- **Candidates.** It takes the files the manifest lists and any new file of an already vendored
  pack, and drops every candidate that matches an `excluded` entry's `globs` before reading it.
- **The scan.** It scans every remaining file with `scripts/public-scrub.py`'s rules and the private
  list, and refuses the run on any finding or binary.
- **The write.** Only then does it write every file and update `VENDORED.json` and
  `methodology.json` in one step.

### Consequences

- Good, because a secret-bearing upstream file cannot reach the tree, excluded or not.
- Good, because the next re-pin is one command, reviewed through its pull request.
- Bad, because an upstream example that merely looks private stops the run. Excluding it is an
  explicit, reasoned entry in `VENDORED.json`, reviewed in the pull request.

### Confirmation

SPEC-037's A1 to A7, and every future re-pin pull request, which runs this script.

## What would make this wrong

- The open-source pack runner exists, and DeckStreak stops vendoring (#60). This script is then
  retired with the vendored copy.

## More Information

SPEC-037; ADR-004 (vendored probes), ADR-033 (the history scrub); the persona-core and privacy-gdpr
packs, whose shapes the scrub composes.

Amended by ADR-056 (2026-09-28): no pack runner will be published, so vendoring does not end;
this script remains the way the probes are refreshed.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).
