---
status: "accepted"
---

# ADR-698: commit subjects and trailers are a census of each run's range that starts at warn

Decides SPEC-675. Path:
`docs/decisions/ADR-698-commit-subjects-and-trailers-are-a-census-of-each-runs-range-that-starts-at-warn.md`.
Amends nothing. Cites ADR-450 D6 (the promotion rule and the kill criteria, which this census
takes as its own) and SPEC-436 R21 (the `WARN` and `STALE` line forms). Read at dev `b9a08e60`.

## Context

- **The rule is asked for and never checked.** The pull request template's checklist asks for a
  Conventional Commit title, the contributor guides ask for `type(scope): description` and no
  attribution trailers, and the builder brief bounds a subject at 72 characters. No workflow step
  and no test reads a commit message.
- **The checkout already holds the history.** The `hygiene` job checks out with the whole history,
  and on a pull request its checkout is the merge ref, whose first parent is the base's tip: the
  commits between them are the pull request's own. On a push the pushed tip's first parent is the
  branch's previous tip when one merge arrives.
- **Tools write subjects of their own.** GitHub writes `Merge pull request #<n> from <branch>` on
  each merge into dev and the merge ref's own subject on each pull request run; git writes
  `Merge branch '<branch>'` when a branch takes dev in, and `Revert "<subject>"` with a
  `This reverts commit <id>.` line. `scripts/mutation-verdict.py` reads GitHub's merge subject to
  name the merged pull request.
- **Git reads a trailer block by its own rule.** The block is the message's last paragraph after a
  blank line, all trailers, or at least a quarter trailers with a line git writes by itself. A
  pull request title with no scope alone in a merge body's last paragraph is such a block.
- **Measured at dev** (SPEC-675 section 1): no site under the arms decided here; 5 false hits for
  a refusal of every trailer key, and 32 merge subjects over 72 characters, all GitHub's.

## Decision Drivers

- No rule already in force is lowered, and no commit is rewritten.
- A new rule starts at warn and is promoted to fail after three iterations of the rule review at
  precision of at least 0.8 with at least five hits (ADR-450's drivers); demoting a fail rule is a
  signed ruling.
- Every check prints what it examined and fails on an empty population, which is VOID.
- A form is left out of the population only on a measurement; without one it stays in.

## Decisions

**D1. The population is the run's range (SPEC-675 R1 to R3).** The census reads every commit
`HEAD^1..HEAD` lists in the `hygiene` checkout: on a pull request, the merge ref's commit and every
commit of the pull request; on a push, the pushed tip and what its first parent lacks. A bot's
commit and a merge commit stay in it; a shallow checkout or a head with no first parent is VOID.
- **False negatives:** a push that carries several merges is read for its last; each earlier
  merge's commits were read by their own pull request's run.

**D2. A judged subject is a Conventional Commit of the house's types, at most 72 characters
(SPEC-675 R4 to R6).** The types are the specification's two, the conventional configuration's
other nine, and `release`, the release pull request's title; case is not read.
- **False positives:** a subject of another word that a reader would accept, such as `deps:`.
- **Not a hit, correctly:** a squash subject that ends with its pull request's number is judged as
  any subject and is clean when its title is.

**D3. Only attribution trailers are refused (SPEC-675 R9, R10).** The block is read by git's rule;
`Co-authored-by` and `Signed-off-by` are sites on every commit, and every other key is counted on
an `INFO` line.
- **Why not every key:** the five measured hits of an every-key rule are all false, so its first
  five hits would propose its demotion under ADR-450 D6 at once.
- **False negatives:** credit given in a body line that is no trailer.

**D4. Subjects a tool wrote are counted, not judged (SPEC-675 R7, R8).** GitHub's and git's merge
subjects on a commit of two parents or more, and git's revert and reapply subjects beside their
body line. Any other subject on a merge is judged.
- **False negatives:** a person who keeps git's default merge or revert subject.

**D5. The listing and the level (SPEC-675 R11 to R14).** `LISTED` holds the sites of commits
reachable from dev's tip, each with its reason, and is empty at the cut; each listed entry's commit
is read by its id, so an entry goes stale only when its commit no longer holds the site, never by
leaving a range. `LEVEL` is `warn`.

**D6. Kill criteria.** ADR-450 D6's promotion rule and its criteria hold for this census as its
own: live precision below 0.8 with at least 5 hits, or 2 or more of the rule's first 5 hits false,
proposes a demotion; a demotion is a signed ruling; `examined 0` is VOID, which blocks.

**D7. Not in this delivery.** No pull request title check, no workflow change, no commit rewritten,
no commit-lint tool and no dependency.

**D8. Tests and rows.** One test per criterion, red first: the red commit holds the tests over
stubs that keep every input but the missing behaviour (the reader reads nothing, each arm finds no
site, the verdict passes everything), so each red is an assertion on the missing behaviour over a
planted fixture or the run's own range. Rows S67500 to S67527 are hand-written over the census's
arms, each killed by a planted test.

**D9. Its own numbers.** ADR-450 D9 keeps that delivery's three rules in SPEC-436 and ADR-450,
so this census takes new numbers and amends neither.

## What it was chosen against

- **A commit-lint tool:** a dependency and a second rule source, where the arms are a few lines.
- **A range from the event's base and head:** a push's previous tip can be a commit the checkout
  lacks after a forced update; the first parent is always held.
- **Refusing every trailer key:** measured false at every hit (D3).
- **Listing the open pull requests' sites:** a closed or rebased pull request's commits leave every
  other checkout, so each such entry would read stale in every run.

## Consequences

- Each pull request run prints its own commits' sites at warn; the release pull request's run reads
  dev's unreleased history.
- Promotion to fail is its own delivery and needs the listing of dev's sites first.

## Confirmation

The planted tests decide each arm; in CI the `hygiene` job reads the run's range by name, and the
red-first record names the job that first measured it.

## What would make this wrong

- A tool that writes a subject outside D4's forms on every merge: its hits print at warn, and the
  rule review reads them before any promotion.
- A run whose checkout loses its history: the census reads VOID and blocks.
