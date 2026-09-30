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

### Decision, round 6: the compiler is the census

Round 5's review generated a population from Cargo's documentation, TOML 1.0 and the Rust Reference
(2390 cases; 2095 compile and are labelled by the compiler's own report) and found the rule of round
3 open again under each of three repairs. Five rounds each widened a textual reader of Rust and TOML
by one more shape, and each new population found the next. This round closes the class by
construction, so the census reads no Rust to find a caller: the compiler does.

**The closure argument.** Progression's build script gives progression alone the cfg `settle_census`
when the census compiles, and under it `settle` carries a deprecation. Rustc reports a use of a
deprecated item wherever a path resolves to it, through every re-export, alias, glob, macro
expansion and `include!`, in every file cargo compiles, and `--force-warn deprecated` makes that
report one no `allow`, `expect`, `deny`, `forbid` or `--cap-lints` can silence. The census has cargo
check every target of every workspace package (libraries, binaries, tests, benches, examples and
build scripts) in the four cfg states a profile can set on stable rustc: debug assertions on and
off, each with the unwind and the abort panic strategy. Each pass names every package with its
state, progression and every dependency among them, so a macro or a re-export compiles as the crate
that defines it wrote it for that state; and when a member has a dev-dependency, the census compiles
the libraries and binaries alone as well, which resolver 2 compiles without the dev-dependency's
features, as a build without tests does. Every use it reports outside progression's package is a
caller: coordination's goes to the cause rule, and any other package's is refused. What cargo is not
asked to compile is refused before it runs: a workspace cargo cannot read or that does not compile,
a member's feature (the census compiles none), a proc-macro member (rustc reports no deprecation
inside a derive's expansion), a `Cargo.toml` in the repository outside the workspace, a path package
that is not a member, a registry or git package that depends on progression, and a `.cargo/config`.
A caller in a build the census compiles is therefore either reported by rustc or sits in something
the census refuses by name. There is no third place, and no list of spellings to extend. Two kinds
of build it does not compile are named rather than guessed, and SPEC-072 section 12 discloses both:
one that compiles its packages in different debug-assertion states, and one that selects some
members only, whose dependencies resolve fewer features than the workspace's.

The options were each measured over round 5's population (VR5: its 2095 valid cases of 2390), the
axes round 5 left unmeasured (S2: 123 cases of build scripts, proc-macro crates, targets, path and
git dependencies, macros, cargo configuration, silencing, and coordination's attribution), round 4's
population (12307 members and 12338 controls, round 3's 144 and 144 among them) and the real tree.
ESCAPE is a member the option accepts, and FALSE REFUSAL is a control it refuses.

| option | VR5 | S2 | round 4 | real tree |
|---|---|---|---|---|
| (A) round 5's rule (every file Cargo builds, read as rustc reads it, or refused) | ESCAPE 0, FALSE REFUSAL 0 | ESCAPE 20, FALSE REFUSAL 13 (119 cases as first generated) | not measured: rejected on S2 | 0 refusals |
| (B) a declared narrow grammar | not measured | not measured | not measured | refuses kernel's build script, 156 test, bench and example files and 51 files with `#[path]` (dev: 170 and 53) |
| (C) the compiler as the census | ESCAPE 0, FALSE REFUSAL 0 | ESCAPE 0, FALSE REFUSAL 0 (11 controls refused by design: 8 in a proc-macro crate, a feature, a cargo configuration, a path package outside the workspace) | ESCAPE 0, FALSE REFUSAL 0 | 0 refusals |
| (D) issue 406's reader (PR 419's token reader and crate-root walk) | ESCAPE 258, FALSE REFUSAL 787 (fail-closed: 174 and 871) | ESCAPE 37, FALSE REFUSAL 24 (fail-closed: 7 and 51) | ESCAPE 12271, FALSE REFUSAL 51 (fail-closed: 3 and 12335) | 3 files refused (fail-closed: 1207) |

- (A) lost because it escapes on the axes it never measured: tests, benches and examples, found by
  Cargo's auto-discovery or declared (10), a proc-macro crate's function-like, attribute and derive
  macros and a name joined inside one (4), a build script reached through `[build-dependencies]` or
  named by `package.build` (2), a git dependency patched onto a workspace crate, a build script's
  own debug assertions, a feature and a cfg set by cargo's configuration. Each is one more shape for
  a textual reader to learn, which is the treadmill this round ends.
- (B) lost on the real tree: the grammar it needs refuses files dev holds today, and a false refusal
  of a real dev file is a failure. Widening it to admit them re-opens the axes (A) escapes on.
- (D) lost because it reads tokens and follows no binding: it cannot see a re-export, a `pub use`
  tree, a glob across members, a manifest's rename or a macro-generated item, so it escapes almost
  every member of round 4 and refuses three files of the real tree, and its fail-closed form refuses
  almost every control. PR 419's reader answers another question (which files a crate root reaches,
  and which tokens they hold). One reader could serve a Python test and this Rust test only across a
  process boundary (the Rust test running `python3`, or a Rust binary the Python test runs), which
  would add a runtime to the `rust` job and still not close the class, so nothing of it is shared.
- (C) was chosen: ESCAPE 0 and FALSE REFUSAL 0 over every population, and no refusal on the real
  tree. It adds no dependency edge: `serde_json`, which reads cargo's JSON, and `tempfile` are
  progression's dev-dependencies already.

**Consequences.**

- Good, because every earlier round's shape (a renamed, grouped, chained or globbed re-export, a
  crate alias, a manifest rename, a comment or a literal in any place, another member's re-export, a
  `#[path]`, a macro) is now followed by name resolution itself, and a new spelling of the same call
  needs no new code in the census.
