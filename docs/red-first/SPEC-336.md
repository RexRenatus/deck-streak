# Red-first record: SPEC-336

The adapter's shape was committed first (fb753166): the crate, its constant allow-list of five
engine calls and an entry point that refuses every call. The six round-trip tests were committed
alone on top of it (3e42132c), and at that commit each fails by assertion, because the adapter
refuses the calls it lists (`cargo nextest run --locked --build-jobs 1 --test-threads 1
--no-fail-fast -p deck-streak-ffi --test round_trip`: 6 tests run, 0 passed, 6 failed). The
dispatch was committed next (0083ccc7), with the tests unchanged: 6 passed.

A8 came with ADR-345 D5. Before it, no default-feature build compiled the generator's binary, so
a test naming `CARGO_BIN_EXE_uniffi-bindgen-swift` could not compile there. A8's test was
therefore committed (f84b7c46) beside the change's first half: the binary's `required-features`
dropped and the generator's call put behind the feature, with no refusal yet, so the
default-feature binary ran a `main` that did nothing and exited 0. At that commit the crate's whole
test population (`cargo nextest run --locked --build-jobs 1 --test-threads 1 --no-fail-fast -p
deck-streak-ffi`) ran 8 tests: 7 passed, and A8 failed by assertion. The refusal was committed
next (61499a76), with the test unchanged: 8 passed.

```red-first
A1: red at 3e42132c: assertion `left == right` failed: left: (Err(NotAllowed { service: 3, method: 0 }), "the allow-list") right: (Ok([]), "the engine")
A2: red at 3e42132c: assertion `left == right` failed: left: (Err(NotAllowed { service: 3, method: 0 }), Err(NotAllowed { service: 7, method: 13 })) right: (Ok([]), Ok(["Default", "Synthetic"]))
A3: red at 3e42132c: assertion `left == right` failed: left: (Err(NotAllowed { service: 3, method: 0 }), Err(NotAllowed { service: 13, method: 3 })) right: (Ok([]), Ok(Queued { .., queue: 0, new: 1, learning: 0, review: 0 }))
A4: red at 3e42132c: assertion `left == right` failed: left: (Err(NotAllowed { service: 3, method: 0 }), Err(NotAllowed { service: 13, method: 4 }), Err(NotAllowed { service: 13, method: 3 })) right: (Ok([]), Ok((1, 1)), Ok(0))
A5: red at 3e42132c: assertion `left == right` failed: left: (Err(NotAllowed { service: 3, method: 0 }), "the allow-list", Err(NotAllowed { service: 3, method: 8 }), Err(NotAllowed { service: 13, method: 3 })) right: (Ok([]), "nobody", Ok("Answer Card"), Ok(Queued { .., queue: 0, new: 1, learning: 0, review: 0 }))
A6: red at 3e42132c: assertion `left == right` failed: the four unlisted pairs refused as expected, but left: (.., Err(NotAllowed { service: 3, method: 0 }), Err(NotAllowed { service: 7, method: 13 })) right: (.., Ok([]), Ok(["Default", "Synthetic"]))
A1: green at 0083ccc7
A2: green at 0083ccc7
A3: green at 0083ccc7
A4: green at 0083ccc7
A5: green at 0083ccc7
A6: green at 0083ccc7
A7: not red: the admission and its test landed together in fb753166; before the admission the hardening test refused this workflow's runner (1 of 38 tests failed)
A8: red at f84b7c46: assertion `left == right` failed: left: Ok((Some(0), "", "")) right: Ok((Some(2), "uniffi-bindgen-swift: this build holds no Swift bindings generator, because it was built without the `bindgen` feature; run it as ..", ""))
A8: green at 61499a76
A9: red at e3e34e37: assertion `left == right` failed: each call is refused for the call and for nothing else: left: 1 right: 5 (the control read ["crates/habits/src/lib.rs calls settle, and only coordination's code may"]; each of the four calls under a feature read only ["crates/habits/Cargo.toml declares a feature, and the census compiles none"])
A10: red at e3e34e37: assertion `left == right` failed: each build script is refused for the script and for nothing else: left: 1 right: 2 (the edge a feature turns on read ["crates/habits/Cargo.toml declares a feature, and the census compiles none"])
A11: red at e3e34e37: assertion `left == right` failed: each tree is judged as its case expects: left: 0 right: 3 (each tree, the control included, read ["crates/habits/Cargo.toml declares a feature, and the census compiles none"])
A12: red at e3e34e37: assertion `left == right` failed: the tree at the limit is accepted and the tree past it refused by name: left: [(3, ["crates/habits/Cargo.toml declares a feature, and the census compiles none"]), (4, ["crates/habits/Cargo.toml declares a feature, and the census compiles none"])]
A13: red at b91c1482: 2 capture(s) bypass the helper: ["crates/ffi/src/engine.rs:71: init", "crates/ffi/src/engine.rs:72: init"]
A14: red at e3e34e37: assertion `left == right` failed: left: ["crates/ffi/Cargo.toml declares a feature, and the census compiles none"] right: []
A9: green at 790ceb5d
A10: green at 790ceb5d
A11: green at 790ceb5d
A12: green at 790ceb5d
A13: green at 3ccf2769
A14: green at 790ceb5d
```

