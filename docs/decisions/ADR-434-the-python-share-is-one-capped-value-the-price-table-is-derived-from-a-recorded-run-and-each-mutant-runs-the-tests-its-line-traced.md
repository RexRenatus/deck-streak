---
status: "accepted"
---

# ADR-434: the Python share is one capped value, the price table is derived from a recorded run, and each mutant runs the tests its line's trace names

Decides SPEC-420.

## Context

Every `path:line` below was read at dev `e64a778e`.

The pull request run's Python share is already a name, `PYTHON_MAX_SHARDS`, but it shares
`scripts/mutation-verdict.py:122` with the count rule's 20 (`PYTHON_SHARD_MUTANTS, PYTHON_MAX_SHARDS
= 20, 32`). The code reads it at call time in six places (lines 1007, 1086, 1091, 1146, 1202 and
1221-1228). Its number is spelled outside the code in three places: `RELEASING.md:66-67`, whose
phrases `scripts/tests/test_rulesets.py:165-166` pins; and
`scripts/tests/test_mutation_python_shard_binding.py:444-455`, which pins
`python legs 2 of ceiling 32` and `run legs 3 of 225: python 2 of 32, rust 1 of 193; projected 93
runner-minute(s) of 35700`, and `:670-677`, which pins `python legs 3 of ceiling 32` and
`run legs 4 of 225: python 3 of 32, rust 1 of 193; projected 105 runner-minute(s) of 35700`.
`.github/workflows/ci.yml`'s `mutation-python` job (line 543) never
spells it, because its matrix is the plan's output. Nothing caps the share.
`test_dispatch_shards.py`'s `TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget` derives
`LEG_CEILING` as the run's 256 jobs less every other job, counting the Python matrix at
`PYTHON_MAX_SHARDS`, and asserts that it equals `{"ci": 193, "battery": 234}` (line 887).

The price table `PYTHON_CONTROL_SECONDS` (line 129) ships empty. Battery run 38242806596 ran each
of its sixteen Python legs to the job's 120-minute bound: two failed, fourteen were cancelled, and
legs 0, 5, 6 and 14 kept no cost records. Its other twelve legs kept 1017 records, and over them
`mutation-verdict.py prices` prints seven figures: `mutation-verdict.py` 224 s, `memory_scope.py`
39, `ios_lane.py` 13, `audit-web-verdict.py` 3, `dynamic-imports-check.py` 2, `ios_icon.py` 2 and
`create-issues.py` 1. The battery takes files in path order, so `mutation-verdict.py` fills every
leg it starts, and no leg reached the eight files after it. The release listing (main...dev) holds
5670 mutants in 14 files. Six of those files are priced (3398 mutants), and eight (2272) are not:
`mutation_rows.py` 1018, `mutation_python.py` 599, `testflight_age.py` 236, `generate.py` 143,
`threat_model.py` 116, `public-scrub.py` 104, `web-engine-size.py` 53 and `write-back-issues.py` 3.

Each mutant runs every test module its file maps to in `scripts/mutation-python.json`, less the
byte readers (`scripts/mutation_python.py`, `Judge.file`). `mutation-verdict.py` maps to six
modules. Its 224 s per mutant is spread over the five that can run outside CI: 38.3, 24.7, 20.3,
16.5 and 0.2 per cent. No single test holds it: the fifteen slowest tests hold 44.8 per cent.

The Python legs have no control mutant. A leg that read every mutant killed, or none, would pass
unseen.

## Decision Drivers

- A price is a measurement with its source, never a guess and never a hand figure (ADR-432's
  driver, kept).
- The share is the owner's value. It is signed as one line and changes nothing else.
- A scoped run never reports a verdict the whole suite would not.
- Every listed mutant is examined, the whole-plan refusal by name above the share stays, legs keep
  their 1800 s, and the Rust share is unchanged.

## Decisions

**D1. The share is one named value, capped at 128 by a refusal and by a pin.**
`scripts/mutation-verdict.py` spells the share once, on its own line: `PYTHON_MAX_SHARDS = 32`. The
count rule's 20 moves to a line of its own, `PYTHON_SHARD_MUTANTS = 20`. Beside them, a new
constant `PYTHON_SHARE_CAP = 128` is the ceiling no signature may pass. Before any deal, `main`'s
`shards` arm calls a new `python_share_refused()`. A share that is not a whole number from 1 to the
cap prints:

`mutation: shards: REFUSED: the python share PYTHON_MAX_SHARDS = <n> is outside 1..<cap> (PYTHON_SHARE_CAP); a share above the cap is never dealt`

It then returns `EXIT_FAIL`, the exit the whole-plan refusal already uses (lines 1137-1148). The
refusal sits in `main`, outside the `shards` and `python_by_cost` spans the formal model covers. The
whole-plan refusal by name above the share (lines 1007, 1086 and 1146) is unchanged.

A test pins `PYTHON_SHARE_CAP == 128`, and pins the share as a multiple of 8 between 8 and the cap.
It never pins the share's number.

Every other spelling reads the one value:
1. The plan's printed lines already read it at call time.
2. The two shard-binding tests that pin those lines patch the share and the Rust ceiling to
   literals of their own.
3. The release runbook names `PYTHON_MAX_SHARDS` instead of 32, and the runbook test's phrases
   change to match.
4. `ci.yml`'s Python matrix is `fromJSON(needs.mutation-plan.outputs.python_matrix)`.

The owner's signature is then this one line, and nothing else, with N a multiple of 8 no greater
than 128:

```diff
-PYTHON_MAX_SHARDS = 32
+PYTHON_MAX_SHARDS = <N>
```

That line is not yet green at any N above 32. At N = 40 the job-budget test derives a Rust ceiling
of 185 against its pinned 193, and goes red. It stays green only if the Rust share falls by N - 32,
or if the 256-job limit is read per matrix rather than per run. This ADR decides neither: both are
the owner's (#825).

**D2. The price table is the `prices` verb's output over one recorded run, committed with the
records it reads.** `PYTHON_CONTROL_SECONDS` takes exactly the seven lines that `mutation-verdict.py
prices --costs <dir>` prints over the twelve `costs.jsonl` files of battery run 38242806596 (its
legs ran to their 120-minute bound). The verb reads `examined 1017 record(s) in 12 file(s); torn 0`.
The records are committed as a fixture beside the shard-binding tests. A test runs the verb over
the fixture and compares its output with the table's lines byte for byte, so a hand edit of any
figure turns it red. The figures are lower bounds, disclosed in ADR-432's amendment.

A listed file the table does not price keeps SPEC-415's count rule of 20 mutants a leg, inside the
share (ADR-432 D2, unchanged). It is never free: each of its legs counts against the share and is
bounded at 1800 s, and a leg that overruns its job reads VOID by name, never a pass. That rule
covers the eight release files that no recorded leg reached, the four absent legs' files, and any
file the population gains later, the web sweep's new script included.

Filling the table raises the release's legs from 284 under the count rule to 557. A priced file's
224 s mutants now go six to a leg instead of twenty, which is the fail-closed direction.

The runner records every file's control and sentinel before any mutant of its leg runs (control
first), so a later run whose legs are cut at their bound still leaves a control record for every
file it holds. A later delivery refreshes the table by running the verb over that run's downloaded
records, and commits the output and the records together in one commit; the same test re-derives
the table.

**D3. Each mutant runs the tests its line's trace names, and every gap runs the whole suite.** In a
`run --plan` leg, the runner traces each file's sentinel run. The sentinel already runs once per file
per leg, over the unmutated lines with one comment appended. Tracing it therefore adds no run and
never touches the control, whose time is the file's price. The trace is taken inside the leg, per
run, on the tree the leg judges.

How the trace is taken:
1. **In the runner's own process.** The copied test runner registers a `sys.monitoring` tool for
   `LINE` events. The first time a test executes a line of the target file, the tool records it and
   returns `sys.monitoring.DISABLE`. It calls `sys.monitoring.restart_events()` at each test's start,
   so each test's set of executed lines is its own.
2. **In child processes.** The runner writes a `sitecustomize.py` to a scratch directory at the front
   of the children's `PYTHONPATH`, never the runner's copy. It installs the same tracer and an audit
   hook, writes one record per process when that process exits, and then runs any `sitecustomize`
   it shadows.
3. **Counting processes.** Each process counts the processes it starts, from the interpreter's
   process-creation audit events: `subprocess.Popen`, `os.system`, `os.posix_spawn`, `os.spawn`,
   `os.exec`, `os.fork` and `os.forkpty`.

A mutant's scoped set is every test any of whose processes executed its line. The mutant runs the
whole suite instead when:
1. its name says `in <module>`;
2. its line ran at import, or outside any test: a module or class body, or a class or module
   fixture;
3. no traced test executed the line;
4. the file's trace failed or is partial: no `sys.monitoring`, a tool id already taken, a record
   missing or unreadable, or a trace run whose test count differs from the suite's;
5. its scoped run ran fewer tests than its set names.

A test joins every scoped set of its file when any of these holds:
1. it started a process that left no record, which shows when a process names fewer child records
   as their parent than the number of processes it started;
2. it replaced its image with `os.exec`;
3. it left a thread running past its end.

A scoped run that passes is run again on the whole suite before the mutant reads SURVIVED. The
scheduled battery (`run --all`) keeps the whole suite for every mutant. Byte readers stay out of
both the scoped and the whole runs, as today.

**Soundness.** A mutant changes the tokens of one line. Take a test none of whose processes
executes that line, which ran nothing at import and started no unrecorded process. It runs the same
code under the mutant as under the control, so it passes again, and dropping it changes no outcome.

The trace finds every process that executes the line: the runner's own, and every child. A child
that left no record is always caught by its nearest recorded ancestor's count, whatever order the
processes exit and write in, and the root's record is always present.

Because a scoped survivor is re-run whole, a wrongly dropped test can only cost a whole run. It
cannot change a verdict.

One divergence is left: a scoped kill the whole suite would not make. It needs a test whose outcome
depends on which tests ran before it. A local measurement sampled 55 mutants across every listed
file it could run, and all 55 read the same outcome scoped and whole.

The sample could not reach:
1. three files, `audit-web-verdict.py`, `memory_scope.py` and `write-back-issues.py`, whose tests do
   not run outside CI;
2. 1030 mutants on lines that no locally traced test executed: 736 in `mutation_rows.py`, whose test
   module is red outside CI, 292 in `mutation-verdict.py` and 2 in `generate.py`;
3. five held modules: `test_audit_web`, `test_bin_kind_census`, `test_rail_contract`,
   `test_deploy_templates` and `test_memory_scope`;
4. two modules that are red outside CI: `test_mutation_equivalent` and `test_mutation_rows`.

Every one of those lines runs the whole suite under the fallbacks above.

The build proves that it drops no executing test in three ways:
1. one structural test per rule, each over a fixture whose executing test is known by construction:
   directly, in a child, in a grandchild started through a shell, in a thread, and at import;
2. a mutation row that kills each rule's removal;
3. the leg's control (D5), whose second mutant only a child process executes.

**D4. The release's Python legs read 557 at landing, and the plan refuses them by name at 32 and at
every value up to 128.** With the table filled (D2) and the deal unchanged, the release listing
prices at 443 priced legs and 114 unpriced legs: 557, which rounds to 560. Under the count rule alone
it read 284.

Scoping (D3) shortens each leg's run but not the plan's count, because the table holds one figure
per file, the control's. A second, scoped figure from a recorded run would bring the count to about
301 (304 rounded). That projection replays the deal over the local measurement's scoped prices,
with a whole control and sentinel in every leg. The measurement's own count, without them, read 274
(280 rounded). The web sweep's new script adds one leg for every 20 of its mutants, and it is
not listed yet.

Every figure is above 128. So the plan refuses the release by name at the in-tree 32, and would at
any signed value up to 128. The build still lands green on dev, because its run plans only its own
listing.

**D5. Every Python leg runs a known control first, read by name.** A fixture under
`scripts/tests/fixtures/mutation-python-control/` holds a target and its checks, named so that
discovery never collects them. Before its first file, a leg runs three named mutants of that
target:
1. one on a line that a test executes in-process, which must read KILLED;
2. one on a line that only a child process executes, which must read KILLED;
3. one on a line that a test executes but nothing observes, which must read SURVIVED.

Any other outcome writes `control: <mutant> read <outcome>, expected <outcome>` as the `void` of
every file entry in the leg's report. The verdict already reads that `void` by name, so
`mutation-verdict.py`'s readers do not change. The control writes no cost record, so it never
prices a fixture.

A root without the fixture records `control: absent`; the runner's own tests use such roots. A test
pins that the repository holds the fixture and its three mutants.

**D6. Red first per criterion, structural, read in CI, in two pushes.** Each acceptance criterion's
test is written before its code. It runs against stubs that keep every input but the missing
behaviour, and it fails on an assertion of that behaviour.

No test asserts a wall-clock time: costs and bounds are fixture records, and order is read from the
sequence of the records. No test module runs outside CI:
1. `test_dispatch_shards.py` is restricted;
2. `test_ci_workflows.py` runs only under its own rules;
3. the `NOT_WORKFLOW_READS` census in `test_ci_workflows.py` lists each new read the tests add.

The pushes:
1. The red commit is pushed alone with its draft pull request, and its reds are read in that run.
2. The green and the record commits are the second push, with the record in the third commit
   (ADR-429 D4).

The band is `S42000-S42099`. SPEC-420 section 8 decides the formal surface: the covered spans are
unchanged, and the trace records are a new surface with a model of their own.

**D7. One delivery under SPEC-420 and ADR-434, with ADR-432 amended insert-only.** The share, the
table, the scope and the control land together under #829. The release re-runs once, after both
narrow fixes land, and the table's first fill and the scope share the plan's tests. ADR-432 gains
one insert-only amendment for the table's source, its lower bounds and its refresh.
`docs/schematics/mutation-testing-python.md` gains section 7, insert-only.

## What it was chosen against

### D1

- The cap as a test pin alone: rejected because the plan job deals its legs before any test's
  verdict is read, so a share above the cap would start its legs; the refusal stops it at the plan.
- The cap as the refusal alone: rejected because a diff that deletes the refusal leaves nothing red
  before a plan runs; the pin and its mutation rows catch that diff.
- A second spelling of the share in `ci.yml`, a fixed matrix width or a `max-parallel`: rejected because
  the matrix is already the plan's output, and a second spelling is a second value to sign.
- A test that pins the share's number: rejected because the signature would then change two lines,
  the value and its pin.
- Raising the share in this delivery: rejected because a value above today's is the owner's alone,
  and the job-budget test goes red at any value above 32 unless the Rust share falls.

### D2

- Typing the seven figures from the verb's printout: rejected because a typed figure is a hand
  figure, and nothing would catch the next one.
- Pricing an unpriced file at the table's dearest figure, 224 s: rejected because it reads the
  release at 825 legs and prices a 1 s script as dear as the 224 s one.
- Refusing any listing that names an unpriced file: rejected because a new script could then never
  run the first leg that prices it.
- Re-running the four absent legs for their records: rejected because that rerun is refused for now,
  and a path-ordered battery leg cut at its bound can stop before a file's mutants again.
- A second, scoped figure per mutant now: rejected because no scoped run has been recorded yet, and
  a projected figure is a hand figure.

### D3

- A coverage map committed to the tree: rejected because it is stale after the next edit of a test
  or of the code, and a stale map drops tests that now execute a line.
- Tracing the control run: rejected because the control's time is the file's price, and a tracer
  inside it moves the price.
- A coverage library in every leg: rejected because it adds a dependency to each leg's environment,
  and its own tracer would hold the monitoring tool the runner needs.
- Keeping the module mapping alone: rejected because that is the 224 s per mutant, spread over five
  modules, that every mutant of the dearest file pays today.
- Skipping a mutant that no traced test reaches: rejected because a skipped mutant is unexamined,
  and the plan counts every listed mutant examined.
- Scoping the scheduled battery too: rejected because the battery is the whole-suite reference that
  shows a scoped kill the whole suite does not make.

### D4

- Writing a share that fits the release: rejected because no value up to the cap fits 557 or 304,
  and any value above today's is the owner's.
- Planning from the projection instead of the table: rejected because the plan prices from the
  table, and a projection is not a recorded run.

### D5

- Naming the leg's control NOT MEASURED: rejected because a leg that read every mutant killed, or
  none, would then pass unseen.
- A killed control alone: rejected because a runner that reads every mutant killed passes it; the
  survivor arm catches that runner.
- A control only in the scheduled battery: rejected because the pull request legs are the ones a
  delivery's verdict reads.

### D6

- Reading the reds outside CI: rejected because two of the modules may not run there, and a third
  runs only under its own rules.
- One push: rejected because the reds must be read at the commit that holds them, before the code
  that turns them green exists.
- A timing assertion for the scoped speed-up: rejected because a timing test measures the machine,
  not the scope; the scoped set is asserted as a set.

### D7

- Two deliveries, the table and then the scope: rejected because the release re-runs once, after
  both, and a second delivery would hold dev's narrow window twice.
- Rewriting ADR-432 D2's paragraph on unpriced files: rejected because an accepted ADR is amended
  insert-only.

## Consequences

Good:
- The share is signed as one line, and a value above 128 never deals a leg.
- The table is measured, and it is re-derived on every test run.
- A mutant's run carries only the tests that execute its line, and every gap runs whole.
- Each leg proves that it can kill and can fail to kill.

Bad:
- The release's plan rises to 557 legs and stays refused until the owner settles the share and the
  run's job budget (#825).
- Each leg pays its control's few seconds.
- The plan does not count the scoped speed-up until a scoped run is recorded and a second figure is
  decided.
- A scoped kill can differ from the whole suite's under an order-dependent test. The battery is the
  reference.

## Confirmation

SPEC-420's acceptance criteria A1-A18, each read red then green in CI. The rows in
`scripts/mutation-rows.d/S42000-S42099.json`. The formal entry SPEC-420 section 8 names. The plan
line of the delivery's own run.

## What would make this wrong

- The 256-job limit holds per matrix, not per run. Then the job-budget test's derivation and
  ADR-432 D2's fixed shares are read again, and the share can rise with the Rust share unchanged.
- A process started by a route the audit events do not see, such as a native extension calling
  `fork`. If it executes the line and leaves no record, no count sees it. That is the one gap in the
  balance rule, and the battery's whole-suite runs are its reference.
- A test that depends on order kills a mutant scoped that the whole suite does not. The battery shows
  that mutant as a survivor.
- The interpreter CI runs has no `sys.monitoring`. Every file then runs whole: slower, never wrong.

## More Information

SPEC-420; ADR-432 and SPEC-418 (the cost deal); ADR-429 and SPEC-415 (the count rule and its
refusal); SPEC-362 R3 (the run's job budget). Issue #829 is this delivery's; #825 tracks the Python
legs' fit; #822 is the release.

## Amendment: the second, scoped figure is decided (SPEC-421, ADR-435)

Insert-only; every earlier byte is kept. D2 above rejected "a second, scoped figure per mutant
now", because no scoped run had been recorded and a projected figure is a hand figure. The
Consequences above left the scoped speed-up uncounted until a scoped run is recorded and a second
figure is decided. ADR-435 decides that figure:
- a dispatched price run, `.github/workflows/mutation-prices.yml`, measures it on the runner class
  of the pull request's Python legs; it is never projected;
- it is a triple per file, (control, traced sentinel, mean scoped mutant), in
  `PYTHON_SCOPED_SECONDS`, which `prices --scoped` prints from the run's committed records, each
  line naming its run, attempt and job;
- the plan reads it only when its legs run scoped, and `PYTHON_CONTROL_SECONDS` stays
  byte-identical as the fallback.

D2's reason is kept: ADR-435 fills the table only from a completed run's records, never from a
projection. The figure of about 301 legs above replayed a local measurement's scoped prices.
ADR-435 D5's projection of 268 replays one file's recorded legs. Neither is the release's figure;
that is the plan's own line after the land. D1 to D7 and the rest of this ADR are unchanged.
