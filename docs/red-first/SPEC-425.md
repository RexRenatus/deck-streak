# Red-first record: SPEC-425

SPEC-425 (R1 to R6, A1 to A3). SPEC-425, ADR-439, the schematic `docs/schematics/web-sink-census.md`
and SPEC-341's section 8 were committed first (ed0dca7b). The tests of A1 to A3 were committed next
and alone (b903cae6), over a census whose matcher already took a table of arms and whose new table,
`ZERO_SHAPES`, was empty, with the walk unchanged and no fall-through flag in
`web/app/tsconfig.json`. Each red below is therefore its criterion's assertion and never a missing
name. The census's arms live in its own test module, so A1's and A2's implementation is an edit of
that module (f8f41858): it fills `ZERO_SHAPES`, adds `blankTargetsWithoutRel` and lets the walk read
`.html`, and it changes no plant, no near miss and no assertion of A1 or A2. The flag came last
(3e53179d), and the records, the rows and the fragment after it.

```text
A1: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "shipped code holds none of the six refused shapes"
A2: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "the census reads every shipped kind, the page shell included"
A3: python3 -m unittest discover -s scripts/tests -p test_web_type_check.py -k test_the_type_check_reads_no_fallthrough_from_the_apps_own_config
```

```red-first
A1: red at b903cae6: card-sinks.test.ts: const run = eval(card);: expected {} to deeply equal { 'eval(': 1 }
A2: red at b903cae6: card-sinks.test.ts: expected [ '.js', '.svelte', '.ts' ] to deeply equal [ '.html', '.js', '.svelte', '.ts' ]
A3: red at b903cae6: test_web_type_check.py: AssertionError: None is not True : web/app/tsconfig.json lets a switch case fall through
A1: green at f8f41858
A2: green at f8f41858
A3: green at 3e53179d
```

Each red and green was read on the builder's own tree by its fence line above. At b903cae6 the whole
Vitest suite reads `Test Files 1 failed | 111 passed (112)` and `Tests 2 failed | 683 passed (685)`,
its two failures A1 and A2, and A3's module reads `FAILED (failures=1)`. At f8f41858 the whole suite
reads `Test Files 112 passed (112)` and `Tests 685 passed (685)`, and the census prints `examined 112
shipped source files under src`, `examined 18 planted zero shapes` and `examined 13 near misses`.
SPEC-341's A7, "card HTML reaches the page only through the card frame", is green at every commit:
it prints `examined 111 shipped source files under src` at b903cae6 and `examined 112 shipped source
files under src` from f8f41858, with its expected census unchanged. At 3e53179d A3 prints `examined
3 links of the type check's chain`, and the type check reads `0 ERRORS 0 WARNINGS` at every commit.

The flag's effect was measured once and never committed. With `return m.sync_status_absent();` in
`web/app/src/lib/sync/status.ts` changed to `m.sync_status_absent();`, the type check read `0
ERRORS` at f8f41858, with the flag off, and `1 ERRORS` at 3e53179d, with the flag on:
`Fallthrough case in switch.`, code 7029, on that case. The file was restored from git after each
reading.
