# SPEC-415: a Python mutation listing past the run's 32 Python legs is refused whole, by name, never capped

- **Issue:** #825, which tracks the Python legs' fit in one run; this delivery refers to it and
  closes nothing. **Context(s):** the mutation scripts (`scripts/`), their tests (`scripts/tests/`)
  and the release runbook; no crate, no web source and no workflow changes.
- **Decided by:** ADR-429 (the Python plan refuses whole above its 32 legs, by name, as the Rust
  plan refuses above its ceiling; sizing by a measured cost, a raised job bound, a joint budget and
  the cap itself are rejected).
- **Schematic:** `docs/schematics/mutation-testing-python.md`, amended insert-only by its section 5:
  the plan before and after, with where the cap and the bound cut.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-415.md`. **Mutation band:** `S41500-S41599` (section 9). **Model:**
  `formal/tla/EveryLegCounted`, its `shards` cover re-read and re-stamped (section 8). Amends
  SPEC-087 (R9), SPEC-362 (R14 and A12's last case) and ADR-373 (its D1 amendment), each by an
  amendment section in its own file.

## 1. The problem, measured

- **The Python plan caps where the Rust plan refuses.** `scripts/mutation-verdict.py`'s
  `python_shards` sizes the `mutation-python` matrix at `count = min(PYTHON_MAX_SHARDS, max(1,
  -(-len(listing) // PYTHON_SHARD_MUTANTS)))`, with `PYTHON_SHARD_MUTANTS, PYTHON_MAX_SHARDS = 20,
  32`. Past 640 listed mutants the count stays 32 and each leg takes more than 20, with no line in
  the plan saying so. The Rust plan's `fewest_shards` refuses a listing past `LEG_CEILING["ci"]`
  whole, by name: "split the change, since a run is never capped".
- **A release's listing needed 272 legs and ran 32.** The release pull request's plan job
  114529617353 (24 seconds) printed `mutation-python: listed 5423` and `mutation: shards: 32
  python shard(s) for 5423 listed mutant(s)`: 169 or 170 mutants a leg, where `ceil(5423 / 20)` =
  272 legs of 20 hold them. The same plan printed `mutation: shards: 192 shard(s) for 9385 listed
  mutant(s), ...; legs 192 of ceiling 209` for the Rust legs.
- **Every Python leg ended without a report.** 28 legs were cancelled at the job's
  `timeout-minutes: 60` (shard 0, job 114529704945, its run step 60 minutes 6 seconds; shard 14,
  job 114529704978, 60 minutes 8 seconds). 4 failed with exit 143, "The runner has received a
  shutdown signal": shard 28, job 114529704969, 33 minutes 45 seconds; shard 27, job
  114529704958, 44 minutes 3 seconds; shard 31, job 114529704961, 50 minutes 37 seconds; shard
  21, job 114529704919, 53 minutes 12 seconds. None uploaded a report ("No files were found").
  `mutation_python.py run` writes its report only when its run ends and prints nothing per
  mutant, so a cut leg leaves no outcome and no cost, and the verdict reads each leg VOID.
- **No per-mutant cost is measured for this listing.** A leg of 170 that did not finish in 3606
  seconds bounds the cost only from below, at more than 21.2 seconds a mutant with its files'
  control and sentinel runs. The tree's one figure, about 96.5 seconds a mutant (34 mutants in 54
  minutes 42 seconds, SPEC-362 R14), comes from another diff's files, and each file's per-mutant
  bound is five times that file's own control run. The scheduled battery's Python leg, which would
  record per-file control seconds, was skipped in every battery run read.
- **No shape fits this release in one run.** The run holds 256 jobs; beside 192 Rust legs and 15
  other jobs, 49 are left, 980 mutants at 20 a leg. At the precedent's 120-minute ceiling for a
  raised bound (ADR-390 D2), the 5423 mutants need 147 legs at half the job, or 74 at the whole job,
  each at the 96.5-second figure. ADR-429 D2 records the arithmetic.

## 2. Requirements

R1. **The legs a listing needs.** `python_needed(n)` returns `max(1, ceil(n / 20))`, the Python
    legs a listing of n mutants needs at `PYTHON_SHARD_MUTANTS` a leg, and 1 for an empty listing.

R2. **Sized at or under the ceiling, refused above it.** `python_shards(listing)` returns the matrix
    `{"count": c, "shards": [{"shard": k, "mutants": names[k::c]} ...]}`, dealt as before, when
    `python_needed(len(listing))` is at most `PYTHON_MAX_SHARDS` (32), and `None` when it is more.
    No listing is ever dealt into fewer legs than it needs.

R3. **The plan refuses whole, by name.** `shards --python-listed` over a listing R2 refuses prints
    `mutation: shards: REFUSED: <n> python mutant(s) need <legs> legs of at most 20, more than the
    pull request run's 32 python legs: split the change, since a run is never capped`, writes no
    step output and no byte of the plan file, and exits 1 (`EXIT_FAIL`), before any Rust leg is
    dealt. The plan job fails, so no Python or Rust leg starts and `ci` fails.

R4. **The plan prints its Python headroom.** A sized plan prints `mutation: shards: <c> python
    shard(s) for <n> listed mutant(s); python legs <c> of ceiling 32`, as the Rust line prints
    `legs N of ceiling C`.

R5. **Nothing else moves.** `PYTHON_SHARD_MUTANTS` (20), `PYTHON_MAX_SHARDS` (32),
    `PYTHON_WEEKLY_SHARDS` (16), `LEG_CEILING`, `python_listing`, `python_reports`,
    `read_python_shard`, `shard_listing_drift`, `judge_python`, `scripts/mutation_python.py`,
    `.github/workflows/ci.yml` and `.github/workflows/mutation-weekly.yml` are byte-identical:
    every leg's bound, every per-mutant bound and `--failfast` are unchanged, and no mutant is
    sampled, filtered or dropped. A refused plan's file is byte-identical to a plan never sized,
    which the verdict already reads as "VOID the plan names no python shards" (SPEC-087 R11).

R6. **The runbook teaches it.** `RELEASING.md` section 2 gains one paragraph: the plan prints
    `python legs N of ceiling 32`, a listing that needs more than 32 Python legs is refused whole,
    by name, never capped, and re-running its failed jobs refuses it again.

R7. **The owning records say so, insert-only.** SPEC-087 (after R9), SPEC-362 (R14, and A12's
    last case, 32 x 20 + 1, which now reads refused under its unchanged id, command and test name),
    ADR-373 (its D1 amendment) and `docs/schematics/mutation-testing-python.md` each gain an
    amendment section, and no earlier byte of any of them changes.

R8. **The model is re-read.** `formal/tla/EveryLegCounted`'s `shards` cover is re-read against the
    refusal arm, the arm is named among the model's abstractions in comment lines, and the cover's
    digest moves to the new span; no property, action, constant or configuration changes.

R9. **Each new behaviour has a row.** Band `S41500-S41599` holds rows S41500 to S41507 (section 9),
    each a hand mutant of `scripts/mutation-verdict.py` or `RELEASING.md` killed by the criterion's
    test it names.

## 3. Acceptance criteria of SPEC-415

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | `python_shards` refuses 641, 2000 and 5423 listed mutants, which need 33, 100 and 272 legs by `python_needed`, and deals 0, 1, 20, 21, 68 and 640 listed into 1, 1, 1, 2, 4 and 32 legs, leg k holding the names k, k + c, and so on (R1, R2) | red at the base, where 641 listed read 32 legs | `test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped` |
| A2 | `shards --python-listed` over 641 listed exits 1, prints the refusal naming 641, 33, 20 and 32, writes no step output and leaves the plan file's bytes as `plan` wrote them (R3, R5) | red at the base, where the plan exits 0 with 32 legs | `test_a_python_listing_past_the_ceiling_refuses_the_plan_whole_by_name` |
| A3 | a sized plan prints `python legs 4 of ceiling 32` for 68 listed and `python legs 32 of ceiling 32` for 640 (R4) | red at the base, where the line ends at "listed mutant(s)" | `test_the_plan_prints_its_python_legs_against_their_ceiling` |
| A4 | `RELEASING.md` section 2 names `python legs N of ceiling 32`, the whole refusal past 32 legs, and that re-running a refused plan's failed jobs refuses it again (R6) | red at the base, where section 2 names only the Rust headroom | `test_the_runbook_names_the_python_ceiling_and_its_whole_refusal` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_python_listing_past_the_ceiling_refuses_the_plan_whole_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_the_plan_prints_its_python_legs_against_their_ceiling
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_ceiling_and_its_whole_refusal
```

Each is read red and green from CI's `hygiene` job, which runs `scripts/tests` through
`scripts/check.sh python`. Two existing tests keep their names and every case, and change one
expectation each, red in the same run: SPEC-362 A12's
`test_the_matrix_takes_twenty_listed_mutants_a_shard_clamped_to_thirty_two` (its 32 x 20 + 1 case
now reads refused), and `test_the_plan_sizes_the_python_matrix_from_its_listing` (its 641 and 2000
cases now read refused, through the plan's exit 1).

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | CI scripts | changed: `python_needed` added; `python_shards` returns `None` above 32 legs, its docstring says so; `shards` prints the refusal and exits 1, and its sized Python line gains `; python legs <c> of ceiling 32` |
| `scripts/tests/test_mutation_python_shard_binding.py` | CI tests | changed: A1's test added; SPEC-362 A12's test keeps its name and cases, its last case reads refused, and its docstring gains one sentence |
| `scripts/tests/test_mutation_python_verdict.py` | CI tests | changed: A2's and A3's tests added; `test_the_plan_sizes_the_python_matrix_from_its_listing` keeps its cases, and 641 and 2000 read refused |
| `scripts/tests/test_rulesets.py` | CI tests | changed: A4's test added in a class of its own |
| `scripts/mutation-rows.d/S41500-S41599.json` | CI tests | added: rows `S41500` to `S41507` |
| `RELEASING.md` | docs | changed: one paragraph inserted in section 2 |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | formal | changed: comment lines naming the refusal among the abstractions, and the `shards` cover's digest |
| `docs/specs/SPEC-415-a-python-mutation-listing-past-the-runs-thirty-two-python-legs-is-refused-whole-by-name-never-capped.md` | docs | added: this file |
| `docs/decisions/ADR-429-the-python-mutation-plan-refuses-a-listing-past-its-thirty-two-legs-whole-by-name-and-never-caps-it.md` | docs | added: the decision |
| `docs/red-first/SPEC-415.md` | docs | added: A1 to A4's reds and greens, and the two changed expectations in prose |
| `docs/specs/SPEC-087-the-guard-scripts-and-the-parity-oracle-prove-their-tests-kill-generated-mutants.md` | docs | changed: an amendment section |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | docs | changed: an amendment section |
| `docs/decisions/ADR-373-a-release-is-judged-in-one-run-of-round-robin-legs-sized-to-the-hosted-job.md` | docs | changed: an amendment section |
| `docs/schematics/mutation-testing-python.md` | docs | changed: section 5 inserted |
| `changelog.d/python-shards-415.md` | docs | added: the fragment |

## 5. What this does NOT cover

- It does not size the Python legs by a measured per-mutant cost. No per-file cost is recorded
  today; pricing each file from the control seconds a completed leg's report records, and sizing
  the legs from it, is #825.
- It does not raise any job's `timeout-minutes` or any per-mutant bound. No shape fits the
  measured release inside the precedent's 120-minute band (ADR-429 D2), and a bound past that band
  is not this delivery's to set; the cost such a bound would need is #825.
- It does not make the release measured here fit one run: its plan is refused until its range is
  split or the Python legs are sized by cost, #822 and #825.
- It does not share the run's jobs between the Python and Rust legs by a joint budget:
  `LEG_CEILING` keeps the Python legs counted at 32, and a joint budget is #825.
- It does not diagnose the four Python legs lost to a runner shutdown at 33 to 54 minutes, before
  their bound; whether a re-run reproduces them is #825.
- It does not touch the `mutation-rows` job, cut at its 90-minute bound in the same run (job
  114529704482, 90 minutes 17 seconds), #825.
- It does not make the scheduled battery's Python leg run; it was skipped in every battery run
  read, and its 16 legs are unchanged, #825.
- It does not shard the web sweep or change any web bound, #693.

## 6. Risks

- **A listing a little past 640 is refused where the cap ran it.** A diff of, say, 700 Python
  mutants dealt 22 to a leg could have finished inside the hour, and now refuses. Detected by the
  plan's own refusal line; cured by splitting the change or by the cost-sized follow-up (#825).
- **A refusal hides the Rust headroom.** The Python refusal comes before the Rust deal, so a
  listing both plans would refuse prints only the Python line. Detected when the split range is
  planned again, which prints the Rust line.
- **A row's anchor is not unique.** Row S41502's anchor holds a `return EXIT_FAIL` line, as row
  S03913's anchor does. Detected by the rows census, which refuses an anchor that
  does not occur exactly once; each anchor here carries the line after it.
- **The rows job selects every row on the edited files.** A diff that touches
  `scripts/mutation-verdict.py` selects every row whose target it is, the new ones included, and the
  release's own rows job was cut at its 90-minute bound. Detected by the `mutation-rows` job's
  verdict, read by name in the second push's run; a cut there is reported, not cured here (#825).
- **The delivery's own Python legs meet the cost this issue tracks.** The second push's diff lists
  the new code's generated mutants, at 20 a leg. Detected by each `mutation-python` leg's verdict,
  read by name; a leg cut at its bound is a measurement for #825, reported, not cured here.
- **The cover moves again.** Any later edit inside `shards` moves the digest. Detected by the
  formal check's `STALE` finding on every property of `tla/EveryLegCounted`.

## 7. What only CI proves

A1 to A4's reds and greens, the two changed expectations, the whole Python suite, the rows' census
and their killers are read in CI by name. No test module runs off CI for this delivery. The formal
check of `tla/EveryLegCounted` is the checker's run on the delivery's head, outside CI.

## 8. Formal model

`formal/tla/EveryLegCounted` covers `scripts/mutation-verdict.py::shards`, which this delivery
edits. The new arm returns `EXIT_FAIL` before any leg is dealt, as the Rust refusal in the same span
already does: it deals no leg, writes no report and reaches no pass, so it moves none of the
model's variables and is a stuttering step for the model. The model's claims (every dealt leg is
counted, each listed mutant is examined exactly once, a pass means every leg reported whole) are
unchanged, so no property, action, constant or configuration changes. The arm is named among the
model's abstractions in comment lines, and the `shards` cover's digest moves to the new span; a
check after the move reads no `STALE`.

## 9. Mutation rows

| stem | file | mutant | killer |
|---|---|---|---|
| `S41500-THE-PYTHON-PLAN-CAPS-PAST-ITS-CEILING` | `scripts/mutation-verdict.py` | `python_shards` sets the count to 32 instead of refusing | `test_mutation_python_shard_binding.ThePythonMatrixIsSizedToTheHostedJob.test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped` |
| `S41501-THE-PYTHON-CEILING-REFUSES-THIRTY-TWO` | `scripts/mutation-verdict.py` | `count > PYTHON_MAX_SHARDS` becomes `count >= PYTHON_MAX_SHARDS`, so 640 listed is refused | `test_mutation_python_shard_binding.ThePythonMatrixIsSizedToTheHostedJob.test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped` |
| `S41502-A-REFUSED-PYTHON-PLAN-EXITS-ZERO` | `scripts/mutation-verdict.py` | the refusal returns `EXIT_OK` | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_python_listing_past_the_ceiling_refuses_the_plan_whole_by_name` |
| `S41503-THE-REFUSAL-NAMES-THE-LISTING-AS-ITS-LEGS` | `scripts/mutation-verdict.py` | the refusal names the listed count where the legs needed stand | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_python_listing_past_the_ceiling_refuses_the_plan_whole_by_name` |
| `S41504-THE-HEADROOM-NAMES-THE-LEG-SIZE-AS-ITS-CEILING` | `scripts/mutation-verdict.py` | the headroom line prints `of ceiling 20` | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_the_plan_prints_its_python_legs_against_their_ceiling` |
| `S41505-AN-EMPTY-LISTING-NEEDS-NO-LEG` | `scripts/mutation-verdict.py` | `python_needed`'s floor of 1 becomes 0 | `test_mutation_python_shard_binding.ThePythonMatrixIsSizedToTheHostedJob.test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped` |
| `S41506-THE-RUNBOOK-NAMES-THE-LEG-SIZE-AS-THE-CEILING` | `RELEASING.md` | the runbook's line reads `python legs N of ceiling 20` | `test_rulesets.TheReleaseRunbookNamesThePythonCeiling.test_the_runbook_names_the_python_ceiling_and_its_whole_refusal` |
| `S41507-THE-RUNBOOK-SENDS-A-REFUSED-PLAN-TO-A-RE-RUN` | `RELEASING.md` | the runbook says re-running a refused plan's failed jobs runs it | `test_rulesets.TheReleaseRunbookNamesThePythonCeiling.test_the_runbook_names_the_python_ceiling_and_its_whole_refusal` |
