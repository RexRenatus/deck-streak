# SPEC-346: the app links one Rust static library, the umbrella FFI crate, and one Swift package wraps it

- **Wave:** the app campaign. **Issue:** #624. **Campaign row:** SPEC-334 row 1.5 (R3, and section
  6's risk "two Rust static libraries in one app clash"). **Context(s):** `deck-streak-ffi` (the
  umbrella FFI crate; no edge changes) and the workflows and their census (no bounded context).
- **Decided by:** ADR-335 (one umbrella crate, the app's one Rust static library), ADR-345 (the
  allow-listed adapter, the exact UniFFI pin, the static library built only by
  `cargo rustc --crate-type staticlib` in the macOS job), ADR-350 (the engine's Swift package fed by
  its own run's artifact), ADR-355 (one Apple job body, a change caller and a tag caller), and
  ADR-357 (the umbrella grows in place, UniFFI lives in it alone, how the one-library rule is
  held, and the change caller follows the umbrella's crates).
- **Status:** one pull request delivers R1 to R7. **Base:** live `dev`, after #623's first pull
  request, #622's (#659) and #616's (#656) have landed. **Mutation band:** `S34600-S34699`.

## 1. The problem, measured

Read at DeckStreak `dev` `ccd36df611ed4ca32d239c5b557af52db243fa2c` (DEV), with #656 at
`d295cd886a520ea61387723c0b081fed29a33c5d` (HARNESS; its merge-base with DEV is `7507d8a2`, so its
own change is the three-dot diff), #659 at `58d03e661a87c1d2e56ba184a7a7bc1e1864311b` (APPLE) and
#660 at `9afb08b5ba04933b90c8926d9a273fa52805e050` (FSRS7). `R=<the DeckStreak checkout>` in every
command. Live `dev` is one merge ahead of DEV (#655), whose 34 paths are web, `ci.yml`, its own
SPEC's documents and rows, and `test_ci_workflows.py`; none is a crate, a lockfile, a Swift file or
`xcframework.yml` (`gh pr view 655 --json files --jq '.files[].path'`, 34 paths). #659 landed at
`611d7427` and #656 at `ac4fbdae`, and the build re-read every figure it relies on at its cut.

### 1.1 What the adapter builds, at DEV

| # | measured | figure | command |
|---|---|---|---|
| M1 | the adapter's manifest | package `deck-streak-ffi`; `[dependencies]` the engine and `uniffi` (lines 14 to 19); one feature, `bindgen` (21 to 24); one `[[bin]]`, `uniffi-bindgen-swift` (28 to 30); **no `[lib]` section and no crate type** | `git -C $R show DEV:crates/ffi/Cargo.toml \| cat -n`; `\| grep -c '^\[lib\]'` (0) |
| M2 | its UniFFI surface | `#![forbid(unsafe_code)]` (line 14) and the crate's one `uniffi::setup_scaffolding!()` (line 16); the derives `uniffi::Error`, `uniffi::Object`, `uniffi::export`, `uniffi::constructor` sit in `src/engine.rs` (lines 12, 54, 59, 66) and nowhere else in `crates/` | `git -C $R show DEV:crates/ffi/src/lib.rs \| cat -n`; `git -C $R grep -n -E 'setup_scaffolding\|uniffi::' DEV -- crates/` (6 lines, all in `crates/ffi`) |
| M3 | the pin | the root manifest pins UniFFI exactly, default features off (line 124); `Cargo.lock` holds one entry each for `uniffi`, `uniffi_bindgen`, `uniffi_core`, `uniffi_macros`, `uniffi_meta` and `uniffi_udl` at the pinned release, and `uniffi_internal_macros` one patch later (the seven `name` lines at 5329, 5344, 5367, 5379, 5392, 5409 and 5420; ADR-345 records why) | `git -C $R show DEV:Cargo.toml \| sed -n '119,124p'`; `git -C $R show DEV:Cargo.lock \| grep -n -A1 '^name = "uniffi'` |
| M4 | who depends on UniFFI | exactly one package: `deck-streak-ffi` (`Cargo.lock` line 1103, inside its entry at 1098 to 1104) | `git -C $R show DEV:Cargo.lock \| grep -n '^ "uniffi",'` |
| M5 | the workspace's crate types | 27 members (`members = ["crates/*"]`, line 4); one declares a crate type, the web engine's `["cdylib", "rlib"]` (`crates/web-engine/Cargo.toml` line 16, ADR-348); **0 declare `staticlib`** | `git -C $R grep -n -E 'crate-type\|crate_type\|staticlib' DEV -- '*.toml'` (1 line) |
| M6 | where a static library is made | one command in one workflow: `cargo rustc --locked --release -p deck-streak-ffi --lib --crate-type staticlib --target "$target"` (`xcframework.yml` line 65), run for the device and the simulator target | `git -C $R grep -n staticlib DEV -- .github/workflows` (1 line) |
| M7 | unmangled exports in members' own code | 27 of 27 members inherit the workspace lints (`[lints] workspace = true`), and the workspace sets `unsafe_code = "forbid"` (root manifest lines 136 to 137), which refuses `no_mangle` and `export_name`; 0 `no_mangle` attributes in `crates/` | `for c in $(git -C $R ls-tree --name-only DEV crates/); do git -C $R show DEV:$c/Cargo.toml \| grep -A1 '^\[lints\]' \| grep -c 'workspace = true'; done \| sort \| uniq -c` (27 x 1) |
| M8 | committed built libraries | 0 files ending `.a`, `.dylib` or `.so`, and 0 paths inside an `.xcframework` or `.framework` directory, at DEV and at HARNESS | `git -C $R ls-tree -r --name-only <sha> \| grep -cE '\.(a\|dylib\|so)$\|\.xcframework/\|\.framework/'` (0, 0) |

### 1.2 What the XCFramework job produces, checks and publishes, at DEV

One job, `xcframework`, on the admitted macOS runner (`xcframework.yml` lines 33 to 35), started by
a pull request into `dev` on `crates/ffi/**`, `Cargo.lock` or the workflow itself, or by a dispatch
(lines 13 to 21).

| lines | step | produces or checks |
|---|---|---|
| 49 to 52 | the pinned toolchain and the two iOS targets | `aarch64-apple-ios` and `aarch64-apple-ios-sim` |
| 53 to 57 | protoc, checksum-verified | the protobuf compiler the engine's build needs |
| 58 to 67 | the two static libraries | `libdeck_streak_ffi.a` per target (M6), clocked |
| 68 to 77 | the Swift bindings, in library mode over the device library | `bindings/`: `deck_streak_ffi.swift`, `deck_streak_ffiFFI.h`, `module.modulemap`, module `deck_streak_ffiFFI` |
| 78 to 85 | the modulemap check | fails on a `_Builtin_` clang module |
| 86 to 94 | the XCFramework | `DeckStreakFFI.xcframework`, slices `ios-arm64` and `ios-arm64-simulator`, each slice's size to the report |
| 95 to 105 | the consumer check | the generated Swift typechecks against each slice's headers |
| 106 to 138 | the report | build and bindings time, both checks, each slice's size |
| 139 to 148 | the upload | the `xcframework` artifact: the framework, the bindings, the report |

**Nothing counts the libraries or the bindings it produced.** A second crate carrying UniFFI
scaffolding inside the library would add its own Swift source and header to `bindings/`, and
nothing asserts that each slice holds the umbrella's library and no other; every step above would
still pass. `git -C $R show
DEV:.github/workflows/xcframework.yml | sed -n '58,105p'`.

### 1.3 The Swift package, and the Apple build's callers

| # | measured | figure | command |
|---|---|---|---|
| M9 | Swift packages | DEV 0; APPLE 0; HARNESS 2: `ios/EnginePackage/Package.swift` (a `binaryTarget` `deck_streak_ffiFFI` at the local path `DeckStreakFFI.xcframework`, line 14, and a target `DeckStreakFFI` for the generated Swift with the engine's link settings, lines 15 to 25) and `ios/HarnessWire/Package.swift` (a codec, no binary target) | `git -C $R ls-tree -r --name-only <sha> \| grep -c 'Package\.swift$'`; `git -C $R show HARNESS:ios/EnginePackage/Package.swift \| cat -n` |
| M10 | how the package is fed | the `harness` job (needs `xcframework`) downloads the same run's `xcframework` artifact and places the framework and `deck_streak_ffi.swift` into `ios/EnginePackage/` (HARNESS `xcframework.yml` lines 273 to 283); git ignores both (HARNESS `.gitignore` lines 42 and 43); ADR-350 D2 rejected a committed framework and a remote binary target | `git -C $R show HARNESS:.github/workflows/xcframework.yml \| sed -n '254,283p'` |
| M11 | who builds the package | the `harness` job, through the app it links, for the simulator only (`-destination "platform=iOS Simulator,…"`, HARNESS lines 297 to 323); no job links the device slice into anything | `git -C $R show HARNESS:.github/workflows/xcframework.yml \| grep -n destination` |
| M12 | what the Apple callers watch | APPLE's change caller paths: `crates/ffi/**`, `ios/**`, `Cargo.lock`, `Cargo.toml`, `rust-toolchain.toml` and the two workflow files (`APPLE_PATHS`, `test_ci_workflows.py` lines 590 to 598); SPEC-344 says "the adapter depends on the engine and the generator alone, so no other workspace crate is an input" (lines 41 to 42) and leaves "the XCFramework's steps" to #624 (line 215) | `git -C $R show APPLE:scripts/tests/test_ci_workflows.py \| sed -n '590,598p'`; `git -C $R show APPLE:docs/specs/SPEC-344-the-apple-build-runs-from-one-job-body-on-a-change-and-on-every-release-tag.md \| sed -n '41,42p;215,216p'` |
| M13 | what #659 changed under `.github` when it landed | `apple-on-change.yml` and `apple-on-tag.yml` added, `xcframework.yml` made the callee: 3 files, 60 insertions, 18 deletions | `git -C $R diff --stat ac4fbdae 611d7427 -- .github` |

### 1.4 What #623 changes for the adapter, and what "umbrella" must mean

#623's first pull request adds `crates/engine-core` (package `deck-streak-engine-core`): the one
client-side holder of the engine, depending on no DeckStreak crate, declaring no feature and no
`staticlib`. The adapter then depends on the core in place of the engine (the engine stays a
dev-dependency for its synthetic collections), and the context map reads `deck-streak-ffi …
depends on: engine-core`. Its graph test refuses any member but the two client adapters naming the
core, and **any member naming an adapter**. Its second pull request adds the native exempt entry's
UniFFI types (`uniffi::Enum` and `uniffi::Error` derives) **in the adapter**, not in the core.

So the umbrella is `deck-streak-ffi` grown in place. A new crate that re-exported the adapter
would be refused by #623's graph test; one that named the core directly would be refused too
(ADR-357 D1).

After #623, **a change to `crates/engine-core/**` changes the static library and starts no Apple
build**: M12's paths name `crates/ffi/**` alone.

### 1.5 The XP crate and the FSRS-7 crate: what they are, and how two schedulers share a library

| # | measured | figure | command |
|---|---|---|---|
| M14 | the XP crate | not in the workspace: the XP rules live in `deck-streak-progression` (`src/review_xp.rs`, `src/level.rs`), which depends on the kernel, ingest (and so the engine) and `sqlx`; #635 extracts an I/O-free crate | `git -C $R show DEV:crates/progression/Cargo.toml \| grep -n workspace` |
| M15 | `fsrs` in DEV's lockfile | one entry, from the registry (lines 1715 to 1717); its one dependent is `anki` (line 90) | `git -C $R show DEV:Cargo.lock \| grep -n -A2 '^name = "fsrs"$'`; `\| grep -n '^ "fsrs'` |
| M16 | `fsrs` at FSRS7 | two entries of the same name and the same version, one from the registry (lines 1729 to 1731) and one from a pinned git revision (1746 to 1748); `deck-streak-fsrs7` depends on the second and dev-depends on the first; its test `crates/fsrs7/tests/coexistence.rs` links both into one binary | `git -C $R show FSRS7:Cargo.lock \| grep -n -E '"fsrs( \|")'`; `git -C $R show FSRS7:crates/fsrs7/Cargo.toml \| cat -n` |

**How two `fsrs` packages coexist in one static library.** Cargo identifies a package by its name,
version and source, and compiles each with its own `-C metadata` hash, so the two `fsrs` builds are
two crates with distinct disambiguators; every Rust symbol is mangled with that hash, and the static
library bundles both without a clash. What would break it: an unmangled symbol defined by both
(`no_mangle`, `export_name`, an `extern` definition); C objects a build script compiles into both
with the same global names; one `links` key in both, which cargo refuses as a graph; and UniFFI
scaffolding in both, whose exported symbols are unmangled and named by crate, and both crates are
named `fsrs`. Neither copy carries UniFFI, and R2 keeps it in the umbrella. The first two are not
measured from source here; the evidence is a link: FSRS7's coexistence test links both on Linux,
and the harness job's app link fails on a duplicate strong symbol.

### 1.6 The census conventions this delivery meets

| rule | where | what it asks of this delivery |
|---|---|---|
| rows | `scripts/mutation-rows.json` header: SPEC-NNN owns `S<NNN>00` to `S<NNN>99` in `scripts/mutation-rows.d/`; `SCRIPT_MUTATIONS` rows have six cells (stem, path from the root, find, replace, behaviour, killer `module.Class.method`) | one band fragment, section 7's rows |
| workflow reads | `test_ci_workflows.py`: every workflow read in a test module goes through `workflow_file_text` (lines 93 to 98, `load` 1432); any other read or step run in a module that imports the reader is listed in `NOT_WORKFLOW_READS` (line 3905) with its exact count | each new site listed; no existing count widened |
| the admitted runner | `ADMITTED_RUNNERS` (line 39): the macOS runner in `xcframework.yml` alone | no new workflow, no new runner |
| the settle census | `crates/progression/tests/xp_census.rs` line 1803: at most three declared features in the workspace (one today, `bindgen`), and a member whose feature another package turns on is refused (ADR-345 D6) | no feature added; a later member joins the umbrella by an edge, never by a feature |
| Swift | no census over Swift sources exists at DEV, HARNESS or APPLE | this delivery adds no Swift file; R4 holds the engine package to its manifest alone |

## 2. Requirements

R1. The umbrella FFI crate is `deck-streak-ffi`, grown in place: the app's one Rust static library
    is built from it alone, and its library name, its UniFFI namespace, its Swift module
    `deck_streak_ffiFFI` and the Swift package's binary target keep their names. The engine core is
    its one workspace dependency at this delivery (#623); the XP crate and the FSRS-7 crate each
    join later by an unconditional dependency edge, drawn in the change that first uses it, with the
    context map's line amended in that change (ADR-357 D1). The crate's manifest description and
    the context map's note for it name it the umbrella FFI crate.
R2. UniFFI lives in the umbrella alone: no other workspace member names `uniffi` in any dependency
    table (a target table, a dotted table and a `package = "uniffi"` rename included); in
    `Cargo.lock`, `uniffi`'s only dependent is `deck-streak-ffi`; every member inherits the
    workspace lints, and the workspace sets `unsafe_code = "forbid"`.
R3. No workspace member declares the `staticlib` crate type, and the crate types members declare
    are exactly the web engine's `cdylib` and `rlib`. Across every workflow, every cargo command
    that names `--crate-type` is a `cargo rustc` of `-p deck-streak-ffi --lib --crate-type
    staticlib`, and those commands sit in `xcframework.yml` alone.
R4. The Swift side links one Rust library: across every `Package.swift` under `ios/`, exactly one
    `.binaryTarget`, named `deck_streak_ffiFFI`, by the local path `DeckStreakFFI.xcframework` and
    never by URL; no package declares a remote package dependency; no generator spec under `ios/`
    (each tracked `ios/*.yml`) declares a framework or Carthage dependency or a remote package,
    only local packages; `ios/EnginePackage/` commits `Package.swift` alone. No built library (a
    file ending `.a`, `.dylib` or `.so`, or a path inside an `.xcframework` or `.framework`
    directory) is committed anywhere in the tree.
R5. The `xcframework` job checks its own outputs. A step named `the bindings hold one module`,
    after the bindings step, fails unless `bindings/` holds exactly `deck_streak_ffi.swift`,
    `deck_streak_ffiFFI.h` and `module.modulemap` and the modulemap declares exactly one module. A
    step named `the XCFramework holds one Rust library`, after the XCFramework step, fails unless
    the framework holds exactly two static libraries, one in each slice, each named
    `libdeck_streak_ffi.a`. Each writes `pass` or `fail` to the report (`one-module`,
    `one-library`), the report gains a row for each, and each script uses POSIX tools only, so a
    Linux test runs it.
R6. The Apple change caller watches every crate the umbrella links: its `crates/` globs equal
    `crates/<dir>/**` for each workspace member of the umbrella's build closure, which is the
    umbrella, every workspace member its normal, build, target and dev tables name, and every
    workspace member those members' normal, build and target tables name in turn. At this
    delivery that is `crates/ffi/**` and `crates/engine-core/**`. SPEC-344's A1 list gains
    `crates/engine-core/**`, and a change to `crates/engine-core/src/lib.rs` starts the build.
R7. Nothing else changes: no `[lib]` section, `.cargo/config.toml`, environment variable, feature,
    workflow file or runner; the harness jobs, the tag caller, `deny.toml`, `Cargo.lock` and the
    root `Cargo.toml` are unchanged; no Swift file is added.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the crate-type census over every member's manifest is exactly `{deck-streak-web-engine: [cdylib, rlib]}` and names no `staticlib`; a planted scratch workspace whose member declares `staticlib` under `[lib] crate-type`, and one under `crate_type`, is each refused naming the member; it prints the manifests examined | `test_one_static_library.py` `NoMemberIsAStaticLibrary.test_no_member_declares_the_staticlib_crate_type` |
| A2 | `uniffi` is named by `deck-streak-ffi` alone, in the manifests and in `Cargo.lock`, and every member inherits the forbidding lints; planted members naming `uniffi` in `[dependencies]`, in a target table, in a dotted table and by a `package =` rename, a planted lockfile entry depending on `uniffi`, and a planted member without `[lints] workspace = true` are each refused by name | `test_one_static_library.py` `UniffiLivesInTheUmbrella.test_only_the_umbrella_names_uniffi` |
| A3 | the Swift packages hold one binary target, `deck_streak_ffiFFI` at `DeckStreakFFI.xcframework`; no `.package(url:`; no tracked `ios/*.yml` names a framework or Carthage dependency or a remote package; it prints the specs examined; `ios/EnginePackage/` commits `Package.swift` alone; planted trees with a second binary target, a URL binary target, a remote package, a project framework dependency, a second generator spec, `ios/app.yml`, holding either, a committed Swift source under `ios/EnginePackage/` and a tree with no binary target are each refused by name | `test_one_static_library.py` `TheSwiftSideLinksOneLibrary.test_the_engine_package_has_one_local_binary_target` |
| A4 | the tracked tree holds no built library; a planted listing holding `x/libfoo.a`, `y/Foo.xcframework/Info.plist` and `z/libbar.dylib` is refused naming each; it prints the paths examined | `test_one_static_library.py` `TheSwiftSideLinksOneLibrary.test_no_built_library_is_committed` |
| A5 | every workflow's cargo commands naming `--crate-type` are exactly the umbrella's `cargo rustc … --crate-type staticlib`, in `xcframework.yml` alone, at least one; a planted `cargo rustc --locked -p deck-streak-engine-core --lib --crate-type staticlib` is refused naming the package | `test_ci_workflows.py` `TheAppHasOneRustStaticLibrary.test_only_the_umbrella_is_built_as_a_static_library` |
| A6 | the `xcframework` job holds `the bindings hold one module` after the bindings step; its own script, run under bash over a planted `bindings/` with the three files, exits 0 and writes `pass`; over one with a second crate's `.swift` and `FFI.h` added, and over a modulemap declaring two modules, it exits non-zero and writes `fail` | `test_ci_workflows.py` `TheAppHasOneRustStaticLibrary.test_the_bindings_hold_one_module` |
| A7 | the `xcframework` job holds `the XCFramework holds one Rust library` after the XCFramework step; its own script passes over a planted framework with `ios-arm64/libdeck_streak_ffi.a` and `ios-arm64-simulator/libdeck_streak_ffi.a`, and fails, writing `fail`, over one with a third library (`ios-arm64/libxp.a`), one whose simulator slice holds `libother.a` in place of the umbrella's, and one with a single slice | `test_ci_workflows.py` `TheAppHasOneRustStaticLibrary.test_the_xcframework_holds_one_rust_library_per_slice` |
| A8 | the change caller's `crates/` globs equal the umbrella's build closure's directories; over a planted manifest graph in which the umbrella gains a member, and that member gains a member, the census demands both paths by name | `test_ci_workflows.py` `TheAppHasOneRustStaticLibrary.test_the_change_caller_watches_every_crate_the_umbrella_links` |
| A9 | SPEC-344's A1, its list amended by R6 and a change to `crates/engine-core/src/lib.rs` among the paths that start the build | `test_ci_workflows.py` `TheAppleBuildRunsFromOneBody.test_the_change_caller_runs_the_build_on_each_apple_path` |
| A10 | on this pull request's run of the change caller, the job `apple / xcframework` runs its steps `the bindings hold one module` and `the XCFramework holds one Rust library` on the real outputs, and the report reads `pass` for each (macOS only: the steps run there, and a Linux runner cannot run `xcodebuild`) | the job `apple / xcframework`, those two steps, and the run's `xcframework` artifact: `xcframework-report/one-module` and `xcframework-report/one-library` |

The red each must show first, at the base the build cuts from:

- A1 to A5: not red. Each is a census over a property the base already holds (M2 to M9); each
  test's planted controls run in the same test and are refused, and section 7's rows prove each
  observes the live tree.
- A6: `AssertionError`: no step named `the bindings hold one module` in the `xcframework` job (the
  presence assertion runs before any script is run).
- A7: `AssertionError`: no step named `the XCFramework holds one Rust library`.
- A8: `AssertionError`: the change caller watches `['crates/ffi/**']`, the closure is
  `['crates/engine-core/**', 'crates/ffi/**']`.
- A9: `AssertionError`: the caller's `paths` differ from the amended list by
  `crates/engine-core/**`.
- A10: red by construction until the two steps exist; read from the macOS run, never locally.

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_one_static_library.py -k test_no_member_declares_the_staticlib_crate_type
A2: python3 -m unittest discover -s scripts/tests -p test_one_static_library.py -k test_only_the_umbrella_names_uniffi
A3: python3 -m unittest discover -s scripts/tests -p test_one_static_library.py -k test_the_engine_package_has_one_local_binary_target
A4: python3 -m unittest discover -s scripts/tests -p test_one_static_library.py -k test_no_built_library_is_committed
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_only_the_umbrella_is_built_as_a_static_library
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_bindings_hold_one_module
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_xcframework_holds_one_rust_library_per_slice
A8: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_change_caller_watches_every_crate_the_umbrella_links
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_change_caller_runs_the_build_on_each_apple_path
A10: gh run download <the change caller's run on this pull request> --name xcframework --dir a10 && grep -Fxq pass a10/xcframework-report/one-module && grep -Fxq pass a10/xcframework-report/one-library
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/xcframework.yml` | none (CI) | changed: the two R5 steps in the `xcframework` job, and their two report rows |
| `.github/workflows/apple-on-change.yml` | none (CI) | changed: `crates/engine-core/**` among its paths (R6) |
| `scripts/tests/test_one_static_library.py` | none (tests) | added: the manifest, lockfile, Swift package and tracked-tree censuses (A1 to A4), and `umbrella_closure`, the one reader of the umbrella's build closure, which `test_ci_workflows.py` imports |
| `scripts/tests/test_ci_workflows.py` | none (tests) | changed: `APPLE_PATHS` and A1's started paths (A9); the class `TheAppHasOneRustStaticLibrary` (A5 to A8); each new read or step-run site listed in `NOT_WORKFLOW_READS` with its count |
| `crates/ffi/Cargo.toml` | `deck-streak-ffi` | changed: the description names the umbrella FFI crate (R1); nothing else |
| `docs/CONTEXT-MAP.md` | docs | changed: the note on `deck-streak-ffi`'s line names the umbrella FFI crate; its `depends on:` is unchanged |
| `scripts/mutation-rows.d/S34600-S34699.json` | none (rows) | added: section 7's rows |
| `docs/specs/SPEC-346-the-app-links-one-rust-static-library-the-umbrella-ffi-crate.md` | docs | added: this SPEC |
| `docs/decisions/ADR-357-the-umbrella-ffi-crate-grows-in-place-and-the-app-links-one-rust-library.md` | docs | added |
| `docs/schematics/umbrella-ffi-crate-and-one-static-library.md` | docs | added |
| `docs/schematics/ffi-adapter-xcframework-and-swift-package.md` | docs | changed: an appended amendment pointing to the new schematic |
| `docs/red-first/SPEC-346.md` | docs | added |
| `changelog.d/umbrella-ffi-346.md` | docs | added |

Unchanged, by R7: the root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.cargo/`, every
`crates/*/src/**`, `ios/**`, `.github/workflows/apple-on-tag.yml`, `ci.yml`, `release.yml` and
`.github/rulesets/*`.

## 5. What this does NOT do

- It builds no XP crate and links none into the umbrella: #635 extracts the I/O-free crate and #639
  draws the umbrella's edge to it, with its context-map line and its UniFFI wrappers.
- It links no FSRS-7 crate: #641 draws that edge, and #660 measures the crate. The coexistence of
  the two `fsrs` packages in the app's library is proved by that delivery's link.
- It changes no engine core, dispatcher, allow-list or UniFFI export: those are #623's, whose first
  pull request rewires the umbrella through the core.
- It adds no Swift source, screen or app target, and no census over an app's own Swift: the app
  shell consumes the same package and brings its Swift and its census (#625).
- It links the device slice into nothing: the harness links the simulator slice; the app's
  unsigned device archive is the app shell's (#625), and signing, uploads and the cargo build's
  deployment target for a release are the TestFlight lane's (#634).
- It adds no simulator slice for another architecture; the spike decided the two arm64 slices
  (#615).
- It changes neither caller's trigger beyond R6's path, nor the harness jobs: those are #622's and
  #616's.
- It adds no mutation row that a Swift test kills (#650); row 07 plants into a package manifest and
  a Python census kills it. It takes no figure on a device (#629).

## 6. Risks

- **#656 has not landed, or lands with another package name or path.** A3 reads
  `ios/EnginePackage/Package.swift` and the binary target name; the build waits for #656 and
  re-reads both at its base.
- **#659's job body lands with other step names or other positions.** R5's steps are placed after
  the bindings step and after the XCFramework step by name; the build re-reads the callee at its
  base and anchors its rows there.
- **`test_ci_workflows.py` is edited by several open pull requests at once** (#655 has landed, #656
  and #659 are open). The build cuts from live `dev` after both land and keeps its additions in one
  class and one block of `NOT_WORKFLOW_READS` entries.
- **A UniFFI upgrade changes library mode's file names.** R5's first step then fails on the real
  outputs, loudly; the exact pin (ADR-345 D2) holds the version, and an upgrade is its own
  delivery.
- **A member reaches UniFFI through another crate's re-export, not by naming it.** That crate is
  then `uniffi`'s dependent in `Cargo.lock`, and A2 refuses it unless it is the umbrella.
- **A C library vendored by both `fsrs` copies clashes.** Neither the manifests nor the lockfile
  show it; the harness job's app link fails on it, and #641's delivery meets it first.

## 7. The mutation rows

Band `S34600-S34699`, one fragment, `SCRIPT_MUTATIONS` (six cells). Each killer selects one test.

| row | target | the mutant | killed by |
|---|---|---|---|
| 00 | `.github/workflows/xcframework.yml` | the one-module step's file-set test replaced by `true` | A6 |
| 01 | `.github/workflows/xcframework.yml` | the one-module step's module-count test replaced by `true` | A6 |
| 02 | `.github/workflows/xcframework.yml` | the one-library step's library-set test replaced by `true` | A7 |
| 03 | `.github/workflows/xcframework.yml` | the one-library step finds only files named `libdeck_streak_ffi.a`, so a library of another name goes unseen | A7 |
| 04 | `.github/workflows/xcframework.yml` | the static-library command's `-p deck-streak-ffi` to `-p deck-streak-engine-core` | A5 |
| 05 | `.github/workflows/apple-on-change.yml` | the `crates/engine-core/**` path removed | A8 |
| 06 | `crates/ffi/Cargo.toml` | a `[lib]` section declaring `crate-type = ["lib", "staticlib"]` inserted before `[features]` | A1 |
| 07 | `ios/EnginePackage/Package.swift` | a second `.binaryTarget` beside the first | A3 |
| 08 | `scripts/tests/test_one_static_library.py` | the closure's walk stops after the umbrella's own tables | A8 |
| 09 | `scripts/tests/test_one_static_library.py` | the `uniffi` census skips target tables | A2 |
| 10 | `scripts/tests/test_one_static_library.py` | the binary-target count `!= 1` read as `> 1` | A3 |
| 11 | `scripts/tests/test_one_static_library.py` | the built-library pattern loses its `.a` arm | A4 |
| 12 | `scripts/tests/test_one_static_library.py` | the generator-spec population narrowed back to `ios/project.yml` alone, so a second tracked `ios/*.yml` goes unread | A3 |

## 8. References

SPEC-334 (row 1.5, R3, section 6), SPEC-336 and ADR-345 (the adapter, the pin, the macOS job),
SPEC-339 and ADR-350 (the harness and the engine's Swift package), SPEC-344 and ADR-355 (the job
body and its callers), ADR-335 (the umbrella), ADR-348 (the web engine's `cdylib`), ADR-357;
`docs/schematics/ffi-adapter-xcframework-and-swift-package.md`,
`docs/schematics/umbrella-ffi-crate-and-one-static-library.md`; UniFFI's manual on library mode and
on several crates in one library.
