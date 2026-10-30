---
status: "accepted"
---

# ADR-451: the release run's Python survivors are killed by behavioural tests in six parts, each file's runs are interleaved across its legs, and each equivalence record is bound only in its own class

Decides SPEC-437, the first of six parts. SPEC-438 to SPEC-442 build the other five, each decided by
its own ADR, ADR-452 to ADR-456.

## Context and Problem Statement

The release pull request (#822) was judged by one run whose Python classes failed. Its verdict
job 117095483593 ran 24 s and printed these class lines:

- `mutation: scripts: examined 6317: generated 5944, rows 373`, then `survived 684: equivalent 1,
  unexplained 683` and `verdict: FAIL: 686 finding(s)`: the 683 survivors and three UNCOVERED
  mutants of `scripts/write-back-issues.py:33`;
- `mutation: oracle: examined 144: generated 143, rows 1`, then `survived 20: equivalent 0,
  unexplained 20` and `verdict: FAIL: 21 finding(s)`: 20 survivors of
  `tools/parity-oracle/generate.py` and one STALE record;
- `mutation: legs: verdict: ok`.

Twenty-four Python legs were cut at the `mutation-python` job's 60-minute bound, and one more, leg
119, was lost to a runner shutdown. Twenty-three of the cut legs uploaded no report, so each Python
class read them VOID `no report`; leg 241 was cut too, but its runner finished first, and the
verdict read its report.

The 683 survivors are 560 in `scripts/mutation-verdict.py`, 107 in `scripts/mutation_rows.py` and
16 in `scripts/public-scrub.py`; the oracle's 20 are in `generate.py`. Each shard's `costs.jsonl`
says why each survived:

| file | in `<module>` | reached, the whole re-run passed | reached by no traced test | total |
|---|---|---|---|---|
| `scripts/mutation-verdict.py` | 131 | 207 | 222 | 560 |
| `scripts/mutation_rows.py` | 13 | 61 | 33 | 107 |
| `scripts/public-scrub.py` | 0 | 16 | 0 | 16 |
| `tools/parity-oracle/generate.py` | 1 | 17 | 2 | 20 |

The 23 VOID legs measured 99 more survivors of `mutation-verdict.py` before they were cut, which the
verdict never saw (in `rust_literal` 35, `c_code_lines` 14, `web_literal` 14, `survivors_in` 12,
`partial_reason` 6, `item_end` 5, `item_keyword` 5, `row_span` 4, `selected_rows` 2, `block_end` 1
and `rust_test_spans` 1), and they left 90 dealt mutants unreached: 60 of `mutation-verdict.py`,
leg 119's 13 among them, and 30 of `scripts/mutation_python.py`, whose 99 measured mutants were all
killed.

Three more facts decide the shape of the cure:

- **The STALE record is a class-scoping defect, not a stale record.** Record 1 of
  `scripts/mutation-equivalent.d/python.json` (`scripts/mutation_python.py:745:73`, #340) binds its
  mutant and excuses it in the scripts class. The oracle class reads it STALE because `judge_python`
  binds every Python record against the oracle's population (`scope = None`,
  `scripts/mutation-verdict.py:2155`) and filters the bound records by class only after binding
  (`:2167-2169`).
- **The 20 oracle survivors are not an artefact of the byte-reader rule.** The two
  `CommittedGoldensAreCurrent` tests of `tools/parity-oracle/test_goldens.py` compare each golden's
  `generator_sha256` with `generate.py`'s digest (`test_goldens.py:149-168`, `311-324`, `428-452`).
  The runner drops them from the file's mutant runs as byte readers (`scripts/mutation_python.py`
  `read_by`, `:516-518`, and `scope_of`, `:901`). Had they run, every mutant would read killed by
  its digest, never by its behaviour. The 20 survive because no behavioural test observes them.
- **Of the three UNCOVERED mutants, two are killable and one is redundant.**
  `scripts/write-back-issues.py` maps `"modules": []` in `scripts/mutation-python.json`, so no
  test runs for it. At line 33, `33:69` (`group(1)` to `group(2)`) raises on any token, and `33:72`
  (`in` to `not in`) leaves a known key unwritten. `33:95` (`group(0)` to `group(1)`) sits in the
  fallback arm, which only a key in `missing` takes, and a non-empty `missing` returns before any
  write: no test can observe it.

## Decision Drivers

