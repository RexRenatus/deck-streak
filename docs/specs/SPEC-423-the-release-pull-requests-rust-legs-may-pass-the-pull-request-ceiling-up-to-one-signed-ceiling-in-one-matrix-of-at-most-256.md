# SPEC-423: the release pull request's Rust legs may pass the pull request ceiling up to one signed ceiling, in one matrix of at most 256

Path:
`docs/specs/SPEC-423-the-release-pull-requests-rust-legs-may-pass-the-pull-request-ceiling-up-to-one-signed-ceiling-in-one-matrix-of-at-most-256.md`.

- **Issue:** #822, the release pull request from `dev` into `main`. This SPEC refers to it and
  closes nothing. It also refers to #825, the legs' fit.
- **Context(s):** the mutation scripts (`scripts/`), their tests, the mutation map, the release
  runbook and one formal model's comments. No crate changes, and no workflow changes.
- **Decided by:** ADR-437 (the release run's Rust legs are held to one signed ceiling, read from
  the same release mark, in the one Rust matrix). SPEC-362, SPEC-422, ADR-373 and ADR-436 are
  amended insert-only.
- **Schematic:** `docs/schematics/mutation-testing.md` section 11, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-423.md`. The ceiling's figure is set by a later, separate commit of the same
  pull request, after the release's final union is measured.

## 1. The problem, measured

Every figure below is read from the tree at dev `a3a052b2`, by the command beside it.

| fact | value | command |
|---|---|---|
| the Rust leg ceilings | `{"ci": 190, "battery": 234}` | `grep -n '^LEG_CEILING = ' scripts/mutation-verdict.py` reads `961:` |
| a Rust leg's bound | 10800 s | `grep -n '^SHARD_BOUND_SECONDS = ' scripts/mutation-verdict.py` reads `958:` |
| the Rust refusal past the ceiling | one, by name | `grep -n "the pull request run's ceiling: split the change" scripts/mutation-verdict.py` reads `1327:` |
| the release mark's one reader | `python_release_ceiling` | `grep -n 'plan.get("release") is True' scripts/mutation-verdict.py` reads `1100:` alone |
| a release ceiling for the Rust legs | none | `grep -c 'RUST_RELEASE_CEILING' scripts/mutation-verdict.py` reads 0 |
| the `ci.yml` jobs whose axis is the plan's Rust matrix | 1, `mutation-rust` | `grep -n 'outputs.matrix) }}' .github/workflows/ci.yml` reads `469:` alone |
| the jobs a matrix generates | 256 | `grep -n '^RUN_JOBS = ' scripts/tests/test_dispatch_shards.py` reads `219:` |
| the per-matrix record | committed | `git ls-tree --name-only HEAD scripts/tests/fixtures/ \| grep -c matrix-limit` reads 1 |

#822's own run refused at its plan, by that refusal: `mutation: shards: REFUSED: 9385 mutant(s),
projected at 1252726 s serially, need more than 190 legs within 10800 s each, the pull request
run's ceiling: split the change, since a run is never capped`. Its Python half raised no refusal,
since SPEC-422 holds the release run's Python legs to their own signed ceiling. The same Rust
listing, sized by `fewest_shards` with no ceiling, needs more than 190 legs and fewer than 256.

So every run, the release pull request's included, refuses a Rust listing whose legs pass 190,
whole and by name. A release's listing is every Rust mutant `dev` gained since the last release,
so a release that needs more legs than 190 cannot be judged at all. An owner-signed ruling admits
the release pull request's run alone above `LEG_CEILING["ci"]`, up to one ceiling: the figure
measured on the release's final union, with every mutant run, every leg counted and no
skipped-mutant record. Every other run keeps 190. Two facts bound the answer. SPEC-422's record
reads the matrix limit per matrix, so one matrix holds 256 legs. And this delivery's own lines add
Python mutants to the release's Python listing, so neither figure read before its last code commit
is the release's.

## 2. Requirements

R1. **The release run is SPEC-422's.** `shards` treats a plan as the release run exactly when
    `plan.get("release") is True` (SPEC-422 R2, R3). No second predicate is added, and no other
    input is read for it. A missing key, `false`, `"true"`, `1` or `null` is not the release run.