- Good, because the macro-written re-export that issue 445 tracked is now followed: a macro in
  progression that writes a `pub use` of `settle` expands before rustc resolves the caller's path.
- Bad, because progression gains production text: a build script that sets the cfg only when the
  census's variable is present, and a `cfg_attr` on `settle`. Neither changes an ordinary build:
  without the variable, the cfg is unset and the attribute is absent.
- Bad, because the census now compiles the workspace: four `cargo check` passes of every target, and
  four of the libraries and binaries alone when a member has a dev-dependency (the real tree has),
  in a target directory of its own, in the `rust` CI job, beside the test that drives it. The killer
  compiles each of its trees alone, so the `rust` job grows by minutes; the census's target is kept
  between runs and the killer builds its stub once for each worker, which brings the job back
  within its budget (the pull request's body carries the job's times, read from CI).
- Bad, because a test that runs for minutes meets the mutation battery's per-mutant timeout of 300
  s: a mutant of progression that no faster test kills would be reported as a timeout rather than as
  missed, and a timeout neither fails the pull request's verdict nor counts as a survivor in the
  weekly table. This round names that decision and does not take it: the owner chooses between a
  nextest filter in `.cargo/mutants.toml` that runs progression's other tests for its mutants
  without `xp_census` (a mutant only the census kills is then reported missed, which is loud), and a
  separate test crate for the census (a new crate and its edges, which this round may not add).
  Either runs fewer tests for a mutant of progression, which ADR-199's rule counts a weakening that
  only the owner's signed ruling takes, as it counts a changed timeout. Keeping the timeout as it is
  would let a survivor pass as killed, so it is not offered.
- Bad, because some code is out of the compiler's reach, and the census says so rather than guess: a
  member's feature, a proc-macro member, a package outside the workspace and a cargo configuration
  are refused by construction; a wrapper function, a function pointer or a generic in progression's
  own code that calls `settle` is a new operation in progression's own code and stays issue 445's;
  and the census judges the code of this repository compiled by this toolchain on this platform, in
  builds that resolve the workspace's features, not doctests, not code a proc-macro writes only when
  it sees the census's variable, not a registry or git package's own code, and not a compile
  trybuild runs at test time. SPEC-072 section 12 lists each, by kind, and issue 445 tracks them.

### Decision, round 7: a build script's cfg is closed by the graph

Round 6's review found a class the four passes cannot close: a build script sets a cfg from the
profile or from a build environment variable, and code under that cfg calls `settle`, so the passes
never compile it. A build script's cfg reaches its own package alone, so the census now reads the
resolve graph (`cargo metadata --format-version 1 --locked --offline`, which compiles nothing) and
refuses, by name, every package that has a `custom-build` target and can name `settle`: the package
that defines it, or one that depends on it by a normal, a build or a dev edge at any depth. Where a
package can name `settle`, its script's cfg is the census's escape; where it cannot, the cfg
governs code that cannot reach `settle`. Progression's own script is admitted at one SHA-256, held
in one constant, and a script that hashes otherwise is refused by name: the pin is the only
admission. A graph cargo cannot give (a failed command, output that is not JSON, or one without the
resolve graph) is a refusal by name, so no step of the census is skipped.

The options were chosen against, each on what it does to the real tree:

- (A) words alone, disclosing a third kind of build the census does not compile. Rejected: it is a
  narrowing that the graph refusal makes needless, since the class is closed by construction.
- (B) release and custom-profile passes. Rejected: it closes the profile-keyed scripts only, the
  variable-keyed scripts stay open, and it at least doubles the passes, so the `rust` job's cost
  grows for a partial closure.
- (C) refusing every build script outside progression. Rejected: it refuses the real tree's kernel
  build script, which has no path to `settle`, so a false refusal of a real file would be the
  failure.
