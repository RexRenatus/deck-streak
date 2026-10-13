---
status: "accepted"
---

# ADR-435: the Python plan prices scoped legs from a dispatched price run, with each figure naming its run and job

Decides SPEC-421.

## Context

Every `path:line` below was read at dev `c726af67`.

**The plan prices a scoped leg as a whole one.** `scripts/mutation-verdict.py:135-143` holds
`PYTHON_CONTROL_SECONDS`: seven whole-suite prices from battery run 38242806596 (legs cut at 120 minutes).
`python_by_cost` (:1071-1135) lets a leg hold
`k = PYTHON_LEG_BOUND_SECONDS // price - 2` of a file's mutants beside its control and sentinel,
and `python_deal` (:1063) loads a run onto a leg at `(size if path in held[leg] else 2 + size) * price`.
Every mutant is priced at its file's whole-suite control.

**The runner already scopes.** Since SPEC-420, `run --plan` traces each file's sentinel and runs
each mutant against only the tests that execute its line. `scripts/mutation_python.py:1138` takes
that trace only `if getattr(self.args, "plan", None)`. Nothing carries the scoped cost back to the
plan, so the plan counts each leg as if every mutant ran the whole suite.

**What a scoped leg costs.** #836's run 38333401795 dealt `scripts/mutation-verdict.py` at 224 s
a mutant, six a leg. Its five priced legs of that file read:

| job | runner-step duration |
|---|---|
| 115088058737 | 456.07 s |
| 115088058742 | 879.88 s |
| 115088058864 | 884.44 s |
| 115088058658 | 713.44 s |
| 115088058757 | 803.78 s |

Each leg paid the whole control and the traced sentinel, about 224 s each, and then about 60 s a
scoped mutant (58 to 67 s across the legs).

**The release listing.** At dev `c726af67`, the plan's deal over the release's 6186 listed Python
mutants reads 447 priced legs and 139 unpriced legs: 586, against the share of 32.
`scripts/mutation-verdict.py` alone takes 439 of the 447 priced legs, six mutants a leg. Nine
listed files carry no price: `scripts/mutation_python.py`, `scripts/mutation_rows.py`,
`scripts/public-scrub.py`, `scripts/testflight_age.py`, `scripts/threat_model.py`,
`scripts/web-engine-size.py`, `scripts/web_mutation_shards.py`, `scripts/write-back-issues.py` and
`tools/parity-oracle/generate.py`.

**The runs that could measure a scoped price.** The weekly battery
(`.github/workflows/mutation-weekly.yml`) runs `run --all`: every mutant against the whole suite,
in path order, each leg cut at its 120-minute bound before the late files, and SPEC-420 R8 keeps
it whole. A pull request's own legs list only its diff, so a file the diff does not touch is never
measured. A reading taken off the runner class the legs use is not a price. The platform's
documentation reads that a dispatch needs the workflow on the default branch, while
`mutation-weekly.yml:17-21` records a dispatch at a feature ref that ran once the workflow had
been registered.

## Decision Drivers

- Every listed mutant still runs, and every shard is counted. No exemption and no skipped-mutant
  record.
- The share stays 32, and `PYTHON_SHARE_CAP` 128 refuses as now. `RUN_JOBS` 256, `LEG_CEILING`,
  the Rust ceiling 190 and the leg bound 1800 do not move.
- A price is a figure that a completed CI run recorded on the legs' runner class, filled by a
  program. It is never an offline reading and never typed.
- Every price line names the run, the attempt and the job its figures came from.

## Decisions

**D1. A dispatch-only price workflow measures every population file, scoped, on the legs' runner
class.** A new `.github/workflows/mutation-prices.yml` has two triggers. The first is
`workflow_dispatch`. The second is a `pull_request` into `dev` whose `paths` name the workflow
itself: a rehearsal that runs one leg over one small population file and registers the workflow,
so that a dispatch at the delivery's branch can run. Its permissions are read-only, every action is
pinned as `ci.yml` pins it, and every job runs on the runner class of `ci.yml`'s
`mutation-python` job. A population job emits the sixteen files of `scripts/mutation-python.json`
on a dispatch, and the one rehearsal file on the pull request event. A matrix job, one leg per
file, runs:

```
python3 scripts/mutation_python.py run --file "$FILE" --scoped --sample 20 --failfast --report "$out/report.json"
```

Each leg sets `MUTATION_JOB_ID: ${{ job.check_run_id }}` and `timeout-minutes: 120`, and uploads
its report directory as `mutation-prices-<job-index>` under `if: always()`. The workflow files
nothing and judges nothing. The run is dispatched once, by hand, at the delivery's branch after the
commit that adds the workflow. The delivery's later commits fill the table from that completed
run's kept records.