R2. **One ceiling constant.** `RUST_RELEASE_CEILING = 190` directly follows the `LEG_CEILING` line,
    with one `#:` line directly above it, and `RUST_MATRIX_LEGS = 256` directly follows it, with
    its own `#:` line. At 190 the release run admits no Rust leg past `LEG_CEILING["ci"]`.
R3. **The helper.** `rust_release_ceiling(plan) -> int | None` returns `RUST_RELEASE_CEILING` for a
    release plan when the constant is above `LEG_CEILING["ci"]`, and None otherwise. Under None,
    `shards` sizes, refuses and prints the Rust legs exactly as it does for a pull request into
    `dev`.
R4. **The release run sizes up to its ceiling and refuses past it whole.** Under a ceiling,
    `shards` calls `fewest_shards(costs, baseline, ceiling)` in place of `LEG_CEILING["ci"]`.
    When no count within it fits, it prints
    `mutation: shards: REFUSED: {n} mutant(s), projected at {s} s serially, need more than
    {ceiling} legs within {SHARD_BOUND_SECONDS} s each, the release run's ceiling
    (RUST_RELEASE_CEILING): a larger figure needs a new measurement and a new owner-signed ruling,
    since a run is never capped`, writes no `shards` into the plan and no output, and exits 1. Under
    None the pull request refusal's lines are not edited, and its text is byte for byte.
R5. **A ceiling outside its range is refused by name.** On every `shards` run, before any deal,
    after `python_share_refused` and `python_ceiling_refused`, a `RUST_RELEASE_CEILING` that is not
    an `int`, is below `LEG_CEILING["ci"]`, or is above `RUST_MATRIX_LEGS` makes `shards` print
    `mutation: shards: REFUSED: the release ceiling RUST_RELEASE_CEILING = {ceiling} is outside
    {LEG_CEILING['ci']}..{RUST_MATRIX_LEGS}; a ceiling outside it is never dealt` and exit 1.
R6. **What the run prints.** Under a ceiling, and on no other run:
    (a) after the Python lines and before the Rust shards line, `mutation: shards: release run:
    {count} rust leg(s) of ceiling {ceiling} (RUST_RELEASE_CEILING): {count} in mutation-rust, at
    most {RUST_MATRIX_LEGS} a matrix`, whether or not the Python legs were dealt by cost;
    (b) the Rust shards line ends `legs {count} of ceiling {ceiling}`;
    (c) the run-legs line's share is `PYTHON_MAX_SHARDS` plus the ceiling, its runner-minute budget
    counts the ceiling's legs at `SHARD_BOUND_SECONDS`, and its Rust term reads `rust {count} of
    {ceiling}`.
    SPEC-422 R6's Python release line keeps every byte. Every other run's printed lines and step
    outputs are byte for byte as at the base.
R7. **One matrix of at most 256.** `mutation-rust`'s axis stays the plan's `matrix` output, which
    `announce` writes as `0..count-1`. Exactly one `ci.yml` job's axis reads it. A test in
    `scripts/tests/test_dispatch_shards.py` holds `RUST_MATRIX_LEGS == RUN_JOBS`, and at a ceiling
    of `RUST_MATRIX_LEGS` holds a release deal of that many legs within `RUN_JOBS` in the one
    matrix. `LEG_CEILING`'s derivation, and every test that reads it, are unchanged.
R8. **Every leg is counted.** The verdict walks the plan's legs 0 to count - 1 for a count past
    `LEG_CEILING["ci"]` exactly as for any count: a leg with no report is VOID by name, and the
    reports must hold the listing exactly.
R9. **The tree cites the signed ruling.** At `LEG_CEILING["ci"]` the ceiling's `#:` line cites no
    path under `docs/rulings/`. Above it, the line cites exactly one `docs/rulings/<file>.md`, that
    file is in the tree, and its first line holds the ceiling's value as a whole word, the word
    `release` and the word `Rust`. No test and no mutation row pins the ceiling's value.
