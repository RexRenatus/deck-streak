# SPEC-418: the Python mutation legs are dealt by each file's measured cost, every listed mutant examined or the plan refused whole

- **Issue:** #829, the Python mutation legs sized by what each file's mutants cost; refers,
  closes nothing. Refers #825, the Python plan's whole refusal, which this builds on.
  **Context(s):** the mutation scripts (`scripts/`) and the release runbook; no workflow and no
  crate changes.
- **Decided by:** ADR-432 (the Python legs are dealt by each file's measured control cost under the
  run's fixed leg shares, and a listing they cannot hold is refused whole).
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 6, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-418.md`. **Mutation band:** S41800-S41899. **Model:**
  `formal/tla/EveryLegCounted`. Amends SPEC-087, SPEC-362 and ADR-373, each by an insert-only
  amendment section in its own file. Builds on SPEC-415 and ADR-429 (the count rule and its whole
  refusal), which it keeps for every unpriced file.

## 1. The problem, measured

A pull request's Python mutants run in the `mutation-python` matrix: one hosted job a leg, with
`timeout-minutes: 60` (`.github/workflows/ci.yml:531-534` at `c74b6b9`), each running
`mutation_python.py run --plan ... --shard k/n --failfast` (`ci.yml:562`). The plan sizes that
matrix by a count, 20 listed mutants a leg (`scripts/mutation-verdict.py:121`, `python_shards` at
`:987-995`), and each leg runs `chosen[k::n]` of the runner's own listing
(`scripts/mutation_python.py:790-792`). A count treats every mutant alike, and they are not alike:
the runner bounds each file's mutants by five times that file's own control run (`BOUND_FACTOR`,
`mutation_python.py:63` and `:742`), so one file's mutant can cost many times another's.

Measured on the release pull request #822's cancelled run:

- Its plan job 114529617353 (24 seconds) listed 5423 Python mutants in 14 files and dealt them into
  32 legs of 169 or 170 (`mutation-python: listed 5423`; `32 python shard(s) for 5423 listed
  mutant(s)`). Per file: `scripts/mutation-verdict.py` 2488, `scripts/mutation_rows.py` 931,
  `scripts/mutation_python.py` 555, `ios_lane.py` 355, `memory_scope.py` 237, `testflight_age.py`
  236, `generate.py` 143, `threat_model.py` 116, `dynamic-imports-check.py` 106, `public-scrub.py`
  104, `audit-web-verdict.py` 55, `web-engine-size.py` 53, `ios_icon.py` 41,
  `write-back-issues.py` 3.
- 30 of the 32 Python leg logs were read; two logs are gone. Every one shows 0 lines printed by the
  runner between the start of its run step and the job's end.
  Leg 0, job 114529704945, ran its run step 60 minutes 6 seconds and printed nothing.
  Job 114529704969 ended at 33 minutes 45 seconds (exit 143);
  job 114529704958 at 44 minutes 3 seconds; job 114529704961 at 50 minutes 37 seconds;
  and job 114529704919 at 53 minutes 12 seconds; each before its bound, each printing nothing.
  Each leg's upload step ran after the cut and found no file.
- The runner keeps each file's control seconds in its report (`mutation_python.py:719-728`) but
  writes the report only when the run ends (`:901-902`) and prints only its closing counts
  (`:911-912`). The upload step keeps the whole report directory (`ci.yml:564-570`), so a file
  written there as the run goes would survive a cut; none is written today.
- So the run holds no per-file control second. Its legs bound the cost from below only:
  leg 0 (job 114529704945), 170 mutants, ran 3606 seconds without finishing, so a mutant costs more
  than 21.2 seconds, its files' control and sentinel runs included. The tree's
  one per-mutant figure, about 96.5 seconds (SPEC-362 R14), came from another diff's files.

**The release listing does not fit.** 32 legs of 1800 seconds hold 57600 seconds, 10.6 seconds a
mutant, and the run measured more than 21.2: 5423 x 21.2 = 114968 seconds, 64 legs at that bound.
Even the job's whole 60 minutes, 32 x 3600 = 115200 seconds, is within 0.2% of a strict lower bound,
which proves no fit. With no file priced the plan stays on SPEC-415's rule, which refuses the
listing at 272 legs, so the release run reads `mutation-python` refused by name ("VOID the plan
names no python shards", SPEC-087 R11). This SPEC builds the deal by cost; it prices no file.

## 2. Requirements

R1. `scripts/mutation-verdict.py` holds `PYTHON_LEG_BOUND_SECONDS = 1800`, half the
`mutation-python` job's `timeout-minutes` of 60, and a test holds it equal to that half against the
workflow, as `SHARD_BOUND_SECONDS` is held for the Rust legs.

R2. It holds `PYTHON_CONTROL_SECONDS: dict[str, int]`, a price in whole seconds per
repository-relative Python path, shipped empty. Every later entry is written from the `prices`
verb's output (R7) and names the run, attempt and shard it came from.

R3. `python_by_cost(listing, prices)` deals a Python listing by cost:
- (a) it returns None when no listed file has a price, and the plan is then SPEC-415's, byte for
  byte;
- (b) for each priced file F at price c, k = floor(1800 / c) - 2 is the most of F's mutants one leg
  holds beside F's control and sentinel; k < 1 refuses the whole plan, naming the path, c, 3c and
  1800: `mutation: shards: REFUSED: <path>'s control is priced at <c> s, so a leg holding one of
  its mutants projects <3c> s, past its 1800 s bound`;
- (c) F's listed mutants, in listing order, are cut into ceil(m / k) runs of near-equal size (sizes
  differ by at most one, the larger first);
- (d) the runs are dealt heaviest first, a run weighing (2 + size) x c (ties by path, then by the
  run's first listing index), each onto the lightest leg (ties to the lowest index); a leg that
  already holds F adds size x c, any other leg adds (2 + size) x c;
- (e) the priced legs are the fewest n, from 1 upward, whose slowest projection is within 1800;
- (f) the unpriced mutants, in listing order, take `python_needed(u)` legs (SPEC-415's count rule,
  at most 20 a leg), dealt round robin after the priced legs, each projected at 1800;
- (g) priced and unpriced legs together above 32 refuse the whole plan: `mutation: shards: REFUSED:
  <n> python mutant(s) need <p> leg(s) priced at most 1800 s each and <q> unpriced leg(s) of at
  most 20, more than the pull request run's 32 python legs: split the change, since a run is never
  capped`;
- (h) a deal has SPEC-415's matrix shape (`count`, and `shards` of `shard` and `mutants`) plus each
  shard's `projected_seconds`.

R4. `shards` calls `python_by_cost(listing, PYTHON_CONTROL_SECONDS)` between SPEC-415's
`python_shards` call and its None refusal; a deal replaces the count rule's matrix. A cost refusal
exits 1 (`EXIT_FAIL`) before any Rust leg is dealt, writes no step output and leaves the plan's
bytes unchanged. On the cost path the plan prints SPEC-415's python line, then
`mutation: shards: python legs dealt by cost: <p> priced leg(s), the slowest at <s> s of its 1800 s
bound; <q> unpriced leg(s) of at most 20`, and after the Rust line
`mutation: shards: run legs <p+q+r> of 225: python <p+q> of 32, rust <r> of 193; projected <X>
runner-minute(s) of 35700`, where X is the sum of every Python and Rust leg's projected seconds over
60, rounded up. When nothing is priced, nothing new prints.

R5. `scripts/mutation_python.py` gains `planned_leg(plan, chosen, k, n)`, which `select` calls for
`--plan` with `--shard k/n`. When the plan carries a `python` matrix: a `count` other than n
refuses (`mutation-python: usage: the plan deals <count> python leg(s), not <n>`); a multiset of
every shard's names other than the multiset of the chosen names refuses (`mutation-python: usage:
the plan's python legs do not partition the listing: <a> missing, <b> extra, first <name>`, the
first missing name in listing order, else the first extra in plan order). Each refusal mutates
nothing and writes no report (`EXIT_USAGE`). Otherwise the leg runs the chosen mutants whose names
shard k lists, in listing order. A plan with no `python` matrix, `--all` and `--file` keep `k::n`
unchanged.

