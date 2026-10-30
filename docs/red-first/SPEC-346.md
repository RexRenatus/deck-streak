# Red-first record: SPEC-346

The SPEC, its schematic and ADR-357 were committed first (482c1276). The tests were then committed
alone (b95184ce), and the first push carried those two commits only. The reds below are quoted from
the `hygiene` job's python stage log of run 37269267865 at head b95184ce, where the suite read
`Ran 958 tests` and `FAILED (failures=4)`, the four being A6 to A9. The change is 12a1e198. The
greens are read from the `hygiene` job of run 37271352050 at 1292983e, where the suite read
`Ran 958 tests` and `OK`; 1292983e adds only the band file to 12a1e198, and none of A5 to A9's
tests reads it.

```red-first
A1: not red: a census over a property the base already holds (no member declares the staticlib crate type); its planted members are refused in the same test
A2: not red: a census over a property the base already holds (uniffi is named by the umbrella alone, and every member inherits the forbidding lints); its planted members and lockfile entries are refused in the same test
A3: not red: a census over a property the base already holds (one local binary target, and no generator spec naming a framework, Carthage or remote package); its planted Swift sides and generator specs are refused in the same test
A4: not red: a census over a property the base already holds (no built library is committed); its planted listing is refused in the same test
A5: not red: the Apple job already builds the umbrella alone as the static library; at b95184ce the test read `examined 1 cargo commands naming a crate type` and passed
A6: red at b95184ce: AssertionError: 'the bindings hold one module' not found in [None, 'the pinned Rust toolchain and its two iOS targets, before the clock starts', 'protoc, checksum-verified', 'the two static libraries, clocked from the first compile to the second archive', 'the Swift bindings, in library mode over the device library', 'the modulemap check, which a builtin clang module refuses', 'the XCFramework, device and simulator slices', "the consumer check, the generated Swift typechecked against each slice's headers", "the harness's synthetic collection, written by this commit's engine", 'the report', 'upload the XCFramework, the bindings and the report']
A6: green at 12a1e198
A7: red at b95184ce: AssertionError: 'the XCFramework holds one Rust library' not found in [None, 'the pinned Rust toolchain and its two iOS targets, before the clock starts', 'protoc, checksum-verified', 'the two static libraries, clocked from the first compile to the second archive', 'the Swift bindings, in library mode over the device library', 'the modulemap check, which a builtin clang module refuses', 'the XCFramework, device and simulator slices', "the consumer check, the generated Swift typechecked against each slice's headers", "the harness's synthetic collection, written by this commit's engine", 'the report', 'upload the XCFramework, the bindings and the report']
A7: green at 12a1e198
A8: red at b95184ce: AssertionError: Lists differ: ['crates/ffi/**'] != ['crates/engine-core/**', 'crates/ffi/**']
A8: green at 12a1e198
A9: red at b95184ce: AssertionError: {'pul[105 chars]*', 'ios/**', 'Cargo.lock', 'Cargo.toml', 'rus[93 chars]l']}} != {'pul[105 chars]*', 'crates/engine-core/**', 'ios/**', 'Cargo.[118 chars]l']}}
A9: green at 12a1e198
```

A10 is not red: commit R touches no Apple path, so no macOS run measured it. Its line left
SPEC-346's acceptance fence in the commit that adds this record, because no fence line can name
this pull request's run before that run exists; the table still states it, so it is recorded
here in prose rather than in the fence. Its green is the change caller's run 37271352793 at
1292983e: the job `apple / xcframework` printed
`bindings: deck_streak_ffi.swift deck_streak_ffiFFI.h module.modulemap`, `modules declared: 1`
and `libraries: ios-arm64-simulator/libdeck_streak_ffi.a ios-arm64/libdeck_streak_ffi.a`, and
its `xcframework` artifact's `xcframework-report/one-module` and
`xcframework-report/one-library` each read `pass`.
