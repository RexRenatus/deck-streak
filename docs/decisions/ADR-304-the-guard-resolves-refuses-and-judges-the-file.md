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
- Over the repository, an arm must examine a stated number of files and refuse none that is read
  today.

## Considered Options (the alternatives it was chosen against)

- Expand the macros before reading (#441) — rejected because it makes the guard a macro expander:
  `macro_rules!` matching and hygiene are the compiler's, a second implementation of them would
  drift, and it would need a toolchain in a test that runs without one.
- Leave a nested `mod tests;` fail-closed (#433) — rejected because the refusal is wrong, not
  cautious: the file exists, rustc reads it from a path the grammar fixes, and the only way to
  satisfy the guard was to move a pinned shape into a place the guard happens to read.
- Read the file through its test declaration and ignore other declarations (#458) — rejected because
  a second declaration that compiles the file without `test` makes its shapes production code, and
  the guard would then accept a pin the production build never sees.
- Skip a declaration whose file the guard cannot name (#458) — rejected because such a declaration
  may name the test module's file (a path literal with an escape, a raw string, a moved module
  directory), and skipping it reads that file although a production build compiles it.
- Gate every path-naming `cfg_attr` by its predicate (#458) — rejected because a second path
  attribute, a nested `cfg_attr` or a plain `#[path]` beside it changes the file rustc reads, and a
  gate that ignored them read a production file as test code in a planted tree.
- Resolve the nested declaration, refuse the macro, and judge the file (chosen) — chosen because the
  path rules are short and measured, the macro arm is a token walk that refuses by name, and the
  file judgement reads every visible declaration from the crate roots and refuses on every doubt.

## Decision Outcome

Chosen option: "Resolve the nested declaration, refuse the macro, and judge the file", because each
disagreement ends in an arm that reads what rustc reads or refuses by name, and every doubt resolves
to a refusal.

- **#433 is resolved.** A declaration inside inline modules names a file below the inline names, in
  the module directory of the declaring file: `src/` for a crate root, a `mod.rs` and a file an
  attribute loaded, and `src/<stem>/` for any other file (the guard tries both directories and
  reads a file only when exactly one exists). A plain `#[path]` on the declaration is relative to
  that directory. A `#[path]` on an enclosing inline module moves it in a way the guard does not
  read, so such a declaration stays refused. The inner attributes that open the file are the
  module's own, as the Reference's module attributes rule states, so a file that opens with
  `#![cfg(test)]` is a test module even when its declaration carries no attribute.
- **#441 fails closed.** A crate file whose `macro_rules!` body declares a module under a `cfg`
  or `cfg_attr` that names `test` is refused, and the refusal names the file. The attribute is read
  in each place that reaches the module once the macro is expanded: before the `mod` keyword, before
  a `$( ... )` repetition that holds it, and as an inner attribute that opens its braces. At this
  delivery's head the arm examined 194 crate files and refused none, so it refuses nothing that is
  read today.
- **#458 is a refusal.** A module file that any visible declaration compiles without `test` is not
  read through its test declaration, so a shape only it spells is refused. A declaration is judged
  by every attribute that reaches the file, the file's own inner attributes included, and one whose
  only path-naming attribute is `cfg_attr(P, path = "...")` reaches the file it names only under P
  and its default file only without P. A declaration whose attributes cannot be decided counts as
  compiled. One whose file the guard cannot name (a path literal with an escape or a raw string, a
  module directory an inline `#[path]` moved, attributes it cannot read back to the previous item's
  end) is taken to name every file. A file compiled only under test is still read.

### Consequences

- Good, because the three disagreements with rustc end, each by an arm that a planted test kills
  and a row (S19305 to S19314) mutates.
- Good, because every doubt resolves to a refusal, which is the direction a guard may err.
- Bad, because the macro arm refuses a file on the text `cfg` plus `test` inside a macro body, so
  a macro that mentions `test` in a `cfg` for another reason is refused until it is moved. The
  repository has none (0 refused in 194 files).
- Bad, because the guard hedges between two module directories for a non-mod-rs file, and reads a
  file only when one candidate exists, so a tree where both exist is refused, not read.
- Bad, because a declaration whose file the guard cannot name refuses every test module file its
  crate declares that the declaration could compile without `test`, until it is respelled with a
  plain path literal.

### Confirmation

`TheGuardResolvesAModuleDeclaredInsideAnInlineModule`,
`TheGuardRefusesAMacroThatDeclaresATestModule` with `EverySettingShapeIsPinnedByItsLiteral`, and
`TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest` decide SPEC-192's A16 to A18,
and the guard's own R8 population (6,309 members judged against rustc) stays green. Rows S19305 to
S19314 are each proved KILLED by full id.

## What would make this wrong

- A repository file that a macro body gives a `cfg(test)` module on purpose: the arm would refuse
  it, and the answer is then to move the module out of the macro, or to write a better arm with a
  measured population, not to loosen this one.
- A rustc change to how an inline module's `#[path]` or its directory is resolved: the R8
  population labels members by rustc and would go red first.
- A guard that later reads the items of a compiled test module (#449) or trait aliases (#436)
  would share the resolver; that is a new decision, and this one does not cover it.
