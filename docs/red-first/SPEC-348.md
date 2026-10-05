# Red-first record: SPEC-348

The SPEC, its schematic, ADR-359 and the changelog fragment were committed first. Each criterion's
test was then committed before the code that turns it green, over a stub that compiles and does
nothing, and each red below is quoted from the run at the red commit. This record covers part a,
A1 to A11; A12 to A23 are the next part's.

```red-first
A1: red at 47ab0eda: the native dispatcher answers DecksService.DeckTree (7,4): NotAllowed { service: 7, method: 4 }
A2: red at 47ab0eda: the adapter answers DecksService.DeckTree (7,4): NotAllowed { service: 7, method: 4 }
A1: green at 46938f8e
A2: green at 46938f8e
A3: red at eb063628: assertion `left == right` failed: the answer's replay: left: [], right: the question's two Sound clips
A4: red at eb063628: assertion `left == right` failed: left: ["dot.png"], right: ["data:image/png;base64,iVBORw0KGgo="]
A5: red at eb063628: assertion `left == right` failed: left: ["sub/dot.png", "..", ...] (the planted path's attribute unchanged)
A6: red at eb063628: assertion `left == right` failed: the speech clips: left: []
A7: red at eb063628: assertion `left == right` failed: the default preset, wished: the replay is full: left: []
A3: green at 68d81d2e
A4: green at 68d81d2e
A5: green at 68d81d2e
A6: green at 68d81d2e
A7: green at 68d81d2e
A11: red at 7b7cd737: the review fixture wrote its collection at <target tmp>/ffi-review-fixture/<run>/collection.anki2
A11: green at 6aa81b35
A8: red at 32de32ac: by day: the image's src is the data: URL of the file, not ""
A8: green at 0f2ad8d4
A9: red at dde76e0e: assertion `left == right` failed: the line before the damaged one is read
A9: green at 7da31e92
A10: red at 50f4a4b5: 1 passed; 5 failed: a_flag_without_a_value_is_refused, a_relative_directory_is_refused, a_missing_directory_is_refused and a_file_is_refused_as_not_a_directory read left: Ok("/default/collection") against their refusal, and an_absolute_existing_directory_is_returned read the default against the directory
A10: green at 4bce975f
```

46938f8e, A1's and A2's green, also grew `crates/engine-core/tests/table.rs`'s `NATIVE` set from
seven entries to ten: the three pairs SPEC-348 R1 adds to the native column. That test pins the
column, so admitting a pair meant editing it; the edit is recorded here as the one weakening a
reader of that table should know about, and the three rows S34803 to S34805 hold each entry.

A11 precedes A8 in the commit order because A8 reads the fixture A11 proves. A8, A9 and A10 each
committed the module or type their test names with a stub body: the native face's reader finds no
file, the voice choice holds nothing, and the collection directory is the default for every
argument. `no_argument_gives_the_default` was green at A10's red commit, because the stub returns
the default; the other five of A10's six tests were red there.
