# SPEC-681: the red-first record

```red-first
A1: red at 97755f2: XCTAssertTrue failed at PackageImportFlowTests.swift:30 on the iPhone and the iPad: the Import control opens the system file importer: its search field or its Cancel button shows
A1: green at 2a256f6
A2: red at cae56dd: assertion `left == right` failed: the package's note and card join the target's own through the one import call, which answered Err(NotAllowed { service: 39, method: 2 })
A2: green at 55d9a90
A3: red at cae56dd: assertion `left == right` failed: the native allow-list admits service 39 method 2 as the package import
A3: green at 55d9a90
A4: red at cae56dd: assertion `left == right` failed: a file that is no package reads as the refusal's words
A4: green at 2a256f6
A5: red at cae56dd: assertion `left == right` failed: a collection member that decompresses one byte past the bound reads as the bound's words
A5: green at 55d9a90
A6: red at 97755f2: XCTAssertTrue failed at PackageImportFlowTests.swift:57 on the iPhone and the iPad: the empty deck list shows the import offer import-deck, ready to tap
A6: green at 2a256f6
```

The native tests were committed in cae56dd with every other input kept. A2, A3, A4's native
test `an_unreadable_package_is_refused_in_words_with_no_row` and A5's collection test read red at
their first assertions on one cause: the native allow-list held no import pair, so the call
answered `NotAllowed { service: 39, method: 2 }`. A5's media test
`an_oversize_media_member_is_refused_before_any_row` read red the same way at its first assertion,
that a media member decompressing one byte past the bound reads as the bound's words, and so did
the options test `the_import_keeps_scheduling_and_note_types_under_its_four_options`, which holds
the four options of R2. The first push's `ci` run (43379037886) read those six red by name in its
`rust` and `release` jobs, and no other test failed. Two earlier local forms of that commit,
67a3731 and 1d2202f, never reached a criterion's first assertion and were amended before any
push.

The interface tests sit in `ios/AppUITests/PackageImportFlowTests.swift`. Their first form, in
cae56dd, read a value the app's test step does not set, so they failed at their setup on both
destinations, which is no red. 97755f2 plants the fresh collection and the refused file in the
test runner's own temporary folder, with every assertion, test name and message unchanged. At
97755f2 the apple run's `release-and-app` job (131931636997), step `the app's tests, Debug, on the
iPhone and then the iPad`, read A1's `testTheDeckListOpensTheSystemFileImporter` red at line 30,
A4's `testARefusedPackageReadsAsWords` red at lines 43 and 47 and A6's
`testTheEmptyDeckListOffersTheImport` red at line 57, on the iPhone and on the iPad, each at its
own assertion. A4's native test is green from 55d9a90, so A4 is green with its interface test, at
2a256f6.

55d9a90 carries the import pair into the four lists that pin the two tables: `EXPECTED` in
`crates/ffi/tests/review_pairs.rs`, `NATIVE` in `crates/engine-core/tests/table.rs` and in
`crates/engine-core/tests/review_pairs.rs`, and `ORDINARY_ROWS` in
`crates/engine-core/tests/data_screen_pairs.rs` each took the pair's entry and the counts it moves,
and no other assertion changed. Between 97755f2 and 2a256f6 these commits change a test file or the
rows, and none changes a criterion's assertion: 46f0585 moves the kernel census's source count from
429 to 430 for the new reader module and re-points three citations of a schematic by line number
alone; 66fd797 adds four tests that kill the reader's four missed mutants; 9ab45af adds a
source-text guard for the reader module's declaration; db9026a adds the mutation rows S68100 to
S68114, each killer read red with its row planted and green without it.

4cf50b5 bounds the reader's varint writer, whose first form looped while a value held more than
seven bits: a mutant of that loop's condition never ended, and the second push's
`mutation-verdict` job (131913322399) read it as a memory cap. The new body is a loop of at most
ten passes with an early return, its bytes equal to the first form's on every input. Its killer,
`the_varint_writer_writes_exact_leb128_bytes`, is the one test inside the reader module, since the
writer is private; it asserts the bytes at 0, 0x7F, 0x80, 0x3FFF, 0x4000 and the largest value.
Each mutant of the new body dies inside the bound: the body replaced by nothing writes no byte at
0; the early exit's `<` as `==` or `>` writes ten continuation bytes at 0, and as `<=` writes one
byte at 0x80; the mask's `&` as `|` or `^` writes 7F 01 at 0x80; the `+` as `*` writes 00 01 at
0x80, and as `-` panics on overflow at 0x80; the shift's `>>=` as `<<=` writes ten continuation
bytes at 0x80. The third push's `mutation-verdict` job reads these mutants by name.

Each Rust criterion's test function in `crates/ffi/tests/package_import.rs` at 55d9a90, its doc
comment excepted, is byte-equal at this record's commit, so the Rust greens read at 55d9a90 stand.
The SPEC's file manifest gains four rows the build measured: the kernel census test, the
schematic, the band of mutation rows and `ios/App/Sources/AppModel.swift`, which holds the import
model.

The green commit `2a256f6` carries the importer, and `git diff --quiet 2a256f6 HEAD -- Cargo.lock changelog.d/ios-file-import-681.md crates/engine-core/src/table.rs crates/engine-core/tests/data_screen_pairs.rs crates/engine-core/tests/review_pairs.rs crates/engine-core/tests/table.rs crates/ffi/Cargo.toml crates/ffi/src/allow_list.rs crates/ffi/src/engine.rs crates/ffi/src/lib.rs crates/ffi/src/package_bound.rs crates/ffi/tests/package_import.rs crates/ffi/tests/review_pairs.rs crates/kernel/tests/allow_list_arms.rs ios/App/Info.plist ios/App/Sources/AppModel.swift ios/App/Sources/DeckListView.swift ios/App/Sources/EngineSession.swift ios/App/Sources/PackageImportModel.swift ios/App/Sources/PackageImportRequest.swift ios/App/Sources/PackageImportView.swift ios/AppUITests/PackageImportFlowTests.swift ios/swift-roles.json scripts/mutation-rows.d/S68100-S68199.json` reads rc 0.
