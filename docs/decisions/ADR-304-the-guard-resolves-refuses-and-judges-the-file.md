---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The settings guard resolves a nested module declaration, refuses a macro that writes a test module, and judges the file rather than the declaration

## Context and Problem Statement

SPEC-192's guard (`scripts/tests/test_setting_shapes.py`) reads a `Setting` implementation's shape
from a source file, and from the out-of-line test module its declaration names. It reads Rust with
a hand-written lexer and decides whether a module is test code from its attributes. Three issues
record where that reading disagrees with rustc. #433: a `mod tests;` declared inside an inline
module is not followed, so a shape only that file spells is refused although it is pinned. #441: a
`macro_rules!` body can declare a `#[cfg(test)]` module, which the reader does not see inside a
token tree. #458: a file that one declaration compiles under test and another compiles without it
is read through the test declaration, although it is production code. How does the guard stop
disagreeing with rustc on these three, without growing into a Rust front end?

## Decision Drivers

- A guard that errs must err toward refusing, because a shape it reads and should not is a shape
  nobody pins.
- The guard may not cost more than it measures: it is a unittest over source text, with no cargo.
- Each arm must be killable by a test that plants the shape, and carried by a row.
- Over the repository at the base, an arm must examine a stated number of files and refuse none
  that is read today.

## Considered Options (the alternatives it was chosen against)

- Expand the macros before reading (#441) - rejected because it makes the guard a macro expander:
  `macro_rules!` matching and hygiene are the compiler's, a second implementation of them would
  drift, and it would need a toolchain in a test that runs without one.
- Leave a nested `mod tests;` fail-closed (#433) - rejected because the refusal is wrong, not
  cautious: the file exists, rustc reads it from a path the grammar fixes, and the only way to
  satisfy the guard was to move a pinned shape into a place the guard happens to read.
- Read the file through its test declaration and ignore other declarations (#458) - rejected
  because a second declaration that compiles the file without `test` makes its shapes production
  code, and the guard would then accept a pin the production build never sees.
- Resolve the nested declaration, refuse the macro, and judge the file (chosen) - the path rules
  are short and measured, the macro arm is a token walk that refuses by name, and the file
  judgement reads every visible declaration from the crate roots.

## Decision Outcome

Chosen option: resolve, refuse and judge the file.

- **#433 is resolved.** A declaration inside inline modules names a file below the inline names, in
  the module directory of the declaring file: `src/` for a crate root, a `mod.rs` and a file an
  attribute loaded, and `src/<stem>/` for any other file (the guard tries both directories and
  reads a file only when exactly one exists). A plain `#[path]` on the declaration is relative to
  that directory. A `#[path]` on an enclosing inline module moves it in a way the guard does not
  read, so such a declaration stays refused.
- **#441 fails closed.** A crate file whose `macro_rules!` body declares a module under a `cfg`
  or `cfg_attr` that names `test` is refused, and the refusal names the file. At the base the arm
  examined 194 crate files and found 0 hits, so it refuses nothing that is read today.
- **#458 is a refusal.** A module file that any visible declaration compiles without `test` is not
  read through its test declaration, so a shape only it spells is refused. A rival whose
  attributes cannot be decided counts as compiled. A file compiled only under test is still read.

### Consequences

- Good, because the three disagreements with rustc end, each by an arm that a planted test kills
  and a row (S19305 to S19309) mutates.
- Good, because every doubt resolves to a refusal, which is the direction a guard may err.
- Bad, because the macro arm refuses a file on the text `cfg` plus `test` inside a macro body, so
  a macro that mentions `test` in a `cfg` for another reason is refused until it is moved. The
  repository has none (0 hits in 194 files).
- Bad, because the guard hedges between two module directories for a non-mod-rs file, and reads a
  file only when one candidate exists, so a tree where both exist is refused, not read.

**Confirmation.** `TheGuardResolvesAModuleDeclaredInsideAnInlineModule`,
`TheGuardRefusesAMacroThatDeclaresATestModule` and
`TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest` decide SPEC-192's A16 to A18,
and the guard's own R8 population (6,309 members judged against rustc) stays green.

## What would make this wrong

- A repository file that a macro body gives a `cfg(test)` module on purpose: the arm would refuse
  it, and the answer is then to move the module out of the macro, or to write a better arm with a
  measured population, not to loosen this one.
- A rustc change to how an inline module's `#[path]` or its directory is resolved: the R8
  population labels members by rustc and would go red first.
- A guard that later reads the items of a compiled test module (#449) or trait aliases (#436)
  would share the resolver; that is a new decision, and this one does not cover it.