The runner gains two flags. `run --scoped` is accepted with `--file` alone; it traces each file's
sentinel and runs each mutant against its scoped set exactly as `run --plan` does. `run --sample S`
is accepted with `--file --scoped` alone; it runs every ceil(m/S)-th selected mutant in listing
order, starting with the first. Either flag beside `--plan` or `--all` is refused by name before
any mutant runs, so no verdict run samples and the battery stays whole.
`scripts/mutation_python.py` gains `SCOPED_SELECTIONS = frozenset({"plan"})` and `selection(args)`.
`prepare` traces when `selection(args) in SCOPED_SELECTIONS or args.scoped`. A traced sentinel
writes one record, `{"kind": "sentinel", "path", "seconds", **where}`, and `where` gains
`"job": os.environ.get("MUTATION_JOB_ID")`. A whole run writes no sentinel record. The mutant
records, `planned_leg`, `child_trace` and `scoped_sets` are unchanged.

**D2. The scoped prices are a second table of triples, printed by a verb from the run's records.**
`scripts/mutation-verdict.py` gains `PYTHON_SCOPED_SECONDS: dict[str, tuple[int, int, int]]`, after
the whole table. Each line reads:

```
    "<path>": (<control>, <sentinel>, <mutant>),  # run <r> attempt <a> job <j>: <n> scoped mutant record(s)
```

`mutation-verdict.py prices --scoped --costs DIR` prints those lines from every `costs.jsonl` under
DIR, one per file of `scripts/mutation-python.json` that its records price, in path order; a record
for any other path is examined and prints no line:
- the control figure is max(1, ceil(the largest control seconds));
- the sentinel figure is max(1, ceil(the largest traced sentinel seconds));
- the mutant figure is max(1, ceil(the mean scoped mutant seconds));
- a file that no test covers reads (1, 1, 1) from its `uncovered` record.

Then it prints one `mutation: prices: <path> unpriced: <reason>` for each population file it cannot
price, and last `examined N record(s) in F file(s); torn T`. It reads VOID, exits `EXIT_FAIL` and
prints no table line when:
- no record is found;
- a control, uncovered or sentinel record names no run, or no job;
- a mutant record's `why` is `all`, which is a whole run's record;
- one path's records span two jobs, or two attempts.

The records are committed as the fixture `scripts/tests/fixtures/mutation-scoped-prices-<run>/`, in
the download's layout with `costs.jsonl` files only, in a commit apart from the table. A test
re-derives the table from the fixture byte for byte. A refresh is a new dispatch read by the same
verb, with the fixture and the table replaced together in one delivery. The whole seven-file table
stays byte-identical, as the fallback.

**D3. The plan reads the scoped triple when the legs run scoped, and the whole price otherwise.**
`python_price(path, prices, scoped)` returns:
- `scoped[path]` when `scoped` is given and holds the path;
- else `(p, p, p)` when the whole table holds `p`;
- else None, and the file keeps the count rule, fail-closed as before.

`python_by_cost(listing, prices, scoped=None)` takes `setup = control + sentinel` and
`k = (PYTHON_LEG_BOUND_SECONDS - setup) // each`, which equals `B // p - 2` for a whole price. Its
refusal reads `{path}'s control is priced at {control} s ... projects {setup + each} s`, which is
byte-identical for a whole price. It sorts runs by `-(setup + len * each)`. `python_deal` runs carry
`(path, setup, each, indices)` and load a leg by `size * each + (0 if held else setup)`.
`python_unpriced_legs` gains the same `scoped=None` parameter. `shards` passes
`PYTHON_SCOPED_SECONDS if "plan" in mutation_python.SCOPED_SELECTIONS else None`, so the plan reads
the same value `prepare` reads. Only when a file is priced scoped, `shards` prints
`mutation: shards: python legs priced scoped: <s> file(s) at their scoped price, <w> at their whole-suite price, <u> unpriced`
after SPEC-418's `python legs dealt by cost` line. A two-argument call is byte-identical to the base's.
The partition and the count are unchanged: no mutant leaves the plan, and every shard is counted.

**D4. Every population file is priced by D1's run.** All sixteen files of
`scripts/mutation-python.json`, the nine unpriced among them, are measured by one dispatched run. A
file the run does not price stays unpriced, named by the verb's `unpriced` line. It keeps its
whole-suite price if it has one, else the count rule.

**D5. The release figure here is a projection, not a measurement.** Only `scripts/mutation-verdict.py`
has a scoped figure this ADR can cite: (224, 224, 60), from #836's legs above. At that triple the
release's listing projects 129 priced legs and 139 unpriced legs, 268 in all; at 58 s a mutant, 263;
at 67 s, 280. The plan still refuses by name above the share of 32. The figure that counts is the
plan's own line over the release after this delivery lands, with the filled table. The share, the
cap, `RUN_JOBS`, `LEG_CEILING`, the Rust ceiling and the bound are untouched; a need to change any
of them is outside this ADR.

**D6. Four pushes: the machinery, then the fill from a completed run.**
1. A red commit: the tests and stubs. The tests of every criterion that is red first fail on an
   assertion of its behaviour.
2. The green commit, with the workflow, and the rows commit. The workflow registers through its
   rehearsal. The price run is dispatched at this push's head.
3. The fixture commit alone. The table test is red on its assertion: the table, still empty, is
   not the verb's output.
4. The table commit, the table's rows, the red-first record and the changelog fragment.