R10. **SPEC-422's criteria read the Python ceiling alone.**
    `scripts/tests/test_release_python_ceiling.py`'s `verb` holds `RUST_RELEASE_CEILING` at
    `LEG_CEILING["ci"]` while it runs a verb, by lines inserted beside its existing patches. Each
    of the four sites in `scripts/tests/test_mutation_python_shard_binding.py` that patch
    `LEG_CEILING` to `{"ci": 193}` and run `shards` gains one line holding `RUST_RELEASE_CEILING`
    at 193, every old line kept.
R11. **The map judges the new lines.** `scripts/mutation-python.json`'s
    `scripts/mutation-verdict.py` entry gains `test_release_rust_ceiling`, every old module kept.
R12. **The runbook says the method.** `RELEASING.md` gains one sentence directly after line 63 and
    one paragraph after line 116, before `## 3. Tag it on main`, insert-only, naming
    `RUST_RELEASE_CEILING`, `mutation-rust` and `RUST_MATRIX_LEGS`, saying a release listing past
    the ceiling is "refused whole, by name, never capped" and that a larger figure needs "a new
    measurement and a new owner-signed ruling". They hold no number outside a SPEC or ADR id.
R13. **What stays.** `LEG_CEILING`, `SHARD_BOUND_SECONDS`, `SECONDS_PER_MUTANT`,
    `BASELINE_SECONDS`, `CENSUS_SECONDS`, `MUTANT_TIMEOUT_SECONDS` and `TIMEOUT_MARGIN` are each
    assigned on one line, byte for byte as at the base; `fewest_shards`, `size`, `RUN_JOBS`,
    `ci.yml` and the Python constants are unchanged; and no new function restates 190, 234, 256 or
    10800 as a literal.
R14. **The figures' commit.** The commit that sets the figure edits exactly two lines of
    `scripts/mutation-verdict.py`, the constant's value and its `#:` line. When the release's final
    union moves the Python figure past its signed value, the same commit also edits
    `PYTHON_RELEASE_CEILING`'s value and its `#:` line, and both `#:` lines cite one new ruling.

## 3. Acceptance criteria of SPEC-423

Every test is in `scripts/tests/test_release_rust_ceiling.py` unless its row names another module.

