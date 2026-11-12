# Schematic: the umbrella FFI crate, the app's one static library, and the Swift package that wraps it

Kind: component (before and after), then the build's data flow (before and after), then the
closure the change caller follows. Read at DeckStreak `dev` `ccd36df611ed4ca32d239c5b557af52db243fa2c`
(`crates/ffi/Cargo.toml`, `crates/ffi/src/lib.rs`, `crates/web-engine/Cargo.toml`, `Cargo.toml`,
`Cargo.lock`, `.github/workflows/xcframework.yml`, `docs/CONTEXT-MAP.md`), with #656 at
`d295cd886a520ea61387723c0b081fed29a33c5d` (`ios/EnginePackage/Package.swift`, `ios/project.yml`,
`.gitignore`, its `xcframework.yml`) and #659 at `58d03e661a87c1d2e56ba184a7a7bc1e1864311b`
(`scripts/tests/test_ci_workflows.py`, SPEC-344), and #623's design for the engine core. Decided by
ADR-357; built by SPEC-346. Amends `ffi-adapter-xcframework-and-swift-package.md`, whose
Swift package is drawn dashed: it now exists.

## The components, before

At `dev`, with #656 and #659 open. The adapter depends on the engine alone (`crates/ffi/Cargo.toml`
lines 14 to 19), declares no crate type, and its static library is made by one workflow command
(`xcframework.yml` line 65). The Swift package is #656's (`ios/EnginePackage/Package.swift` line
14). Nothing counts the static libraries or the UniFFI components that enter the app.

```mermaid
flowchart LR
  subgraph rust["the workspace"]
    engine["Anki's engine, the pinned fork"]
    ffi["deck-streak-ffi, the FFI adapter: allow-list, Engine, UniFFI scaffolding, the generator binary"]
    ctx["every context, the coordination crate and the daemon"]
    web["deck-streak-web-engine: cdylib and rlib, the only declared crate type"]
    ffi --> engine
    web --> engine
  end
  subgraph job["xcframework job, the macOS runner"]
    lib["libdeck_streak_ffi.a per target, from cargo rustc --crate-type staticlib"]
    bind["bindings: deck_streak_ffi.swift, deck_streak_ffiFFI.h, module.modulemap"]
    xcf["DeckStreakFFI.xcframework: ios-arm64 and ios-arm64-simulator"]
  end
  subgraph ios["ios, open in issue 616"]
    pkg["EnginePackage: binaryTarget deck_streak_ffiFFI and target DeckStreakFFI"]
    harness["the harness app, on simulators"]
  end
  ffi -- "built as" --> lib
  lib -- "read in library mode" --> bind
  lib --> xcf
  bind --> xcf
  xcf -. "placed from the run's artifact" .-> pkg
  bind -. "deck_streak_ffi.swift placed" .-> pkg
  harness --> pkg
```

## The components, after

The adapter is the umbrella FFI crate, grown in place (ADR-357 D1). It depends on the engine core
(#623), and the XP crate and the FSRS-7 crate join it by edges drawn in #639 and #641 (dashed: not
built by this delivery). UniFFI is named by the umbrella alone (D2). The required CI's census and
the macOS job's two new steps hold one library and one component (D3).

```mermaid
flowchart LR
  subgraph rust["the workspace"]
    engine["Anki's engine, the pinned fork"]
    core["deck-streak-engine-core, issue 623: no UniFFI, no feature, no crate type"]
    umb["deck-streak-ffi, the umbrella FFI crate: the one UniFFI component"]
    xp["the I/O-free XP crate, issues 635 and 639"]
    f7["the FSRS-7 crate, issue 641"]
    web["deck-streak-web-engine: cdylib and rlib"]
    umb --> core
    core --> engine
    umb -. "edge drawn in issue 639" .-> xp
    umb -. "edge drawn in issue 641" .-> f7
    web --> engine
  end
  subgraph census["the required ci, on Linux, every pull request"]
    c1["no member declares staticlib, the declared crate types are the web engine's"]
    c2["uniffi named by the umbrella alone, in the manifests and the lockfile"]
    c3["every workflow --crate-type command builds the umbrella, in xcframework.yml"]
    c4["one local binary target, no remote package, no tracked built library"]
    c5["the change caller's crate globs equal the umbrella's build closure"]
  end
  subgraph job["xcframework job, the macOS runner"]
    lib["libdeck_streak_ffi.a per target: the app's one Rust static library"]
    bind["bindings: one module's three files"]
    xcf["DeckStreakFFI.xcframework: one library per slice"]
  end
  subgraph ios["ios"]
    pkg["EnginePackage: one binaryTarget, deck_streak_ffiFFI, by local path"]
    harness["the harness app, issue 616"]
    shell["the app shell, issue 625"]
  end
  umb -- "built as" --> lib
  lib -- "read in library mode" --> bind
  lib --> xcf
  bind --> xcf
  xcf -. "placed from the run's artifact" .-> pkg
  harness --> pkg
  shell -.-> pkg
  census -. "reads" .-> rust
  census -. "reads" .-> job
  census -. "reads" .-> ios
```

