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
A16: green at 6abff4a
A17: red at e8f9695: AssertionError: 44 != 0 : deck-streak-ingest: 44 unexplained mutant(s) in its row (the opening sweep, run 36502008965, read listed 297, killed 190, equivalent 0, unexplained 44, unviable 63)
A17: green at 2e0c239
A18: red at 2311afa: AssertionError: 17 != 0 : deck-streak-kernel: 17 unexplained mutant(s) in its row (the opening sweep, run 36502933533 at a7b8025, read listed 368, killed 303, equivalent 0, unexplained 17, unviable 48)
A18: green at 0896902
A20: red at cd59d2a: AssertionError: 8 != 0 : deck-streak-daemon: 8 unexplained mutant(s) in its row (the opening sweep, run 36515002230 at 2fd66f4, read listed 101, killed 64, equivalent 0, unexplained 8, unviable 29)
A20: green at b42ef34
A22: red at 3d271a1: AssertionError: 2 != 0 : deck-streak-api: 2 unexplained mutant(s) in its row (the opening sweep, run 36528558184 at 5216bcf, read listed 70, killed 48, equivalent 0, unexplained 2, unviable 20)
A22: green at ab99ab8
A28: red at e260627: AssertionError in each of its four subtests, each for its own reason: 'True is not false : the rust class applies on test lines [31, 36, 43, 47, 53]' (the test-only diff); 'Lists differ: [46, 58] != [58]' (the mixed diff counted its test module's line as production code); "'mutation: plan: rust applies: 3 production code line(s) in 1 file(s)' not found" (the production-only diff's plan named no production line); and '3 != 0 : mutation: shards: VOID the rust class applies and ... holds no cargo-mutants listing' (cargo-mutants' empty --in-diff output read as no listing)
A28: green at 8c87e5b
A19: red at 8eba7ef: AssertionError: 16 != 0 : deck-streak-identity: 16 unexplained mutant(s) in its row (opening sweep run 36511057164 listed 143, killed 97, equivalent 0, unexplained 16, unviable 30)
A19: green at a911483
A21: red at 660dda8: AssertionError: 5 != 0 : deck-streak-coordination: 5 unexplained mutant(s) in its row (the opening sweep, run 36526822999 at 36b283a, read listed 409, killed 321, equivalent 0, unexplained 5, unviable 83)
A21: green at c7391f9
A26: not red: the opening sweep, run 36531093051 at 5216bcf, already read listed 146, killed 111, equivalent 0, unexplained 0, unviable 35, so the row had no unexplained mutant to be red for (R16)
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
| R22 | a line inside a `#[cfg(test)]` module read as production code, so a test-only diff reads VOID; a test line counted beside a production line; a `cfg(not(test))` function, or a brace or test mark inside a literal or a comment, read as a test item; a line that holds a test item's closing brace and production code read as test-only; cargo-mutants' empty listing read as a missing one, or as not-applicable | A28 |

A16 is the vault's row of section 7. Its opening sweep, the weekly battery dispatched with
`package=deck-streak-vault` at 5767fbe (run 36438243392), reported every one of its 32 shards whole
and tested every listed vault mutant; its `table` line became the row, committed with A16's test
(bfae431), which reads red on the 225 unexplained mutants. It turns green at the commit that carries
the closing sweep's row.

Its first closing sweep, the battery dispatched with `package=deck-streak-vault` at 8b18276 (run
36461589655), came after the delivery's first kills and records: 138 of `rails.rs`'s
unexplained mutants killed and 11 recorded (9dbdc70), and the other six files' 69 killed and 7
recorded (8b18276). Every one of its 32 shards reported whole: 16 exited 0, 13 exited 2 on missed
mutants, and 3 exited 3, each on one `rails.rs` mutant that ran to the tool's timeout, which counts
killed; the 18 missed mutants, in 14 shards, are the 18 records the fragment then held. The battery
counted 33 of 33 reports whole, and its table read `listed 939, killed 854, equivalent 18,
unexplained 0, unviable 67`.
That line was the row committed at 18cff1c, where A16's test, re-run from a `git archive` export
of that sha, passed. The fix round below refilled the row from its later closing sweeps.

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

## The vault's delivery: its first fix round (seven records, two more closing sweeps, S05709)

The first verification found seven of the 18 records short of the record's bar, that no test can
tell the mutant apart (R5): each argued from the function's callers, while a unit test in `src`
observes the function itself. They were `rails.rs` 563:38 (`+` to `*` and to `-` in `tag_at`),
622:21 (`>` to `>=` in `attribute_value`), 755:48 (`+` to `*` in `link_tail`) and 809:23 (`+` to
`*` and to `-` in `autolink_at`), and `fs.rs` 145:9 (`RealFile::sync` to `Ok(())`: a sync of the
null device fails, and the mutant reports success). Five unit tests, four in `rails.rs` and one in
`fs.rs`, came first (3af5212). On a `git archive` export of that sha, `cargo mutants --in-place`
over exactly those seven mutants read 7 caught, each log naming its own new test as the only
failure, with both files restored byte for byte; then the seven records were deleted (726bbe5),
leaving 11: `rails.rs` 5, `sha256.rs` 4 and `note.rs` 2.

The second closing sweep, the battery dispatched with `package=deck-streak-vault` and
`file_issues=false` at 726bbe5 (run 36479539295), reported every one of its 32 shards whole: 20
exited 0, 9 exited 2 on missed mutants, and 3 exited 3 (shards 0, 13 and 23), each on one of the
same three `rails.rs` mutants that ran to the tool's timeout, which count killed; shard 0 also
missed one. It tested 939 mutants: 858 caught, 3 timed out, 11 missed and 67 unviable. The 11
missed mutants were the fragment's 11 records, and each of the seven was caught. Its table read
`listed 939, killed 861, equivalent 11, unexplained 0, unviable 67`, and its row was committed at
0227889. At 726bbe5 A16 reads `18 != 11 : deck-streak-vault: equivalent against its fragment`,
since the row still named the first sweep.

The box run's tdd probe then refused two of the five tests as written: `absence-only-assertions`,
since `an_equals_sign_with_no_value_after_it_takes_nothing` asserted only a 0, and
`examined-counts`, since `fs.rs`, whose `RealFs::list` reads a folder, now held a test and reported
no examined count. The test gained a positive control, `=x` taking 2, and `fs.rs`'s test module a
test of the listing that counts the entries it examined (c2bf6d3). On a `git archive` export of that
sha the seven mutants again read 7 caught, each log naming its own test, and the closing sweep ran
once more there (run 36483219611): its 32 shards reported whole, with the same exits (20 exited 0, 9
exited 2 and 3 exited 3, shards 0, 13 and 23), the same 939 mutants tested (858 caught, 3 timed out,
11 missed, 67 unviable), each of the seven caught, 33 of 33 reports whole, and the same table line
with `table: verdict: ok`. Its table line is the row committed at 6abff4a, where A16's test, re-run
from a `git archive` export of that sha, passes; so the fence's A16 green line names 6abff4a in
place of 18cff1c, since a criterion carries one green line. `crates/` and the fragment are
byte-identical between c2bf6d3 and the delivery's head.

The verification also found that no test read the plan's `inside - outside` (R22): cutting it to
`inside`, which reads a line that holds a test item's closing brace and production code as
test-only, passed A28 and every test of `test_mutation_verdict.py`. A28's production-only fixture
gained line 54, a constant after the brace that closes the test module (5db9a52), and cargo-mutants
27.1.0's own `--list --json --in-diff` output over the changed diff is the same sixteen mutants,
since the constant lists none. Under that hand mutant, on a `git archive` export of 5db9a52, A28
failed in its production-only subtest alone, `'mutation: plan: rust applies: 4 production code
line(s) in 1 file(s)\n' not found` (the plan set line 54 apart as test-only), and passed again once
the script's sha256 matched. Row S05709 pins it (793e95e): `mutation_rows.py prove` read it KILLED,
its killer passing without the mutant and failing with it, with the file restored byte for byte.
A28's own red and green lines stand, since the plan at dd734e5 read every line as production and
the new line adds no red of its own.

A18 is the kernel's row of section 7. Its opening sweep, the weekly battery dispatched with
`package=deck-streak-kernel` (run 36502933533 at the base, dev a7b8025), listed 368 mutants, all 33
reports whole, and left 17 unexplained across `redact.rs` (5), `study_day.rs` (4), `db.rs`, `verdict.rs` and
`credentials.rs` (2 each), `offload.rs` and `settings.rs` (one each); its `table` line was the row
committed at 2311afa with A18's test, which read `17 != 0`. Every one is observed by a test and none is
recorded equivalent: the tests, at d702900, read `66 mutants tested: 61 caught, 3 unviable, 2
timeouts` in place on a `git archive` export of that commit, none missed. A merge of dev then added
`courses.rs` and two `Db` methods, so the crate at the head lists 423 mutants (55 more than the 368,
of which 49 are in `courses.rs` and 6 in `db.rs`), and the sweep of d702900 no longer measured the
head. The closing sweep (run 36508893368 at 592bc43) read 33 of 33 reports whole, `listed 423, killed
367, equivalent 0, unexplained 0, unviable 56` and `table: verdict: ok`; eight shards exited 3 for
one timeout each, counted killed (six in `redact_tokens`: `redact.rs` 204:12, 205:16, 209:21 twice,
210:19 and 215:21; two in `OffloadWorkers::get`: `settings.rs` 157:9, returning 0 and 1), and the
other twenty-four exited 0. The same eight also timed out in the opening sweep. Its table line is the
row committed at 0896902, where A18's test passes. Rows S05730 to S05739 are unused: every mutant was
mutable and killed by a test.

## The ingest delivery: its row (A17)

A17 is the ingest crate's row of section 7. Its opening sweep, run 36502008965, counted 33 of 33
reports whole. Forty-one of the 44 survivors are now killed by tests (retry reopens and waits, the
jitter draws, the engine's error mapping, the run record's reads, the settings' host check and
cleartext warning, the reset row of the data-rights declaration). Three are recorded equivalent in
`scripts/mutation-equivalent.d/deck-streak-ingest.json`: `engine_client` (the same call spelled
two ways), `Held::release` and `Held::drop` (closing the only open file description releases the
flock). The closing sweep, run 36505515113, counted 33 of 33 whole and read `table: verdict: ok`,
listed 309, killed 239, equivalent 3, unexplained 0, unviable 67. Shards 8, 9 and 28 exited 2 (one
missed mutant each, the three recorded) and shard 23 exited 3 (one timeout, counted killed).

## The daemon delivery: its row (A20)

A20 is the daemon crate's row of section 7. Its opening sweep, run 36515002230 at the base (dev
2fd66f4), counted 33 of 33 reports whole, listed 101 mutants and left 8 unexplained: both values of
`Notifier::is_enabled`, the deleted `!` in `Notifier::notify`, both `millis` values, the Linux
`send_abstract`, the non-Linux `send_abstract` and the `name == "data"` guard of `Role::from_arguments`; its `table` line is the
row committed at cd59d2a with A20's test, which read `8 != 0`. Seven are now killed by tests in
`crates/daemon/tests/` (the notifier's enabled state, delivery to a path and to an abstract socket,
the once-per-episode warning, the watchdog warning's two millisecond figures, and the refusal of a
non-`data` name followed by a data command). One is recorded equivalent in
`scripts/mutation-equivalent.d/deck-streak-daemon.json`: the non-Linux `send_abstract`, which
`#[cfg(not(target_os = "linux"))]` removes from every Linux build. The closing sweep, run
36517001675 at the branch head with dev 8903f71 merged, counted 33 of 33 whole and read
`table: verdict: ok`, listed 101, killed 71, equivalent 1, unexplained 0, unviable 29. Shard 1
exited 2 (the one recorded mutant) and the other shards exited 0. Rows S05750 to S05759 are unused:
the tool mutated every invariant of the crate, so none needed a hand-proved row (R20). No shard exited 3, so no mutant of this crate timed out in the closing sweep.