| id | criterion | decided by |
|---|---|---|
| A1 | `rust_release_ceiling` returns a set ceiling for a `true` mark alone; a missing, `false`, `"true"`, `1` or `null` mark and the placeholder each return None | `test_only_a_true_release_mark_holds_the_rust_ceiling` |
| A2 | at the placeholder, a release plan's Rust lines and outputs equal the pull request plan's golden lines, while a set ceiling changes them | `test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run` |
| A3 | a release listing that needs exactly the ceiling's legs is sized at the ceiling; one mutant more is refused whole by R4's text, exit 1, with no shards and no outputs | `test_the_release_run_sizes_up_to_its_ceiling_and_refuses_one_mutant_past_it` |
| A4 | a plan that is not the release run keeps `LEG_CEILING["ci"]`, its refusal text and its golden lines byte for byte at ceilings 191, 200 and 256 | `test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling` |
| A5 | a ceiling that is a string, a float, a bool, 189 or 257 is refused by R5's text before any deal; 190 and 256 are admitted | `test_a_rust_ceiling_outside_its_range_is_refused_by_name` |
| A6 | on the release run under a set ceiling the Rust shards line and the run-legs line read it, and on a pull request plan they read 190 | `test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone` |
| A7 | R6 (a)'s line prints directly after SPEC-422 R6's line when both print, prints with no Python deal, and never prints on a pull request plan or at the placeholder | `test_the_rust_release_line_stands_beside_the_python_release_line` |
| A8 | `RUST_MATRIX_LEGS == RUN_JOBS`, one `ci.yml` axis reads the plan's `matrix`, and a release deal at that ceiling writes one matrix of that many legs | `test_dispatch_shards.py` `test_the_rust_matrix_of_the_release_run_fits_the_run_jobs` |
| A9 | over a plan of more legs than `LEG_CEILING["ci"]`, the verdict reads every leg's report, and one leg missing is VOID by its name | `test_the_verdict_walks_every_leg_past_the_pull_request_ceiling` |
| A10 | R9: one ceiling assignment with its `#:` line; no ruling path at the placeholder; above it one path, present, whose first line holds the value, `release` and `Rust`; four planted sources each refused | `test_the_rust_ceiling_cites_the_signed_ruling_only_above_the_pull_request_ceiling` |
| A11 | with the module's Rust ceiling set, SPEC-422's `deal` still prints its golden lines on a release plan at the Python placeholder, while the same plan run without that hold prints R6 (a)'s line | `test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder` |
| A12 | the runbook holds R12's sentence and paragraph at their places, with no number outside a SPEC or ADR id | `test_the_runbook_names_the_rust_release_ceiling_its_matrix_and_its_refusal` |
| A13 | R13: each kept value has one assignment line, and no new function holds 190, 234, 256 or 10800 as a literal | `test_the_release_run_restates_no_rust_bound_and_assigns_each_once` |
| A14 | the map's `scripts/mutation-verdict.py` entry names `test_release_rust_ceiling`, every old module kept | `test_the_map_judges_the_verdict_scripts_mutants_with_the_rust_ceiling_tests` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_only_a_true_release_mark_holds_the_rust_ceiling
A2: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run
A3: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_release_run_sizes_up_to_its_ceiling_and_refuses_one_mutant_past_it
A4: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling
A5: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_a_rust_ceiling_outside_its_range_is_refused_by_name
A6: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone
A7: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_rust_release_line_stands_beside_the_python_release_line
A8: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_rust_matrix_of_the_release_run_fits_the_run_jobs
A9: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_verdict_walks_every_leg_past_the_pull_request_ceiling
A10: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_rust_ceiling_cites_the_signed_ruling_only_above_the_pull_request_ceiling
A11: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder
A12: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_runbook_names_the_rust_release_ceiling_its_matrix_and_its_refusal
A13: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_release_run_restates_no_rust_bound_and_assigns_each_once
A14: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_map_judges_the_verdict_scripts_mutants_with_the_rust_ceiling_tests
```

A9 and A13 are not red first. The base verdict already walks the plan's legs 0 to count - 1 for
any count (SPEC-290, SPEC-362 R8), so A9 pins that a count past 190 is walked whole; its own
negative arm, one leg missing, is its control. The base already assigns each kept value once and
restates none in a new function (A13). `docs/red-first/SPEC-423.md` records each as `not red`, with
that reason. Every other criterion is red at the red commit, on an assertion of its behaviour: the
red commit carries stubs that keep every input and drop the missing behaviour
(`rust_release_ceiling` returns None, `rust_ceiling_refused` returns None and `main` does not call
it, and `RUST_MATRIX_LEGS = 256` exists with its `#:` line), and no `RUST_RELEASE_CEILING`. A test
that sets a ceiling patches it with `mock.patch.object(..., create=True)`, so it reaches its
behavioural assertion at the red commit. A2, A4 and A11 are each red on their positive arm (a set
ceiling on a release plan, which the base does not size past 190), which is the behaviour each
criterion's negative arms are measured against. A10 is red on its first assertion: the base holds
no `RUST_RELEASE_CEILING` assignment with its `#:` line. A14 is red because the map change lands in
the green commit. Every expected value is a literal the test spells, never one the code under test
builds, and every enumerating test prints its examined count and refuses zero.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | the plan | `RUST_RELEASE_CEILING` and `RUST_MATRIX_LEGS` with their `#:` lines after `LEG_CEILING`; `rust_ceiling_refused` and `rust_release_ceiling` after `python_release_ceiling`; `main`'s range check after `python_ceiling_refused`; in `shards`, the ceiling's reading, R4's release refusal, R6's release line, and the Rust shards and run-legs lines reading the ceiling |
| `scripts/mutation-python.json` | the map | the `scripts/mutation-verdict.py` entry gains `test_release_rust_ceiling`; its seven modules stay |
| `scripts/tests/test_release_rust_ceiling.py` | the tests | added: A1 to A7 and A9 to A14 |
| `scripts/tests/test_dispatch_shards.py` | the job budget | A8 added; `RUN_JOBS`, `generated()` and every existing test unchanged |
| `scripts/tests/test_release_python_ceiling.py` | SPEC-422's helper | `verb` holds `RUST_RELEASE_CEILING` at `LEG_CEILING["ci"]`, inserted lines only (R10) |
| `scripts/tests/test_mutation_python_shard_binding.py` | four patch sites | one line each beside the `LEG_CEILING` patch, holding `RUST_RELEASE_CEILING` at 193 (R10) |
| `scripts/tests/test_ci_workflows.py` | the census | `NOT_WORKFLOW_READS` and `DYNAMIC_IMPORTS` gain one entry for each read and import site the new and edited test modules add, each with its exact count and reason; every old entry kept |
| `scripts/mutation-rows.d/S42300-S42399.json` | the rows | added: S42300 to S42308 (section 9) |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | the formal record | `\*` comment lines and the `digest=` value of `shards` (section 8) |
| `RELEASING.md` | the runbook | insert-only (R12) |
| `docs/specs/SPEC-423-the-release-pull-requests-rust-legs-may-pass-the-pull-request-ceiling-up-to-one-signed-ceiling-in-one-matrix-of-at-most-256.md` | this SPEC | added |
| `docs/decisions/ADR-437-the-release-runs-rust-legs-are-held-to-one-signed-ceiling-read-from-the-same-release-mark-in-the-one-rust-matrix.md` | the decision | added |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | an amendment | insert-only, after its last line |
| `docs/specs/SPEC-422-the-release-pull-requests-python-legs-may-pass-the-share-up-to-one-signed-ceiling-dealt-into-matrices-of-at-most-256.md` | an amendment | insert-only, after its last line |
| `docs/decisions/ADR-373-a-release-is-judged-in-one-run-of-round-robin-legs-sized-to-the-hosted-job.md` | an amendment | insert-only, after its last line |
| `docs/decisions/ADR-436-the-release-run-is-named-by-a-fail-closed-predicate-and-its-python-legs-pass-the-share-only-up-to-one-signed-ceiling-over-two-matrices.md` | an amendment | insert-only, after its last line |
| `docs/schematics/mutation-testing.md` | the schematic | insert-only, section 11 after its last line |
| `docs/red-first/SPEC-423.md` | the red-first record | added |
| `changelog.d/release-rust-ceiling-423.md` | the changelog | added |

