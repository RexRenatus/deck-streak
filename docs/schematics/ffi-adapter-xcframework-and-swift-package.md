# Schematic: the FFI adapter, its XCFramework build and the Swift package that would consume it

Kind: component, then two data flows (one call, one build), then the generator's two builds. Read
at DeckStreak `spike/ffi-umbrella-336` (`crates/ffi/Cargo.toml`, `crates/ffi/src/lib.rs`,
`crates/ffi/src/allow_list.rs`, `crates/ffi/src/engine.rs`,
`crates/ffi/src/bin/uniffi-bindgen-swift.rs`, `crates/ffi/tests/bindings_generator.rs`,
`.github/workflows/xcframework.yml`, `docs/CONTEXT-MAP.md`). Decided by ADR-345; built by
SPEC-336. The Swift package is drawn, dashed, as the consumer the framework is shaped for: SPEC-336
section 5 writes no Swift, no Xcode project and no client (#347).

## The components, and the one edge the adapter has

`deck-streak-ffi` depends on the engine and on no crate of this workspace (R4, ADR-345 D1). The
daemon does not compose it: a native client links it into its own binary.

```mermaid
flowchart LR
  subgraph client["a native client (not built by this spike, #347)"]
    app["the iPhone and iPad app"]
    pkg["a Swift package: a binary target for the XCFramework, and a Swift target holding the generated bindings"]
    app --> pkg
  end
  subgraph artifact["the xcframework artifact"]
    xcf["DeckStreakFFI.xcframework"]
    dev["slice ios-arm64: the device static library and its headers"]
    sim["slice ios-arm64-simulator: the simulator static library and its headers"]
    hdr["Headers: deck_streak_ffiFFI.h and module.modulemap, module deck_streak_ffiFFI"]
    swift["bindings/deck_streak_ffi.swift, which imports deck_streak_ffiFFI"]
    xcf --> dev
    xcf --> sim
    dev --> hdr
    sim --> hdr
  end
  subgraph ffi["deck-streak-ffi (the adapter at the edge)"]
    lib["the library: Engine, Engine::run, EngineRefusal, and the UniFFI scaffolding"]
    allow["allow_list: ALLOW_LIST, five pairs, and allowed()"]
    gen["the binary uniffi-bindgen-swift"]
    lib --> allow
  end
  subgraph engine["Anki's engine (the pinned fork, ADR-058)"]
    backend["Backend::run_service_method"]
  end
  subgraph ds["DeckStreak's contexts and the daemon"]
    ctx["every context, the coordination crate and deckstreakd"]
  end
  pkg -. "links" .-> xcf
  pkg -. "compiles" .-> swift
  swift -- "calls through the C module" --> hdr
  lib -- "its one dependency, an external crate" --> backend
  gen -- "reads the device library in library mode" --> lib
```

No arrow joins `ds` and `ffi`: the context map's line for the adapter reads `depends on: nothing`,
and the ddd probe holds the manifest equal to it.

## One call, from the client to the engine and back

```mermaid
flowchart TD
  call["Engine.run(service, method, input bytes)"] --> lookup{"allowed(service, method)"}
  lookup -- "None: the pair is not one of the five" --> refused(["EngineRefusal::NotAllowed with the pair, before the engine sees the call"])
  lookup -- "Some(call)" --> run["Backend::run_service_method(service, method, input)"]
  run -- "Ok(bytes)" --> answer(["the response's protobuf bytes"])
  run -- "Err(encoded BackendError)" --> engine_refused(["EngineRefusal::Engine carrying the engine's encoded error"])
  refused --> serving["the engine keeps serving: the next allowed call is answered"]
  engine_refused --> serving
```

The five pairs: (3, 0) `OpenCollection`, (7, 13) `GetDeckNames`, (13, 3) `GetQueuedCards`,
(13, 4) `AnswerCard` and (3, 8) `Undo` (R2). A1 to A6 round-trip each one, and refuse four
unlisted pairs, on a synthetic collection the test builds.

## One build of the XCFramework

`xcframework.yml` runs on a pull request into `dev` that changes `crates/ffi/**`, `Cargo.lock` or
the workflow itself, and on a dispatch, on a macOS runner the hardening test admits to this file
alone (R9, A7). Every step that fails stops the job; the report and the upload run regardless.

```mermaid
flowchart TD
  trigger["pull_request into dev on the three paths, or workflow_dispatch"] --> tool["the pinned toolchain and the two iOS targets"]
  tool --> protoc["protoc, its archive checked against its digest"]
  protoc --> libs["cargo rustc --release --lib --crate-type staticlib, for aarch64-apple-ios then aarch64-apple-ios-sim, clocked"]
  libs --> bind["cargo run --features bindgen --bin uniffi-bindgen-swift, library mode over the device library, clocked"]
  bind --> out["bindings/: deck_streak_ffi.swift, deck_streak_ffiFFI.h, module.modulemap"]
  out --> mm{"does module.modulemap name a _Builtin_ module?"}
  mm -- "yes" --> fail(["the job fails: the modulemap check reads fail"])
  mm -- "no" --> assemble["xcodebuild -create-xcframework from both slices and the headers"]
  assemble --> consumer{"swiftc -typecheck of the bindings against each slice's headers"}
  consumer -- "either slice refuses" --> fail2(["the job fails: the consumer check reads fail"])
  consumer -- "both typecheck" --> report["report.md and the run summary: build time, bindings time, both checks, each slice's size, the runner's CPUs"]
  fail --> report
  fail2 --> report
  report --> upload(["the xcframework artifact: the framework, the bindings and the report"])
```

## The generator's two builds

The generator is the adapter's own binary, so its bindings come from the bindings crate the library
links (ADR-345 D3). Every build of the crate builds the binary, and only the `bindgen` feature puts
the generator's code in it (ADR-345 D5, R10).

```mermaid
flowchart TD
  build["a build of deck-streak-ffi"] --> feature{"is the bindgen feature on?"}
  feature -- "no: every workspace build, every test run, the mutation run" --> refuse["main writes one line to stderr naming the feature and the command that builds it"]
  refuse --> exit2(["exit status 2, nothing on stdout, nothing written"])
  feature -- "yes: the workflow's bindings step alone" --> generate["main runs uniffi::uniffi_bindgen_swift()"]
  generate --> files(["the three generated files, as before the feature-less build learned to refuse"])
  exit2 -. "A8 asserts it, and kills the mutant that empties main (row S33601)" .-> test["bindings_generator::a8_the_generator_refuses_a_build_without_its_feature"]
```

Neither arm is a function of its own: `main` holds both, each a statement behind its `cfg`, so
cargo-mutants lists one mutant for the binary, `replace main with ()`, and the default-feature
mutation run builds the arm that mutant changes. A function behind `cfg(feature = "bindgen")` would
be listed, never built, and read missed.

## The settle census and the adapter's feature

The settle census is progression's test of its own invariant, that only coordination settles
(SPEC-072, ADR-197). It refused every member that declared a feature, because it compiled none,
so the adapter's `bindgen` feature made it refuse the real tree. It now compiles every build the
members' features allow (ADR-345 D6, R11) and judges the adapter's code as it judges any member's.

```mermaid
flowchart TD
  meta["cargo metadata with every feature on"] --> edges["the graph: every optional edge a feature can turn on"]
  edges --> scripts{"a build script that reaches progression?"}
  scripts -- "yes" --> refuse1(["refused: it can name settle"])
  scripts -- "no" --> members["each member's declared features, as package/feature"]
  members --> forced{"does another package turn one on, by naming it or keeping its default?"}
  forced -- "yes" --> refuse2(["refused by name: the census cannot compile the member without it"])
  forced -- "no" --> bound{"more than three features declared?"}
  bound -- "yes" --> refuse3(["refused by name: the bound on the combinations"])
  bound -- "no" --> combos["every combination, the empty one first"]
  combos --> check["cargo check per selection, per combination, per pass: --no-default-features, then --features with the combination"]
  check --> uses["every use of settle rustc reports, in any of those builds"]
  uses --> judge{"outside coordination?"}
  judge -- "yes" --> refuse4(["refused for the call"])
  judge -- "no" --> pass(["accepted: the adapter, under and without bindgen, holds no use"])
```

A member whose feature another package turns on is refused rather than compiled, because that
feature is on in every workspace build, so code under its absence would be compiled in no pass,
though a build of the member alone compiles it. The census admits no member by name.

## Amendment: where the build is triggered (SPEC-344, ADR-355)

The pull-request trigger described under "One build of the XCFramework" now lives in
`apple-on-change.yml`, which calls `xcframework.yml` and also watches `ios/**`, `Cargo.toml` and
`rust-toolchain.toml`. A run on every release tag is added by `apple-on-tag.yml`, which calls the
same job body. `xcframework.yml` itself takes `workflow_call` and `workflow_dispatch`. See
`apple-build-on-change-and-on-tag.md`.

## Amendment: the call's spelling (SPEC-344, ADR-355)

The CCALL and TCALL labels and the edge label above read `uses $/.github/workflows/xcframework.yml`
and `"$/.github/workflows/FILE"`, the self-repository form; their endpoints are unchanged.
