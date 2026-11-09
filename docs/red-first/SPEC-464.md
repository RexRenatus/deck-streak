# Red-first record: SPEC-464

SPEC-464 (R1 to R6, A1 to A3). The SPEC, ADR-481, the schematic and the two insert-only amendments
(SPEC-410 R3, ADR-424 D1) were committed first (`58912c80fadffc472ddd6b42a8868d4338752bd2`), with
no Rust changed. The three tests were committed next, alone
(`28d54bd5b8ec2bea337e3c6a06729ea504f3f2a5`), and A1 and A2 each read red there for its own
criterion, on the assertion about the missing elements: for the reader that asks for video, the
escaped `mp4`, `m4v` and `mov` spellings yielded no element. The hand table's checks against both
tables, the element helper's control and both plain-spelling controls passed before that
assertion. A3 passed there: it pins what the base already does. Every red below is quoted from the
run of its fence line over that commit, locally.

```text
A1: cargo test -p deck-streak-engine-core --test face -- --exact an_escaped_dot_presents_as_its_plain_spelling_over_the_whole_type_table
A2: cargo test -p deck-streak-engine-core --test face -- --exact an_escaped_extension_letter_presents_as_its_plain_spelling_over_the_whole_type_table
A3: cargo test -p deck-streak-engine-core --test face -- --exact escaped_names_admit_refuses_stay_refused_under_both_caps_for_both_readers
```

```red-first
A1: red at 28d54bd5b8ec2bea337e3c6a06729ea504f3f2a5: assertion `left == right` failed: a reader that asks, the dot escaped: the elements after the side's text
A1: green at 2dce2bd360a5da789604a0253c3da32877be5569
A2: red at 28d54bd5b8ec2bea337e3c6a06729ea504f3f2a5: assertion `left == right` failed: a reader that asks, the extension's first letter escaped: the elements after the side's text
A2: green at 2dce2bd360a5da789604a0253c3da32877be5569
A3: not red: pins what dev already does: names admit refuses stay refused under both caps when spelt with escapes, for both readers (MUTATION COVERAGE, row S46401)
```

At the red commit each of the two failing assertions read `left: ""`, and on the right the three
elements of `x.mp4`, `x.m4v` and `x.mov`, in that order: `video/mp4` over `eC5tcDQ=`, `video/mp4`
over `eC5tNHY=` and `video/quicktime` over `eC5tb3Y=`. The core's whole suite over that commit,
every binary run: 155 tests, 153 passed and 2 failed (A1 and A2). The formatter and the lints of
the crate were green.

The green is the commit `2dce2bd360a5da789604a0253c3da32877be5569`, which changes
`crates/engine-core/src/media.rs` only: `Budget::shows_video` binds `written` to the name
`decoded_name` gives, above its one line, which is unchanged. The tests are those of the red commit,
unchanged since. A1, A2 and A3 each passed there, one test selected each, and the core's whole
suite passed, 155 tests of 155; the formatter and the lints of the crate were green.
