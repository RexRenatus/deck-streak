# Red-first record: SPEC-638

SPEC-638 (R1 to R7, criteria A1 to A3). SPEC-638, ADR-661 and the insert-only section 7 of the
answer's schematic were committed first and alone (ffad0d56), and the tests of the three criteria
were committed with this record in the commit after it. All three are MUTATION COVERAGE, not red
first, as ADR-661 D5 says: both handovers are on `dev`, so no edit of the base could make a
criterion red for its reason without changing what is pinned. Each criterion's red is its rows
instead, rows S63801 and S63802 for the native press and row S63803 for the web census, each proved
by hand below; the browser test has no row, since no row's killer can run it, and its text is held
by the census row. There is no greening commit. The fence runs from the worktree root, as SPEC-638
section 3 writes it.

```text
A1: cargo test -p deck-streak-ffi --test stale_press -- --exact a_native_press_on_a_card_changed_since_it_was_shown_is_the_engines_refusal_and_records_nothing
A2: pnpm --dir web/app test:engine -g "a rating on a card a sync changed since it was shown is the engine refusal and records nothing"
A3: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
```

```red-first
A1: not red: pins the base's native handover of the engine refusal; its reds are hand rows S63801 and S63802, killed by A1
A2: not red: pins the base's web handover in two browsers; no row can run it, and its text is held by A3's row S63803
A3: not red: pins the base's census of rate; its red is hand row S63803, killed by A3
```

## Each row's proof, by hand

At e41a382f, the commit that adds the tests, each row's mutant was installed alone in its file, its
killer was run, and the file was then restored by writing its saved bytes back and checked
byte-equal to the committed file, its sha256 reading as at the base, and touched, before the next
row. Each quote below is a run of the killer's own output.

`S63801-THE-NATIVE-PRESS-NAMES-THE-ENGINES-REFUSAL`, the native adapter naming the engine's refusal
`PressRefusal::NotTheCard` (`crates/ffi/src/engine.rs`, the arm of `press_refusal`): its mutant was
installed once from the band file by a script that refuses unless the row's find occurs once, and
the A1 test, run by its fence line's `--exact` selection with the crate recompiled under the mutant,
read `FAILED` at its one `assert_eq!`. The second press reached the caller as `Other(NotTheCard)`
where the pin holds the engine's refusal, while the first press and the queue read as pinned:

```text
panicked at crates/ffi/tests/stale_press.rs:124:5:
left: (true, Other(NotTheCard), Head {
right: (true, EngineRefused(0, "card was modified"), Head {
```

The file was then restored, and `crates/ffi/src/engine.rs` read
`f39f7f103fdb9c0bc6a649ba47e91388e949abb4346484e05a78c0e5756fbb45` again before the next
measurement.

`S63802-A-REFUSED-NATIVE-PRESS-RECORDS-NOTHING`, the native adapter reading the core's refusal as an
empty reply (`.unwrap_or_default()` in place of `.map_err(press_refusal)?` in `Engine::answer`):
installed the same way, the A1 test read `FAILED` at its one `assert_eq!`. The second press reached
the caller as recorded with an empty reply where the pin holds the engine's refusal; the engine
itself refused and wrote nothing, so the queue read as the first press left it:

```text
panicked at crates/ffi/tests/stale_press.rs:124:5:
left: (true, Recorded(0), Head {
right: (true, EngineRefused(0, "card was modified"), Head {
```

The file was then restored, and `crates/ffi/src/engine.rs` read
`f39f7f103fdb9c0bc6a649ba47e91388e949abb4346484e05a78c0e5756fbb45` again before the next
measurement.

`S63803-THE-WEB-RATE-THROWS-THE-ENGINES-REFUSAL`, the web `rate` swallowing the core's refusal
(`.unwrap_or_default()` in place of `.map_err(refuse)?` in `crates/web-engine/src/wasm.rs`):
installed the same way, the A3 test, which reads that file's text when it runs, read `FAILED` at its
one `assert_eq!`, naming `rate` and the statement its body lacks:

```text
panicked at crates/web-engine/tests/boundary.rs:505:5:
left: (["rate records only the kept card's press, as the owner's answer, with the state its grade picks, and its body lacks `&request.encode_to_vec()).map_err(refuse)?`"], [])
right: ([], [])
```

The file was then restored, and `crates/web-engine/src/wasm.rs` read
`483fa9d8d4a81c2d178a7777fda7517a69a7b22c24846d87f944a25fd71124c3` again before the next
measurement.

None of the three mutants failed to compile. After the third restore, both killers ran green on the
restored tree, one run each: the A1 test (the crate recompiled) and the A3 test each read `test
result: ok. 1 passed; 0 failed`. CI's rows job decides the three rows by id after the push.
