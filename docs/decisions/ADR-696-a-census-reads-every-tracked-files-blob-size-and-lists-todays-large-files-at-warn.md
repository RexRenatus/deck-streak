---
status: "accepted"
---

# ADR-696: a census reads every tracked file's blob size and lists today's large files at warn

Decides SPEC-673. Path:
`docs/decisions/ADR-696-a-census-reads-every-tracked-files-blob-size-and-lists-todays-large-files-at-warn.md`.
Amends nothing. Cites ADR-450 D5 (a census's level and listing) and D6 (its kill criteria and
promotion rule), SPEC-436 R21 to R24 (the `WARN` and `STALE` line forms), ADR-359 (the review
screens' media caps) and ADR-433 (a file's bytes rejected as a proxy for its mutant count). Read at
dev `b9a08e60`.

## Context

- **No check reads a tracked file's size.** No test, census or CI job reads one, and no SPEC or
  ADR states a size rule for one; ADR-359's caps bound media a review screen sends while it runs,
  and ADR-433 speaks of bytes only as a proxy it rejects (SPEC-673 section 1).
- **The forge's own limits sit far above any bound a review needs.** The forge warns on a push
  that holds a file past 50 MiB, refuses one past 100 MiB, and takes at most 25 MiB through its
  web upload; a file kept in its large-file storage is tracked as a small pointer file. None of
  these reads a pull request's growth of a file.
- **git's own large-file rules refuse no size.** The `binary` attribute turns off diff, merge and
  text conversion for a path, and `core.bigFileThreshold` only stops git delta-compressing a large
  blob. The root `.gitattributes` marks images and fonts `binary`, and routes no path to large-file
  storage.
- **Today's large files are parity goldens.** Five files read are past 1 MiB, each a golden of
  the second oracle that its driver writes and a Rust test reads; none read is past 2 MiB.
- **A checkout reads every blob's size cheaply.** `git ls-tree -r -l` prints each blob's size in
  bytes beside its path, and `-` for an entry that is not a blob; its `--format` prints the same
  fields with no object name, and `-z` prints each path verbatim.

## Decision Drivers

- No rule already in force is lowered, and no tracked file changes.
- A new rule starts at warn and is promoted to fail only by its own delivery, under ADR-450 D6;
  demoting it once at fail is a weakening.
- Every check prints what it examined and fails on an empty population, which is VOID, never a
  pass.
- A check that reads the tree keeps every tracked file in its population.
- Today's sites are listed with their reasons, not cured.

## Decisions

**D1. One census reads every tracked file's blob size at HEAD (SPEC-673 R1, R3).** The census in
`scripts/tests/test_large_files.py` reads `git ls-tree -r -z --full-tree` at HEAD in a format of
mode, type, blob size and path, so it opens no tracked file and every tracked file is in its
population whatever its name. It runs in the python stage beside the other guard tests, so the
`hygiene` job of CI runs it at every push; in a pull request that job's HEAD is the merge into the
base.

**D2. The bound is 1 MiB, strictly past (SPEC-673 R2).** Every file read but the five goldens sits
at or under 1 MiB, so the bound lists five sites, each kept on purpose, and watches each one's
growth.

**D3. A listing records each site's size and its reason (SPEC-673 R4, R5, R8).** A site is a
file past the bound with no entry, or a listed file grown past the size its entry records. A
listed file at or under its recorded size is not a site, so a golden regenerated at the same size
passes and one that grows says so.

**D4. A stale entry fails at both levels, and no arm follows a rename (SPEC-673 R6).** An entry
whose path holds no tracked file, or whose file is back at or under the bound, names no site and
fails, so the listing never outlives its sites. A renamed or moved listed file therefore leaves a
stale entry and an unlisted site, and the pull request that moves it moves its entry.

**D5. An entry with no blob size is undecided (SPEC-673 R7).** A submodule's commit has no size
in the tree; it prints `UNDECIDED` and passes at warn, and fails at fail.

**D6. The level, the void and the promotion (SPEC-673 R9, R10).** `LEVEL` is `warn`: a site and
an undecided entry print and pass, and a stale entry fails. An empty population is VOID. Moving
`LEVEL` to `fail` is its own delivery, by ADR-450 D6's rule; ADR-450 D6's kill criteria apply as
they stand.

**D7. Not in this delivery (SPEC-673 R12).** No tracked file is removed, shrunk, split or moved to
large-file storage, and no `.gitattributes`, workflow, script, production file or existing test
changes.

**D8. Tests and rows (SPEC-673 sections 3 and 9).** One test per acceptance criterion, red first
over a stub that finds no site; the planted listings are text, and no planted file is large. The
hand rows on the census's arms are killed by the planted tests.

## What it was chosen against

### D1

- The pull request's changed files alone (a diff against its base): it needs the base in every
  run, and it misses a site that lands by any other route; the whole tree at HEAD needs neither.
- One `git cat-file -s` per path: one process per tracked file, for the same sizes.
- The forge's file list for the pull request: a network call inside a test.

### D2

- 512 KiB: twelve sites today, most of them generated, so the listing grows with little gain.
- 2 MiB: no site today, so no golden's growth is watched until one passes it.
- 25 MiB or 50 MiB, the forge's own limits: no site, and nothing a review would catch first.

### D3

- A listing by path alone, with no size: a listed file could grow without bound and pass.

### D4

- Following a rename by blob or by `git diff -M` against the base: it needs the base or the old
  blob in every run, and a rename that also grows the file is still a judgement no size makes.

### D5

- Refusing a submodule's commit at both levels: the tree holds none today, and a refusal at warn
  would make the census fail where it only reports.

## Consequences

- A pull request that adds a file past 1 MiB, or grows a listed golden, prints a `WARN` line in
  the `hygiene` job at warn, and fails it once the level is `fail`.
- A pull request that regenerates a golden past its recorded size records the new size in
  `LISTED` with it.
- A pull request that removes, moves or shrinks a listed golden removes or moves its entry, or the
  census fails.

## Confirmation

- The planted tests run by name anywhere; the census over the repository is read in the `hygiene`
  job of CI.
- The rows of SPEC-673 section 9 are proved by `python3 scripts/mutation_rows.py prove --row <id>`.

## What would make this wrong

- A tracked file that must exceed 1 MiB and change often, such that each pull request records a
  new size: then the bound or the listing's form is revisited in its own delivery.
- A forge or checkout that does not hold every blob at HEAD, such as a partial clone: then the
  sizes are not the tree's, and the census must say so before any promotion.

## More Information

Issue #1393; issue #913; SPEC-436; ADR-450; ADR-359; ADR-433; SPEC-498; SPEC-029.
