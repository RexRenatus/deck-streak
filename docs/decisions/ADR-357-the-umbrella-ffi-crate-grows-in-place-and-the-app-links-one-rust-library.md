---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# The umbrella FFI crate grows in place, UniFFI lives in it alone, and the one-library rule is a census

## Context and Problem Statement

ADR-335 decided that one umbrella crate exposes the engine to the iPhone and iPad app through
UniFFI and is the app's one Rust static library, "because two Rust static libraries in one app
clash", and that the engine core, then the XP crate and the FSRS-7 crate, link into it. ADR-345
built the first half: one allow-listed adapter, `deck-streak-ffi`, whose static library the macOS
job makes with `cargo rustc --crate-type staticlib` and never declares in a manifest, and whose
bindings the job generates in library mode. ADR-350 put the XCFramework in a local Swift package
fed from the same run's artifact, and ADR-355 runs the Apple build from one job body.

Four things none of them settles remain for SPEC-346 (#624):

1. Which crate is the umbrella. #623's engine core makes `deck-streak-ffi` depend on the core,
   keeps UniFFI out of the core, adds its next UniFFI types to the adapter, and its graph test
   refuses any member naming an adapter.
2. Where UniFFI may appear, now that several crates will link into one library.
3. How "one static library" is held, when no manifest declares a static library and the one
   command that makes it lives in a workflow.
4. What the Apple change caller watches, now that the umbrella links a crate outside
   `crates/ffi/**`.

## Decision Drivers

- The crate graph is the context map. An edge is drawn in the change that first uses it, so the
  XP and FSRS-7 edges cannot be declared ahead of the crates.
- An adapter translates at the edge. FFI vocabulary (UniFFI derives, foreign error types) belongs
  to the adapter, never to a bounded context, and the XP crate is shared with the web engine's
  wasm build.
- A host build of the workspace must not pay for a static library or the generator (ADR-345 D4,
  D5), and the settle census refuses a fourth declared feature (ADR-345 D6).
- Library mode generates one Swift source and one header per crate that carries UniFFI metadata,
  so every crate with scaffolding adds a component to the app's bindings.
- The macOS job runs only when its paths change. A rule held only there is held only for the
  changes that already touch it.
- A census is held by a positive artifact and a planted control, and it prints what it examined.

## Considered Options (the alternatives it was chosen against)

- D1, `deck-streak-ffi` grows in place into the umbrella - chosen, because the library, its
  module, the package's binary target and the harness's import keep their names, the map gains no
  context, and #623's rule that no member names an adapter stays whole. The XP crate and the
  FSRS-7 crate each join it by an unconditional dependency edge in the change that first exposes
  them, and the adapter wraps their types in its own UniFFI types.
- D1, a new umbrella crate re-exporting the adapter, the XP crate and FSRS-7 - lost: it would
  depend on all three, and #623's graph test refuses a member naming an adapter; the adapter's
  own scaffolding would still be a component of the library, so library mode would generate two
  Swift sources and two headers; the library, the module and the binary target would all be
  renamed for no behaviour; and the map would gain a context that holds no logic.
- D1, UniFFI scaffolding in each exposed crate, several UniFFI components in one library - lost: it
  puts FFI vocabulary inside bounded contexts, compiles UniFFI derives into the XP crate's wasm
  build for nothing, and gives the app one Swift source and one header per crate where the adapter
  gives one.
- D1, membership by cargo features on the umbrella (`xp`, `fsrs7`) - lost: each feature doubles the
  settle census's compiles and the census refuses a fourth, a feature turned on by another package
  is refused by name, and features unify across a workspace build; an unconditional edge costs a
  host build only two small I/O-free crates, because the static library is built by the macOS job
  alone.
- D2, UniFFI in the umbrella alone, held by a manifest and lockfile census - chosen, because a
  crate cannot expand UniFFI's macros without depending on UniFFI, so the dependency is the
  necessary condition, it is read from two files (every member's dependency tables, and
  `Cargo.lock`'s dependents of `uniffi`), and a crate that reaches UniFFI through another's
  re-export makes that crate a dependent the lockfile census refuses. The inherited
  `unsafe_code = "forbid"` keeps unmangled exports out of every member's own code.
- D2, a scan of every source file for `uniffi::` and `setup_scaffolding!` - lost: it reads every
  source to find what the manifests already decide, and it misses a macro called under another
  path.
- D2, review alone - lost: a second component shows only at generation or at link, on the macOS
  runner, and only when that job's paths change.
- D3, a Linux census in the required CI plus two macOS output checks - chosen, because what
  enters the app is decided by what the tree declares, which the census reads on every pull
  request, and the two steps measure the outputs the declarations produce. The census: no member
  declares `staticlib`; every workflow's `--crate-type` command builds the umbrella; one local
  binary target across the Swift packages; no built library tracked. The steps, in the macOS job
  over its own outputs: one module's files in `bindings/`, one library per slice; a Linux test
  runs their scripts over planted trees.
- D3, a manifest `staticlib` crate type, guarded by a count of such manifests - lost: ADR-345 D4
  rejected the manifest crate type, because every host build would then link a static library of
  the engine, and a manifest count reads zero at this tree and cannot see the
  `cargo rustc --crate-type staticlib` that makes the library.
- D3, a duplicate-symbol census with `nm` over the slices - lost: it runs only on macOS, every
  static library carries symbols a second copy of the standard library would repeat, so its rule
  for a clash is unmeasured, and the harness job's app link already fails on a duplicate strong
  symbol; it measures a consequence the declarations decide earlier.
- D3, the two macOS steps alone - lost: the job runs only when its paths change, so a change that
  adds a second library outside those paths would never meet them.
- D4, the change caller's crate globs equal the umbrella's build closure - chosen, because the
  macOS build then runs on every change that alters the library, the next edge (#639, #641) fails
  the test until its glob is added, and no other context's change starts it. A test derives the
  closure from the manifests: the umbrella, every workspace member its normal, build, target and
  dev tables name, and every workspace member those name in their normal, build and target
  tables.
- D4, `crates/**` - lost: every change to any context, the server's and the web engine's included,
  would start a macOS build of a library it does not change.
- D4, a hand-kept list with no census - lost: it misses the engine core the moment #623 lands, and
  would miss each later edge the same way, silently.

Not decided here: where the Swift package lives and how its binary target is fed. ADR-350 D2
settled it (a local package whose binary target the job places from the same run's artifact,
against a committed framework and a remote binary target), and this delivery holds it by census.

## Decision Outcome

Chosen options: D1 `deck-streak-ffi` grows in place, D2 UniFFI in the umbrella alone held by
manifest and lockfile census, D3 a Linux census plus two macOS output checks, and D4 a change
caller whose crate globs equal the umbrella's build closure, because together they keep one
UniFFI component in one static library, keep FFI vocabulary out of every context, hold the rule on
every pull request rather than only on the macOS job's paths, and build the library whenever a
crate it links changes.

### Consequences

- Good, because the library, the Swift module, the binary target and the harness's import keep
  their names, so the delivery changes no Swift and no consumer.
- Good, because a second static library, a second UniFFI component, a committed framework and a
  remote binary target are each refused by the required CI before any macOS run.
- Good, because the change caller follows the graph: an edge drawn without its glob fails a test
  named for it.
- Bad, because the adapter grows a wrapper type for each exposed type of the XP crate and the
  FSRS-7 crate, where per-crate scaffolding would export them directly.
- Bad, because every host build of the adapter will compile the XP crate and the FSRS-7 crate once
  their edges are drawn, though only the macOS job needs them linked.
- Bad, because the census holds the declarations, not the link: a C library compiled by two
  crates' build scripts with the same global names is caught only by a link, on Linux by the
  coexistence test #641 carries and on macOS by the harness job's app link.
- Bad, because the closure reader is a second reader of cargo manifests beside the settle census's
  `cargo metadata` read; it reads the manifests' own tables, with no compiler, so it can run in the
  required CI's Python stage.

### What would make this wrong

- A measured clash between two UniFFI components in one library that the adapter's wrappers cannot
  avoid, or a crate that must export UniFFI types the adapter cannot wrap: D1 and D2 reopen.
- A host build made measurably slower by the unconditional edges: D1's feature option reopens,
  with the settle census's bound.
- A second Rust library the app needs that cannot link into the umbrella (a prebuilt vendor
  library, for instance): D3's census refuses it, and the decision to admit it is a new ADR.

### Confirmation

SPEC-346 A1 to A4 hold the manifests, the lockfile, the Swift packages and the tracked tree,
each beside planted controls it refuses by name; A5 holds every workflow's `--crate-type` command;
A6 and A7 run the two macOS steps' own scripts over planted good and bad trees; A8 holds the change
caller's globs equal to the closure, with a planted two-hop graph; A9 is SPEC-344's A1 amended;
A10 reads both steps' `pass` from the pull request's own run. The rows in band `S34600-S34699` prove each
census observes the live tree.

## More Information

#624; SPEC-334 (row 1.5, R3); ADR-335; ADR-345 (D1, D4, D6); ADR-350 (D2); ADR-355; #623 (the
engine core); #635 and #639 (the XP crate and its edge); #641 (the FSRS-7 crate's edge); UniFFI's
manual on library mode; the Rust Reference on linking more than one static library.
