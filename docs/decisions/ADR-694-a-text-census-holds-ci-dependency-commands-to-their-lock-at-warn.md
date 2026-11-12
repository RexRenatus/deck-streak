---
status: "accepted"
---

# ADR-694: a text census holds CI dependency commands to their lock, at warn

Decides SPEC-671. Path:
`docs/decisions/ADR-694-a-text-census-holds-ci-dependency-commands-to-their-lock-at-warn.md`.
Amends nothing. Cites ADR-450 D6 (a census starts at warn; promotion to fail is its own delivery),
SPEC-436 R21 (WARN per unlisted site, STALE per listed entry no site matches, at both levels),
SPEC-344 R6 (the Apple build's cargo commands carry `--locked`) and SPEC-332, SPEC-416 and
SPEC-458 (the jobs that fetch the locked graph first). Read at dev `b9a08e60`.

## Context

The tree pins its graphs in `Cargo.lock`, `pnpm-lock.yaml` and the hash-pinned pip requirements,
and its Swift packages name their dependencies by local path. A CI command that resolves without its
lock flag can build a graph nobody reviewed. One test holds the Apple workflow's cargo commands to
`--locked`; nothing reads the other workflows or the scripts their steps run.

## Decision Drivers

- Default-deny: a form is left out only with a reason; every other form is a site.
- A key that survives edits elsewhere in a file: open pull requests change the workflows often.
- No gate narrows: the existing hold and every existing test stay as they are.

## Decisions

D1. The census reads text: every workflow and every script a run text names by path, each read
once (SPEC-671 R1, R2). It never runs, imports or follows further.

D2. A tool's word is found at any position of a simple command (R3), because a command run after a
wrapper whose value is a variable, or after `xargs`, is missed by a reader that only looks at the
first word.

D3. Each tool's lock flag is the one its own documentation names (R4): `--locked` or `--frozen`
for cargo and its plugins, `--cargo-arg` for cargo-mutants, `--locked` for `cargo install`,
`--frozen-lockfile` for a pnpm install, `npm ci` for npm, the resolved-file flags for Swift, and
`--require-hashes` for pip. pnpm turns `--frozen-lockfile` on by itself in CI when a lockfile is
present; the census still asks for the flag in the text, so a run outside CI reads the same. The
census asks no flag of xcodebuild: every xcodebuild action or project call is a site, listed with
its reason, and requiring a resolved-file flag for xcodebuild is its own delivery's question.

D4. A site's key is its file, its job or function, its tool, its subcommand and its ordinal, never
its line (R7).

D5. The census starts at `warn`. Promotion to `fail` follows ADR-450 D6, by its own delivery; a
STALE entry fails at both levels from the start.

D6. A form the text cannot decide is printed as undecided and never judged (R6). A flag a site
lacks is never added here.

## What it was chosen against

- A reader of the first word only: it missed two commands the word scan finds.
- Line-keyed entries: an edit above a site would turn its entry STALE.
- A fail level from the start: the population holds sites whose reasons are recorded but whose
  flags are a later change.

## Consequences

Every unlisted site prints at warn in the `hygiene` job; each listing entry needs a current site and
a reason. A new workflow or called script is read without any change to the census.

## Confirmation

SPEC-671's planted tests (R1 to R7) run on planted text; its repository test (R1, R7) runs in CI. The
rows of `S67100-S67199` are killed by the planted tests.

## What would make this wrong

A tool whose lock flag can only be set by its configuration file, a step that installs through an
action input the census does not read, or a script the steps reach only through another script.
Each would be a site the census cannot see; the SPEC names them as not read.

## More Information

SPEC-671, `docs/schematics/ci-lock-flags.md` and `docs/red-first/SPEC-671.md`.
