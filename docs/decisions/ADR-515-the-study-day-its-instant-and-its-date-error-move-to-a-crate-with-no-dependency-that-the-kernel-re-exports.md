# ADR-515: The study day, its instant and its date error move to a crate with no dependency, and the kernel re-exports them

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-497 |
| builds on | ADR-002, ADR-020, ADR-029 (none amended) |
| measured at | dev `b0e7edd5` |

## Context

Both clients are to compute the study day exactly as the server does: the day that turns over at the rollover hour
in a fixed UTC offset (CHARTER 7; SPEC-020 R1, R2 and R5). The rule, its calendar and its types live in the kernel
(`crates/kernel/src/study_day.rs`), which depends on `sqlx`, `tokio` and the log setup (`crates/kernel/Cargo.toml:17-24`)
and so cannot be linked into a `wasm32` module or an iOS static library. This ADR decides where the study day goes,
what moves with it, how 331 importing files keep building, how the crate stays free of dependencies, how CI proves
both clients' targets, and how the context map records the new edge. Every fact below was read at dev `b0e7edd5`.

## D1. Where the study day goes, and what it was chosen against

- **Chosen: a new crate, `deck-streak-study-day` at `crates/study-day`, in the map's bottom layer, depending on
  nothing; the kernel depends on it.** It is the one place the server and both clients can link without the kernel,
  and its one new edge, kernel to study day, points down.
- **A second copy in each client:** lost, because two copies of one rule drift, and the predecessor's golden would
  then have to hold three implementations instead of one.
- **The client crates depending on the kernel:** lost, because the kernel brings `sqlx`, `tokio` and `tracing` into
  a `wasm32` graph and an iOS graph, and the clients' crates sit beside the server's contexts, never above the kernel
  (`docs/CONTEXT-MAP.md:37-42`).
- **The XP crate:** lost, because it holds one context's game rule (ADR-371), and the study day is the kernel's
  shared concept; a crate that held both would be two contexts in one.
- **The engine core:** lost, because it holds Anki's engine, which the server's kernel never links.

## D2. What moves, and what stays

- **Chosen: the whole of `study_day.rs`, `UtcMillis` and `IsoDateError` move; `Clock`, `SystemClock` and
  `ManualClock` stay in the kernel's `clock.rs`, and every other kernel error stays in `error.rs`.** These three are
  exactly what `study_day.rs` imports (`study_day.rs:8-12`). `UtcMillis::from_system_time` moves with its type: it
  converts a `SystemTime` its caller hands it and reads no clock. `SystemClock`, the one reader of the system time
  (SPEC-020 R6), stays where it is, so the crate reads nothing outside its arguments. The kernel's `Setting` impls
  for `Hour` and `UtcOffset` stay in `settings.rs`: a local trait may be implemented for a foreign type.
- **The whole of `clock.rs`:** lost, because `SystemClock` reads the system time, which the crate must not do, and
  `ManualClock` is the server's test clock, which no client needs.
- **The whole of `error.rs`:** lost, because `KernelError` wraps `sqlx`'s errors.
- **The instant kept in the kernel, the crate taking an `i64`:** lost, because `StudyDayRule::study_day` takes a
  `UtcMillis`, and changing its signature changes every caller.
- The crate's modules are `study_day` (the file, its two imports re-pointed), `instant` (`UtcMillis`) and `error`
  (`IsoDateError`), each type also exported at the crate's root.

## D3. The re-export, and what it was chosen against

- **Chosen: the kernel re-exports the crate's items at every path it exported them from.** `lib.rs` names the
  crate's `study_day` module in place of its own (`pub use deck_streak_study_day::study_day;`), `clock.rs` re-exports
  `UtcMillis`, and `error.rs` re-exports `IsoDateError`; the root's `pub use` lines are unchanged. So
  `deck_streak_kernel::study_day::StudyDay`, `deck_streak_kernel::clock::UtcMillis`,
  `deck_streak_kernel::error::IsoDateError` and every root name resolve to the crate's own types, and the 331
  importing files build without a change. `ManualClock` reads the instant through `from_epoch_millis` and
  `epoch_millis`, its field being the crate's.
