# SPEC-336: a native client reaches the engine through one allow-listed FFI crate

- **Issue:** #615, the app campaign's FFI spike. **Context(s):** `deck-streak-ffi`, an adapter at
  the edge that depends on no context; and `deck-streak-progression`, whose settle census (a test
  of the context's own invariant, SPEC-072) now compiles the features the members declare.
- **Decided by:** ADR-345 (one adapter crate, the UniFFI pin, the generator behind a feature, the
  macOS job, a generator binary that refuses without its feature, and a settle census that
  compiles every combination of the declared features), resting on ADR-022 and ADR-058 for the
  engine and ADR-197 for the census.
- **Status:** a spike, delivered by the draft pull request that adds this file, with its tests and
  `docs/red-first/SPEC-336.md`. **Mutation band:** S33600-S33699.

## 1. The problem, measured

The question: can an iPhone and iPad client call DeckStreak's engine, Anki's engine through the
workspace's pinned fork (ADR-058), through ONE FFI crate packaged as an arm64 XCFramework holding a
device slice and a simulator slice, built in CI? This spike measures and records; it decides no
client.

What was measured before the adapter was written:

- The engine's own clients address a backend call by a pair of indices and send protobuf bytes:
  `Backend::run_service_method(service, method, input)`. Amgi
  (github.com/antigluten/amgi) puts the same call behind a four-function C FFI. The indices are
  generated when the engine is built; read from its generated dispatch at the pinned rev, the
  backend collection service is 3 (`OpenCollection` 0, `CloseCollection` 1, and the collection
  service's `Undo` reached through it as 8), the decks service 7 (`GetDeckNames` 13) and the
  scheduler service 13 (`GetQueuedCards` 3, `AnswerCard` 4).
- The bindings generator's Swift modulemap template, read at each upstream tag, names no builtin
  clang module up to and including the pinned release and names three from the next release on
  (ADR-345 D2 holds the measurement).
- `cargo mutants --no-shuffle --list -p deck-streak-ffi` lists 10 mutants of the adapter, the
  generator's `main` among them. While the generator's binary declared `required-features =
  ["bindgen"]`, no default-feature build compiled it, so the whole-crate run
  (`cargo mutants --no-shuffle --in-place -p deck-streak-ffi`) read `main` missed: 10 tested, 8
  caught, 1 unviable, 1 missed. R10 and ADR-345 D5 make the binary reachable and A8 catches it.

## 2. Requirements

R1. A native client reaches the engine through one entry point, `Engine::run(service, method,
    input)`: the request's protobuf bytes in and the response's protobuf bytes out, over the
    engine's `run_service_method`.
R2. The calls a client may make are a constant table of exactly five pairs: (3, 0)
    `OpenCollection`, (7, 13) `GetDeckNames`, (13, 3) `GetQueuedCards`, (13, 4) `AnswerCard` and
    (3, 8) `Undo`.
R3. A call outside the table is answered `EngineRefusal::NotAllowed { service, method }` before the
    engine sees it, never with a panic, and the engine keeps serving; an engine error is
    `EngineRefusal::Engine` carrying the engine's encoded error; every refusal reads as its own
    sentence.
R4. `deck-streak-ffi` depends on the engine and on no crate of this workspace; the context map
    declares it with no internal edge, and the daemon does not compose it.
R5. The bindings crate is pinned exactly, `uniffi = "=0.29.1"`, with its default features off; the
    generator's features are reached only through the adapter's `bindgen` feature.
R6. `xcframework.yml` builds the adapter's static library for `aarch64-apple-ios` and
    `aarch64-apple-ios-sim` in release, clocked.
R7. It generates the Swift bindings in library mode, over the device library, with the generator
    the adapter itself builds, naming the C module `deck_streak_ffiFFI`.
R8. It fails when the generated modulemap names a builtin clang module, assembles one XCFramework
    from both slices, and uploads the framework, the bindings and a report of each slice's size and
    the build time as the `xcframework` artifact.
R9. It typechecks the generated Swift against each slice's own headers. It runs on a macOS runner,
    admitted to this one workflow by file name in the workflow hardening test, which refuses that
    runner under any other workflow and any other runner for this one.
R10. Every build of the adapter builds the generator's binary. Built without the `bindgen`
    feature, it holds none of the generator's code and refuses every run: one line on stderr that
    names the feature and the command that builds it, nothing on stdout, and exit status 2. Built
    with the feature, its `main` runs the generator as before.
R11. The settle census (`crates/progression/tests/xp_census.rs`) reads the workspace's graph with
    every feature on, and compiles each selection in each pass once for every combination of the
    features the members declare, each member's default off unless the combination holds it. A
    call to `settle` outside coordination is then refused for the call under any feature, under a
    feature's absence and under any mix of them, and a build script that reaches progression only
    through an optional dependency is refused as one that can name `settle`. It refuses by name a
    member whose feature another package turns on, by naming it or by keeping its default, and a
    workspace whose members declare more than three features. It admits no member by name.
R12. The adapter passes the workspace's censuses as they stand: the log-capture census reads no
    capture in it, because the engine constructor's parameter is `message` and carries no install
    token, and the settle census compiles its `bindgen` feature under R11 and finds no use of
    `settle` in it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | an allowed `OpenCollection` opens a synthetic collection built in the test, and the engine then holds it | `cargo test -p deck-streak-ffi --test round_trip -- --exact a1_opens_a_synthetic_collection` |
| A2 | an allowed `GetDeckNames` lists that collection's two decks by name | `cargo test -p deck-streak-ffi --test round_trip -- --exact a2_lists_the_collections_decks` |
| A3 | an allowed `GetQueuedCards` gives the synthetic note's one new card | `cargo test -p deck-streak-ffi --test round_trip -- --exact a3_gets_the_next_card` |
| A4 | an allowed `AnswerCard` rates that card Good, and it leaves the new queue | `cargo test -p deck-streak-ffi --test round_trip -- --exact a4_answers_the_card` |
| A5 | an allowed `Undo` reverts the answer, names it, and the card is new and first again | `cargo test -p deck-streak-ffi --test round_trip -- --exact a5_undoes_the_answer` |
| A6 | four unlisted pairs are each refused `NotAllowed`, and the collection keeps serving | `cargo test -p deck-streak-ffi --test round_trip -- --exact a6_refuses_an_unlisted_call_and_keeps_serving` |
| A7 | the macOS runner is admitted to `xcframework.yml` alone, by file name | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_admitted_runner_is_admitted_to_its_one_workflow_only` |
| A8 | built without `bindgen`, the generator refuses the workflow's own library-mode call: exit status 2, the feature named on stderr, nothing on stdout | `cargo test -p deck-streak-ffi --test bindings_generator -- --exact a8_the_generator_refuses_a_build_without_its_feature` |
| A9 | a call to `settle` in habits under a declared feature, under a default feature's absence, under one feature and another's absence, and under two features together is each refused for the call alone, as the same call under no feature is | `cargo test -p deck-streak-progression --test xp_census -- --exact a9_a_call_under_a_feature_in_another_crate_is_refused_for_the_call` |
| A10 | a build script whose package reaches progression only through an optional dependency a feature turns on is refused as one that can name `settle`, as one reaching it through a dependency that is always on is | `cargo test -p deck-streak-progression --test xp_census -- --exact a10_a_build_script_that_reaches_settle_only_under_a_feature_is_refused` |
| A11 | a member whose feature another package turns on, by naming it or by keeping its default, is refused by name; one depended on with its default off is accepted | `cargo test -p deck-streak-progression --test xp_census -- --exact a11_a_feature_another_package_turns_on_is_refused_by_name` |
| A12 | members declaring three features are compiled in every combination and accepted; four are refused by name | `cargo test -p deck-streak-progression --test xp_census -- --exact a12_the_census_compiles_every_combination_up_to_its_feature_limit_and_refuses_past_it` |
| A13 | the log-capture census over the real tree, the adapter included, finds every capture going through the helper | `cargo test -p deck-streak-kernel --test log_capture_class -- --exact every_capture_in_the_workspace_goes_through_the_helper` |
| A14 | the settle census over the real tree, the adapter and its feature included, finds only coordination settling | `cargo test -p deck-streak-progression --test xp_census -- --exact only_progression_writes_xp_settlement_and_only_coordination_settles` |

```acceptance
A1: cargo test -p deck-streak-ffi --test round_trip -- --exact a1_opens_a_synthetic_collection
A2: cargo test -p deck-streak-ffi --test round_trip -- --exact a2_lists_the_collections_decks
A3: cargo test -p deck-streak-ffi --test round_trip -- --exact a3_gets_the_next_card
A4: cargo test -p deck-streak-ffi --test round_trip -- --exact a4_answers_the_card
A5: cargo test -p deck-streak-ffi --test round_trip -- --exact a5_undoes_the_answer
A6: cargo test -p deck-streak-ffi --test round_trip -- --exact a6_refuses_an_unlisted_call_and_keeps_serving
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_admitted_runner_is_admitted_to_its_one_workflow_only
A8: cargo test -p deck-streak-ffi --test bindings_generator -- --exact a8_the_generator_refuses_a_build_without_its_feature
A9: cargo test -p deck-streak-progression --test xp_census -- --exact a9_a_call_under_a_feature_in_another_crate_is_refused_for_the_call
A10: cargo test -p deck-streak-progression --test xp_census -- --exact a10_a_build_script_that_reaches_settle_only_under_a_feature_is_refused
A11: cargo test -p deck-streak-progression --test xp_census -- --exact a11_a_feature_another_package_turns_on_is_refused_by_name
A12: cargo test -p deck-streak-progression --test xp_census -- --exact a12_the_census_compiles_every_combination_up_to_its_feature_limit_and_refuses_past_it
A13: cargo test -p deck-streak-kernel --test log_capture_class -- --exact every_capture_in_the_workspace_goes_through_the_helper
A14: cargo test -p deck-streak-progression --test xp_census -- --exact only_progression_writes_xp_settlement_and_only_coordination_settles
```

Every test builds its own synthetic collection with the engine's API; no real collection data is
read. `refusal_text::each_refusal_reads_as_its_own_sentence` holds R3's text and is MUTATION
COVERAGE, not a criterion (`docs/red-first/SPEC-336.md`). A8's test is also the killer of row
S33601, which installs cargo-mutants' own mutant of the generator's `main`. R6 to R8, and R10's
build with the feature, are measured by the workflow's run, in section 7. A9 to A12 plant small
workspaces in a temporary directory; A13 and A14 judge the real tree, and their tests are the
kernel's and progression's own, unchanged in what they judge.

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml` | the workspace | the adapter as a member, and the `uniffi` pin |
| `Cargo.lock` | the workspace | the bindings crate's packages |
| `crates/ffi/Cargo.toml` | `deck-streak-ffi` | added |
| `crates/ffi/src/lib.rs` | `deck-streak-ffi` | added |
| `crates/ffi/src/allow_list.rs` | `deck-streak-ffi` | added |
| `crates/ffi/src/engine.rs` | `deck-streak-ffi` | added |
| `crates/ffi/src/bin/uniffi-bindgen-swift.rs` | `deck-streak-ffi` | added |
| `crates/ffi/tests/round_trip.rs` | `deck-streak-ffi` | added |
| `crates/ffi/tests/refusal_text.rs` | `deck-streak-ffi` | added |
| `crates/ffi/tests/bindings_generator.rs` | `deck-streak-ffi` | added: A8, R10 |
| `crates/progression/tests/xp_census.rs` | `deck-streak-progression` | the settle census compiles every combination of the declared features: R11, A9 to A12 |
| `.github/workflows/xcframework.yml` | none (CI) | added |
| `scripts/tests/test_ci_workflows.py` | none (the gate) | the runner admission and its test |
| `scripts/mutation-rows.d/S33600-S33699.json` | none (the gate) | added |
| `docs/CONTEXT-MAP.md` | the map | the adapter's line and paragraph |
| `docs/schematics/ffi-adapter-xcframework-and-swift-package.md` | the record | added: the adapter's components, one call, one build, and the generator's two builds |
| `docs/red-first/SPEC-336.md` | the record | added |
| `docs/specs/SPEC-336-a-native-client-reaches-the-engine-through-one-allow-listed-ffi-crate.md` | the record | added |
| `docs/decisions/ADR-345-one-allow-listed-ffi-adapter-pinned-uniffi-and-a-macos-xcframework-job.md` | the record | added |
| `changelog.d/spike-ffi-umbrella-336.md` | the record | added |

## 5. What this does NOT do

- It writes no Swift, no Xcode project and no client: the Swift half has no campaign issue yet, so
  it cites the nearest real record, the Apple sign-in client registration (#347).
- It does not sign, ship or run the framework on a device or a simulator; the consumer check
  typechecks the bindings and links nothing (#347).
- It changes no engine code: the engine stays the fork ADR-058 pins (#233).
- It allows no call beyond the five in R2; a client that needs another call widens the table in a
  delivery of its own, with its round trip (#347).
- It reaches no DeckStreak table or use case, and the daemon does not compose the adapter (#233).
- It changes no other census: the log-capture census and its install vocabulary stay as they are,
  and the adapter's parameter was renamed instead (#615).
- The settle census compiles at most three declared features, because each one doubles its
  compiles; a workspace that declares a fourth is refused until the bound is raised in review
  (#615).

## 6. Risks

- The engine renumbers its services or methods on an upgrade. Detected by A1 to A6, which call each
  entry by literal indices and decode its answer.
- The bindings crate's internal macros crate is resolved through a non-exact requirement, so the
  lockfile, not the pin, fixes its version (ADR-345 D2). Detected by `cargo deny` and the lockfile
  check.
- A runner image changes the platform SDK. Detected by the modulemap check and the consumer check,
  each of which fails the job.
- No host test runs the generator built with the `bindgen` feature: A8 holds the build without
  it. Detected by the workflow: its bindings step fails when the generator exits nonzero or
  writes no `bindings` directory, and each step after it fails when a generated file it reads is
  missing.
- The macOS protobuf compiler's digest is Anki's own pin, and the hardening test's protoc guard
  reads only the Linux pin. Detected by the job's own checksum step, which fails on a mismatch.
- The settle census now compiles the real workspace twice as many times, once without the
  adapter's feature and once with it. Detected by A14's duration in the CI run, against
  `CARGO_LIMIT`, which fails the census by name past its bound.

## 7. Measured by the XCFramework workflow

The workflow's run on the pull request measures R6 to R8 and R9's consumer check; its report is the
run's summary and `xcframework-report/report.md` in the `xcframework` artifact.

| measure | measured |
|---|---|
| two static libraries, release | 9.3 minutes (558 s) |
| Swift bindings, library mode | 2.4 minutes (142 s) |
| modulemap names no builtin clang module | pass |
| generated Swift typechecks against each slice | pass |
| slice `ios-arm64` | 81.0 MiB (84,957,000 bytes) |
| slice `ios-arm64-simulator` | 81.0 MiB (84,899,224 bytes) |
| runner CPUs | 3 |