`.github/workflows/ci.yml`, the rulesets, `scripts/mutation_python.py` and `scripts/mutation_rows.py`
are not changed: `mutation-rust` already expands the plan's `matrix` output, whatever its count.
The owner-signed ruling file the ceiling cites is not this delivery's file: it is committed to
`dev` on its own, and this pull request takes it from `dev` before it lands.

## 5. What this does NOT cover

- It does not set the Rust figure: the figure is read on the release's final union and set by its
  own later commit, and #822 is the release it is read for.
- It does not add a second Rust matrix, a verdict across runs or a further Python matrix, #691.
- It does not raise `LEG_CEILING`, `RUN_JOBS`, the Python share or its cap for any run, #825.
- It does not lengthen any leg, change any price, or size the weekly battery differently, #825.
- It does not sample, skip or exempt any mutant, and adds no skipped-mutant record, #822.
- It does not change which head reaches `main`: `base-is-dev` and ADR-034's model stand, #691.

## 6. Risks

- The matrix limit is counted per run after all. Then SPEC-422 A19 fails on its record, and one
  matrix past 190 legs cannot start: the release needs another route.
- A test that moves `LEG_CEILING["ci"]` and runs `shards` reads the placeholder below the range.
  The four such sites are named (R10); a fifth would read R5's refusal, red and by name, never a
  silent pass.
- A release whose Python legs are dealt by the count rule prints no SPEC-422 R6 line. R6 (a)'s
  Rust line prints on every release run under a set ceiling, so the Rust figure is still named.
- A run of this many legs waits for runners, and its verdict job reads that many reports. A leg
  that never reports is VOID by name and `ci` reads red: closed, never a silent pass.
- The figure's `#:` line passes the lint's line length with the ruling's path. The path is fixed
  when the ruling is cut, so a path too long is a stop before the figure's commit.