R6. With `--report`, the runner appends flushed JSON lines to `costs.jsonl` in the report's
directory as it goes: a `control` line right after each file's control run (`path`, `seconds` as
used, `--control-seconds` included, `ran`, `failures`, `bound`, `shard`, and the run's id and
attempt from the CI environment, or null), an `uncovered` line for a file the map names no module
for (`seconds` 0), and a `mutant` line after each mutant (`path`, `name`, `seconds`, `outcome`).
`report.json` and the runner's standard output stay byte-identical to a run without the record.

R7. `mutation-verdict.py prices --costs DIR` reads every `costs.jsonl` under DIR and derives each
path's price as max(1, ceil(max(the largest control seconds, the mean mutant seconds))). It prints
one table line per path, sorted by path, each naming the run, attempt and shard of its figures, a
torn line (one that does not parse) counted and not read, then
`mutation: prices: examined <r> record(s) in <f> file(s); torn <t>`. Zero records reads VOID by
name and exits 1.

R8. Unchanged: `.github/workflows/ci.yml`, `.github/workflows/mutation-weekly.yml`, `LEG_CEILING`,
`PYTHON_MAX_SHARDS`, `PYTHON_SHARD_MUTANTS`, `PYTHON_WEEKLY_SHARDS`, `python_reports`,
`read_python_shard`, `shard_listing_drift`, `judge_python` and every per-mutant bound.