## The api delivery: its row (A22)

A22 is the api crate's row of section 7. Its opening sweep, run 36528558184 at the base (dev
5216bcf), counted 33 of 33 reports whole, listed 70 mutants and left 2 unexplained: the `Display`
implementation of `ListenAddress` and the `Debug` implementation of `OwnerAccess`, each replaced by
an empty `Ok`. Its `table` line is the row committed at 3d271a1 with A22's test, which read
`2 != 0`. Both are now killed by tests in `crates/api/tests/` (the listen address's display form,
and the owner access's debug form, which names its parts and never the signing token). The closing
sweep, run 36529229929 at 618a392, counted 33 of 33 whole and read `table: verdict: ok`, listed 70,
killed 50, equivalent 0, unexplained 0, unviable 20. All 32 shards exited 0 and none logged a
timeout. No record was needed, and rows S05770 to S05774 are unused: the tool mutated every
invariant of the crate (R20).

## The coordination delivery: its row (A21)

A21 is the coordination crate's row of section 7. Its opening sweep, run 36526822999 at the base (dev
36b283a), listed 409 mutants and left 5 unexplained, in `obligations.rs`, `liveness.rs`, `delivery.rs`
and `runner.rs` (two). Three were killed by tests: the registry's debug line and the reason renderer
(`crates/coordination/tests/render.rs`) and the minute-count skew at the lower bound
(`crates/coordination/tests/liveness.rs`). Two are recorded equivalent in
`scripts/mutation-equivalent.d/deck-streak-coordination.json`: the identical `NoNotifier::counts`
default and the always-true job guard in `run_job`. The closing sweep, run 36528662545 at e8a8e59, reads
listed 409, killed 324, equivalent 2, unexplained 0, unviable 83. Rows S05760 to S05764 are unused.

## The agent delivery: its row (A26)

A26 is the agent crate's row of section 7. Its opening sweep, run 36531093051 at the base (dev
5216bcf), counted 33 of 33 reports whole and read `table: verdict: ok`: listed 146, killed 111,
equivalent 0, unexplained 0, unviable 35, every shard exiting 0. The crate was never swept before,
and no mutant of it survives, so no test was added, no record was written and no production file of the crate changed. The opening sweep already read unexplained 0, so A26 is disclosed not red, naming that
run (R16). The closing sweep, run 36531457597 at a69e0af, counted 33 of 33 whole and read the same
figures, with no shard exiting 2 or 3 and no `TIMEOUT` line. Rows S05765 and S05766 pin `ROSTER`, the setting name SPEC-044 R2 names, and `RosterPath`'s `SHAPE`, the shape a malformed roster setting is refused with: the tool never mutates either constant, and no test named either by its literal; S05767 to S05769 are unused. Four by-hand plantings in
`SLOTTED`, one emptying the slot list of each of the identity (its bio slot), voice, personality
and disclosure sections, each failed `a_template_with_a_filled_slot_is_refused`, so that
constant's invariant is already pinned by an existing test (R20).
