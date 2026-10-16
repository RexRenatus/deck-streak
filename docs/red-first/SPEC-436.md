# Red-first record: SPEC-436

SPEC-436 (R1 to R24, A1 to A14). The SPEC, ADR-450, the schematic, the tests of A1 to A14 in
`crates/daemon/tests/await_lints.rs`, `crates/daemon/tests/blocking_census.rs` and
`crates/daemon/tests/handler_panic_census.rs`, the shared reader
`crates/daemon/tests/shipped_sources/mod.rs` and the daemon's `proc-macro2` dev-dependency were
committed first and alone, over stubs that keep every input but the missing behaviour: the
manifest reader finds no entry, the lowering scan finds no lowering, each census's walk examines
nothing and finds no site, the verdict passes everything, and no refusal refuses. The listings
were already there (the blocking census's eleven keys, the panic census's empty listing, both
levels at `Warn` and an empty `SUPPRESSIONS`), and `Cargo.toml` gained R1's two entries only in
the green commit, so each red below is its criterion's assertion and never a missing name.

```text
A1: cargo test -p deck-streak-daemon --test await_lints -- --exact the_workspace_denies_both_await_lints
A2: cargo test -p deck-streak-daemon --test await_lints -- --exact a_missing_lowered_or_misplaced_await_lint_entry_is_refused
A3: cargo test -p deck-streak-daemon --test await_lints -- --exact a_lowering_of_an_await_lint_or_its_group_is_refused_wherever_it_is_planted
A4: cargo test -p deck-streak-daemon --test await_lints -- --exact the_repository_lowers_no_await_lint_anywhere
A5: cargo test -p deck-streak-daemon --test blocking_census -- --exact every_blocking_call_is_found_in_an_async_fn_body_however_it_is_named
A6: cargo test -p deck-streak-daemon --test blocking_census -- --exact an_async_block_and_an_async_closure_are_async_bodies
A7: cargo test -p deck-streak-daemon --test blocking_census -- --exact a_rail_a_nested_fn_test_code_and_the_async_equivalents_are_not_blocking_sites
A8: cargo test -p deck-streak-daemon --test blocking_census -- --exact the_repository_lists_exactly_todays_blocking_sites
A9: cargo test -p deck-streak-daemon --test blocking_census -- --exact the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A10: cargo test -p deck-streak-daemon --test blocking_census -- --exact the_blocking_census_refuses_an_empty_population
A11: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact every_panic_form_in_a_handler_crate_is_found
A12: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact test_code_debug_assertions_const_contexts_and_lookalikes_are_not_handler_panics
A13: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact the_handler_crates_hold_exactly_todays_listing
A14: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact the_panic_census_refuses_an_empty_population
```

Eleven reds were read by cargo over planted text: the three binaries run with the three
repository tests skipped failed exactly the tests the fence above names for A1 to A3, A5 to A7,
A9 to A12 and A14 (three, five and three per binary), each on its first assertion, and no other
test of the daemon's suite. The three repository tests read the tree, so their reds were read in
CI: the rust job 117043344169, a job of 2545 s, failed exactly the fourteen tests of A1 to A14 and no other of the 1910 it ran, with its clippy stage passing, each repository test on its first assertion with an empty left side.

Test edits after the red commit: none. The green commit replaces the stub bodies with the readers
and adds R1's two entries; it changes no test function, planted text or expected value. The rows
commit and this record's commit edit no test.

No CI run reads the green commit or the rows commit alone: A4's, A8's and A13's greens are read at
the second push's head, which carries the green commit, the rows and this record. The planted-text
greens were read by cargo at the green commit.

```red-first
A1: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no level read for either lint)
A1: green at d58bd8445472c4f1a321355fc932b9523168db1f
A2: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed: [workspace.lints.clippy], left: [] (no refusal for a missing entry)
A2: green at d58bd8445472c4f1a321355fc932b9523168db1f
A3: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no lowering refused)
A3: green at d58bd8445472c4f1a321355fc932b9523168db1f
A4: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no manifest examined, against the 32 listed manifests)
A4: green at d58bd8445472c4f1a321355fc932b9523168db1f
A5: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no site found)
A5: green at d58bd8445472c4f1a321355fc932b9523168db1f
A6: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no site found)
A6: green at d58bd8445472c4f1a321355fc932b9523168db1f
A7: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (the control site not found)
A7: green at d58bd8445472c4f1a321355fc932b9523168db1f
A8: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no site found, against the eleven listed keys)
A8: green at d58bd8445472c4f1a321355fc932b9523168db1f
A9: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: Ok([]) (no WARN line printed)
A9: green at d58bd8445472c4f1a321355fc932b9523168db1f
A10: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: None (an empty population not refused)
A10: green at d58bd8445472c4f1a321355fc932b9523168db1f
A11: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no panic form found)
A11: green at d58bd8445472c4f1a321355fc932b9523168db1f
A12: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (the control panic not found)
A12: green at d58bd8445472c4f1a321355fc932b9523168db1f
A13: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: [] (no handler crate examined, against api, bot, daemon and mcp)
A13: green at d58bd8445472c4f1a321355fc932b9523168db1f
A14: red at ce91da21514fd24bbab2dd5a0f8f67ee59e8e615: assertion `left == right` failed, left: None (an empty population not refused)
A14: green at d58bd8445472c4f1a321355fc932b9523168db1f
```