- Every survivor is killed by a test that observes its behaviour, or is recorded equivalent with
  its argument in the record; nothing is excused by a blanket record, a sample or a skip.
- No bound loosens: not the job's 60 minutes, not a timeout, not a leg ceiling, not the byte-reader
  rule, and no mutant leaves the population.
- The release's Python figure moves only by a measured, bounded delta that keeps the plan within
  `PYTHON_RELEASE_CEILING`.
- Each part fits one build, lands on `dev` before #822's one closing run, and moves nothing another
  part reads.
- Prices stay the measured prices; a refresh is its own delivery (#825).

## Considered Options (the alternatives each was chosen against)

D1, the survivors by group. Chosen against:

- Chosen: a behavioural killer for every function group, 681 of the 683, and an equivalence record
  for each of the other two, because each group names a behaviour a test can observe, and the two
  records each carry an argument no test can refute.
- A blanket equivalence record per file: rejected because it hides 681 mutants that a test can
  kill, and a record is an argument about one mutant, never about a file.
- Pinning a constant by its source text: rejected because a test that reads its target file's
  bytes fails under the runner's sentinel, is dropped from that file's mutant runs as a byte
  reader, and so kills nothing.
- Asserting a substring or a shape of an output: rejected because a mutant that changes the rest
  of the output passes it; each killer asserts a whole output, line or exit code by equality.

D2, the three UNCOVERED mutants. Chosen against:

- Chosen: a new mapped test module, `scripts/tests/test_write_back_issues.py`, and a script that
  refuses an unknown key before it substitutes anything, so the fallback arm and its mutant are
  gone, because the arm is a redundant guard and the other two mutants die by a known key written.
- An equivalence record for `33:95`: rejected because a record keeps a redundant guard and its
  unkillable mutant in the tree for good; a redundant arm is removed, never recorded.
- Leaving the three UNCOVERED: rejected because each is a verdict finding, and the release cannot
  pass over one.

D3, the 21 oracle findings. Chosen against:

- Chosen: 20 behavioural killers in `tools/parity-oracle/test_generate.py`, with both digest tests
  kept as byte readers, and each record bound only in its own class, because the survivors are
  behaviours no test observes and the STALE reading is a binding across classes.
- Letting the digest tests judge the mutants, or loosening the byte-reader rule: rejected because
  every mutant would then read killed by a changed digest, which observes no behaviour.
- Deleting the STALE record: rejected because it is bound and equivalent in the scripts class, so
  deleting it adds one scripts survivor.
- Re-anchoring the record: rejected because no anchor in a scripts file can bind an oracle mutant.
- Excusing a STALE reading in the oracle class: rejected because it relaxes a rule for one class,
  and a record stale in its own class would then pass unseen.

D4, the 24 cut legs and leg 119. Chosen against:

- Chosen: each priced file's runs are interleaved, run j holding the file's listing positions j,
  j + n, j + 2n and so on, with the prices unchanged, because the deal then spreads each costly
  region of a file over all of its legs while every run's size, order and projection stays the
  contiguous deal's (simulated: no leg past 3600 s, the slowest 3187 s; with the kill parts, the
  slowest 2654 s).
- Raising the job's bound or a timeout: rejected because it loosens a bound, which this round
  never does.
- Re-pricing alone: rejected because the contiguous deal at measured post-kill prices still puts 7
  legs past 3600 s, and pricing at the costs this run measured needs 387 legs, past the ceiling.
- Worst-case pricing, every mutant priced as a survivor: rejected because the plan would need far
  more legs than `PYTHON_RELEASE_CEILING` holds, and a larger ceiling needs a new measurement and a
  new owner-signed ruling.
- Round robin across files: rejected because every leg would then pay every file's control and
  sentinel (ADR-432 D2); the interleave is within one file.
- A price refresh in this round: rejected because the interleave fits at the current prices; the
  refresh after the kill parts land (simulated: 237 legs, the slowest 2440 s) stays with #825.
- Skipping, sampling or capping mutants: rejected because every listed mutant is examined once or
  the plan is refused whole.
- Curing leg 119: rejected because it is a runner event, not a cost; its 13 dealt mutants are
  measured by #822's closing run.

D5, the shape and the split. Chosen against:

- Chosen: six deliveries in #822's chain, each with its own SPEC, ADR and band, landing in order
  before the closing run, because 782 survivors over about ninety function groups cannot be killed
  in one build, and only the first part edits production code.