R9. `RELEASING.md` section 2 gains one paragraph after SPEC-415's: the Python legs are dealt by
each file's measured cost once it is priced, the table is filled only from
`mutation-verdict.py prices --costs <dir>` over a run's downloaded leg artifacts, an unpriced file
keeps the count rule, and a listing the run cannot hold is refused whole by name.

R10. Insert-only amendment sections, every earlier byte kept: SPEC-087 (each plan leg runs its
plan's names and records its costs), SPEC-362 (the Python legs' 1800-second bound and the joint
line beside the unchanged leg ceiling), ADR-373 (the same, as a decision amended), and section 6 of
the Python mutation schematic.

R11. `formal/tla/EveryLegCounted` covers the deal and the runner's leg selection (section 8).

R12. Rows S41800-S41814 in `scripts/mutation-rows.d/S41800-S41899.json` (section 9).

## 3. Acceptance criteria of SPEC-418

| id | criterion | decided by |
|---|---|---|
| A1 | a priced listing is dealt by cost into the fewest legs within the bound: a.py at 100 s x 30 and b.py at 10 s x 50 read 3 legs, [a 0-14], [a 15-29], [b], projected 1700, 1700 and 520 | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_priced_listing_is_dealt_by_cost_into_the_fewest_legs_within_the_bound` |
| A2 | a listing the deal cannot fit is refused whole by name: a file at 601 s (1803 > 1800), and c.py at 600 s x 33 (33 legs > 32) | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_the_cost_deal_cannot_fit_is_refused_whole_by_name` |
| A3 | a listing with no priced file keeps the count rule: None, and the shipped table is empty | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_with_no_priced_file_keeps_the_count_rule` |
| A4 | unpriced mutants keep 20 a leg beside the priced legs: 41 unpriced read 3 legs after them | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_unpriced_mutants_keep_twenty_a_leg_beside_the_priced_legs` |
| A5 | the plan deals by cost and refuses whole by name: its lines and exit 0 on a fit; exit 1, no step output and the plan's bytes unchanged on a refusal | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_deals_by_cost_and_refuses_whole_by_name` |
| A6 | a leg runs the names its plan deals it | `python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_leg_runs_the_names_its_plan_deals_it` |
| A7 | a plan whose python legs do not partition the listing, or whose count is not the leg's n, is refused before any mutant | `python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_plan_whose_python_legs_do_not_partition_the_listing_is_refused` |
| A8 | each file's control cost is recorded beside the report before its mutants, standard output unchanged | `python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants` |
| A9 | the prices verb derives each file's price from the recorded runs, torn lines counted, zero records VOID | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_prices_verb_derives_each_files_price_from_the_recorded_runs` |
| A10 | the Python bound is half of the Python legs' timeout | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_python_bound_is_half_of_the_python_legs_timeout` |
| A11 | the runbook names the Python cost deal | `python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_cost_deal` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_priced_listing_is_dealt_by_cost_into_the_fewest_legs_within_the_bound
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_the_cost_deal_cannot_fit_is_refused_whole_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_with_no_priced_file_keeps_the_count_rule
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_unpriced_mutants_keep_twenty_a_leg_beside_the_priced_legs
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_deals_by_cost_and_refuses_whole_by_name
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_leg_runs_the_names_its_plan_deals_it
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_plan_whose_python_legs_do_not_partition_the_listing_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_prices_verb_derives_each_files_price_from_the_recorded_runs
A10: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_python_bound_is_half_of_the_python_legs_timeout
A11: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_cost_deal
```

A10 is `TheReleaseFitsOneRun.test_the_python_bound_is_half_of_the_python_legs_timeout`, and its
reading of the verdict module is allowed by one census tuple in `scripts/tests/test_ci_workflows.py`
beside the two that already name `TheReleaseFitsOneRun`. The red commit carries stubs that keep the
base behaviour (`python_by_cost` returns None, `planned_leg` returns `k::n`, `prices` examines
nothing, `PYTHON_LEG_BOUND_SECONDS` is 3600), so each red is its criterion's assertion and never a
missing name. A3 pins the count rule the base already has and is recorded `not red`, with its
reason. R11 is decided by the formal check of `formal/tla/EveryLegCounted` at review, not by a
unittest, so it has no line in the fence.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | mutation scripts | changed: `PYTHON_LEG_BOUND_SECONDS`, `PYTHON_CONTROL_SECONDS`, `python_by_cost`, the cost path in `shards`, the `prices` verb |
| `scripts/mutation_python.py` | mutation scripts | changed: `planned_leg` in `select`, the `costs.jsonl` record |
| `scripts/tests/test_mutation_python_shard_binding.py` | tests | changed: A1 to A5 and A9 |
| `scripts/tests/test_mutation_python.py` | tests | changed: A6 to A8 |
| `scripts/tests/test_dispatch_shards.py` | tests | changed: A10 |
| `scripts/tests/test_ci_workflows.py` | tests | changed: one census tuple for A10 |
| `scripts/tests/test_rulesets.py` | tests | changed: A11 |
| `scripts/mutation-rows.d/S41800-S41899.json` | mutation rows | added: S41800-S41814 |
| `RELEASING.md` | runbook | changed: one paragraph in section 2 |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | formal | changed: two covers added, the `shards` cover re-stamped, comment lines only |
| `docs/specs/SPEC-418-the-python-mutation-legs-are-dealt-by-each-files-measured-cost-every-listed-mutant-examined-or-the-plan-refused-whole.md` | records | added: this SPEC |
| `docs/decisions/ADR-432-the-python-mutation-legs-are-dealt-by-each-files-measured-control-cost-under-the-runs-fixed-leg-shares.md` | records | added: its decision |
| `docs/red-first/SPEC-418.md` | records | added: the red-first record |
| `docs/specs/SPEC-087-the-guard-scripts-and-the-parity-oracle-prove-their-tests-kill-generated-mutants.md` | records | changed: an insert-only amendment section |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | records | changed: an insert-only amendment section |
| `docs/decisions/ADR-373-a-release-is-judged-in-one-run-of-round-robin-legs-sized-to-the-hosted-job.md` | records | changed: an insert-only amendment section |
| `docs/schematics/mutation-testing-python.md` | records | changed: section 6, insert-only |
| `changelog.d/python-cost-418.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- It prices no file: `PYTHON_CONTROL_SECONDS` ships empty, and a later delivery commits the first
  figures from a recorded run (#829, #825).
- It raises no job's timeout and sets no per-mutant bound (#825).
- It does not make the release's listing fit one run; that range is refused by name (#822, #825).
- It does not let the Python legs take the leg count the Rust plan leaves; the shares stay fixed
  (#825).
- It does not explain why four legs of the release run ended before their bound (#825).
- It does not bound the `mutation-rows` job (#825).
- It does not change the scheduled battery's Python job, which a pull request's run skips (#825).
- It does not shard the web sweep (#693).

## 6. Risks

- **A cut before a flush.** A leg cut between a mutant and its flush loses that one line. The
  record is flushed per line, so at most the line in flight is lost; no row holds the flush, since
  no structural test can kill its removal, and `prices` counts what it reads.
- **A control that is not a fair price.** If a file's mutants run well above its control, `prices`
  takes the dearer of the control maximum and the mutant mean; a file whose mutants time out at
  five times the control needs a larger factor, measured, not guessed.
- **A torn line.** A line cut mid-write does not parse; `prices` counts it as torn and never reads
  it, so a torn record cannot under-price a file.
- **A plan and a runner that disagree.** A runner whose listing differs from the plan's legs is
  refused before its first mutant (R5), and a refusing leg writes no report, which the verdict
  already reads VOID by name.
- **A stale price.** A table entry ages as its file's tests change; each entry names its run, and a
  later `prices` run replaces it.

## 7. What only CI proves

- The reds and greens of `test_dispatch_shards.py` and `test_ci_workflows.py`, which run only in
  CI; every red and green of this delivery is read there and recorded in `docs/red-first/SPEC-418.md`.
- That a cut leg's `costs.jsonl` reaches its artifact: the first run after this lands whose legs
  carry `--report` shows it.
- The cost line and the joint line, which print only after a delivery commits a price.

## 8. Formal model

`formal/tla/EveryLegCounted` already claims that every dealt leg is counted, each listed mutant is
examined exactly once, and a pass needs every leg reported whole, over `shards` and the verdict's
readers. A deal by cost is a new deal of legs under those claims, and each Python leg becomes a
reader of its plan's legs with a partition refusal. The entry gains two covers,
`scripts/mutation-verdict.py` anchor `python_by_cost` and `scripts/mutation_python.py` anchor
`planned_leg`, and the `shards` cover's digest is re-stamped. If the model's deal admits any
partition within the ceiling, only comment lines change; if it deals by index alone, the model is
re-read before any code lands.

## 9. Mutation rows

| row | behaviour | mutant | killer |
|---|---|---|---|
| S41800 | the Python bound is half the job | `PYTHON_LEG_BOUND_SECONDS` 1800 to 3600 | A10 |
| S41801 | a run pays its file's control and sentinel | `2 +` to `1 +` in a run's projection | A1 |
| S41802 | a one-mutant run past the bound refuses | `k < 1` to `k < 0` | A2 |
| S41803 | a run goes onto the lightest leg | lightest to heaviest leg | A1 |
| S41804 | more than 32 legs refuse | `> 32` to `> 33` | A2 |
| S41805 | unpriced mutants take 20 a leg | 20 to 40 in the unpriced legs | A4 |
| S41806 | no priced file keeps the count rule | the None return to a deal | A3 |
| S41807 | a cost refusal fails the plan | the refusal's exit to `EXIT_OK` | A5 |
| S41808 | a price takes the dearer reading | `max` to `min` in `prices` | A9 |
| S41809 | zero records read VOID | the zero-record exit to 0 | A9 |
| S41810 | a leg runs its plan's names | `planned_leg` back to `k::n` | A6 |
| S41811 | the partition check refuses | the partition check removed | A7 |
| S41812 | the count check refuses | the count check removed | A7 |
| S41813 | the control line is recorded | the control record dropped | A8 |
| S41814 | the runbook names the deal | the runbook phrase changed | A11 |

Each row's anchor is unique in its file and byte-identical to the line the mutant replaces; its
killer is the criterion's test, run by CI's `rows` job.
