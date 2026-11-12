# SPEC-681: import an Anki deck package from the Files app on iPhone and iPad

- **Context(s):** the native adapter (`crates/ffi`), the engine core's table
  (`crates/engine-core/src/table.rs`), the iPhone and iPad app (`ios/App`), their tests and `docs`.
- **Decided by:** ADR-703
- **Status:** delivered with its tests and `docs/red-first/SPEC-681.md`.

## 1. The problem, measured

The app has no route to a deck without sync. Each finding below is read by name and line at this
change's base.

- **The app.** `ios/App/Sources/DeckListView.swift` (74 lines) shows the decks in a `List` (line
  57) titled `Decks` (line 68), with no empty state: a fresh install shows an empty list.
  `ios/App/Info.plist` (52 lines) declares no document type. `EngineCall` in
  `ios/App/Sources/EngineSession.swift` (line 8) names six calls, and none imports.
- **The engine's own import.** `ImportExportService.ImportAnkiPackage`
  (`proto/anki/import_export.proto` lines 15 and 16) takes `ImportAnkiPackageRequest`, whose
  `package_path` is field 1 and `options` field 2 (lines 56 to 66), and answers `ImportResponse`
  with its log (lines 69 and 74). `rslib/src/import_export/package/apkg/import/mod.rs` opens the
  zip (`ZipArchive::new`, line 58) and copies the collection member out (`meta.copy`, line 151);
  `package/meta.rs` decodes it by `copy_decode` or `io::copy` (lines 121 and 123), and
  `package/media.rs` decodes each media file and the media list the same way (lines 133, 178
  and 180). None of these reads bounds an inner member's decompressed bytes.
- **No DeckStreak reader.** No DeckStreak crate decodes zip. `crates/anki-format/src/opener.rs`
  bounds a foreign SQLite file at `1 << 30` bytes (`Limits` and `max_bytes`, lines 64 to 75) and
  reads no package.
- **The two tables.** `crates/ffi/src/allow_list.rs` holds `ALLOW_LIST: [Call; 12]` (line 34)
  and `allowed` (line 99), which `run` in `crates/ffi/src/engine.rs` reads before the engine
  (lines 125 to 140). `crates/engine-core/src/table.rs` holds `ORDINARY: [Ordinary; 18]` (line
  123), `ANSWERED` (line 274) and `EXEMPT` (line 287). The parity test
  `each_adapter_table_equals_its_transport_column` (`crates/engine-core/tests/parity.rs` line 70)
  holds the allow-list equal to the Native column of `ORDINARY`. The engine numbers its backend
  services in descriptor order, and a backend service answers its own methods first: the import
  is service 39 (`BackendImportExportService`, with two methods of its own), method 2. Four lists
  pin the tables: `EXPECTED` (`crates/ffi/tests/review_pairs.rs` line 67), `NATIVE`
  (`crates/engine-core/tests/table.rs` line 28 and `crates/engine-core/tests/review_pairs.rs`
  line 43) and `ORDINARY_ROWS` (`crates/engine-core/tests/data_screen_pairs.rs` line 21, its row
  total at line 78).
- **The refusal path.** `EngineRefusal` keeps three cases (`crates/ffi/src/engine.rs` line 32):
  `NotAllowed`, `Engine` and `Start`. `Engine` carries an encoded `BackendError`, whose message
  `EngineSession.swift` shows as the refusal's sentence (lines 358 to 361). A worded refusal
  therefore reaches the app with no new case.
- **The planned engine removal.** The records that retire the engine are not on dev, and ADR-151
  stands as proposed. Until they land, the engine's own import is the one import path, reached
  through one allow-listed call; when the engine leaves, the import leaves with it and its
  replacement keeps this request, this bound and these words.

The data flow: the learner taps Import, on the empty list's offer or in the toolbar; the system
file importer returns the picked file's security-scoped address; the app copies the file into its
temporary folder while it holds that access; the session sends one call,
`ImportExportService.ImportAnkiPackage`, with the copy's path; the native adapter reads the
package under the bound before the engine runs; the engine imports the notes and cards and the
deck list refreshes, or the list shows the worded refusal and nothing was written.

