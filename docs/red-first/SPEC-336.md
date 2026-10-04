# Red-first record: SPEC-336

The adapter's shape was committed first (fb753166): the crate, its constant allow-list of five
engine calls and an entry point that refuses every call. The six round-trip tests were committed
alone on top of it (3e42132c), and at that commit each fails by assertion, because the adapter
refuses the calls it lists (`cargo nextest run --locked --build-jobs 1 --test-threads 1
--no-fail-fast -p deck-streak-ffi --test round_trip`: 6 tests run, 0 passed, 6 failed). The
dispatch was committed next (0083ccc7), with the tests unchanged: 6 passed.

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
```

Two plants, never committed, show the tests can tell a wrong adapter apart (the same command, at
0083ccc7):

```text
the allow-list check removed (every call reaches the engine): A6 FAILS, 5 of 6 pass; the unlisted
  CloseCollection is answered Ok([]) by the engine, which closes the collection
Undo's pair changed from (3, 8) to (3, 9): A5 FAILS, 5 of 6 pass; Undo is refused by the allow-list
```

## Mutation coverage

MUTATION COVERAGE, not red-first. cargo-mutants lists 10 mutants of the adapter
(`cargo mutants --no-shuffle --list -p deck-streak-ffi`), and two of them were observed by no test
when the record above was written.

- `engine.rs: replace <impl fmt::Display for EngineRefusal>::fmt -> fmt::Result with
  Ok(Default::default())`: `refusal_text::each_refusal_reads_as_its_own_sentence` was added at
  e9ac46bf, after the code, and changes no production file. It is green at its base 232c2135 and at
  its own commit. Row S33600 installs the same mutant by hand, and
  `python3 scripts/mutation_rows.py prove --band S33600-S33699` reads it KILLED, its killer selecting
  one test with and without the mutant and the target restored byte for byte. With the mutant the
  test fails by assertion: `left: (Err(""), "", "")`.
- `bin/uniffi-bindgen-swift.rs: replace main with ()`: not killed. The generator is built only with
  the adapter's `bindgen` feature, and no default-feature build compiles it, the pull request's
  mutation run included, so no test of the crate can run it. Each cure changes production code, the test
  scope or the listing, and is the seat's ruling, not this delivery's.
