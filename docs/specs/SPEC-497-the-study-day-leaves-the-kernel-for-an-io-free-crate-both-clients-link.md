# SPEC-497: The study day leaves the kernel for an I/O-free crate both clients link

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-515, under ADR-002, ADR-020 and ADR-029 |
| measured at | dev `b0e7edd5` (every file this SPEC cites is byte-equal there and at the delivery's base) |
| schematic | `docs/schematics/context-map.md` (changed: the shared kernel's layer and the new crate) |
| mutation band | S49700-S49799 (no hand row: section 9) |

## 1. The problem, measured

Both clients are to compute the study day the server computes, with the same rollover hour and the same fixed
offset, and neither can link the crate that holds it. What was read, at dev `b0e7edd5`:

- **The study day lives in the kernel.** `crates/kernel/src/study_day.rs` (257 lines) holds `Hour`, `UtcOffset`,
  `StudyDayRule`, `StudyDay`, `DEFAULT_ROLLOVER_HOUR` and the ISO calendar arithmetic; it imports the kernel's
  `UtcMillis` (`crates/kernel/src/clock.rs:10-43`) and `IsoDateError` (`crates/kernel/src/error.rs:128-131`) and
  nothing else of the kernel (`study_day.rs:8-12`).
- **The kernel cannot be linked by a client.** Its manifest depends on `sqlx`, `tokio`, `tracing`,
  `tracing-subscriber`, `thiserror`, `serde` and `serde_json` (`crates/kernel/Cargo.toml:17-24`); the database,
  the runtime and the log setup have no place in a `wasm32` module or an iOS static library.
- **The moved code reads nothing.** `study_day.rs` is integer arithmetic over its inputs. `UtcMillis` is a number
  of milliseconds; its `from_system_time` converts a `SystemTime` the caller hands it and reads no clock
  (`clock.rs:31-42`). The one reader of the system time is `SystemClock` (`clock.rs:51-59`, the type at `:53`).
- **`IsoDateError` is one unit struct with one message**, derived through `thiserror`
  (`error.rs:128-131`); no other error type of `error.rs` names it.
- **Who imports the moved items.** `git grep -l -w` for the seven names, then `grep -l deck_streak_kernel`, gives
  331 files outside the kernel's own source: 125 production files and 206 test files. Inside the kernel,
  `settings.rs:20` imports `Hour`, `StudyDayRule` and `UtcOffset`, and `lib.rs:31,35,43,48` declares and re-exports
  them.
- **The edges.** The context map's kernel line depends on nothing (`docs/CONTEXT-MAP.md:20`), the schematic draws the
  kernel in the bottom layer beside `xp` and `fsrs7` (`docs/schematics/context-map.md:61-65`), and its census holds
  every manifest edge to an arrow and a fence entry (`scripts/tests/test_context_map_schematic.py`).
- **Tests that read the moved code's text.** The relight census writes out `ManualClock`'s state lines, one of them
  `now: AtomicI64::new(start.0),` (`crates/coordination/tests/relight_order.rs:1269-1272`), and lists the sources
  the router holds as the notifications crate and the kernel (`:1199-1207`).
- **The targets.** CI adds `wasm32-unknown-unknown` in the `web-engine` job (`.github/workflows/ci.yml:1021`) and
  builds `aarch64-apple-ios` only in the Apple workflow, for the umbrella FFI crate (`xcframework.yml:58-79`).
- **Not measured:** the study day's weight in a client's binary; no client links the crate yet.

## 2. Requirements

R1. **The crate.** `crates/study-day`, package `deck-streak-study-day`, holds `src/study_day.rs` (the kernel's file,
its imports re-pointed), `UtcMillis` in `src/instant.rs` and `IsoDateError` in `src/error.rs`. Its manifest has no
`[dependencies]`, `[build-dependencies]` or target table; its dev-dependencies are the golden reader's `serde` and
`serde_json` alone (ADR-029). No source of it reads a clock, a file, the environment, the network, a process or a
thread. `IsoDateError` keeps its message, `not an ISO calendar date (YYYY-MM-DD)`, written without `thiserror`.

R2. **The kernel re-exports them.** `deck_streak_kernel::study_day` is the crate's module, `deck_streak_kernel::clock::
UtcMillis` and `deck_streak_kernel::error::IsoDateError` are the crate's types, and the kernel's root re-exports keep
every name they export today. Every importing file builds unchanged, and every workspace test passes.

R3. **The golden runs against the crate.** The crate's own test reads `tools/parity-oracle/goldens/study_day.json` and
checks every case against `deck_streak_study_day`. The crate's own tests also hold the date's rendering and parsing,
the hour's bound, the instant's conversion and the error's text, so a mutant of the moved code is killed by its own
crate's tests. The kernel's tests stay as they are.

R4. **CI builds it for both clients' targets.** The `web-engine` job builds the crate's library for
`wasm32-unknown-unknown` and `aarch64-apple-ios`, with `--locked`.

R5. **The map carries the edge.** The fence gains `deck-streak-study-day ... depends on: nothing` and the kernel's line
becomes `depends on: study-day`; the schematic draws `kernel --> study_day`, pointing down, with the kernel in a layer
of its own above the crate.

R6. **`SystemClock` stays in the kernel.** `Clock`, `SystemClock` and `ManualClock` stay in
`crates/kernel/src/clock.rs`, which takes `UtcMillis` from the crate; no source of the crate names `SystemClock` or
reads `SystemTime::now`.

R7. **Unchanged.** Every behaviour of the moved code, every importing file, every golden and registry module, the
kernel's tests, every formal entry and vector, and every mutation row. The only lines changed outside the kernel and
the crate are the workspace's dependency entry, the lockfile's two entries, the map, the schematic, the CI step and
the relight census's written-out state (section 6).

## 3. Acceptance criteria of SPEC-497

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: the crate holds the study day, the instant and the date error, each defined there and nowhere in the kernel, with no dependency table and no I/O, every planted defect refused by name | the stub crate has no `study_day.rs` and the kernel still defines all six types | ADR-515 D1, D2, D4 |
| A2 | R2: each kernel path to a moved item names the crate's own type, and the workspace builds and passes | the kernel's types are its own | ADR-515 D3 |
| A3 | R3: the golden over `study_day.json`, and the date, hour, instant and error tests, run against the crate | the stub's study day is 0, it renders nothing and parses nothing | ADR-515 D4 |
| A4 | R4: CI builds the crate for `wasm32-unknown-unknown` and `aarch64-apple-ios` | none: only CI proves it (section 7) | ADR-515 D5 |
| A5 | R5: the fence and the schematic carry the kernel's edge to the crate, held by the census | the census names the crate's missing node, fence row and arrow | ADR-515 D6 |
| A6 | R6: `SystemClock` stays in the kernel's clock, which takes the instant from the crate, and the crate names no system clock | the kernel's clock still defines the instant | ADR-515 D2 |

```acceptance
A1: cargo test -p deck-streak-study-day --test crate_shape -- --exact the_crate_holds_the_study_day_the_instant_and_the_date_error_with_no_dependency_and_no_io
A2: cargo test -p deck-streak-kernel --test study_day_crate -- --exact the_kernel_re_exports_the_study_day_crates_own_types
A2: cargo test --workspace --locked --no-fail-fast
A3: cargo test -p deck-streak-study-day --test study_day -- --exact the_study_day_matches_the_predecessors_golden
A3: cargo test -p deck-streak-study-day --test study_day -- --exact every_hour_past_23_is_refused_and_23_is_admitted
A3: cargo test -p deck-streak-study-day --test study_day -- --exact a_study_day_renders_as_its_iso_date_and_parses_back
A3: cargo test -p deck-streak-study-day --test study_day -- --exact a_signed_year_is_written_only_where_four_digits_cannot_hold_it
A3: cargo test -p deck-streak-study-day --test instant -- --exact a_system_time_after_the_epoch_reads_as_its_milliseconds
A3: cargo test -p deck-streak-study-day --test instant -- --exact a_time_before_the_epoch_reads_as_negative_milliseconds
A3: cargo test -p deck-streak-study-day --test instant -- --exact the_date_error_reads_as_it_did_in_the_kernel
A4: cargo build --locked -p deck-streak-study-day --lib --target wasm32-unknown-unknown --target aarch64-apple-ios
A5: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
A5: python3 -m unittest discover -s scripts/tests -p test_context_map_register.py
A6: cargo test -p deck-streak-kernel --test study_day_crate -- --exact system_clock_stays_in_the_kernel_and_the_instant_leaves_it
```

The kernel's own suites (`--test study_day`, `--test clock`, `--test settings`) run unchanged and stay green: they
reach the moved code through the re-exports.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/study-day/Cargo.toml` | study day | new: no dependency; the golden reader's two dev-dependencies |
| `crates/study-day/src/lib.rs` | study day | new: the crate's doc and its three modules |
| `crates/study-day/src/study_day.rs` | study day | moved from `crates/kernel/src/study_day.rs`, its two imports re-pointed |
| `crates/study-day/src/instant.rs` | study day | new: `UtcMillis`, moved from `crates/kernel/src/clock.rs` |
| `crates/study-day/src/error.rs` | study day | new: `IsoDateError`, moved from `crates/kernel/src/error.rs`, its message written by hand |
| `crates/study-day/tests/study_day.rs` | study day tests | new: A3, the golden and the date's tests against the crate |
| `crates/study-day/tests/instant.rs` | study day tests | new: A3, the instant's conversion and the error's text |
| `crates/study-day/tests/crate_shape.rs` | study day tests | new: A1, the crate's census with its planted controls |
| `crates/kernel/Cargo.toml` | kernel | changed: the one internal edge, to the crate |
| `crates/kernel/src/lib.rs` | kernel | changed: the `study_day` module is the crate's, re-exported |
| `crates/kernel/src/clock.rs` | kernel | changed: `UtcMillis` re-exported from the crate; `ManualClock` reads it through its two methods |
| `crates/kernel/src/error.rs` | kernel | changed: `IsoDateError` re-exported from the crate |
| `crates/kernel/tests/study_day_crate.rs` | kernel tests | new: A2 and A6 |
| `crates/coordination/tests/relight_order.rs` | coordination tests | changed: the router's sources name the crate, and `ManualClock`'s changed line is written out |
| `Cargo.toml` | workspace | changed: the crate's workspace dependency entry |
| `Cargo.lock` | workspace | changed: the crate's package and the kernel's edge to it |
| `docs/CONTEXT-MAP.md` | docs | changed: the crate's fence row, the kernel's edge and the prose |
| `docs/schematics/context-map.md` | docs | changed: the shared kernel's layer, the crate's node and the arrow |
| `.github/workflows/ci.yml` | CI | changed: the `web-engine` job builds the crate for both clients' targets |
| `docs/specs/SPEC-497-the-study-day-leaves-the-kernel-for-an-io-free-crate-both-clients-link.md` | docs | new: this file |
| `docs/decisions/ADR-515-the-study-day-its-instant-and-its-date-error-move-to-a-crate-with-no-dependency-that-the-kernel-re-exports.md` | docs | new: the decisions |
| `docs/red-first/SPEC-497.md` | docs | new: the red-first record |
| `changelog.d/study-day-crate-497.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- Either client linking the crate: the umbrella FFI crate's and the web engine's edges, their exports of the study day,
  and the study day's weight under the web engine's budget; each is drawn by the change that first uses it.
- Any change to the study day's rule, the rollover hour, the offset's bounds, the ISO rendering or the settings that
  read them.
- Moving `Clock`, `ManualClock` or any other kernel type.
- Repointing any importer at the new crate.

## 6. Risks

- **A behaviour moves.** Detected by A3 against the predecessor's golden in the crate, and by the kernel's unchanged
  suites through the re-exports.
- **A path an importer names no longer resolves.** Detected by A2's workspace build, and A2's type identity check
  names each path.
- **The crate gains a dependency or a reader of the outside world.** Detected by A1's census, whose planted manifests
  and sources must each be refused by name.
- **A census that reads nothing.** Each census prints its examined count and refuses zero.
- **A test that pins the moved text.** The relight census writes out `ManualClock`'s state lines; the one line that
  changes, `now: AtomicI64::new(start.epoch_millis()),`, is written out in the same change, and the crate's sources
  join the sources the router holds, so the census still reads every type the router keeps.
- **A sibling change edits the kernel's clock.** The move changes `clock.rs` at its `std::time` import, the
  instant's definition and `ManualClock`'s three reads of its field, and nothing else of that file.

## 7. What only CI proves

The crate's builds for `wasm32-unknown-unknown` and `aarch64-apple-ios` (`web-engine`, A4), the whole workspace's
clippy and tests (`rust`), the release profile's tests (`release`), and the mutation jobs over the moved files
(`mutation-rust`), each read by name at the head.

## 8. Formal model

- **Lean: NOT APPLICABLE.** The move changes no rule; the study day's arithmetic is unchanged and is closed by the
  predecessor's golden.
- **TLA+: NOT APPLICABLE.** Nothing interleaves.
- **No cover moves.** No formal entry covers `crates/kernel/src/study_day.rs`, `clock.rs` or `error.rs`. The open
  lapse's Lean file names the study day's type and its old path in a comment, which this change leaves as written:
  the type keeps its definition, byte for byte.

## 9. Mutation rows

None. The move adds no rule; cargo-mutants generates the moved code's mutants in the crate, and the crate's own tests
(A3) are their killers. The band S49700-S49799 holds no row.