## 2. Requirements

R1. On iPhone and iPad the deck list opens the system file importer (SwiftUI `.fileImporter`)
for an Anki deck package (`.apkg`), which `ios/App/Info.plist` declares as an imported type.

R2. The picked package is copied into the app's temporary folder while its security-scoped access
is held, and its notes and cards enter the open collection through one engine call,
`ImportExportService.ImportAnkiPackage`, with four options: their scheduling kept
(`with_scheduling` true), no note type the collection holds merged or changed (`merge_notetypes`
false), and the two update conditions, for notes and for note types, at the engine's defaults.

R3. The native allow-list and the Native column of the core's table both admit that call as one
ordinary pair (ADR-356 D3), the parity test holds the two equal, and each list that pins the
tables carries the pair.

R4. Before the engine reads the package, the native adapter reads every zip entry and every inner
compressed member, the collection or a media file, under one bound of 1 GiB (`1 << 30` bytes),
counting its bytes as they decompress and never trusting a declared size. A member past the
bound, or a package the reader cannot read, is refused before any row is read or written.

R5. A refused package reads as a worded error, carried in the engine refusal's message with no new
refusal case: past the bound, "This package holds a file larger than the app imports (1 GiB), so
nothing was imported."; unreadable, "This file is not an Anki deck package the app can read, so
nothing was imported."

R6. The empty deck list offers the import.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | on iPhone and iPad the deck list opens the system file importer (R1) | `ios/AppUITests/PackageImportFlowTests.swift` `testTheDeckListOpensTheSystemFileImporter` |
| A2 | a valid package's notes and cards enter the collection through the one call, counted after it (R2) | `crates/ffi/tests/package_import.rs` `a_valid_package_imports_its_notes_and_cards_in_one_call` |
| A3 | the allow-list admits the import pair, and the core's table admits it on the Native column alone (R3) | `crates/ffi/tests/package_import.rs` `the_import_pair_is_admitted_on_the_native_column` |
| A4 | a package the app refuses reads as a worded error and writes no row, beside the valid package of A2 (R4, R5) | `crates/ffi/tests/package_import.rs` `an_unreadable_package_is_refused_in_words_with_no_row`; `ios/AppUITests/PackageImportFlowTests.swift` `testARefusedPackageReadsAsWords` |
| A5 | a collection member and a media member each decompressing past the bound are refused in words before any row is read, beside the valid package of A2 (R4, R5) | `crates/ffi/tests/package_import.rs` `an_oversize_collection_member_is_refused_before_any_row` and `an_oversize_media_member_is_refused_before_any_row` |
| A6 | the empty deck list offers the import (R6) | `ios/AppUITests/PackageImportFlowTests.swift` `testTheEmptyDeckListOffersTheImport` |

```acceptance
A1: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -only-testing:DeckStreakUITests/PackageImportFlowTests/testTheDeckListOpensTheSystemFileImporter
A2: cargo test -p deck-streak-ffi --test package_import -- --exact a_valid_package_imports_its_notes_and_cards_in_one_call
A3: cargo test -p deck-streak-ffi --test package_import -- --exact the_import_pair_is_admitted_on_the_native_column
A4: cargo test -p deck-streak-ffi --test package_import -- --exact an_unreadable_package_is_refused_in_words_with_no_row
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -only-testing:DeckStreakUITests/PackageImportFlowTests/testARefusedPackageReadsAsWords
A5: cargo test -p deck-streak-ffi --test package_import -- --exact an_oversize_collection_member_is_refused_before_any_row
A5: cargo test -p deck-streak-ffi --test package_import -- --exact an_oversize_media_member_is_refused_before_any_row
A6: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -only-testing:DeckStreakUITests/PackageImportFlowTests/testTheEmptyDeckListOffersTheImport
```

