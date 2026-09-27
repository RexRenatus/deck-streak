---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A leak is remediated by a fresh repository, and the scrub reads every blob

## Context and Problem Statement

The public-readiness review of the first private repository found two private values the scrub had
passed. Compiled Python caches embedded a host path, and a reference client that only history still
held named a private secret. The scrub read the tree's text files, so neither a binary nor a deleted
file reached it. The repository was still private, but GitHub keeps a ref for every pull request
(`refs/pull/<n>/head`). Three Dependabot pull requests pinned the old commits, and no user can delete
those refs. The questions: how is a leak remediated so that nothing reachable in the published
repository holds it, and what does the gate read so the same class cannot pass again (SPEC-033)?

## Decision Drivers

- Nothing that becomes public may hold a private value, including history and pull-request refs.
- Nothing is deleted without the owner: the leaked repository must stay recoverable and private.
- The gate must judge what a push publishes, not what one checkout happens to hold.
- A check that examined less than it claims must read VOID, never green.

## Considered Options (the alternatives it was chosen against)

- Rewrite, archive and re-seed: chosen, because it is the only option here that leaves no reachable
  copy in the published repository, keeps the leaked one recoverable, and needs no one outside the
  project. Every commit is rewritten to drop the leaked paths, the leaked repository is renamed to a
  private archive, and a fresh repository is seeded under the same name.
- Force-push the rewritten history to the same repository. Rejected, because GitHub keeps
  `refs/pull/<n>/head` for every pull request, so the old commits would stay fetchable once the
  repository is public.
- A purge by GitHub support: rejected as the remedy, because it is slow and external, and nobody
  here can verify it before the flip. It would remove the cached views and the pull-request refs,
  and it stays open to the owner for the archive.
- Delete the leaked repository and create it again. Rejected, because deletion is irreversible and
  is the owner's decision. Renaming reaches the same public outcome and stays reversible.
- Keep the tree scan and add a scan of each commit's diff. Rejected, because `git show` prints
  `Binary files differ` for a binary, which is exactly how the compiled caches passed.
- Scan every object in the object store (`git cat-file --batch-all-objects`). Rejected, because it
  also reads unreachable local objects that are never published, so the local verdict would depend
  on the checkout's past rather than on what a push publishes.

## Decision Outcome

Chosen option, in two parts.

**The remedy.** Every commit was rewritten with `git filter-branch` in an isolated clone. The
rewrite dropped `*.pyc`, `.packs/scripts/proxy-client-scan.py` and
`.packs/skills/packs/subscription-proxy/`. It made each rewritten commit's `.packs/VENDORED.json`
agree with its tree, and it remapped the short commit ids that the red-first records and the
schematics cite. The authors, dates, messages and graph shape were verified equal. The leaked
repository was renamed to a private archive. A fresh private repository then received the issues
first, so their numbers are unchanged, then the one seed push of `main` and `dev`, and then the
rulesets at once.

**The gate.** `scripts/public-scrub.py --history` reads each blob reachable from `HEAD` once:
- it refuses a binary (a NUL byte in the first 8000 bytes, or bytes that are not UTF-8) and a blob
  over the size limit;
- it searches the private literals inside binaries as well;
- it reads VOID on a shallow repository.

The gate's scrub stage runs it. CI checks out the full history (`fetch-depth: 0`), and the
maintainer's box adds the private list on every verification.

### Consequences

- Good, because a leak in a deleted file or in a binary is refused before it is pushed, in CI and on
  the box.
- Good, because the committed rulesets are tested against the enforced dev-to-main workflow.
- Bad, because every binary the project will need, such as icons and social cards, now needs its own
  allow-list decision.
- Bad, because the scan's cost grows with the history. The stage's seconds are printed on every run.

### Confirmation

- SPEC-033's acceptance criteria A1 to A13.
- The public-readiness report's blob-level scan of a fresh mirror clone, with the private list.

## What would make this wrong

- GitHub lets a repository's owner delete pull-request refs. A force-push would then be enough, and
  a fresh repository would be unnecessary.
- The project needs committed binaries often. An allow-list with a metadata check would then become
  the rule rather than the exception.

## More Information

SPEC-033; ADR-004 (vendored probes and the box-run packs); ADR-017 (the branch model and hosted CI);
the persona-core and privacy-gdpr packs, whose deny shapes the scrub composes.
