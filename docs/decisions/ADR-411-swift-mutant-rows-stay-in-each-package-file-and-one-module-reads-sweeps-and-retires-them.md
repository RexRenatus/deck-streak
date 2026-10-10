---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Swift mutant rows stay in each package's row file, and one module reads, sweeps and retires them in the package's own macOS job

## Context and Problem Statement

#650 asks which job runs the Swift mutants, what form the battery takes, and where its verdict is
read and recorded. SPEC-339 left its Swift mutants out of the shared row population and cited #650
for it in its exclusions. Read at DeckStreak `dev` `164ac206`:

- Two Swift packages carry hand-written mutant rows: `ios/HarnessWire/swift-mutants.json` (29
  rows, 6 distinct killers) and `ios/CardIsolation/swift-mutants.json` (38 rows, 8 distinct
  killers). Each file is `{population, mutants}`, and each row has exactly the keys `id`, `file`,
  `find`, `replace`, `killer` and `why`. An id is `SW` and five digits, the first three the SPEC
  that added the row (SW339xx to SW348xx in the codec, SW349xx, SW355xx and SW361xx in the card
  view's isolation).
- Each file is swept by an inline Python program pasted into one job of
  `.github/workflows/xcframework.yml`: `harness-wire` (`:193-258`) and `card-isolation`
  (`:282-347`). The two programs differ only in their package path. Each runs a killer with
  `swift test --filter` and no per-run bound (`:208-210`), and writes `sweep.md` and the step
  summary (`:254-256`).
- Nothing on Linux reads the rows. An anchor that drifts, a killer renamed or a row deleted is
  found only when the macOS jobs run, and `.github/workflows/apple-on-change.yml` runs them only
  when one of its paths changes (`:11-19`).
- The shared row battery, `scripts/mutation_rows.py`, reads band files whose name and ids must
  match `S[0-9]+` (`:73-75`), and its three tables name Rust and Python killers (`:79-89`). Its
  `retired` check (`:1228-1259`) refuses a row that left while its target stays unless
  `scripts/mutation-rows.retired.json` records a reason and an approval. No check of that kind
  reads the Swift rows.
- ADR-350 D8 chose hand-written codec mutants for the spike, rejected a Swift mutation tool and a
  Swift killer kind in the shared rows for the spike, and left the permanent form to #650.
- The card view's link work (#664, #677) claims SW39200-SW39204 in
  `ios/CardIsolation/swift-mutants.json`, in the same keys.

## Decision Drivers

- A row is evidence only while its anchor occurs once, its killer exists, and it has not left
  unseen. Each of these is checkable without a compiler.
- Swift compiles and its package tests run only on the hosted macOS runner, which runs only for a
  change to one of the change caller's paths.
- One format has one reader. Two pasted copies of one reader drift, and nothing tests either.
- The shared row battery serves every Rust and Python row on every pull request, so a change to
  its reader is a change to all of them.
- The ids already cited in red-first records (SPEC-349, SPEC-355, SPEC-361) and the ids the card
  view's link work claims keep their meaning.

## Considered Options (the alternatives it was chosen against)

### D1. The population: which Swift a mutant may touch, and which tests can kill one

- Host-tested package sources — chosen, because a mutant there is built and judged by the
  package's own job in one incremental test run. Each mutant is killed by one test method of
  its package, and these packages hold the Swift that carries invariants: the wire codec
  (`ios/HarnessWire/Sources/HarnessWire/`, 2 files, killed under
  `ios/HarnessWire/Tests/HarnessWireTests/`) and the card view's isolation
  (`ios/CardIsolation/Sources/CardIsolation/`, 9 files, killed under
  `ios/CardIsolation/Tests/CardIsolationTests/`).
- Every `.swift` under `ios/` — rejected, because the app, harness and probe-host targets' tests
  run only through the simulator build of the `harness` job (`xcframework.yml:356-359`), so each
  mutant would pay a simulator rebuild, and the thin-Swift census (`ios/swift-roles.json`,
  `scripts/tests/test_ios_thin_swift.py:35-58`) already holds the app's Swift to per-role decision
  ceilings (#625).
- The wire codec alone — rejected, because the card view's isolation package already carries 38
  rows proved the same way, and leaving them out keeps a second, untested reader.

### D2. The battery's form, the id form, and where the rows live

- Hand-written rows in each package file — chosen, because it is the form the Rust and Python rows
  already take (an anchor that occurs once, a replacement, one killer). The rows stay in
  `ios/<package>/swift-mutants.json` with their keys unchanged, and one standard-library-only
  module, `scripts/swift_mutants.py`, is their one reader, with the verbs `census`, `retired` and
  `sweep`: all 67 ids and the five the link work claims hold, and one tested module replaces two
  pasted programs.
- The id form `SW<NNN><nn>` — chosen, because it is the form every row already has, so no red-first
  record is rewritten. `SW<NNN>00`-`SW<NNN>99` is SPEC-NNN's band across every package file, an id
  is unique across all of them, and SW39200-SW39204 hold unchanged.
- A `SWIFT_MUTATIONS` table in the shared band files — rejected, because a band refuses an id that
  does not match `S[0-9]+` (`mutation_rows.py:74`), so all 67 cited ids would be renumbered, and
  the reader, the weekly `prove --all` battery and the `mutation-verdict` job would each learn a
  killer only the macOS runner can run; ADR-350 D8 rejected it for the spike for that reason.
- A Swift mutation tool — rejected, because a generator rebuilds per mutant across the generated
  project, adds an unpinned third-party binary to the macOS runner, and the house proves a
  language with no generator in its gate by hand rows.
- A scripted whole-file swap — rejected, because a mutated copy of each source file kept beside it
  drifts from its source silently and carries no anchor a census can check.
- Keeping the two inline programs — rejected, because they are two copies of one reader that no
  test runs, while a module under `scripts/` is tested on Linux on every pull request and joins
  the Python mutant generator's population (`scripts/mutation-python.json`).
- Importing the shared reader — rejected, because if the module imported `scripts/mutation_rows.py`
  for its retired logic, a change to the shared reader would change the macOS sweep without
  starting the macOS jobs, whose trigger lists paths; the module imports only the standard library
  and its own path joins the trigger.
- A second approvals file beside each package — rejected, because one act, retiring a row whose
  target stays, would then have two records; the Swift `retired` check reads
  `scripts/mutation-rows.retired.json`, whose reader keys entries by id and checks no id form
  (`mutation_rows.py:1236-1240`).

### D3. Which job runs it, its bound, where the verdict is read, and the check that holds the rows

- Each package's own job — chosen, because the package is already built there, and its label is
  the one `ADMITTED_RUNNERS` admits (`scripts/tests/test_ci_workflows.py:44`, held by
  `test_one_job_body_serves_the_change_and_the_tag` at `:912`), so no larger runner is needed.
  `harness-wire` and `card-isolation` in `xcframework.yml` each run `python3
  scripts/swift_mutants.py sweep --package ios/<package> --report "$REPORT" --run-seconds <n>`
  after their test step, on the admitted hosted macOS label.
- A separate `swift-mutants` job — rejected, because each leg of a job matrixed over the packages
  would rebuild from a cold checkout the package its own job has just built, a second macOS job
  per change.
- The `harness` simulator job — rejected, because its bound already holds the card view's planted
  simulator suite (`HARNESS_TIMEOUT_MINUTES`), and the sweep would wait behind simulator steps it
  does not need.
- A Swift toolchain on the Linux row runners — rejected, because the isolation package imports the
  platform's web view framework
  (`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:2`), which no Linux toolchain
  carries.
- The bound from the measured cost — chosen, because it is the web legs' rule
  (`WEB_MUTATION_TIMEOUT_MINUTES`, `test_ci_workflows.py:2782-2788`) sized from this job's own
  measurement, and a hung killer becomes one named VOID row instead of a cancelled job with no
  report. Each sweeping job's `timeout-minutes` lies between one and a half times and twice its
  projected run, each end rounded up to the five, where the projection is the job's measured
  minutes outside the sweep plus the live run count (rows and distinct killers) times the measured
  seconds per run. Each killer run is bounded at three times the measured seconds per run, rounded
  up to the minute, and a run past it is VOID by name with its whole process group ended.
- The web legs' ceiling — rejected, because twice the projection rounded down to the ten falls
  below the floor for a projection under fifteen minutes and leaves no admissible bound.
- No per-run bound — rejected, because as the inline programs have it (`xcframework.yml:208-210`)
  one hung killer spends the whole job's bound, and the report names no row.
- The module's last line as the verdict — chosen, because SPEC-355 already records its sweep the
  same way (`docs/red-first/SPEC-355.md:66-70`), and the line names the package, the population
  and every outcome. The line is `swift-mutants ios/<package>: examined N row(s): K killed, S
  survived, V void`, read by name in the job log beside `sweep.md` in the job's report artifact
  and the same table in the step summary; a delivery that adds or changes Swift rows records that
  line, with its run and job ids, in its own red-first record.
- SPEC-057's campaign table — rejected, because it counts a generator's listed mutants per crate,
  and a hand-written Swift row is no listed mutant.
- Feeding each sweep's report to the `mutation-verdict` job — rejected, because it adds a protocol
  between jobs on different runners and different triggers, which the leg counting the verdict
  relies on (`formal/tla/EveryLegCounted`) does not model, for a verdict the job's own exit gives.
- A Linux test over the rows — chosen, because an anchor, a killer and an id are checked without a
  compiler, so a drift is caught on the pull request that causes it.
  `scripts/tests/test_swift_mutants.py`, run by the Linux python stage on every pull request, runs
  `census` over the live tree and refuses a planted defect of each kind by name; the workflow
  criteria live in `scripts/tests/test_ci_workflows.py` and
  `scripts/tests/test_mutation_workflows.py`.
- The Swift `retired` check on the existing retired step — chosen, because that step of the
  `mutation-rows` job (`.github/workflows/ci.yml:630-632`) already runs on every diff with the
  base at `HEAD^1`, and writing it on the same one `run:` line beside the row battery's keeps
  every `ci.yml` line the schematics cite where it is.
- A new step for the Swift `retired` check — rejected, because it moves every later `ci.yml`
  line, and `docs/schematics/mutation-testing.md` cites `ci.yml` by line.
- The red commit pushed alone — chosen, because each red is then read in CI by its test's name, the
  same red the gate sees, and the workflow tests and the macOS sweep are read in CI.

### D4. FORMAL, decided by surface

- Not applicable — chosen, because no surface is interleaved and no pure invariant is left to a
  proof. Each sweep is one process in one job on its own runner and checkout, applying one mutant,
  running one killer and restoring the file in sequence, with no state another actor reads or
  writes; the `retired` check only reads git objects; no report passes between jobs; and the one
  total function, the verdict, ranges over finite classes (each count is zero, one or more, each
  exit zero or not) that SPEC-397 A6 enumerates whole against an oracle the test writes.
- A TLA+ model of the sweep — rejected, because a model needs two actors over shared state, and
  the sweep has one.
- A Lean proof of the verdict — rejected, because it would prove a port of the Python, while A6
  decides the Python itself over every class of its domain.
- A Lean proof that the bound's band is never empty — rejected, because the band is computed by a
  test from constants and live counts, not by product code, and rounding both ends up keeps one
  and a half times the projection at or below twice it.

### D5. Drift, and the push count

- Exactly two pushes — chosen, because each red is read in CI by name before the fix exists: the
  red commit alone, then the fix and record commits. The second push is also the first sweep
  through the module, which the change caller starts because the workflow and the module's path
  both change.
- One push — rejected, because a red nobody has seen fail proves nothing.
- A third push to record the module's first sweep — rejected, because that verdict is read by
  name from the second push's run; the record names the push that carries it.

## Decision Outcome

- **D1:** the Swift population is the sources of the two host-tested packages, killed by their
  own test methods; chosen against every `.swift` under `ios/` and against the codec alone.
- **D2:** hand-written rows in each package's `swift-mutants.json`, keys and `SW` ids unchanged,
  read by one standard-library module; chosen against a shared-band table, a generator, a
  whole-file swap, the two pasted programs, an import of the shared reader and a second approvals
  file. SPEC-397's own Swift band, SW39700-SW39799, is claimed and unused; its own rows live in
  `scripts/mutation-rows.d/S39700-S39799.json`, from S39700.
- **D3:** each package's own macOS job sweeps it, bounded from its measured cost, and its last
  line is the verdict a delivery records; the Linux python stage holds the rows on every pull
  request, and the row battery's retired step holds their departures; chosen against a new job,
  the simulator job, a Linux toolchain, the web ceiling, no per-run bound, the campaign table, the
  verdict job and a new step.
- **D4:** FORMAL not applicable; chosen against a model, a proof of a port, and a proof of a
  test's arithmetic.
- **D5:** two pushes; chosen against one and against three.

### Consequences

- Good, because a drifted anchor, a renamed killer or a deleted row is refused on the pull
  request that causes it, on Linux, whatever paths it touches.
- Good, because a hung killer becomes one VOID row named in the report, and the job keeps its
  report under `if: ${{ always() }}`.
- Good, because one module replaces two pasted programs, and the Python mutant generator mutates
  it.
- Bad, because the sweep itself runs only when the change caller runs; on Linux it is proved
  through a fake killer. A change to the module starts the macOS jobs, because its path joins the
  trigger.
- Bad, because the bound's constants are a step's mean per run, not each run's cost: the module
  prints each run's seconds, so the next delivery that adds rows re-sizes from the worst one.
- Neutral, because a delivery that adds Swift rows can redden the bound's test by name when its
  rows push a job's projection past its timeout; the timeout is chosen at the band's ceiling so
  the link work's five rows fit.

### Confirmation

SPEC-397's criteria A1-A11, read in CI by name, and the two package jobs' last lines in the
second push's run.

## What would make this wrong

- A Swift package whose tests stop running on the macOS host, for example one that needs a
  simulator: its rows would need the simulator job, and D1 and D3's cost premise would no longer
  hold.
- A `python3` on the macOS image that lacks a call the module makes: the sweep would fail at start.
  The module keeps to the calls the base's inline programs already make, plus process-group
  control; the step prints the interpreter's version first.
- A Swift mutation tool that builds per mutant incrementally and needs no new binary on the
  runner: D2's rejection rests on the rebuild and the binary.
- A protocol that carries sweep reports between jobs: D4 would then be an interleaving surface
  and need a model.

## More Information

Built by SPEC-397. The schematic is `docs/schematics/mutation-testing.md` §9.
