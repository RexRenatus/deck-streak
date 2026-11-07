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