Two plants, never committed, show the tests can tell a wrong adapter apart (the same command, at
0083ccc7):

```text
the allow-list check removed (every call reaches the engine): A6 FAILS, 5 of 6 pass; the unlisted
  CloseCollection is answered Ok([]) by the engine, which closes the collection
Undo's pair changed from (3, 8) to (3, 9): A5 FAILS, 5 of 6 pass; Undo is refused by the allow-list
```

A9 to A14 came with ADR-345 D6 and R11 to R12. The engine constructor's parameter was renamed
first (3ccf2769) against the log-capture census, which reads `init` as an install token: at the
absorb of `dev` (b91c1482) A13 failed by assertion, and at the rename it passed (`cargo nextest run
--build-jobs 1 --test-threads 1 -p deck-streak-kernel --test log_capture_class`: 8 run, 8 passed).
The settle census's four new tests, and the changed expectations of
`the_census_refuses_what_the_compiler_is_not_asked` and verify round 7's H07, were committed alone
(e3e34e37). At that commit the census refused every tree whose member declared a feature before it
compiled any code, so each planted call under a feature read only that refusal, while each control
was refused for its own reason: 5 tests run, 0 passed, 5 failed. The census change was committed
next (790ceb5d), with the tests unchanged: 5 passed, and the whole file green.

The census's own killer, `the_census_refuses_every_caller_the_compiler_finds`, held one control
refused by construction: a feature gating a call to the other crate's `settle`, which the census
refused for the feature because it compiled none. Under the change it compiles the feature and
accepts that control, as it accepts every control whose call reaches the other crate (the whole
file at the change, before the expectation moved: 47 run, 46 passed, 1 failed, `wrong: control S2
C configuration: a feature gates the call | accepted`, members escaping 0). So that control's
expectation and the population's pinned digest changed in the change's commit (790ceb5d); the
member beside it, whose call reaches `settle`, is still refused.

## Mutation coverage

MUTATION COVERAGE, not red-first. cargo-mutants lists 10 mutants of the adapter
(`cargo mutants --no-shuffle --list -p deck-streak-ffi`), the same 10 before and after A8's
change, the generator's `main` moving from line 9 to line 15. Two of them were observed by no test
when the record above was written.

- `engine.rs: replace <impl fmt::Display for EngineRefusal>::fmt -> fmt::Result with
  Ok(Default::default())`: `refusal_text::each_refusal_reads_as_its_own_sentence` was added at
  e9ac46bf, after the code, and changes no production file. It is green at its base 232c2135 and at
  its own commit. Row S33600 installs the same mutant by hand, and `python3 scripts/mutation_rows.py
  prove --band S33600-S33699` reads it KILLED, its killer selecting one test with and without the
  mutant and the target restored byte for byte. With the mutant the test fails by assertion: `left:
  (Err(""), "", "")`.
- `bin/uniffi-bindgen-swift.rs: replace main with ()`: caught by A8, whose red commit above is this
  mutant's behaviour in a default-feature build. Row S33601 installs the same mutant by hand, and
  the same `prove --band` reads it KILLED; replayed by hand, the killer selects one test and fails
  by assertion, `left: Ok((Some(0), "", ""))`, and the binary's source is restored byte for byte.

The crate's whole population, measured at 4b9516aa with
`cargo mutants --no-shuffle --in-place -p deck-streak-ffi`: 10 mutants tested, 9 caught, 1
unviable and 0 missed. The unviable mutant replaces `allowed` with
`Some(Box::leak(Box::new(Default::default())))`, which does not compile because `Call` has no
`Default`.

Two controls on R11 are MUTATION COVERAGE, not red-first.
`the_census_refuses_a_call_gated_on_two_features_together` plants habits' call to `settle` under
`cfg(all(feature = "a", feature = "b"))`, and `the_census_refuses_a_call_gated_on_a_features_absence`
plants it under `cfg(not(feature = "a"))` with no default, each beside the same call under no
feature. They were added at 45c5e7ec, after the census change, and change no line of the census:
the census there is 790ceb5d's, and it already refused both plants for the call, so no red was owed
(`cargo nextest run --build-jobs 1 --test-threads 1 --no-capture -p deck-streak-progression --test
xp_census -E 'test(/^the_census_refuses_a_call_gated_on_/)'`: 2 run, 2 passed; each plant and each
control read `["crates/habits/src/lib.rs calls settle, and only coordination's code may"]`). Row
S33613 installs a combination that holds only its highest feature, and row S33614 a census that
skips the empty combination. `python3 scripts/mutation_rows.py prove --band S33600-S33699` reads
all 15 rows of the band KILLED, each killer selecting one test with and without its mutant; replayed
by hand, each killer fails by assertion with its plant read `[]` (S33613: `left: 1`, `right: 2`;
S33614: `left: 0`, `right: 2`), and the census is restored byte for byte.