- One delivery for the whole round: rejected because its killers, rows and reds exceed what one
  build can write, read and prove.
- One delivery per file: rejected because `mutation-verdict.py`'s 560 survivors, and the 159 more
  that the cut legs measured or left unreached, are themselves too many for one build.

D6, the tests, the kill proof and the rows. Chosen against:

- Chosen: each acceptance criterion is a test read red first in CI where it can be, the kill proof
  is the pull request's own `mutation-rows` lines by id and its changed-line Python legs, with
  #822's closing run as the whole reading, because a test-only part generates no mutant of an
  unchanged file and only the rows and the next whole run reach it.
- A serial local mutation run as the kill proof: rejected because hundreds of mutants at minutes
  each cannot be run serially, and a local run is no verdict.
- Rows for every survivor in the first part: rejected because each later part rows its own groups
  in its own band.

D7, what the round leaves out. Chosen against:

- Chosen: the Rust legs, the lint deliveries, the closing run's own measures and the price refresh
  stay out, each with its issue, because each is decided or measured elsewhere.
- Folding the lint rules into the kill round: rejected because they land in their own deliveries
  (#901, #913).

## Decision Outcome

Chosen: the round kills by behaviour, interleaves within each file, and binds each record only in
its own class.

- **D1.** Every group of the 683 gets a killer in its part's module, except two mutants recorded
  equivalent, each citing #322, whose close condition is every survivor killed by a new test or
  recorded equivalent with its argument in the record:
  - `scripts/mutation-verdict.py:109:17`, `sys.path.insert(0, ...)` to `1`, in the sixth part: run
    as a script, `sys.path[0]` already is `scripts/`, so index 0 and index 1 resolve the same
    directory first;
  - `scripts/public-scrub.py:118:58`, the flags default `""` to `"XX"`, in the second part: the
    default is read only for `"i"`, which neither value holds.
  No killer reads its target file's bytes, and every killer asserts a whole output by equality.
- **D2.** `scripts/write-back-issues.py` finds every unknown key across all of its files before it
  substitutes anything, refuses by name with nothing written, and then substitutes each token by
  its number with no fallback arm; its printed lines are unchanged. The new module
  `test_write_back_issues` is mapped for it in `scripts/mutation-python.json`, and its three
  loader sites are registered in `DYNAMIC_IMPORTS` in the commit that adds it.
- **D3.** Twenty killers in `test_generate.py`, one or more per surviving operand, and both digest
  tests left as they are. `python_excuses` binds only the records whose file `classify` places in
  the judged class, and `judge_python`'s filter of the bound records after binding, now redundant,
  is removed.
- **D4.** `python_by_cost` appends each run as `indices[cut::cuts]`, with no `divmod`, `start` or
  `end`; the count of legs and every `projected_seconds` are byte-identical to the contiguous
  deal's, and only membership moves. Leg 119 is measured by the closing run.
- **D5.** The six parts:

| part | SPEC | ADR | band | branch | fragment | what it holds |
|---|---|---|---|---|---|---|
| 1 | SPEC-437 | ADR-451 | S43700-S43799 | `test/python-kills-437` | `changelog.d/python-kills-437.md` | D2, D3, D4 and the record-class binding; the only part that edits production code |
| 2 | SPEC-438 | ADR-452 | S43800-S43899 | `test/python-kills-scrub-rows-438` | `changelog.d/python-kills-scrub-rows-438.md` | `public-scrub.py` 16 and `mutation_rows.py` 107, one record |
| 3 | SPEC-439 | ADR-453 | S43900-S43999 | `test/python-kills-verdict-readers-439` | `changelog.d/python-kills-verdict-readers-439.md` | the verdict's readers, 88, and 130 cut-leg and unreached mutants |
| 4 | SPEC-440 | ADR-454 | S44000-S44099 | `test/python-kills-verdict-judges-440` | `changelog.d/python-kills-verdict-judges-440.md` | the verdict's judges, 118, and 29 cut-leg and unreached mutants |
| 5 | SPEC-441 | ADR-455 | S44100-S44199 | `test/python-kills-verdict-tools-441` | `changelog.d/python-kills-verdict-tools-441.md` | the verdict's tool readers and records, 223 |
| 6 | SPEC-442 | ADR-456 | S44200-S44299 | `test/python-kills-verdict-constants-442` | `changelog.d/python-kills-verdict-constants-442.md` | the verdict's `<module>` constants, 131, one record |

  Parts 2 to 6 change tests, the map and, in parts 2 and 6, one record each. They move no
  production line and no listed figure, and each adds its module to the map, so they land one
  after another.
- **The figure.** Part 1 edits `scripts/mutation-verdict.py`, so it moves the release's Python
  figure. Its net change to that file's listed mutants, read from `mutation_python.py list --file`'s
  `mutation-python: listed <n>` line before and after, is at most 12, and to
  `scripts/write-back-issues.py`'s at most 40, which keeps the plan at `PYTHON_RELEASE_CEILING`
  legs; one more in the verdict file asks one leg more, which the ceiling refuses. Before the closing
  run, the final union's plan reads its Python legs at most `PYTHON_RELEASE_CEILING`.
- **D6.** Part 1's criteria are red first where the base can fail them (the interleaved
  membership, a record of another class judged in its own class only, the map entry, and the
  record binding's agreement with its proof's vectors); the rest are mutation coverage, each read
  KILLED by its row. Part 1's rows are 28: two on the interleave, two on the binding, four on the
  write-back's new lines, and one mirroring each of the 20 oracle survivors.
  `formal/tla/EveryLegCounted` is re-read for the interleave (one deal its `Plan` admits) and its
  `python_by_cost` digest moves; the record-class binding gets a Lean proof of its own.
