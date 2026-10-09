# SPEC-397: every Swift mutant row is held on each pull request and proved by its package's macOS job

- **Issue:** #650, how the Swift mutants are proved and recorded. **Context(s):** the row battery
  under `scripts/`, which gains a reader of the Swift rows beside `scripts/mutation_rows.py`; the
  macOS job body `.github/workflows/xcframework.yml` and its change caller
  `.github/workflows/apple-on-change.yml`; and the `mutation-rows` job of `.github/workflows/ci.yml`.
- **Decided by:** ADR-411 (the population, the battery's form and its id band, the job, its bound
  and its verdict line, the check that holds the rows, FORMAL by surface, and the two pushes),
  resting on ADR-350 D8 (hand-written codec mutants, proved by each run of the codec's job) and
  ADR-358 D6 (no logic in Swift, measured by a lexical census).
- **Status:** planned, delivered by the draft pull request that adds this file with its tests and
  `docs/red-first/SPEC-397.md`. **Mutation band:** `S39700-S39799`. **Swift id band:**
  `SW39700-SW39799`, claimed and unused: this SPEC adds no Swift row.

## 1. The problem, measured

Read at DeckStreak `dev` `164ac206` with `git show 164ac206:<path>` and `git grep -n` at that
commit.

1. Two Swift packages carry mutant rows. `ios/HarnessWire/swift-mutants.json` holds 29 rows over
   `Sources/HarnessWire/Messages.swift` and `Wire.swift`, with 6 distinct killers under
   `Tests/HarnessWireTests/`. `ios/CardIsolation/swift-mutants.json` holds 38 rows over 8 of the 9
   files in `Sources/CardIsolation/`, with 8 distinct killers under `Tests/CardIsolationTests/`.
   Each file is `{population, mutants}`; each row has exactly the keys `id`, `file`, `find`,
   `replace`, `killer` and `why`; every killer reads `<Target>.<Class>/<method>`, the selector
   `swift test --filter` takes.
2. A read-only census of the 67 rows at `164ac206` found every one of them in the form R2 states:
   the keys exact, the id `SW` and five digits and unique across both files, the file under its
   package's `Sources/`, the find occurring once and differing from its replacement (one
   replacement is empty), the killer resolving to exactly one test method of one class in the
   named test target, and the reason non-empty.
3. Each file is swept by an inline Python program pasted into one job of
   `.github/workflows/xcframework.yml`: `harness-wire` (`:178-265`, the sweep step at `:193-258`) and
   `card-isolation` (`:267-355`, the sweep step at `:282-347`). The two programs differ only in the
   package path. Each runs every distinct killer once unmutated, then each row's killer with its
   mutant installed, through `subprocess.run` with no timeout (`:208-210`); a row is KILLED when one
   case started, one failed and the exit is non-zero, SURVIVED when one started, none failed and
   the exit is zero, and VOID otherwise (`:230-248`); the table goes to `sweep.md` and the step
   summary (`:254-256`). Both jobs run on the label `ADMITTED_RUNNERS` admits
   (`scripts/tests/test_ci_workflows.py:44`) with `timeout-minutes: 30` (`xcframework.yml:180`,
   `:269`).
4. Nothing on Linux reads the rows. The only test that names a sweep,
   `card_probe_problems` (`scripts/tests/test_ci_workflows.py:8699`), checks that the
   `card-isolation` job's run text holds `pathlib.Path("ios/CardIsolation")` and
   `swift-mutants.json` (`:8741-8743`); no test reads an anchor, a killer or an id.
5. The macOS jobs run on a pull request only when `.github/workflows/apple-on-change.yml`'s paths
   match (`:11-19`): the FFI crates, `ios/**`, the lockfile, the workspace manifest, the toolchain
   pin and the two workflows. A change under `scripts/` alone starts none of them.
6. The shared row battery cannot hold a Swift row as it stands: a band file's name and ids must
   match `S[0-9]+` (`scripts/mutation_rows.py:73-75`), its tables name a cargo or unittest killer
   (`:79-93`), and its `retired` check (`:1228-1259`) reads only band rows. It records an approval
   in `scripts/mutation-rows.retired.json`, keyed by id with no id form checked (`:1236-1240`).
