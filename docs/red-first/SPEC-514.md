# Red-first record: SPEC-514

The SPEC, ADR-532 and the data-flow schematic were installed first (90482e20). The tests came next, whole, over a stub
that keeps every shape but the behaviour (f2ad826f): the stub crate's parse refuses every search as blank, its engine
day is zero, its matching matches nothing and its evaluator selects nothing; ingest keeps its balance check and its
settings as they were, gains `SearchRefusal::OutsideTheSkipProfile` unused, and its parity tool refuses every read.
64c20887 turns them green: the grammar, the profiles, the limits, the matching, the engine's day, the evaluator and the
structural skip wrap, ingest's skip search and setting through the skip profile, the parity tool, the Lean entry and
its ledger row, and the context map's new edges. 635dcff7 stamps the Lean entry's two covers. Each red below is quoted
from a run of the fence's tests on the tree of f2ad826f, each green from a run on the tree of 635dcff7.

```red-first
A1: red at f2ad826f: crates/ingest/tests/search_profile.rs:19: assertion `left == right` failed: the profile and the check disagree on "prop:due=0 -is:suspended -is:buried"
A1: green at 635dcff7
A2: red at f2ad826f: crates/ingest/tests/search_profile.rs:73: assertion `left == right` failed: "dekc:Spanish" stops start; search_profile.rs:97 (a refused search is refused before the wrap); crates/search/tests/grammar.rs:192: assertion `left == right` failed: "flag:1"; grammar.rs:21: the 1024-byte search of "a" parses: the search is blank (depth_length_and_arity_limits_refuse); grammar.rs:166: assertion `left == right` failed: "zebra)" (every_shape_refusal_is_named_and_carries_no_text)
A2: green at 635dcff7
A3: red at f2ad826f: crates/search/tests/eval_golden.rs:176: "prop:due=0" parses: the search is blank (eval_equals_the_engines_search_golden, the_rollover_class_passes and the_run_second_is_one_of_the_seconds_the_golden_admits); eval_golden.rs:223: assertion `left == right` failed: anki.sched.timing_today: {"crt":1021254400,"now":1043326799,"rollover_hour":13,"utc_offset_minutes":0}; crates/search/tests/grammar.rs:21: "a b OR c" parses: the search is blank; crates/search/tests/profile.rs:31: skip admits text
A3: green at 635dcff7
A4: red at f2ad826f: crates/search/tests/holds.rs: most drawn predicates are the skip profile's (examined 3000 drawn predicate(s): 0 wrapped, 0 selected card(s)); holds.rs:279: "(deck:A) or (deck:A) -is:new -is:learn prop:due=0 -is:suspended -is:buried -deck:filtered" parses: the search is blank
A4: green at 635dcff7
A5: VOID: no Kani run is pinned for this tree; K-operator-profile is unratcheted and nothing runs (SPEC-514 section 8)
A6: red at f2ad826f: crates/ingest/tests/search_parity.rs:47: "deck:Spanish": the read-only read of the collection failed
A6: green at 635dcff7
```

## What each pair disclosed

- **The fence's tests are byte-identical at f2ad826f and 635dcff7.** No file under `crates/search/tests/` or the two
  new ingest test files changes after the stub commit.
- **Fifteen tests are RED-FIRST:** each failed by name at f2ad826f (6 test binaries, 1 passed, 15 failed) and passed at
  635dcff7 (6 binaries, 16 passed, 0 failed). The one test that passed at the stub,
  `the_skip_profile_is_the_closed_list_its_holds_were_written_against`, reads the profiles' constant lists, which the
  stub keeps; no criterion rests on it alone.
- **The existing suites that read the changed code stay green at 635dcff7:** `deck-streak-ingest --test skip_record`
  (12 passed) and `--test settings` (7 passed), and `deck-streak-coordination --test skip_flow` (7 passed).
- **A3's golden depends on the run's time of day.** At 635dcff7 the census prints that 480 of the day's 1,440 minutes
  (04:00 to 11:59 UTC) give every one of the 88 cases its recorded answer, and the run's second, 10:00 UTC, is among
  them.
- **The Lean entry is green at 635dcff7** under the formal checker's gate (`--kind lean --gate`, exit 0, 23 entries,
  62 properties, all 62 clean, 65 witnesses examined): `lean/SearchHolds/holds_inescapable` reads clean, and its
  witness, the text wrap of a closing parenthesis substituted for the structural wrap, refutes it. Both covers read
  FRESH.

## The weakening table

Across `git diff 48a4dd95...HEAD` (the held-clock test fix on the wave-3 integration head): no test is deleted, renamed
or skipped, no golden, other test or non-test file changes, and no filter, exclusion, skip list, sample or bound is
added on the test side. One assertion is replaced.

| entry | commit | ruling |
|---|---|---|
| `eval_golden.rs` `the_run_second_is_one_of_the_seconds_the_golden_admits` REPLACED: it held one run time of day (10:00 UTC, `RUN_SECOND_OF_DAY`) inside the band of the day's 1,440 minutes that give every case its recorded answer; it now moves each case's own recorded instant by every minute of a day around it (1,440 shifts over all 88 cases) and holds the unshifted instant inside the admitted shifts, those shifts one band, and fewer than all 1,440 admitted (643, from -17,940 to 20,580 seconds). `RUN_SECOND_OF_DAY` and `run_instant` are removed, and the evaluator's and the rollover class's tests read each case at its own instant. The name is kept, so A3's `--exact` command still selects it | the commit whose parent is 48a4dd95 | the parity goldens now hold the clock at each case's own recorded instant (9ca8c0fc), so one run time of day no longer describes them: at 48a4dd95 case 72 differs and 0 of 1,440 minutes admit every case |