- **Repointing every importer at the new crate:** lost, because it churns 331 files and gives every domain context a
  second edge for a type it already reaches through the kernel.
- **Type aliases in the kernel:** lost, because an alias re-exports neither a module nor a tuple struct's constructor
  path, and two names for one type invite drift in the docs.

## D4. No dependency, and the tests, and what it was chosen against

- **Chosen: `IsoDateError`'s `Display` and `Error` are written by hand, with the message `thiserror` derived; the
  crate's manifest has no dependency table; the golden reader's `serde` and `serde_json` are its dev-dependencies
  alone (ADR-029).** A dev-dependency is not an edge of the map (`docs/schematics/context-map.md:10-13`) and is not
  linked into a client.
- **Keeping `thiserror`:** lost, because it is a dependency, and a proc macro for one unit struct.
- **The crate's tests:** the golden over `study_day.json` runs against the crate (SPEC-497 A3), beside copies of the
  kernel's date, hour and instant tests, because cargo-mutants runs the mutated crate's own tests, and a mutant of
  the moved code must be killed in its own crate. The kernel's tests stay unchanged and now prove the re-export
  path. A census in the crate's own tests (A1) reads the manifest's lines and the crate's sources; it needs no TOML
  parser, because a dependency or a build script can only be declared by a table header or a key, and each is
  refused by name, with a planted control.
- **Moving the kernel's tests into the crate:** lost, because the kernel's tests are importing files, which stay
  unchanged, and they are the proof that the re-export path still holds.
- **A Python census of the crate's lockfile closure, as the XP crate has:** lost, because the crate has no
  dependency to close over; the census reads the manifest itself.

## D5. The targets, and what it was chosen against

- **Chosen: the `web-engine` job builds the crate's library for `wasm32-unknown-unknown` and `aarch64-apple-ios`,
  with `--locked`, after it restores the Rust cache and before it builds the module.** The job already installs the
  `wasm32` target; an `rlib` for the iOS target needs that target's standard library and no linker or SDK, so the
  job's Linux runner builds both.
- **A new CI job:** lost, because the `ci` job's needs are held to an exact list, and a job of its own would cost a
  runner for a build of a few seconds.
- **The Apple workflow:** lost, because it runs only on Apple changes, on the Apple runner, and builds the umbrella's
  static library alone.
- **No target build until a client links the crate:** lost, because the crate exists for the clients, and a target
  that stops compiling should fail the change that breaks it, not the change that first links it.

## D6. The map, and what it was chosen against

- **Chosen: the fence gains the crate's row, `depends on: nothing`, and the kernel's row becomes
  `depends on: study-day`; the schematic moves the kernel into a layer of its own, the shared kernel, above the
  bottom layer, where the crate joins `xp` and `fsrs7`.** Every arrow into the kernel comes from a layer above it,
  and the new arrow points down. The relight census, which reads the sources of every type the notification
  router holds, adds the crate's sources, and writes out `ManualClock`'s one changed line.
- **The crate in the kernel's own layer:** lost, because the census refuses an arrow that stays in its layer.

## Consequences

- Good, because a client can link the study day with `std` alone, and CI compiles it for both clients' targets on
  every pull request.
- Good, because every importer, every kernel test and every golden is unchanged.
- Bad, because the crate's tests copy the kernel's date and instant tests, so two suites hold one rule; the kernel's
  copy proves the re-export, the crate's the code.
- Bad, because the kernel gains its first internal edge, so the kernel is no longer the bottom of the graph.

## Confirmation

SPEC-497 A1 (the crate's census, with planted controls), A2 (the re-exports' identity and the workspace build), A3
(the golden and the moved behaviour in the crate), A4 (CI's target builds), A5 (the map's census) and A6 (the clock
split).

## What would make this wrong

A client that needs the study day in another representation than an epoch day, or a rollover rule that depends on a
time-zone database, would need a different crate boundary; either is a new decision, not a fix to this one.
