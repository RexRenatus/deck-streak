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
- Read progression's own re-exports and aliases — chosen, because it is the census's own method:
  the census tokenises progression's `src`, reads every `pub use` tree (grouped, nested,
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
- Good, because a source in a member that neither globs progression's crate root nor names any
  name the census binds to the crate cannot be refused for a common word, so a homonym there is not
  a false refusal.
- Bad, because a glob of the crate root opens every file of the member that holds it: in any file
  of that member, the member's own function called `settle`, or a homonym of one of progression's
  renamings, is refused, even where the member's own item shadows the glob. It is a loud failure,
  not a silent pass.
- Bad, because the names that denote the crate are one set for the workspace: a name bound to the
  crate in one member, or a module another member exports the crate by, is followed in every
  member, so a member's own module of that name that holds its own `settle` is refused. It is a
  loud failure, not a silent pass.
- Bad, because the reading is textual: a public wrapper function that calls `settle` inside
  progression is not a `pub use` or a `pub type` and is not followed, and neither is a re-export that a
  `macro_rules!` macro in progression writes, since the census does not expand macros (a
  metavariable such as `$name` in such a macro is read as a name too, which can refuse an
  unrelated caller: a loud failure, not a silent pass). Both are progression's own code, and a
  reviewer sees them there. Issue 445 tracks both.

### Confirmation

`crates/progression/tests/xp_census.rs`: the planted-tree tests named in SPEC-072 A32 and A33 (red before
the change, green after), and A12's census, whose examined counts on the real tree are unchanged.

### Decision, round 1

The census also follows a crate alias (`pub use deck_streak_progression as prog;`) and a
`pub type` alias of the request or the operation, treats a `)` as public only when it closes
`pub(`, and matches the operation as a word, so a name that merely begins with `settle` and a
private homonym behind an attribute are accepted. It was chosen against leaving each as a known
gap, which would have left four evasions to be found again, and against a parser such as `syn`,
which the first decision already rejected. A wrapper function stays a residual.

### Decision, round 2

The census follows a crate alias in each of these binding forms: renamed inside a group (`{self as
prog}`), in raw spelling (`r#prog`), through a chain of aliases read in any file order, by an
`extern crate`, by a glob of the crate (which opens every file of the member that holds it, since a
glob import is visible to the whole member as `crate::name`), and by a manifest's `package` rename
in the workspace's or a member's manifest. It reads a raw identifier as its plain name, so `r#tally`
is `tally`. A source that reaches `settle` by any of those names (`x::settle` in a path or a `use`,
or a bare `settle` in a member that globs the root) is refused as a direct caller. It was chosen
against naming each spelling a residual, which would leave the crate alias of round 1 open in five
other spellings, and against refusing every non-canonical spelling at its source, which would refuse
legitimate code in crates the census does not own.

### Decision, round 3

The census follows the crate across members and past every comment. A member that exports the
crate root to another member (a plain `pub use` or `pub extern crate` whose path passes through a
name of the crate, as a glob or an alias, at its root or in a module), or that re-exports the
operation or one of its renamings, makes its own crate name, or the module that holds the export,
a name of the crate; the names run to one fixpoint across every member. Rust is read by one lexer
that makes every comment a space (nested blocks and doc comments included) and every literal
empty before any name is read, and manifests by one TOML reader that removes comments outside
strings and reads headers and keys as keys; a manifest it cannot read to its end is refused.

The rule was measured against two others over a population generated from the test's own tables
(12307 members and 12338 controls, a stratified sample of which compiles under the pinned
toolchain):

- one set for each member crate plus the names each member exports: every control accepted, but six
  members escaped, each a file that sits in one member's `src` and that another member compiles by
  `#[path]`, where the binding is the compiling member's private name;
- the global set that also lets a member's own item shadow its glob: every control accepted, but
  three members escaped, each a glob member whose own homonym stands in another module while the
  caller reaches the operation through the glob.

The global set was chosen, because it is the only one of the three with no escape, and it refuses
no control of the population. Its false refusals are the two Bad consequences above, and each is a
loud failure. A parser was not an option this round: `syn` and `toml` are in the lock only as other
crates' dependencies, and making either a dependency of the test is a new edge, which the drivers
above rule out. A wrapper function and a macro-written re-export stay the residuals named above,
tracked by issue 445.

## More Information

Issue 397; SPEC-072 (its amendment of 2026-09-29); ADR-072; issue 350, which delivered the census.
