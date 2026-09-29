---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The settle census reads progression's own re-exports, as it reads the other crates'

## Context and Problem Statement

SPEC-072 A12 keeps `settle` coordination's alone. Its census searches every crate but progression
for the operation's path and for the request type `SettleRequest`, because nothing can call
`settle` without building that request by name. Progression's own `src` is skipped, since its
code owns the operation. A `pub use` there that renames the operation or its request
(`settle as tally`, `SettleRequest as TallyRequest`) hands every other crate two names the census
does not search for, so a caller that imports only those names calls `settle` and is not refused.
Issue 397 measured it. How should the census see through a renaming that progression itself
makes?

## Decision Drivers

- A12's purpose is that only coordination settles, whatever the caller names the operation.
- The refusal must name the file, the alias and the original, so a reader can act on it.
- The census is a test with no dependency beyond the standard library and `tempfile`, and a new
  dependency edge in any manifest is an ADR of its own.
- Progression's own use of its names, and coordination's callers, must stay accepted.

## Considered Options (the alternatives it was chosen against)

- Forbid any `pub use` of `settle` or `SettleRequest` inside progression: the census would refuse
  the renaming at its source and need read nothing more. It lost because it bans a re-export the
  crate may legitimately make for its own tests and views (the crate root already re-exports the
  settled rows), and because a ban leaves the census blind to the next shape it did not think of.
- Resolve names with a full parser such as `syn` over every crate: exact, and it would follow
  every alias a compiler follows. It lost because it adds a dependency to the test crate and
  parses every source in the workspace to answer a question about one crate's exports.
- Read progression's own re-exports and aliases as the census already reads the other crates':
  chosen. The census tokenises progression's `src`, reads every `pub use` tree (grouped, nested,
  with or without `as`, inside a nested module, a whole-module renaming, and a chain of
  renamings), and adds each new name to the names it searches for. A source that names the
  progression crate and one of those names is a caller. No measurement pointed the other way: the
  tree's real `src` holds no renaming today, and the planted tree proves each shape.

## Decision Outcome

Chosen option: "read progression's own re-exports and aliases", because it closes the measured
gap with the census's own method, keeps the test dependency-free, and leaves every earlier
refusal and examined count as it was.

### Consequences

- Good, because a renamed re-export can no longer hide a caller, and the refusal names the file,
  the alias and the original.
- Good, because a source that never names the progression crate cannot be refused for a common
  word, so a homonym elsewhere is not a false refusal.
- Bad, because the reading is textual: a `pub type` alias of the request, or a public wrapper
  function that calls `settle` inside progression, is not a `pub use` and is not followed. A
  wrapper is a new operation in progression's own code, and a reviewer sees it there.
- Bad, because the operation's path is still matched as a prefix, so `deck_streak_progression::`
  followed by a name that begins with `settle` reads as a call; that predates this decision and is
  left as it was.

### Confirmation

`crates/progression/tests/xp_census.rs`: the planted-tree test named in SPEC-072 A32 (red before
the change, green after), and A12's census, whose examined counts on the real tree are unchanged.

## More Information

Issue 397; SPEC-072 (its amendment of 2026-09-29); ADR-072; issue 350, which delivered the census.