7. The `mutation-rows` job's retired step is one line, `run: python3 scripts/mutation_rows.py
   retired --base HEAD^1` (`.github/workflows/ci.yml:630-632`), under
   `if: ${{ needs.mutation-plan.outputs.scope == 'diff' }}`; `docs/schematics/mutation-testing.md`
   cites `ci.yml` lines after it (`:644-646`, `:672-673`, `:687`, `:873-876`).
8. The app, harness and probe-host targets' tests run only through `xcodebuild` on the simulators
   of the `harness` job (`xcframework.yml:356-359`), and their Swift is held to per-role decision
   ceilings by the thin-Swift census (`ios/swift-roles.json`,
   `scripts/tests/test_ios_thin_swift.py:35-58`).

## 2. Requirements

R1. **The population.** A Swift mutant may touch only a source file under
`ios/<package>/Sources/` of a package whose row file is `ios/<package>/swift-mutants.json`, and is
killed by one test method under `ios/<package>/Tests/<Target>/`. The app, harness and probe-host
targets carry no row.

R2. **The row form**, read by `python3 scripts/swift_mutants.py census` over every
`ios/*/swift-mutants.json`. Each file's top-level keys are exactly `population` (a non-empty
string) and `mutants` (a list). Each row's keys are exactly `id`, `file`, `find`, `replace`,
`killer` and `why`. `id` is `SW` and five digits, unique across every file. `file` is a relative
path under the package's `Sources/` that exists. `find` is non-empty, occurs exactly once in that
file, and differs from `replace`, which may be empty. `why` is non-empty after trimming. `killer`
is `<Target>.<Class>/<method>`, `<method>` starts with `test`, and exactly one file under
`ios/<package>/Tests/<Target>/` declares `class <Class>`, in which `func <method>(` is declared
exactly once. Each refusal prints one line naming its reason, the file and the row, and the census
then exits 2. The census prints `examined N row(s) in M file(s)` and exits 3 when N is 0.

R3. **The id band.** `SW<NNN>00`-`SW<NNN>99` is SPEC-NNN's band across every package file. The
existing ids keep their numbers. This SPEC claims SW39700-SW39799 and adds no Swift row.

R4. **Departures.** `python3 scripts/swift_mutants.py retired --base <ref>` reads every
`ios/*/swift-mutants.json` at `<ref>` and in the work tree. An id present at `<ref>` and absent
from every work-tree file is refused by id, and the check exits 1, unless its source file
`ios/<package>/<file>` no longer exists or `scripts/mutation-rows.retired.json` holds an entry
with that id, a non-empty reason and a non-empty approval. It prints `examined N` for the base's
rows.

R5. **The sweep.** `python3 scripts/swift_mutants.py sweep --package ios/<package> --report <dir>
--run-seconds <n>` first runs the census over that package's file and applies nothing if it
refuses (exit 2). It runs each distinct killer once unmutated; then, row by row, installs the
mutant by one replacement of `find`, runs the row's killer alone, restores the file byte for byte
and checks its sha256. Each killer run is `swift test --package-path ios/<package> --filter
^<the escaped killer>$`, started in its own session and bounded by `--run-seconds`; past the
bound its whole process group is ended and the run is VOID. Each row's verdict line, with the
run's seconds, is printed and flushed as it is decided. `sweep.md` holds the table `| id | verdict
| why |` and the closing line, and the same table is appended to the runner's step summary when
the runner names one. The last line printed is `swift-mutants ios/<package>: examined N row(s): K
killed, S survived, V void`. It exits 0 when every row is KILLED, 1 when any is SURVIVED or VOID,
2 on a refusal, 3 when it examined no row, and 4, at once, when a file is not restored byte for
byte (the exit codes of `scripts/mutation_rows.py:106`).

R6. **The verdict.** One function decides each row from what its runs observed, first match
wins: the killer was not green unmutated (one case started, none failed, `Executed 1`, exit zero,
within its bound) -> VOID; the find does not occur exactly once -> VOID; the file was not restored
-> VOID; the mutated run passed its bound -> VOID; one case started, one failed and the exit is
non-zero -> KILLED; one case started, none failed and the exit is zero -> SURVIVED; anything else
-> VOID. Each VOID names its reason.

R7. **The job.** Each `ios/*/swift-mutants.json` is swept by exactly one job of
`.github/workflows/xcframework.yml`, through one step that runs `python3 scripts/swift_mutants.py
sweep --package ios/<package> --report "$REPORT" --run-seconds <n>` after the package's test step
and prints the interpreter's version first. That job runs no inline Python program, and its
report upload runs under `if: ${{ always() }}`. `harness-wire` sweeps `ios/HarnessWire` and
`card-isolation` sweeps `ios/CardIsolation`, on the admitted label.

R8. **The bound.** For each sweeping job, the projection P is the job's measured minutes outside
the sweep plus the live run count (the file's rows plus its distinct killers) times the measured
seconds per run, over sixty. Its `timeout-minutes` lies between one and a half times P and twice
P, each rounded up to the five. Its `--run-seconds` equals three times the measured seconds per
run, rounded up to the minute. The measured constants name the run and the job they were read
from.

R9. **The trigger.** `.github/workflows/apple-on-change.yml`'s `paths` include
`scripts/swift_mutants.py`, so a change to the module runs the sweeps it changes.

R10. **The departures check in CI.** The `mutation-rows` job's retired step, under
`if: ${{ needs.mutation-plan.outputs.scope == 'diff' }}`, runs `python3 scripts/swift_mutants.py
retired --base HEAD^1` beside `python3 scripts/mutation_rows.py retired --base HEAD^1`, and fails
when either fails. It stays one `run:` line, so no `ci.yml` line any schematic cites moves.

R11. **The Python generator.** `scripts/mutation-python.json` maps `scripts/swift_mutants.py` to
the `scripts/tests` module `test_swift_mutants`, so the Python mutant generator's population census
holds and its mutants of the module have a killer.

R12. **The module's reach.** `scripts/swift_mutants.py` imports only the standard library, so the
macOS jobs need only its own path in their trigger, and it makes no call newer than the base's
inline programs make, besides `subprocess.Popen(start_new_session=True)`,
`Popen.communicate(timeout=)`, `os.killpg`, `os.getpgid` and `signal`.

R13. **Where the verdict is read and recorded.** The verdict is the sweep's last line in its job's
log, read by name, with `sweep.md` in the job's report artifact and the table in the step summary.
A delivery that adds or changes Swift rows records each changed package's last line, with the run
and job ids, in its own `docs/red-first/SPEC-<n>.md`. SPEC-057's campaign table is not that
record.

R14. **The card view's sweep predicate.** `card_probe_problems` reads the module's invocation for
`ios/CardIsolation` in place of the inline program's two strings, and the planted job its test
builds (`scripts/tests/test_ci_workflows.py:8771`) carries that invocation in place of them. The
predicate is replaced, not loosened: both name the one package's row file being swept, and A7
holds the rest.

R15. **Rows.** The band `scripts/mutation-rows.d/S39700-S39799.json` holds rows from S39700 over
the module's decisions and the changed workflow lines, each killed by a criterion's test (§8).

## 3. Acceptance criteria of SPEC-397

Every criterion is a Python test that runs in the Linux python stage of the `hygiene` job on every
pull request. A1-A6 and A11 build their trees under a temporary directory and drive the module's
command line; A5 puts a fake `swift` first on `PATH`, which prints the test runner's case lines as the planted
source decides.

| A | criterion | decided by |
|---|---|---|
| A1 | the census over the live tree reads no problem, and its examined count equals the rows the test counts in every `ios/*/swift-mutants.json` itself | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_every_live_swift_row_holds_its_form` |
| A2 | a planted defect of each row-form kind (a key missing or extra, an id out of form or repeated in another package's file, a file outside `Sources/` or absent, a find absent, repeated or equal to its replacement, an empty reason) is refused by name with exit 2, and a clean planted tree reads its examined count with exit 0 | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_each_planted_row_defect_is_refused_by_name` |
| A3 | a killer whose target, class or method does not resolve, whose method does not start with `test`, or whose class or method is declared twice is refused by name | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_each_unresolvable_killer_is_refused_by_name` |
| A4 | an id that left while its file stays is refused by id with exit 1; it passes when its file left with it, or when the approvals record holds a reason and an approval for it, and not when either is empty | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_a_row_leaves_only_with_its_file_or_an_approval` |
| A5 | through a fake killer, a sweep reads KILLED, SURVIVED, VOID for a killer red unmutated, VOID for a find that is not once, and VOID for a run past its bound with its process group ended; a killer that rewrites its file leaves it restored byte for byte; `sweep.md` and the last line name every row; the exits are 0, 1 and, for an empty file, 3 | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_a_fake_killer_reads_each_verdict_and_the_file_is_restored` |
| A6 | the verdict function, over every class of its inputs, equals the first-match table the test writes from R6 | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_the_verdict_over_every_observed_class` |
| A7 | each `ios/*/swift-mutants.json` is swept by exactly one `xcframework.yml` job through the module's `sweep` step, that job runs no inline Python program, and its report upload runs under `if: ${{ always() }}` | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_swift_row_file_is_swept_by_one_job_through_the_module` |
| A8 | each sweeping job's `timeout-minutes` lies in its band and its `--run-seconds` equals the per-run rule, from the measured constants and the live counts | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_sweep_is_bounded_from_its_measured_cost` |
| A9 | the change caller's paths equal the pinned list, which names `scripts/swift_mutants.py` | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_change_caller_runs_the_build_on_each_apple_path` |
| A10 | the `mutation-rows` job's retired step, under the diff scope, runs both retired checks on its one line and fails when either fails | `python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rows_job_runs_the_swift_retired_check_on_a_diff` |
| A11 | the module imports only the standard library: a planted module importing anything else is refused by name, and the live module is read and passes | `python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_the_module_imports_only_the_standard_library` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_every_live_swift_row_holds_its_form
A2: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_each_planted_row_defect_is_refused_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_each_unresolvable_killer_is_refused_by_name
A4: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_a_row_leaves_only_with_its_file_or_an_approval
A5: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_a_fake_killer_reads_each_verdict_and_the_file_is_restored
A6: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_the_verdict_over_every_observed_class
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_swift_row_file_is_swept_by_one_job_through_the_module
A8: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_sweep_is_bounded_from_its_measured_cost
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_change_caller_runs_the_build_on_each_apple_path
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rows_job_runs_the_swift_retired_check_on_a_diff
A11: python3 -m unittest discover -s scripts/tests -p test_swift_mutants.py -k test_the_module_imports_only_the_standard_library
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-397-every-swift-mutant-row-is-held-on-each-pull-request-and-proved-by-its-packages-macos-job.md` | spec | added: this file |
| `docs/decisions/ADR-411-swift-mutant-rows-stay-in-each-package-file-and-one-module-reads-sweeps-and-retires-them.md` | decision | added |
| `docs/schematics/mutation-testing.md` | schematic | amended, insert-only: §9, the Swift rows' flow |
| `docs/red-first/SPEC-397.md` | record | added: one line per criterion |
| `changelog.d/swift-mutation-rows-397.md` | changelog | added |
| `scripts/swift_mutants.py` | row battery | added: `census`, `retired`, `sweep` |
| `scripts/tests/test_swift_mutants.py` | row battery | added: A1-A6, A11 |
| `scripts/mutation-python.json` | row battery | changed: maps the module to `test_swift_mutants` |
| `scripts/mutation-rows.d/S39700-S39799.json` | row battery | added: the band's rows (§8) |
| `.github/workflows/xcframework.yml` | macOS job body | changed: each sweep step runs the module; each sweeping job's `timeout-minutes` in its band |
| `.github/workflows/apple-on-change.yml` | macOS change caller | changed: one path, `scripts/swift_mutants.py` |
| `.github/workflows/ci.yml` | CI | changed: the retired step's one `run:` line, in place |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: `APPLE_PATHS`, `card_probe_problems`' sweep predicate and its plant, and A7, A8 with the measured constants |
| `scripts/tests/test_mutation_workflows.py` | CI tests | changed: A10 |

No file under `ios/` changes, and neither do `scripts/mutation_rows.py`, `scripts/mutation-rows.json`
nor `scripts/mutation-rows.retired.json`.

## 5. What this does NOT cover

- It mutates no Swift outside the two host-tested packages: the app, harness and probe-host
  targets are held by the thin-Swift census and their simulator tests (#625).
- It adds, removes and re-anchors no Swift row; the card view's link rows SW39200-SW39204 arrive
  with their own delivery (#664).
- It adds no Swift mutant generator; a generated population would be its own decision (#650).
- It feeds no sweep report to the `mutation-verdict` job or to SPEC-057's campaign table: each
  package job's exit is its verdict (#650).
- The formal surfaces ratchet reads the band files only; the Swift rows' departures are held by the
  `retired` step this SPEC adds (#650).
- It takes no figure on a device or a simulator; the bound is sized from the hosted job's own
  steps (#629).
- It changes no runner label and no other macOS job's bound; the macOS CI timing record is its own
  delivery (#752).

## 6. Risks

- The macOS image's `python3` may lack a call the module makes. Detected by the sweep step failing
  at start, after it printed the interpreter's version; R12 keeps the module to the calls the
  base's programs already made, plus process-group control.
- The constants are a step's mean per run, so a slow row may pass `--run-seconds` and read VOID by
  name rather than KILLED. Detected by the per-row seconds the module prints; the next delivery
  that changes rows re-sizes the bound from the worst run.
- A delivery that adds Swift rows moves the live count and can push a job's projection past its
  timeout. Detected by A8 on that delivery's pull request, by name; the timeout is chosen at the
  band's ceiling, so the link work's five rows fit.
- A killer method found only in a comment satisfies the lexical census. Detected by the sweep: the
  unmutated run must start exactly one case, so the row reads VOID by name.
- #752 edits `.github/workflows/xcframework.yml` and `scripts/tests/test_ci_workflows.py`.
  Detected at the cut and at the merge; the build re-measures both.
- A later test of `scripts/mutation-rows.retired.json` may constrain its ids to the band form.
  Detected by that test on the pull request that first records a Swift approval; this SPEC records
  none.

## 7. Formal

FORMAL: not applicable - each sweep is one process in one job on its own runner and checkout,
applying a mutant, running one killer and restoring the file in sequence with no state another
actor reads or writes, the `retired` check only reads git objects, no report passes between jobs,
and the one total function, the verdict, is decided over every class of its finite domain by A6.

## 8. Mutation rows

The band `scripts/mutation-rows.d/S39700-S39799.json`, `SCRIPT_MUTATIONS`, from S39700. Each find
is written against the code as committed and occurs exactly once in its file.

| row | file | the mutant | killed by |
|---|---|---|---|
| S39700 | `scripts/swift_mutants.py` | KILLED accepts any count of failed cases | A6 |
| S39701 | `scripts/swift_mutants.py` | SURVIVED accepts a non-zero exit | A6 |
| S39702 | `scripts/swift_mutants.py` | the unmutated run is green on any executed count | A6 |
| S39703 | `scripts/swift_mutants.py` | a find that occurs twice is installed | A5 |
| S39704 | `scripts/swift_mutants.py` | the restore writes nothing | A5 |
| S39705 | `scripts/swift_mutants.py` | a killer run has no bound | A5 |
| S39706 | `scripts/swift_mutants.py` | past the bound, only the killer's own process is ended | A5 |
| S39707 | `scripts/swift_mutants.py` | an id of any length after `SW` is admitted | A2 |
| S39708 | `scripts/swift_mutants.py` | id uniqueness is checked per file | A2 |
| S39709 | `scripts/swift_mutants.py` | a method declared twice resolves | A3 |
| S39710 | `scripts/swift_mutants.py` | an approval entry needs only a reason | A4 |
| S39711 | `scripts/swift_mutants.py` | an empty row file exits 0 | A5 |
| S39712 | `.github/workflows/xcframework.yml` | `harness-wire` sweeps no package | A7 |
| S39713 | `.github/workflows/xcframework.yml` | `card-isolation` sweeps no package | A7 |
| S39714 | `.github/workflows/xcframework.yml` | `harness-wire`'s `--run-seconds` changes | A8 |
| S39715 | `.github/workflows/xcframework.yml` | `card-isolation`'s `--run-seconds` changes | A8 |
| S39716 | `.github/workflows/apple-on-change.yml` | the module's path leaves the trigger | A9 |
| S39717 | `.github/workflows/ci.yml` | the Swift retired check leaves the step | A10 |
| S39718 | `scripts/swift_mutants.py` | a non-standard import is admitted | A11 |

A changed `timeout-minutes` line takes one more row, killed by A8.

## 9. Measured by the package jobs

Not a repository test: the second push's run of the change caller sweeps both packages through
the module. Its two last lines, `swift-mutants ios/HarnessWire: ...` and
`swift-mutants ios/CardIsolation: ...`, are read by name from the `harness-wire` and
`card-isolation` job logs, with the run and job ids, and each must read every row killed.
