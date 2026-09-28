# Red-first record: SPEC-057

SPEC-057 is one campaign of deliveries, one per crate (R15), and each appends its own lines here.

## The vault's delivery: the machinery (A1 to A15) and its opening row (A16)

The plan's SPEC, ADR-070 and the schematic `docs/schematics/mutation-equivalence-record.md` came
with the plan (#225). The vault's delivery first committed every test of A1 to A15 (e43dae8)
beside stub entry points that kept each interface and did nothing, as SPEC-057 section 3 asks: the
`census` verb examined nothing, `table` printed no line, `judge` bound no record, the subject rule
named no squash, `exclusions` held its old form, and the `audit-rust` stage ran no resolve. Each
criterion was run there with its fenced command and failed by assertion for its own reason, never
by a compile error, a missing fixture or an empty selection. The riders came green first
(621842f), then the record, its census, binding and table (417163f), the battery's scope and its
binding (bff925f), and the teaching and the retirements (5c4d6b6). Every sha below was re-run from
a `git archive` export of that sha, so each verdict rests on the committed tree alone; at 5c4d6b6
all fifteen pass.

Two tests outside this SPEC moved with it. SPEC-038 A5's table of each stage's tools gained `git`
for `audit-rust`, which now runs `git diff` (red at e43dae8, green at 621842f). SPEC-039's A20,
which let a justified exclusion pass, is retired insert-only (SPEC-039 section 10), and A8's test
replaced its test.

```red-first
A1: red at e43dae8: AssertionError: Regex didn't match: '(?m)^examined 2 record\\(s\\)$' not found in 'examined 0 record(s)' (the census examined no record, and refused none of the planted ones)
A1: green at 417163f
A2: red at e43dae8: AssertionError: 0 != 1 : examined 0 record(s) (the census refused no record it cannot bind)
A2: green at 417163f
A3: red at e43dae8: AssertionError: 'mutation-rust-shard-0: EQUIVALENT crates/fix/src/lib.rs:3:7: replace * with / in double' not found (the verdict bound no record, so the recorded mutant failed as MISSED)
A3: green at 417163f
A4: red at e43dae8: AssertionError: 0 != 1 (no record was bound, so none was named STALE or AMBIGUOUS)
A4: green at 417163f
A5: red at e43dae8: AssertionError: 'REFUTED deck-streak-fix.json record 1 (crates/fix/src/lib.rs: replace * with + in double): its mutant crates/fix/src/lib.rs:3:7: replace * with + in double was caught' not found (no record was bound, so a caught mutant refuted nothing)
A5: green at 417163f
A6: red at e43dae8: AssertionError: 1 != 0 (the record excused nothing after the lines above it moved, so its mutant failed as MISSED)
A6: green at 417163f
A7: red at e43dae8: AssertionError: "EQUIVALENT web/app/src/lib/start.ts:1: ConditionalExpression -> 'true'" not found (no web record was read, so the recorded survivor failed as Survived)
A7: green at 417163f
A8: red at e43dae8: AssertionError: False is not true : exclusions: .cargo/mutants.toml: the exclude_re key (the check passed a justified exclude_re and refused only mutants::skip)
A8: green at 417163f
A9: red at e43dae8: AssertionError: 0 != 1 (table printed no line and exited 0)
A9: green at 417163f
A10: red at e43dae8: AssertionError: 0 != 3 (table named no listed mutant that no report tested)
A10: green at 417163f
A11: red at e43dae8: AssertionError: unexpectedly None : the dispatch takes no package input
A11: green at bff925f
A12: red at e43dae8: AssertionError: 0 != 1 : the plan never lists the whole tree's mutants
A12: green at bff925f
A13: red at e43dae8: AssertionError: 'diff' != 'not-applicable' (a squash merge's subject named no pull request)
A13: green at 621842f
A14: red at e43dae8: AssertionError: 0 != 1 (the audit-rust stage passed the lock with a version qualifier that an unlocked resolve rewrites)
A14: green at 621842f
A15: red at e43dae8: AssertionError: 'scripts/mutation-equivalent.d/<package>.json' not found (the brief's mutation section taught the exclusion form)
A15: green at 5c4d6b6
A16: red at bfae431: AssertionError: 225 != 0 : deck-streak-vault: 225 unexplained mutant(s) in its row (the opening sweep, run 36438243392 at 5767fbe, read listed 939, killed 647, equivalent 0, unexplained 225, unviable 67)
A16: green at 18cff1c
A28: red at e260627: AssertionError in each of its four subtests, each for its own reason: 'True is not false : the rust class applies on test lines [31, 36, 43, 47, 53]' (the test-only diff); 'Lists differ: [46, 58] != [58]' (the mixed diff counted its test module's line as production code); "'mutation: plan: rust applies: 3 production code line(s) in 1 file(s)' not found" (the production-only diff's plan named no production line); and '3 != 0 : mutation: shards: VOID the rust class applies and ... holds no cargo-mutants listing' (cargo-mutants' empty --in-diff output read as no listing)
A28: green at 8c87e5b
```

| requirement | the behaviour a wrong implementation would get wrong | criterion |
|---|---|---|
| R4 to R7 | a record missing a field, a two-line reason, evidence repeating the reason, an issue that is not #N, an anchor other than once, a file outside its package, a `reached_by` that names no test or two, a fragment of no package, a record held twice, each passing | A1, A2 |
| R8 | a record bound against the diff alone, or one that binds nothing or two passing | A4, A12 |
| R9 | a recorded missed mutant still failing, an unrecorded one passing, a caught or unviable one's record passing, a record lost when lines move | A3, A5, A6 |
| R10 | an uncovered or killed Mini App mutant excused, an ignored one passing, a record binding two mutants that start at one position | A7 |
| R11 | an exclusion with a reason passing, an empty key passing, an attribute or an excluded mutator unread | A8 |
| R12 | a record bound only on a pull request, an issue drafted for an equivalent mutant | A12 |
| R13 | a row whose counts do not sum to its listed, no failure on an unexplained mutant or a failing record, a listed mutant no report tested counted as none | A9, A10 |
| R14 | a scoped dispatch sweeping the whole workspace, owing a report from a shard its scope gave no mutant, or ending without its table line | A11 |
| R17 | the brief, the configurations' comments or the drafts teaching an exclusion | A15 |
| R18 | a squash merge's push judged again, the title's issue read as the pull request; a lock only `--locked` accepts passing the gate | A13, A14 |
| R22 | a line inside a `#[cfg(test)]` module read as production code, so a test-only diff reads VOID; a test line counted beside a production line; a `cfg(not(test))` function, or a brace or test mark inside a literal or a comment, read as a test item; cargo-mutants' empty listing read as a missing one, or as not-applicable | A28 |

A16 is the vault's row of section 7. Its opening sweep, the weekly battery dispatched with
`package=deck-streak-vault` at 5767fbe (run 36438243392), reported every one of its 32 shards whole
and tested every listed vault mutant; its `table` line became the row, committed with A16's test
(bfae431), which reads red on the 225 unexplained mutants. It turns green at the commit that carries
the closing sweep's row.

Its closing sweep, the battery dispatched with `package=deck-streak-vault` at 8b18276 (run
36461589655), came after every kill and record of the vault's delivery: 138 of `rails.rs`'s
unexplained mutants killed and 11 recorded (9dbdc70), and the other six files' 69 killed and 7
recorded (8b18276). Every one of its 32 shards reported whole: 16 exited 0, 13 exited 2 on missed
mutants, and 3 exited 3, each on one `rails.rs` mutant that ran to the tool's timeout, which counts
killed; the 18 missed mutants, in 14 shards, are the fragment's 18 records. The battery counted 33
of 33 reports whole, and its table read `listed 939, killed 854, equivalent 18, unexplained 0,
unviable 67`.
That line is the row committed at 18cff1c, where A16's test, re-run from a `git archive` export of
that sha, passes.

## The vault's delivery: the third rider (A28)

SPEC-057 R22 came to this delivery after its closing sweep, when its own diff read VOID in
`mutation-plan` and `mutation-verdict` (SPEC-057 section 1.6). A28's one test plants four
fixtures, one subtest each, and was committed (e260627) against the plan as it stood at
dd734e5, before the fix (8c87e5b). Each subtest failed by assertion for its own reason: the
test-only diff applied the Rust class on its five test lines; the mixed diff counted its test
module's line 46 among its production code lines; the production-only diff's plan printed
`rust applies` and named no production line; and `shards` read cargo-mutants' empty `--in-diff`
output as no listing, VOID. The listings the test plants are cargo-mutants 27.1.0's own
`--list --json --in-diff` output over the fixtures' diffs, in a workspace that holds the
fixture: a listing parses and builds nothing. Both shas were re-run from a `git archive` export
of that sha: four failures at e260627, and a pass at 8c87e5b.

The fix was also read against the tool's own listing of the whole tree. The listing
`mutation-plan` kept at dd734e5 (run 36463302615) names 2,887 mutants over the 105 files under
`crates/*/src`, and none of them lies on any of the 662 lines the fixed plan reads as test-only,
in the 11 files that hold a `#[cfg(test)]` module; each of those modules reads test-only whole,
from its attribute to its closing brace.