- (C') the graph refusal above, chosen: it refuses exactly the packages whose script's cfg can reach
  code that names `settle`, and on the real tree it lists progression alone, whose script is the
  pinned one.

The disclosed kind that (C') leaves, decided by main's round-6 ruling and named in SPEC-072 section
12: a build script's cfg in a package that cannot name `settle`, read by a macro that package
exports, where the macro expands a call to `settle` inside a package that can name it. The package
with the script is not refused, for it has no path to `settle`. It holds for a git, a registry and a
workspace package, and kernel's admitted script is of the same kind. The population of round 6
measured such a member for each condition of the build environment, and the census's test prints
what it says of each without asserting it. The other kind left open is an `include!` of a
recompute file from outside the folder, within one crate: rustc names the included file alone, so
the census names it too (issue 481's "the outermost file the compiler points to"); no textual
reader of `include!` is added. Both are tracked by issue 445.

Two speed-ups keep every tree, control, pass and test. The census's target lies under
`target/tmp/settle-census`, which the job's `cargo clean --workspace` leaves alone, and it is kept
between runs under a cache key of the toolchain pin and `Cargo.lock`; cargo fingerprints every unit
and recompiles a member whose source changed, so a stale entry is never served, and the census
reads what its own passes compile. The killer builds its stub once for each worker: a worker plants
its trees at one fixed path with a kept target, so only the files of a case recompile. A cold run
with no cache judges the same trees.

The textual census of rounds 1 to 3 is removed with its tables, its population test and its rows
S07230 to S07240. Round 6's rows, S07241 to S07275, pin the census's refusals by design, its passes,
its attribution, its bounds, its environment and the probe itself, each killed by a planted-tree
test; round 7's rows pin the graph refusal, its edges and its pin.

### Decision, round 8: the verdict depends only on the tree it judges

Round 8 states the rule the census keeps: its verdict depends only on the tree it judges. The design
lists every input a verdict can read (the tree, the lock file, the toolchain, the environment, the
target directory's build-script output and fingerprints, cargo's home, cargo's configuration from
the tree's root up to `/`, a compiler wrapper, and a CI cache) and places each one: part of the
tree, pinned by the lock file or the toolchain pin, made empty before each census compile, or
refused by name. SPEC-072 section 14 carries the rule and each refusal.

The census finds its owner by path: the workspace member whose manifest is
`crates/progression/Cargo.toml`. A graph where no member has that manifest, where more than one has
it, where that member's package is not named `deck-streak-progression`, where another package of the
graph carries the owner's name at any version or source, or where the owner has no build script or
more than one, is refused by name, so no graph without its owner is accepted. Every other lookup
the census makes is keyed by the package's id in the resolve graph, or is unique, or refuses.

Round 7 said the census's cached target is safe because "a stale entry is never served". That does
not hold for everything a kept target carries, so round 8 keeps no census target at all: each census
compiles in an empty target directory made for it and removed when it ends, the killer's workers
included, and the CI cache of the census's target is removed. The census's cargo starts from an
empty environment and inherits only the variables that locate the programs, the homes and the
pinned toolchain. A cargo configuration above the tree or in cargo's home, and a member's code that
reads a file outside the tree or a variable the host sets, are refused by name.

Round 7 also called kernel's build script admitted. The pin is the only admission, and it admits
progression's script alone; kernel's script is not refused, because kernel cannot name `settle`,
and it is not admitted by anything.

The options were chosen against:

- (a) drop the census's cache, and compile each census in an empty target. Chosen: nothing is
  carried from one census to the next, so the verdict can read nothing but the tree and what the
  lock file and the toolchain pin. Every census compile is cold, and the `rust` job pays for it; a
  slower census is not a weakening.
- (b) keep the cache and strip build-script output and fingerprints before saving it. Rejected: a
  strip is complete only against every input a build script in the graph can read, and the build
  scripts of packages that cannot name `settle` read what the census does not name, so no strip
  can be shown complete.
- (c) keep the cache and purge after restoring it. Rejected for the same reason as (b), and the
  restored entry is on the runner until the purge runs.
- (d) keep the name lookup of round 7 and refuse a second package of the owner's name. Rejected: a
  member at the owner's path under another package name would still be missed.
- (e) extend round 6's list of scrubbed variables. Rejected: a list of names to drop cannot name
  every variable a build script reads; a list of names to keep can.

What stays disclosed by kind, as main's round-6 ruling decided, is named in SPEC-072 section 14 and
tracked by issue 445: progression's own code reading the census's cfg, which the pin does not cover
(the proc-macro bullet of round 6 is one instance of it); and the build-time code of a package that
cannot name `settle`, which cargo runs during the census's compile. Rows S07290 to S07299 pin the
owner's lookup, the empty target, the environment and the refusals of configuration and of reads
beyond the tree.

## More Information

Issue 397; SPEC-072 (its amendment of 2026-09-29); ADR-072; issue 350, which delivered the census.