Each Rust test reaches the engine through the native adapter's own surface, and each package it
needs is planted at run time: the valid one is a synthetic collection the engine exports, and each
oversize member is a zstd frame of run-length blocks, with no content size declared, that the test
writes itself. The UI tests plant a fresh collection and, for the refusal, a file that is no
package, passed to the app by a launch argument that imports it through the importer's own path.

## 4. Read from CI by name

- Each Rust test above passes by name in the nextest output of the `ci` workflow's `rust` job and
  of its `release` job (the release profile), never the aggregate `ci` job. Beside them, the tests
  that read the four pinned lists pass by name: `the_allow_list_carries_the_review_pairs`,
  `every_pair_is_admitted_held_or_refused_by_its_transport`,
  `the_review_pairs_run_natively_and_no_other_pair_joins`,
  `the_review_pairs_run_natively_and_no_other_pair_joins_on_each_side_of_the_cutoff` and
  `the_data_screens_add_no_engine_pair`.
- Each Swift test above passes by class, test name and destination in the apple run's
  `release-and-app` job, step `the app's tests, Debug, on the iPhone and then the iPad`, on both
  the iPhone and the iPad.

## 5. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-681-import-an-anki-deck-package-from-the-files-app-on-iphone-and-ipad.md` | docs | added |
| `docs/decisions/ADR-703-the-package-import-is-one-allow-listed-engine-call-behind-an-inner-member-bound.md` | docs | added |
| `docs/red-first/SPEC-681.md` | docs | added |
| `changelog.d/ios-file-import-681.md` | docs | added |
| `crates/ffi/src/package_bound.rs` | native adapter | added |
| `crates/ffi/src/allow_list.rs` | native adapter | changed |
| `crates/ffi/src/engine.rs` | native adapter | changed |
| `crates/ffi/src/lib.rs` | native adapter | changed |
| `crates/ffi/Cargo.toml` | native adapter | changed |
| `Cargo.lock` | workspace | changed |
| `crates/engine-core/src/table.rs` | engine core | changed |
| `crates/ffi/tests/package_import.rs` | native adapter tests | added |
| `crates/ffi/tests/review_pairs.rs` | native adapter tests | changed |
| `crates/engine-core/tests/table.rs` | engine core tests | changed |
| `crates/engine-core/tests/review_pairs.rs` | engine core tests | changed |
| `crates/engine-core/tests/data_screen_pairs.rs` | engine core tests | changed |
| `ios/App/Info.plist` | app | changed |
| `ios/App/Sources/DeckListView.swift` | app | changed |
| `ios/App/Sources/EngineSession.swift` | app | changed |
| `ios/App/Sources/PackageImportRequest.swift` | app | added |
| `ios/App/Sources/PackageImportModel.swift` | app | added |
| `ios/App/Sources/PackageImportView.swift` | app | added |
| `ios/AppUITests/PackageImportFlowTests.swift` | app tests | added |
| `ios/swift-roles.json` | app | changed |

## 6. What this does NOT do

- The Mac's import, an open panel and a drop target, which the macOS app's own work carries.
- The restore door and the copy for a learner with no sync server.
- A whole-collection package (`.colpkg`), which replaces the collection rather than adding to it.
- The first deploy, any change to the engine pin or to an engine golden, and any new
  `EngineRefusal` case.

## 7. Risks

- The `ShellFlowTests` class on the iPhone and the load-wait classes `ReviewFlowTests` and
  `ReviewActionsFlowTests` can red any change to the app; such a red is read by name against its
  record and is never re-run to clear it.
- The system file importer's own elements differ between system versions; A1 reads the importer
  by what it shows on both destinations, and a red there is read by name before any change.
- The import pair is numbered from the engine's descriptor order; a re-pinned engine that
  renumbers it reds A2 and A3 by name.
- A decode of 1 GiB in a debug build costs time; each oversize plant is run-length blocks, which
  decode at little cost.

## 8. FORMAL, decided by surface

Not applicable: `config/formal.json` and `formal/obligations.json` name no path this change
touches, each counted 0, and no Kani proof reads one.