**D7. One delivery under SPEC-421 and this ADR.** SPEC-418 is amended insert-only, because its R3
(b) and (d) and its R4 now read the triple. ADR-434 is amended insert-only, because its D2 rejected
a second, scoped figure until a scoped run was recorded, and its Consequences left that figure to
be decided; this ADR decides it. `docs/schematics/mutation-testing-python.md` gains section 8,
insert-only. ADR-432 and SPEC-420 need no amendment.

## What it was chosen against

### D1

- The weekly battery as the source: rejected because it runs `--all`, whole and path-ordered, and its legs are cut at 120 minutes before late files; SPEC-420 R8 keeps it whole.
- The weekly battery with a `package=python` input: rejected because it is still `run --all`, which measures whole prices, never scoped ones.
- A `package=prices` input on `mutation-weekly.yml`: rejected because its survivors job files issues on events that are not pull requests, so a price run would file survivor issues.
- The delivery's own pull request legs: rejected because they list only the diff, so the nine unpriced files are never reached.
- No dispatch, with the `pull_request` event running all sixteen price legs: rejected because every push that touches the workflow would spend sixteen legs, and the fill could not pin one completed run before its own commit.
- An offline reading of the scoped cost: rejected because a figure read off the legs' runner class is not a price.
- `--sample` allowed beside `--plan`: rejected because a verdict run must examine every listed mutant; a sample belongs only to a price run.

### D2

- Rewriting the whole table with scoped figures: rejected because the whole price stays the fallback for an unscoped run and for a file with no scoped figure.
- Deriving whole prices from scoped records: rejected because a scoped record does not measure a whole-suite run.
- One figure per file: rejected because it hides the traced sentinel and the setup a leg pays once per file.
- A typed figure: rejected because a hand figure has no record behind it, and nothing would catch the next one.
- Taking the job id from the jobs listing alone, off the record: rejected because a record that names its own job cannot be joined to the wrong leg.

### D3

- A separate scoped deal function: rejected because two deals would have to be kept in step, and the partition and count proofs would cover one of them.
- A command-line flag on `shards`: rejected because it is a second spelling that the legs' own command could disagree with.
- Reading `ci.yml` from the verb to learn whether the legs run scoped: rejected because the runner's own constant is what decides whether a leg traces.

### D4

- Pricing only the nine unpriced files: rejected because `scripts/mutation-verdict.py`, already priced whole, holds 439 of the release's 447 priced legs.
- Borrowing another file's figure for an unpriced file: rejected because a file's price is its own tests' cost, and ADR-434 D2 already refused the dearest figure.

### D5

- Raising the share or the cap on the projection: rejected because this delivery changes prices only, and a projection is not a measured figure.
- Reporting the projection as the release's figure: rejected because it replays the deal over one file's figures, and the count that decides is the plan's own line after the land.

### D6

- Three pushes, with the table test left `not red`: rejected because its only red would be a missing fixture, which is the wrong reason; the fixture commit alone makes it red on its assertion.

### D7

- Rewriting SPEC-418 R3 and R4 in place: rejected because a landed record keeps every byte; an insert-only amendment shows what changed.
- Two deliveries, one for the machinery and one for the fill: rejected because the first would land a verb and a table that nothing has filled, and the release window would be held twice.

## Consequences

Good:
- A scoped leg is priced at what it runs: its setup once, then each scoped mutant.
- Every price line names its run, attempt and job, and a test re-derives the table from committed
  records.
- An unscoped run and an unpriced file are priced exactly as before.

Bad:
- A sample of 20 prices a file by the mean of those mutants. A file whose unsampled mutants run
  slower is underpriced. The leg bound of 1800 s is half the job's 60-minute limit, which absorbs a
  figure up to twice too low.
- The fill waits on one dispatch by hand. If a dispatch at the branch is refused, the machinery
  lands with an empty scoped table, every deal stays byte-identical, and a later delivery fills the
  table from a run dispatched at `dev`.
- A refresh spends sixteen legs of up to 120 minutes each.
- The mutant figure includes each whole-suite fallback, so a file with many fallbacks reads dearer.
  That is the honest cost.

## Confirmation

SPEC-421's acceptance criteria A1-A16, each read red then green in CI, or recorded `not red` with
its reason. The rows in `scripts/mutation-rows.d/S42100-S42199.json`. The formal entry SPEC-421
section 8 names. The plan's `python legs priced scoped` line over the release's listing after the
land.

## What would make this wrong

- A dispatch at the delivery's branch is refused even after the rehearsal registered the workflow.
  Then the fallback above applies.
- A file's sampled mutants are not representative of its others. A leg then runs past its
  `projected_seconds`, which the plan prints per shard.
- `ci.yml`'s `mutation-python` runner class changes. A test holds the price workflow's runner class
  equal to it, so the change is red until both move, and the table is re-measured.
- A scoped kill differs from the whole suite's under an order-dependent test. The battery stays the
  reference, as ADR-434 records.

## More Information

SPEC-421; SPEC-420 and ADR-434 (the share, the measured prices and the scoped set; ADR-434 amended
insert-only); SPEC-418 and ADR-432 (the cost deal; SPEC-418 amended insert-only). Issue #837 is
this delivery's; #825 tracks the Python legs' fit; #822 is the release.
