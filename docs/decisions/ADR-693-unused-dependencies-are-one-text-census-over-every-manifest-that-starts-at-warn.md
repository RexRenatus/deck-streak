---
status: "accepted"
---

# ADR-693: unused dependencies are one text census over every manifest, and it starts at warn

Decides SPEC-670. Path:
`docs/decisions/ADR-693-unused-dependencies-are-one-text-census-over-every-manifest-that-starts-at-warn.md`.
Amends nothing. Cites SPEC-436 and ADR-450 D6 (a new census starts at warn and is promoted by its
own delivery under the kill criteria there) and SPEC-431 (a census reads `Cargo.toml` text with
`tomllib` and starts no process).

## Context

- **No check reads whether a declared dependency is used.** The workspace's lints, the dependency
  audit and the lock-file checks read what is declared, never whether a file uses it.
- **Four ecosystems declare dependencies.** The Rust workspace (the root manifest's
  `[workspace.dependencies]` and each member's tables), the web packages (`package.json` files
  reached from the root and from `pnpm-workspace.yaml`), the Swift packages and XcodeGen specs, and
  one Python requirement file.
- **The manifests are text a standard library reads.** TOML and JSON parse exactly with `tomllib`
  and `json`; `Package.swift`, the XcodeGen specs and requirement files are line forms.
- **Some uses are invisible to text.** A crate used through another crate's re-export, a crate
  called only by code a macro generates, a generated source the tree does not hold and a tool a
  workflow runs leave no name in the package's own files.

## Decision Drivers

- No rule already in force is lowered, and no dependency is added or removed by the check.
- A new rule starts at warn and is promoted only by its own delivery (ADR-450 D6).
- Every check prints what it examined and refuses an empty population as VOID.
- A form text cannot decide is reported as undecided, never as a pass or a failure.

## Decisions

**D1. One Python census over all four ecosystems (SPEC-670 R1).** The census lives in
`scripts/tests/test_unused_dependencies.py` beside the other manifest censuses. It reads every
manifest each ecosystem declares, with `tomllib`, `json` and line patterns, and starts no process,
build or install. ADR-450 kept its own censuses in Rust because they read Rust tokens through the
pinned lexer; this census reads names in paths and import lines, which needs no lexer, and three of
its four ecosystems are not Rust.

**D2. The use forms, by ecosystem (SPEC-670 R2-R8).** A Rust entry is used by `use`, `extern crate`
or a `N::` path of its source name (the key when renamed, a member's library name, else the package
name with `-` as `_`), with plain comments blanked and doc tests kept; normal and dev entries are
judged against the member's files but the build script, build entries against the build script
alone, each with the `#[path]` and `include!` files they name. A workspace key no member inherits is
a site. A web entry is used by an import specifier, a test environment docblock, a script word, a
configuration string or the package an `@types` entry types. A Swift dependency is used by an import
in its target's sources; a Python requirement by an import under its folder.

- **False positives:** a use through a re-export, generated code or an outside tool is a site; each
  is listed with its reason.
- **False negatives:** a name in a string or a doc comment counts as a use; a lookalike path in a
  macro's input counts as a use.

**D3. Three undecided forms (SPEC-670 R9).** A Rust entry a feature names, a peer dependency and a
dependency of a target with no source print `UNDECIDED` and never fail: a feature may enable code
the text never names, a peer is a requirement of the toolchain, and a sourceless target's sources
are generated.

**D4. The listing and the level (SPEC-670 R10-R12).** Each site is keyed by manifest, table and
name. `LISTED` holds today's sites, each with a non-empty reason; an unlisted site prints `WARN` and
fails only at `Fail`; a listed key that is no current site prints `STALE` and fails at both levels.
`LEVEL` starts at `Warn`.

**D5. Kill criteria and promotion (SPEC-670 R13).** ADR-450 D6 binds: `LEVEL` moves to `Fail` after
three iterations of the rule review at precision of at least 0.8 with at least five hits, by its own
delivery; demoting a fail rule is a signed ruling; `examined 0` is VOID and blocks.

## What it was chosen against

### D1

- **An existing tool (cargo-machete, cargo-udeps, knip, depcheck).** Each needs an install and a pin
  the tree does not hold, and knip's documentation says it needs the installed packages. Adding one
  is its own decision, beside this census.
- **One census per ecosystem.** Four modules would repeat the listing, the verdict and the refusal.

### D2

- **Following re-exports through other crates.** It needs every external crate's source; the
  listing's reason carries the re-export instead.
- **A lock-file arm for `[patch]`.** A patch entry replaces a source and declares no use.

### D4

- **A configuration file for the listing.** The sibling censuses keep their listings in the module.

## Consequences

- A new dependency with no use prints a `WARN` line in the `hygiene` job.
- A removed use leaves a listed key stale, which fails until the listing is cut.
- No dependency changes in this delivery.

## Confirmation

The planted tests of SPEC-670 section 3 read each form red first, and the rows of SPEC-670 section 9
are each killed by a planted test.

## What would make this wrong

- A use form the census does not read is common in the tree, so its warnings are mostly false; the
  kill criteria of D5 then propose a demotion or an amendment naming the form.
- An installed tool proves more precise over the same population; it then replaces this census by
  its own decision.
