# ADR-695: tracked build output and caches are a path census over every tracked path, which starts at warn

Status: accepted. Decides SPEC-672 (#1391), a follow-up to SPEC-436 and ADR-450 from #913. File:
`docs/decisions/ADR-695-tracked-build-output-and-caches-are-a-path-census-that-starts-at-warn.md`.

## Context

- The public scrub reads every blob's content and refuses a binary by the rule `binary`,
  whatever its path (SPEC-033 R2). A compiled Python cache is refused that way; a text file under a
  build or cache directory, a generated module or a test runner's report, is not.
- An ignore line does not guard a tracked path: git leaves a path it already tracks alone whatever
  a `.gitignore` says, and `git add -f` adds an ignored one (ADR-039 rejected an ignore line as a
  guard for that reason). The root `.gitignore` names `mutants.out/`, and SPEC-290 tracks two
  recorded files under one by force.
- Each tool this tree runs writes its output and caches to a default directory: cargo to a
  `target/` at the workspace root, or at the package root outside a workspace; Vite to `dist/`
  under its project root and its cache to `node_modules/.vite`; SvelteKit to `.svelte-kit/`, and
  its static adapter to `build/`; CPython to a `__pycache__/` beside each module. This tree's own
  configuration moves some of them and names others: Vitest's coverage reports under the root's
  `target/`, the Mini App's study build in `build/`, Paraglide's messages, the generated Xcode
  project, the placed framework and its bindings, and the derived data each Xcode job keeps
  outside the checkout.
- ADR-450's drivers bind every new rule of #913: a rule starts at warn, today's sites are listed
  and not cured, every check prints what it examined, and an empty population is VOID.

## Decision Drivers

- No rule already in force is lowered; the scrub's binary refusal stays as it is.
- A new rule starts at warn and moves to fail only by its own delivery, under ADR-450 D6.
- Every check prints what it examined and refuses an empty population, which is VOID.
- No tracked file is removed, moved or ignored by this delivery: today's sites are listed.

## Decisions

**D1. A path census, never a content check (SPEC-672 R1, R6).** The census reads the path list
`git ls-files` prints, every tracked path once, and never a file's bytes. The scrub keeps judging
content, so a binary under a build directory is refused twice, by two rules, and a text file under
one is refused by this one alone.

**D2. Four forms, read from the root down (SPEC-672 R2 to R6).** A directory name only a tool
writes, by its exact name or suffix; a cache file by its own name; a name a source tree may also
use (`target`, `build`, `dist`, `coverage`, `reports`), a site only beside the manifest of the
tool that writes it there (`Cargo.toml`, `package.json`); and a directory this tree's own
configuration names as an output. The first directory down from the root that any form names
decides the tool and the form; a cache file's name is read only when no directory decides.

**D3. Planted controls are path lists (SPEC-672 R12).** Every planted case is a list of paths in a
`.fixture` file, read as text, so no control is a directory on disk and none adds a site to the
tree it judges.

**D4. Today's sites are listed by path, with their records (SPEC-672 R7, R8, R10).** `LISTED`
names each site's path, tool, form and the record that keeps it. At warn an unlisted site prints
one line; at fail it fails; at both levels a stale entry fails, so a removal also removes its
entry, which is how a fix is counted.

**D5. Kill criteria (SPEC-672 R11).** ADR-450 D6's, for this census: `LEVEL` moves from warn to
fail after three iterations of the rule review at precision of at least 0.8 with at least five
hits, by its own delivery; demoting it once at fail is a signed ruling.
- **Live precision** is fixes divided by fixes plus new listing entries, over the last 30 merged
  pull requests. Below 0.8 with at least 5 hits, propose a demotion.
- **Tooling:** `examined 0`, or a `git ls-files` that fails, is VOID, which blocks. It is never a
  pass.

**D6. A new SPEC and ADR (SPEC-672 section 9).** SPEC-436 and ADR-450 hold Rust censuses read as
tokens, and SPEC-436's band has nine free ids; this census is a Python reader of paths with more
rows than that, so it takes a number and a band of its own and amends neither.

## What it was chosen against

### D1

- An ignore line for each tool directory: rejected because git does not apply an ignore line to a path it already tracks.
- `git ls-files --cached --ignored --exclude-standard`: rejected because it follows the ignore files, which also name files that are no build output, miss every shared name no ignore line names, and change whenever an ignore line does.

### D2

- Refusing every directory named `build`, `dist`, `target`, `coverage` or `reports`: rejected because a source tree may carry those names, and this tree does (`tools/coverage/`, a fixtures folder named `coverage`).
- Exempting the fixtures folder: rejected because a real cache written there reads the same as a planted copy, so a fixture that copies an output layout is listed with its record instead.

### D4

- Keying the listing by directory: rejected because a new file in a listed directory would pass unlisted.

### D6

- Amending SPEC-436 and ADR-450: rejected because their band has nine free ids and their census reads Rust tokens, not paths.

## Consequences

- Good: every tracked path is examined on each run, and a new path under a tool's directory
  prints its path, tool and form.
- Good: a fix is counted when its listing entry is removed.
- Bad: at warn a new site passes; it is visible only in the hygiene job's output.
- Bad: a tool directory spelt in another case, or one a setting moves where this tree's
  configuration does not name it, is not found until a delivery adds its form.

## Confirmation

SPEC-672's acceptance criteria, each read red then green or recorded not red, in
`docs/red-first/SPEC-672.md`; the rows S67200 to S67243; the hygiene job of the delivery's
first push, which prints every unlisted site over the whole tracked path list.

## What would make this wrong

- A listed site is found correct in review more than once in five: then precision falls and D5
  proposes a demotion to the owner.
- A tool writes its output under a directory none of the forms names: then the census holds 0
  while output is tracked, and that tool's form is the next delivery.
- `git ls-files` lists paths the checkout does not hold: then the census judges a stale index, and
  its population is read from the commit's tree instead.
