---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The `bin` killer kind reads the files, targets and binaries the compiler and cargo build, and is judged by an oracle

## Context and Problem Statement

A `bin::<path>` killer runs a binary's own unit tests (SPEC-039 section 15). The runner had to
decide three things from the crate's files: which source files the binary's module walk reads,
whether a test target named `bin` shadows the kind, and how many binaries the crate holds. It read
each from a shape it expected, and three shapes were wrong (#405): a `mod` with `#[path]` still
contributed a default `name.rs` the compiler never builds, a `[[test]]` named `bin` at a path other
than two expected ones was not refused, and the refusal's binary count could count a module file.
Each fix of a shape left the next shape open. What is the reader's source of truth, and how is it
judged?

## Decision Drivers

- A killer that selects the wrong file or a wrong test target reads a mutant KILLED or VOID for a
  reason that is not the test's.
- The compiler and cargo define the answer, so the reader should agree with them on every layout,
  not on the layouts someone listed.
- A layout the reader cannot decide must be refused by name, never read open.
- The tests must not depend on the workspace's current layouts, which change.

## Considered Options (the alternatives it was chosen against)

- The compiler's and cargo's own rules in the reader, held to them by a generated oracle population: chosen, because the oracle,
  and not a reviewer's memory, decides each member, and a new axis row joins by itself (#405). The reader implements the target
  census, `cfg` for a test build, `#[path]`, inline-module directories and lexemes, and `cargo metadata --no-deps` and
  `rustc --emit=dep-info` judge it.
- A hand list of layouts, each with an expected answer: rejected, because the list holds the shapes its author thought of, which is
  how the three gaps arose, and its expected answers are a second reading of the rules with no check against the tools (#405).
- Following `#[path]` with the runner's own resolver: rejected, because a second resolver is another reading of the rules that can
  drift from the compiler's, and the kind needs to know which files the walk reads, which `#[path]` modules it does not read (#405).
- Reading only the workspace's current layouts: rejected, because asking cargo and the compiler on the workspace would judge
  today's crates and no other, runs the tools on the workspace, and gives a layout no member until a crate has it (#405).
- A scratch measurement with its answers pinned in a table: rejected, because a pinned table is a hand list whose answers were once
  measured and cannot notice a change in the tools; the checked-in test therefore runs `cargo metadata` and `rustc` itself, in
  scratch crates only (#405).

## Decision Outcome

Chosen option: the reader reads targets, binaries and the module walk by the compiler's and
cargo's rules and refuses what it cannot decide by name, and the test is a generated population
with the oracle run inside it. The population crosses binary layouts with test layouts and module
layouts with crate-root positions, plus a generated set of `cfg` predicates, and each test prints
and asserts an `examined` figure derived from its tables. The tests need `cargo` and `rustc` on the
path of the machine that runs `scripts/tests`, and they fail, rather than skip, where either is
absent, because a skipped oracle would read as a pass.

### Consequences

- Good, because every member of the population is read as the tools read it or is refused by name,
  and the three planted shapes of the issue are members of it.
- Good, because a layout a later change adds to an axis table is judged by the tools at once.
- Bad, because the suite needs a Rust toolchain on the machine that runs it, and runs a tool per
  member, so it is slower than a table of answers would be.
- Bad, because a `cfg` predicate outside `test`, `not`, `all` and `any` is refused by name, so a
  binary that gates a module on a feature cannot carry a `bin::` killer until the reader decides
  that predicate.

### Confirmation

SPEC-039's A46 to A51 (`scripts/tests/test_bin_kind_census.py`), and the rows `S03986` to `S03995`.

## What would make this wrong

- A release of the tools that reads a layout differently: the oracle runs the installed tools, so
  a change in their reading fails the population and is seen at once, not silently absorbed.
- A refusal that is too wide: a crate in the workspace that the reader refuses by name would lose
  its `bin::` killers. The census reports the refusal, so it is seen, and the remedy is to decide
  that shape in the reader and add it to an axis table.

## More Information

SPEC-039 sections 19 and 20, issue #405.
