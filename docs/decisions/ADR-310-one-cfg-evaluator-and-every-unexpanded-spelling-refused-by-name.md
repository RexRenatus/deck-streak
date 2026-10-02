---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The settings guard keeps a pin only where one cfg evaluator proves it compiled, and refuses by name every spelling it does not expand

## Context and Problem Statement

SPEC-192's guard (`scripts/tests/test_setting_shapes.py`) reads Rust source with a hand-written
lexer. Four issues record where its reading still disagrees with rustc after ADR-304. #436: it
found an implementation by a pattern anchored at a line start, so a raw identifier, an import alias,
a macro-made implementation, an attribute on the same line and a second item on the line were not
examined. #449: it read the whole text of a test module, so a pin inside an item a `cfg` strips
counted. #535: four spellings fail open: an attribute a macro passes in, a macro that passes its
tokens through, a `#[path]` rival declared in a block or by a macro, and `include!`. #536: three
trees rustc reads plainly are refused, and `modules()` returns a value its caller drops. How does
the guard close all four without growing into a Rust front end?

## Decision Drivers

- A guard that errs must err toward refusing: a pin it reads and should not is a shape nobody pins.
- The guard stays a unittest over source text, with no cargo, and with rustc only as a test oracle.
- Each arm must be killable by a test that plants the shape, and carried by a row.
- Over the repository, each new refusal must examine a stated number of files and refuse none.

## Considered Options (the alternatives it was chosen against)

- Read an item under a predicate the guard cannot evaluate as compiled (#449) — rejected because a
  `feature = ...`, a target key or a `cfg_attr` may strip it, and then a pin rustc never compiles is
  read, which fails open.
- Evaluate item attributes and the macro narrowing with two functions (#449, #536) — rejected
  because two readings of one attribute drift apart, and the narrowing could then read as proven
  what the item walk treats as unknown.
- Expand macros, or narrow every macro module that is not plainly `cfg(test)` (#436, #535, #536) —
  rejected because a macro expander is the compiler's job, and a narrowing that only looks for
  `test` reopens #441: `cfg(any(test, feature = "slow"))` is a test-only module whenever the feature
  is off.
- Model rustc's block-scope module paths and follow `include!` (#535) — rejected because a text
  reader would approximate both, and an approximation that names the wrong file reads a production
  file as a test.
- Resolve a trait alias per scope (#436) — rejected because it needs name resolution across modules,
  globs and re-exports; a crate-wide closure over `use ... as` can only add names, so its one error
  is a disclosed false refusal.
- One evaluator, macros refused by name, block scope and `include!` refused by name (chosen) —
  chosen because each disagreement then ends in an arm that reads what rustc reads or refuses by
  name, and every doubt resolves to a refusal.

## Decision Outcome

Chosen option: "One evaluator, macros refused by name, block scope and `include!` refused by name",
because it closes #436, #449, #535 and #536 with arms that each read what rustc reads or refuse.

- **One cfg evaluator.** `kept(attributes, test)` decides both #449's items and #536's narrowing.
  It evaluates `cfg` over `test`, `any`, `all`, `not`, `true` and `false` exactly; every other
  option, every `cfg_attr` and every attribute it cannot read is unknown. An item it does not read
  as true under `test` counts as not compiled, so an unknown can only cause a false refusal.
- **Macros stay fail-closed.** An implementation a macro body or a macro invocation writes is
  refused by its file (#436). A module a macro body or invocation declares is refused by its file
  when an attribute that reaches it holds a metavariable or names `path` (#535), or names `cfg` or
  `cfg_attr` with `test` (#441). #536 narrows only the last arm, and only where `kept` proves the
  module is no test-only module; an out-of-line module a macro declares that `kept` does not prove
  removed without `test` is a declaration that may name any file, by ADR-304's #458 rule.
- **Block scope and `include!` are refused by name.** An out-of-line module declared in a block,
  unless `kept` proves it removed without `test`, and every `include!`, refuse their file (#535).

### Consequences

- Good, because a pin in a stripped item is never read: a generated population, judged by rustc,
  holds the guard to it.
- Good, because every new refusal is by name, so a refused tree says which file to change.
- Bad, because an item under a feature, a target key or a `cfg_attr` is not read even where rustc
  compiles it, and a kept item that holds such an attribute is not read at all; each is a disclosed
  false refusal.
- Bad, because a shadowing alias makes the guard read an implementation of another trait as a
  `Setting` one, and refuse it when it has no pinned shape.
- Bad, because a `cfg(any(test, P))` module a macro declares stays refused when `kept` cannot decide
  P.

### Confirmation

SPEC-192's A19 to A22 decide it: `TheGuardReadsAnImplementationByToken`,
`TheGuardReadsOnlyTheItemsRustcCompilesUnderTest`, `TheGuardRefusesASpellingItDoesNotExpand` with
`EverySettingShapeIsPinnedByItsLiteral`, and `TheGuardReadsATreeItOverRefused`. The guard's R8
population stays green, and each new row in `scripts/mutation-rows.d/S19300-S19399.json` from
S19315 is proved KILLED by full id.

## What would make this wrong

- A repository file that needs a pin under a feature or a `cfg_attr`: the guard would refuse it,
  and the answer is a measured arm that evaluates that predicate, not reading it as compiled.
- A rustc change to how a cfg strips an item: the population's oracle would go red first.
- A repository file that declares a module in a block, or uses `include!`, on purpose: the arm
  refuses it, and the answer is to move the declaration or to write a resolver with a measured
  population, not to loosen this arm.