- **D7.** Out of this round: the Rust legs (#822), the lint rules (#901, #913), the closing run's
  measures of the 90 unreached mutants and of leg 119 (#822), and the price refresh (#825).

### Consequences

- Good: every Python leg fits its job at the current prices once part 1 lands, before any kill
  part, and the kill parts only shorten the legs further.
- Good: a record is judged once, in its own class, so a record stale in its own class still reads
  STALE, and none reads STALE in a class it does not belong to.
- Good: the parts after the first are test-only, so each moves no figure and none can conflict with
  another's production line.
- Bad: six deliveries land one after another before the closing run, and the release waits for
  them all.
- Bad: the 90 unreached mutants and leg 119's 13 are measured for the first time by the closing
  run, so a survivor among them would need one more kill.
- Neutral: the deal's count and projections are unchanged, so a plan that fitted still fits, and a
  refused plan is still refused.

### Confirmation

SPEC-437's acceptance criteria, each read in CI, and its 28 rows read KILLED by id in the pull
request's `mutation-rows` job. For the round, #822's closing run reads every Python class with zero
unexplained survivors and every leg within its job.

## The 24 legs cut at the job bound, and leg 119

Each leg's job id stands beside its duration. Dealt, projected and setup are the plan's figures and
the leg's measured control and sentinel; done, killed and survived count the mutants the leg
recorded before it was cut; measured is setup plus those mutants' seconds. The verdict file is
priced (152, 158, 107) and `mutation_python.py` (184, 204, 53): control, sentinel and each mutant.

| leg | job, duration | file | dealt | projected s | setup s | done | killed | survived | measured s |
|---|---|---|---|---|---|---|---|---|---|
| 36 | 116852790646, 3619 s | `mutation_python.py` | 26 | 1766 | 515 | 18 | 18 | 0 | 3504 |
| 40 | 116852790664, 3617 s | `mutation_python.py` | 26 | 1766 | 433 | 19 | 19 | 0 | 3453 |
| 41 | 116852790454, 3620 s | `mutation_python.py` | 26 | 1766 | 539 | 18 | 18 | 0 | 3429 |
| 68 | 116852794612, 3619 s | `mutation_python.py` | 26 | 1766 | 494 | 25 | 25 | 0 | 3532 |
| 76 | 116852794536, 3620 s | `mutation_python.py` | 25 | 1713 | 505 | 19 | 19 | 0 | 3568 |
| 99 | 116852794757, 3617 s | `mutation-verdict.py` | 13 | 1701 | 488 | 9 | 3 | 6 | 3285 |
| 100 | 116852794766, 3619 s | `mutation-verdict.py` | 13 | 1701 | 456 | 8 | 1 | 7 | 3444 |
| 102 | 116852794793, 3618 s | `mutation-verdict.py` | 13 | 1701 | 484 | 12 | 7 | 5 | 3529 |
| 103 | 116852794798, 3616 s | `mutation-verdict.py` | 13 | 1701 | 501 | 11 | 1 | 10 | 3375 |
| 104 | 116852794768, 3620 s | `mutation-verdict.py` | 13 | 1701 | 493 | 7 | 0 | 7 | 3586 |
| 105 | 116852794702, 3617 s | `mutation-verdict.py` | 13 | 1701 | 488 | 12 | 9 | 3 | 3340 |
| 106 | 116852794634, 3619 s | `mutation-verdict.py` | 13 | 1701 | 477 | 10 | 4 | 6 | 3199 |
| 107 | 116852794654, 3618 s | `mutation-verdict.py` | 13 | 1701 | 475 | 11 | 5 | 6 | 3497 |
| 110 | 116852794751, 3620 s | `mutation-verdict.py` | 13 | 1701 | 510 | 10 | 5 | 5 | 3600 |
| 113 | 116852794977, 3622 s | `mutation-verdict.py` | 13 | 1701 | 404 | 10 | 3 | 7 | 3352 |
| 114 | 116852795298, 3617 s | `mutation-verdict.py` | 13 | 1701 | 392 | 9 | 7 | 2 | 3259 |
| 122 | 116852795075, 3626 s | `mutation-verdict.py` | 13 | 1701 | 482 | 11 | 6 | 5 | 3346 |
| 124 | 116852795205, 3617 s | `mutation-verdict.py` | 13 | 1701 | 494 | 12 | 7 | 5 | 3567 |
| 126 | 116852795045, 3622 s | `mutation-verdict.py` | 13 | 1701 | 455 | 12 | 11 | 1 | 3286 |
| 133 | 116852802752, 3618 s | `mutation-verdict.py` | 13 | 1701 | 506 | 12 | 6 | 6 | 3530 |
| 171 | 116852802926, 3621 s | `mutation-verdict.py` | 13 | 1701 | 480 | 12 | 6 | 6 | 3509 |
| 216 | 116852810632, 3621 s | `mutation-verdict.py` | 13 | 1701 | 484 | 10 | 5 | 5 | 3469 |
| 217 | 116852810443, 3620 s | `mutation-verdict.py` | 13 | 1701 | 491 | 9 | 2 | 7 | 3455 |
| 241 | 116852810658, 3607 s | `mutation-verdict.py` | 13 | 1701 | 504 | 13 | 0 | 13 | 3592 |
| 119 | 116852794964, 790 s | `mutation-verdict.py` | 13 | 1701 | none | 0 | 0 | 0 | a runner shutdown, no artifact |

Why each overran: a leg's setup measured 392 to 539 s against its priced 310 or 388 s, and its
mutants cost well above their price. Every `mutation_python.py` mutant was killed, at a mean of
148 s for a line-scoped mutant and 166 s for a `<module>` one, against a price of 53 s. A
`mutation-verdict.py` kill took 105 to 170 s against a price of 107 s, and a survivor 230 to 450 s:
about 234 s for one whole re-run when no traced test reached it, about 426 s for a scoped run and
then the whole re-run when one did. The price is a file's mean, while a file's cost is correlated
by region, since one region's mutants share one traced test set. A contiguous run holds one region,
so a costly region lands whole on one leg. Leg 241 is the 24th cut leg: its runner finished, so its
13 survivors were read. Leg 119 ended at 790 s for a runner shutdown and uploaded nothing.

Simulated over these measured costs, each mutant at its own seconds and each setup at its median:
the plan as dealt puts 19 legs past 3600 s, against 24 cut in the run; the kills alone leave 5; the
interleave alone leaves none, the slowest 3187 s; the interleave with the kills leaves none, the
slowest 2654 s, in the same 305 legs.

## The groups, by part

| part | file | groups, by function and survivor count | killer module |
|---|---|---|---|
| 2 | `public-scrub.py` | `compiled` 4 (one recorded equivalent), `load_deny` 3, `rules` 2, `main` 7 | `test_public_scrub` |
| 2 | `mutation_rows.py` | `<module>` 13, `spec_band` 2, `_row_id` 3, `assemble` 7, `tree_fragments` 1, `declared_spellings` 4, `row_target` 4, `test_functions` 1, `locate_killer` 4, `resolve_killer` 3, `census` 2, `cargo_flags` 3, `run_killer` 13, `shell_parser` 2, `parses` 1, `builds` 2, `sha256` 1, `prove_row` 14, `prove` 7, `retired` 6, `main` 14 | `test_mutation_rows_kills`, first in the map's list |
| 3 | `mutation-verdict.py` | `scope_of` 4, `git` 1, `comment_or_blank` 1, `quoted_end` 7, `block_end` 3, `rust_literal` 7, `template_part` 8, `mark` 1, `c_code_lines` 22, `language_of` 4, `rust_tokens` 3, `is_test_mark` 3, `item_keyword` 11, `rust_test_spans` 5, `rust_test_lines` 4, `changed_lines` 4; and 79 cut-leg and 51 unreached mutants of the same readers | `test_mutation_verdict_reader_kills` |
| 4 | `mutation-verdict.py` | `selected_rows` 3, `class_case` 1, `plan_diff` 6, `say_plan` 1, `size` 2, `shards` 3, `not_applicable` 5, `whole_reports` 7, `examined_sum` 3, `judge_rust` 6, `judge_web` 7, `judge` 3, `rust_leg_owed` 9, `rows_leg_owed` 9, `legs` 5, `survivors_in` 15, `draft_body` 8, `survivors` 7, `battery` 18; and 20 cut-leg and 9 unreached mutants | `test_mutation_verdict_judge_kills` |
| 5 | `mutation-verdict.py` | `takes` 16, `cargo_mutants_findings` 9, `stryker_findings` 44, `configs` 18, `exclusions` 53, `load_records` 2, `Sources.offset` 2, `Sources.window` 1, `cargo_mutant` 1, `binds` 3, `Excuses.bind` 1, `held_twice` 2, `refutation` 3, `rust_excuses` 1, `workspace_packages` 1, `record_problems` 8, `census` 1, `table` 6, `table_rust` 7, `table_web` 18, `main` 26 | `test_mutation_verdict_tool_kills` |
| 6 | `mutation-verdict.py` | the `<module>` constants, 131 (one recorded equivalent): `TOOL_EXITS` 10, the JavaScript keywords 14, `TEST_ITEMS`, `QUALIFIERS` and `BRACKETS` 11, `STRYKER_EXAMINED` 2, `LEG_RESULTS` 2, the cargo-mutants schema about 51, `STRYKER_THRESHOLDS` 5, the config names 8, the Stryker mutate globs 7, `EXCLUDING_KEYS` 5, `EXCLUDING_ATTRIBUTE` 1, `WEB_SOURCES` 9, frozen dataclasses 2, `sys` 2, the exit tuple 1 | `test_mutation_verdict_constant_kills`, first in the map's list |
| 1 | `write-back-issues.py` | line 33: two killed, one removed with its arm | `test_write_back_issues` |
| 1 | `generate.py` | `<module>` 1, `check` 7, `resolve` 1, `plain` 2, `draw` 1, `cases_of` 1, `main` 7 | `test_generate` |

Part 3 observes each reader through golden spans and line sets over Rust and JavaScript source
fixtures (strings, raw strings, nested block comments, a regex after each keyword, template parts,
test attributes, items). Part 4 asserts whole printed lines and exit codes over fixture plans and
reports. Part 5 feeds each cargo-mutants key a wrong-typed and a right-typed value, plants each
exclusion form and reads its refusal by name, and asserts each Stryker config form and whole table
lines. Part 6 observes each constant through the function that reads it (a refusal, a span, an exit
code, a frozen assignment that raises), never by its source text.

## What would make this wrong

- A killer that passes for a reason other than its mutant: each row is read KILLED by its own
  criterion's test, and the closing run counts every class whole.
- A region whose cost the interleave cannot spread, such as a file of one run: such a run fits one
  leg by the deal's own bound, so only a price below the measured mean could put it past the job,
  which is #825's refresh to measure.
- A second class-scoping site: the binding is judged by a test over both classes with a record of
  each, so a record judged in a foreign class anywhere in `judge_python` reads red.
- A part that edits a production line: it moves the figure the final union is measured against,
  and is refused by its own SPEC's manifest.

## More Information

- SPEC-418 and ADR-432 (the cost deal, amended insert-only by SPEC-437 for the interleave),
  SPEC-421 and ADR-435 (the scoped prices), SPEC-422 and ADR-436 (the release ceiling), SPEC-087
  (the Python runner and its map).
- #822 is the release whose run measured all of the above; #825 tracks the Python legs' fit and
  the price refresh; #322 holds the two equivalence records; #901 and #913 hold the lint rules.