No arrow joins a context to the umbrella or to the core: the core's graph test (#623) refuses a
member naming either adapter, and the map's line for the umbrella keeps `depends on: engine-core`.

## The build, before

`apple-on-change.yml` (#659) calls the one job body on a pull request into `dev` touching its paths.

```mermaid
flowchart TD
  trigger["pull request into dev on crates/ffi/**, ios/**, Cargo.lock, Cargo.toml, rust-toolchain.toml or the two workflows"] --> tool["the pinned toolchain and the two iOS targets"]
  tool --> protoc["protoc, checksum-verified"]
  protoc --> libs["cargo rustc -p deck-streak-ffi --lib --crate-type staticlib, device then simulator"]
  libs --> bind["the bindings, library mode over the device library"]
  bind --> mm{"does the modulemap name a _Builtin_ module?"}
  mm -- "yes" --> fail1(["fail"])
  mm -- "no" --> xcf["xcodebuild -create-xcframework from both slices"]
  xcf --> consumer{"does the Swift typecheck against each slice's headers?"}
  consumer -- "no" --> fail2(["fail"])
  consumer -- "yes" --> fixture["the harness's synthetic collection, written by this commit's engine"]
  fixture --> report["the report: times, both checks, each slice's size"]
  report --> upload(["the xcframework artifact"])
  upload --> harness["the harness job: place into EnginePackage, generate the project, test on simulators"]
```

A change to `crates/engine-core/**` alone, once #623 lands, starts nothing here, though it changes
the library.

## The build, after

Two steps join the job, each after the step whose output it reads, and each writes `pass` or `fail`
to the report before it exits. Both use POSIX tools only, so a Linux test runs each step's own
script over planted trees (SPEC-346 A6, A7).

```mermaid
flowchart TD
  trigger["pull request into dev on the same paths and crates/engine-core/**"] --> tool["the pinned toolchain and the two iOS targets"]
  tool --> protoc["protoc, checksum-verified"]
  protoc --> libs["cargo rustc -p deck-streak-ffi --lib --crate-type staticlib, device then simulator"]
  libs --> bind["the bindings, library mode over the device library"]
  bind --> onemod{"the bindings hold one module: exactly the three files, one module declared?"}
  onemod -- "no: a second component's source or header, or a second module" --> fail0(["fail, one-module reads fail"])
  onemod -- "yes" --> mm{"does the modulemap name a _Builtin_ module?"}
  mm -- "yes" --> fail1(["fail"])
  mm -- "no" --> xcf["xcodebuild -create-xcframework from both slices"]
  xcf --> onelib{"the XCFramework holds one Rust library: two .a files, one per slice, each libdeck_streak_ffi.a?"}
  onelib -- "no" --> fail3(["fail, one-library reads fail"])
  onelib -- "yes" --> consumer{"does the Swift typecheck against each slice's headers?"}
  consumer -- "no" --> fail2(["fail"])
  consumer -- "yes" --> fixture["the harness's synthetic collection, written by this commit's engine"]
  fixture --> report["the report: times, all four checks, each slice's size"]
  fail0 --> report
  fail3 --> report
  report --> upload(["the xcframework artifact"])
  upload --> harness["the harness job, unchanged"]
```

## The closure the change caller follows

```mermaid
flowchart TD
  members["every workspace member: crates/NAME/Cargo.toml, its package name and directory"] --> start["the umbrella, deck-streak-ffi"]
  start --> first["the umbrella's normal, build, target and dev tables: the workspace members they name"]
  first --> walk["each member reached: its normal, build and target tables, repeated until nothing new"]
  walk --> closure(["the closure's directories"])
  closure --> globs{"equal to the change caller's crates/ globs?"}
  globs -- "yes" --> ok(["pass"])
  globs -- "no: a crate the library links goes unwatched, or a glob watches a crate it does not" --> red(["the census fails, naming the difference"])
```

A dependency on a non-member (the engine, UniFFI) is outside the workspace and is watched through
`Cargo.lock` and `Cargo.toml`, which are already among the paths.