- A later release needs more legs than the signed figure. R4 refuses it whole, by name, and it
  waits for a new measurement and a new signature; one that needs more than one matrix waits for a
  new decision.

## 7. What only CI proves

- Every red and every green is read in CI. `scripts/tests/test_dispatch_shards.py` (A8) runs only
  there.
- The release pull request's own run, after this delivery lands, must reproduce both signed
  figures in R6 (a)'s line and SPEC-422 R6's line.

## 8. Formal model

`formal/tla/EveryLegCounted` covers `shards` in `scripts/mutation-verdict.py`, and this delivery
edits it; `python_by_cost`, `whole_reports`, `partition`, `examined_sum`, `population_gaps` and
`planned_leg` are not edited, and their digests stay. Each hunk of `shards` is re-read against the
model and recorded in its `\*` comments by item name (`mutation-verdict.py::shards`), and its
`digest=` value moves to the digest rule's value at the head, with the base's recorded value as the
rule's control. No variable, action or property changes: which bound holds the Rust count, the
pull request ceiling or the release ceiling, changes neither the round-robin partition of the
listed mutants over legs nor the per-leg actions; a refusal is the stuttering step the model's
abstractions name; and the lines printed after the deal deal nothing. `main`,
`rust_ceiling_refused` and `rust_release_ceiling` are under no cover. No new model is written, and
no model checker runs: this repository's CI has no formal job.

## 9. Mutation rows (S42300-S42399)

Rows S42300 to S42308, in `scripts/mutation-rows.d/S42300-S42399.json`. Each find is taken from
the green commit's text and occurs exactly once in its file; none sits on a ceiling constant or its
`#:` line.

| row | file | find (the text the mutant changes) | mutant | killer |
|---|---|---|---|---|
| S42300 | `scripts/mutation-verdict.py` | R4's text `the release run's ceiling (RUST_RELEASE_CEILING): a larger figure needs a ` | `the pull request run's ceiling (RUST_RELEASE_CEILING): a larger figure needs a ` | A3 |
| S42301 | `scripts/mutation-verdict.py` | R5's text `is outside {LEG_CEILING['ci']}..{RUST_MATRIX_LEGS}` | `is within {LEG_CEILING['ci']}..{RUST_MATRIX_LEGS}` | A5 |
| S42302 | `scripts/mutation-verdict.py` | R6 (a)'s `(RUST_RELEASE_CEILING): {count} in mutation-rust, at most` | `in mutation-python, at most` in its place | A7 |
| S42303 | `scripts/mutation-verdict.py` | the Rust shards line's `bound; legs {count} of ceiling {rust_ceiling}"` | `{LEG_CEILING['ci']}` in place of `{rust_ceiling}` | A6 |
| S42304 | `scripts/mutation-verdict.py` | the run-legs line's `rust {count} of {rust_ceiling}; "` | `{LEG_CEILING['ci']}` in place of `{rust_ceiling}` | A6 |
| S42305 | `RELEASING.md` | R12's sentence ``up to `RUST_RELEASE_CEILING` (SPEC-423).`` | ``up to `LEG_CEILING` (SPEC-423).`` | A12 |
| S42306 | `RELEASING.md` | R12's paragraph `a new measurement and a new owner-signed ruling` | `a new measurement` | A12 |
| S42307 | `scripts/tests/test_release_python_ceiling.py` | `verb`'s hold `"RUST_RELEASE_CEILING", module.LEG_CEILING["ci"]` | `"RUST_RELEASE_CEILING", 200` | A11 |
| S42308 | `scripts/mutation-python.json` | the entry's `"test_release_python_ceiling",` and `"test_release_rust_ceiling"` lines | `"test_release_python_ceiling"` alone | A14 |

The listing tool's own mutants of the new Python lines (the helper's mark and bound, the range
check's three arms, the ceiling's fallback, the refusal's condition, the release line's condition,
the share's sum, the budget's arithmetic and the returns) are judged by the delivery's own mutation
run, and each operand of R3 and R5 has a test row in which it alone decides the result (A1, A5).
