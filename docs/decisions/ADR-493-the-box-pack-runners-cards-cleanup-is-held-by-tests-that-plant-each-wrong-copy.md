# ADR-493: the box-pack runner's cards cleanup is held by tests that plant each wrong copy

Status: accepted. It decides SPEC-476, for issue #542. Every `path:line` below was read at dev
`36c4783a`.

## Context

Pull request #539 closed #531: `scripts/box-packs.sh` removes the temporary cards directory it made
when the run ends, whether the run passed, failed or stopped on an error. The review of #539 found
the cleanup correct and asked, in #542, that the tests hold it rather than the code alone. Its five
items: no test seeds a sibling named `deckstreak-box-cards.<other>`; no test pins an empty
`BOX_PACKS_OUT` or `BOX_PACKS_KEEP=0`; three lines of the runner's Python body run past the
repository's 100 columns (`ruff.toml:6`); two assignments sit between making the directory and the
`try` that removes it; and CI's mutation verdict reads a change to a `.sh` file as not-applicable.

## D1. The mechanism the tests hold, as measured

- **Signals.** `main` sets SIGTERM and SIGHUP to call `sys.exit(128 + signum)`
  (`scripts/box-packs.sh:194-196`), so either signal raises `SystemExit` in the main flow; SIGINT
  raises `KeyboardInterrupt` by Python's default. Both unwind through any `finally` they are raised
  inside.
- **Before the make.** `run` (`:223`) refuses an unset or unusable `PACKS_WIRING`, `PACKS_CHECKOUT`
  or `PACKS_RUNNER`, a checkout not at the private file's pin, a missing owned source and a missing
  advisory lint (`:224-242`). No directory exists yet, so none of these leaves one.
- **The make.** `out = cards_directory()` (`:243`). `cards_directory` (`:537-543`) reads
  `BOX_PACKS_OUT`: a non-empty value is made with `mkdir(parents=True, exist_ok=True)` and returned as
  given (`:539-542`); an unset or empty one makes `tempfile.mkdtemp(prefix="deckstreak-box-cards.")`
  (`:543`), a new directory named `deckstreak-box-cards.<random>` whose parent is the process's
  temporary directory (`TMPDIR` when it is set).
- **The two readings.** `made = not os.environ.get("BOX_PACKS_OUT")` (`:246`) and
  `keep = bool(os.environ.get("BOX_PACKS_KEEP"))` (`:247`). So `BOX_PACKS_OUT` unset or empty means
  the run made its own directory, and any non-empty value names the caller's, which is never
  removed. `BOX_PACKS_KEEP` unset or empty keeps nothing, and any non-empty value keeps: `0`, `1` and
  any other. The opening comment (`:65-70`) and the help epilog (`:184-186`) state the same readings.
- **The try and each path out.** The `try` (`:248-295`) holds the issue-state read, the scratch
  directory of the exported tree, every pack, probe, scan and owned file, and the cards each writes
  under `out`. Its `finally` (`:296-300`) runs `shutil.rmtree(out, ignore_errors=True)` when
  `made and not keep`, and otherwise prints `cards: <out>` on stderr. The removal names exactly the
  path the make returned, never its parent and never a pattern. Every way out of the `try` passes
  the `finally`: the return (`:295`), a `Refusal` or any other exception raised inside it, and a
  signal raised inside it.
- **The window.** Lines 246 and 247 run after the make and before the `try`. A signal there unwinds
  outside the `try`, and the made directory stays behind; nothing else in those two lines can fail.
  With both reads above `:243`, the make is the last statement before the `try`. A signal raised
  inside `mkdtemp` after its directory exists and before it returns is not measured here, and moving
  the reads does not reach it.
- **The long lines.** `:262` runs to 102 columns, `:264` to 101 and `:267` to 101, by
  `awk 'length($0) > 100'` over the whole file; none of its other 1130 lines does, and the file is
  ASCII. No test pins the text of any of the three.
- **What the tests pin, and how they run the runner.** `scripts/tests/test_box_packs.py` runs the
  real runner with `bash` (`:370-377`) against a synthetic DeckStreak repository it commits in its
  own temporary directory, with a fake runner, a fake issue-state reader first on the path and a
  synthetic checkout (`scripts/tests/fixtures/box-packs/`). Every run sets `TMPDIR` to the test's own
  `temp` directory and `BOX_PACKS_OUT` to its `cards` directory, unless `cards=False` removes it;
  `keep=True` sets `BOX_PACKS_KEEP=1` (`:342-363`). The run passes no working directory, so the runner
  inherits the test process's (`:370-377`). `test_the_created_cards_directory_is_gone_after_every_end`
  (`:1002-1016`) compares the temporary directory's entries before and after a pass, a failure and an
  error, starting from an empty directory. `test_a_given_or_kept_cards_directory_survives_the_run`
  (`:1018-1033`) pins a given `BOX_PACKS_OUT` and `BOX_PACKS_KEEP=1`. A copy that removes `out.parent`
  makes the first test's `iterdir` raise `FileNotFoundError`: an error, never an assertion. A copy that
  removes every `deckstreak-box-cards.*` entry passes both, since the directory holds no other. No test
  sets an empty `BOX_PACKS_OUT`, an empty `BOX_PACKS_KEEP` or `BOX_PACKS_KEEP=0`. The module holds 16
  tests, and `scripts/tests/test_box_scan_expected.py:10` imports the same `Box`.
- **CI on a change to the runner.** `scripts/mutation-verdict.py` classes a path as `scripts` only
  when it matches `scripts/[^/]+\.py` (`:120`, `:259-269`), so the runner and its test module are
  `other`, and every class reads `not-applicable: the diff changes no <class> production file`
  (`:802`). Rows are selected by their target's path, by being added or changed, or by their
  killer's file changing (`:703-732`); the `mutation-rows` job proves them
  (`.github/workflows/ci.yml:643-711`), and `judge_python` judges every selected row before it asks
  whether its class applies (`:2143-2146`), so a selected row that is not KILLED fails the verdict.
  `scripts/mutation_rows.py` refuses a shell mutant only when `bash -n` refuses it (its docstring,
  `:36-41`), which never parses the Python inside the heredoc. Thirteen rows target the runner,
  S12301 to S12313 (`scripts/mutation-rows.d/S12300-S12399.json`); S12313 alone guards the cleanup.
- **Formal.** No file under `formal/` names `box-packs`, so no `@phx covers` line covers the runner.

## D1 chosen against

- Assume the window is only the two lines: a signal inside the standard library's make is a second window this reading cannot show, so it is named as not measured rather than closed.
- Read the mechanism from the changelog line of #531 (`CHANGELOG.md:1152-1153`): it says `BOX_PACKS_KEEP=1` keeps, while the runner keeps for any non-empty value, `0` included; the code and its own documentation are the record.

## D2. The tests that hold each reading by an assertion

Four new tests in `TheRunnerCleansUpItsCards`, each running the real runner over the fixture tree,
each green at the base and read red against its own planted copy:

| test | reading it holds | the copy it must catch |
|---|---|---|
| `test_a_decoy_sibling_and_the_parent_survive_every_end` | a sibling `deckstreak-box-cards.decoy`, seeded in the temporary directory with one file, survives a pass, a failure and an error byte for byte, and so does the directory both sit in; the made directory is gone | one that removes `out.parent` (S47601), one that removes every `deckstreak-box-cards.*` entry (S47602) |
| `test_an_empty_out_is_read_as_unset` | with `BOX_PACKS_OUT` set and empty, the run makes its own directory in the temporary directory, removes it, writes nothing into its working directory and prints no `cards:` line; with `BOX_PACKS_KEEP=1` beside it, that directory is kept holding the cards and named on stderr | one that tests `BOX_PACKS_OUT` for presence in `cards_directory` (S47603), one that decides `made` by presence (S47604) |
| `test_a_keep_of_0_keeps_the_made_directory` | `BOX_PACKS_KEEP=0` keeps the made directory holding its cards and names it as `cards: <path>` | one that keeps only on `1` (S47605) |
| `test_an_empty_keep_removes_the_made_directory` | an empty `BOX_PACKS_KEEP` keeps nothing: the made directory is removed and no `cards:` line is printed | one that decides `keep` by presence (S47606) |

Each asserts something present: the decoy's file bytes, the parent as a directory, the kept
directory's cards, the `cards: <path>` line and the run's `BOX PACKS OK` summary. The decoy test
asserts the parent is a directory before it lists it, so a copy that removes the parent fails by
an assertion. The harness changes twice: every run is started in a fresh empty working directory
inside the test's own temporary directory, and `run` takes `environ`, variables set over every
other. The working directory is a guard, not a convenience: a relative path resolves against it,
and under S47603 an empty `BOX_PACKS_OUT` becomes `Path("")`, the working directory itself, which
the runner writes its cards into and then removes the contents of. Started from the test's own
directory, that copy can reach nothing outside it.

## D2 chosen against

- Seed the decoy inside the existing `test_the_created_cards_directory_is_gone_after_every_end`: it changes that test's before and after lists, moving a pin it already holds.
- Catch the `FileNotFoundError` in the existing test and fail on it: it still edits an existing pin, and a parent check placed in a new test asserts the same thing without touching it.
- One table test over every pair of `BOX_PACKS_OUT` and `BOX_PACKS_KEEP` values: one killer for six copies leaves each red unattributable; a test per reading dies only on its own copies.
- Call `cards_directory` in-process from the heredoc's body: the runner is a script with no importable module, so the test would exercise an extracted copy, not the file CI and the maintainer run.
- Pin a working directory only in the empty-`BOX_PACKS_OUT` test: every other run would stay in the test process's working directory, where any later copy that resolves an empty or relative path reaches the tree the tests run from.
- Read the cells in CI by name only: they run locally over a fixture tree in seconds, so nothing about them waits for CI.

## D3. Production or test: the two cures, and which test lines change

The runner changes in two places, neither of which changes what it does:

1. **The three long lines are wrapped.** The loop over the two places the scratch directory must not
   sit in (`:262`) takes a parenthesised tuple over four lines, the refusal (`:264`) takes its message
   on its own line, and the `judging` line's text (`:267`) is split into two adjacent f-strings with
   the same characters. The runner's syntax tree, read before and after this step alone, is
   unchanged. `test_every_runner_line_fits_100_columns`, in a new class
   `TheRunnerSourceHoldsItsCures`, reads every line of the file and asserts none runs past 100
   columns: red at the base by its three lines, green after.
2. **Both readings move above the make.** `made` and `keep` are assigned before
   `out = cards_directory()`, which becomes the last statement before the `try`, with one added
   comment line saying why. `test_the_cards_directory_is_made_just_before_its_try` parses the
   heredoc's body, finds the one assignment from `cards_directory()` in `run`, and asserts the next
   statement is the `try` whose `finally` removes `out`, with `made` and `keep` assigned before the
   make: red at the base, where two statements sit between, green after.

Test lines that change: `Box.__init__` gains the working directory, `Box.run` gains `environ` and
passes `cwd`, the class docstring of `TheRunnerCleansUpItsCards` names #542, the module docstring
names SPEC-476, and six tests are added. No existing test, assertion, fixture or examined count is
changed or removed, and the 16 existing tests and the 9 of `test_box_scan_expected.py` run as
before. No option weighed here would weaken a pin or move a count.

## D3 chosen against

- Waive the long lines: the repository's style is 100 columns, and the runner's Python escapes the linter only because it sits in a heredoc.
- Move the make inside the `try` with `out = None` before it and a guard in the `finally`: it reaches the window inside the make too, but changes the `finally`'s logic and the line S12313 guards, which is more than the issue's move.
- Hold the cures by a reading in the record alone: an acceptance criterion is decided by a test command, and a reading leaves the next edit free to re-open the window or lengthen a line.
- Change a reading to match the changelog's wording (keep only on `1`): it changes behaviour the runner documents, and the issue asks the tests to pin the readings as they are.

## D4. Shape: one delivery

One delivery under SPEC-476, this ADR and band S47600-S47699: the six tests, the harness change,
the two cures, ten rows, an insert-only section of `docs/schematics/box-pack-runner.md` (its line 44
says the cards are kept under `$BOX_PACKS_OUT`, which since #531 is true only when it is set and not
empty), the red-first record and a changelog fragment. It changes no file under `.github/` and no
mutation script. #542's fifth item, a mutation leg that covers the runner's script, changes
`scripts/mutation-verdict.py`, its tests and the workflow, files that open pull requests change, so
it is left to an issue of its own, proposed with this delivery. #531's delivery left no SPEC, ADR or
red-first record (a search of the tree for `#531` finds only `CHANGELOG.md:1152` and an unrelated
line), so no amendment is owed; this SPEC is the first to state the cards directory's readings.

## D4 chosen against

- Join the mutation leg to this delivery: it mixes a CI change into a test delivery and shares files with open pull requests that change the workflow and the mutation scripts.
- Two deliveries, the tests and then the cures: the two cure tests are red until the cures land, so a split leaves either a red test on dev or a cure without its red.
- A new schematic of its own: it would leave the owning schematic's sentence on the cards unqualified.
- An amendment of #531's SPEC or ADR: none exists to amend.

## D5. Tests and rows, the formal decision, the pushes

- **Red first, per criterion.** A5 and A6 are RED-FIRST: red at the test commit, against the
  unchanged runner, by assertion; green at the cure commit. A1 to A4 are MUTATION COVERAGE: green at
  the base, each read red by an assertion, never an error, against each copy D2 names, planted by
  hand in the unchanged runner, the reading being `FAILED (failures=1)` with no error. Before the new
  tests exist, the same copies are planted against the module as it is, to measure the gap #542
  states.
- **Rows.** Ten rows in `scripts/mutation-rows.d/S47600-S47699.json`, table `SCRIPT_MUTATIONS`:
  S47601 and S47602 on the removal line, S47603 on the `if chosen:` line, S47604 on the `made` line,
  S47605 and S47606 on the `keep` line, S47607 on the moved pair (it puts `keep` back after the
  make), and S47608 to S47610 on the three wrapped sites (each puts its line back as one). Every
  changed code line of the runner sits in some row's find. No existing row is re-anchored: none of
  S12301 to S12313 has a find on a line the cures change, and each still occurs once.
- **FORMAL.** Lean: not applicable. The pure decision, removal when `BOX_PACKS_OUT` is unset or empty
  and `BOX_PACKS_KEEP` is unset or empty, decides no gate verdict, ledger, seal, ratchet, money or
  release; it is closed by the cell tests' literal expectations, written from the runner's own
  documentation, with S47603 to S47606 as the hostile copies. TLA+: not applicable, surface present.
  Concurrent runs share one temporary directory; each run writes only the entry its own atomic make
  created, reads and removes only the path it holds, and orders nothing across runs; the guard is
  the removal naming `out` alone, held by A1 against a seeded sibling and by S47601 and S47602. A
  signal is raised in the run's own thread, not by a second actor over shared state.
- **Pushes: one.** Every red is read locally over a fixture tree, so CI reads only the green head.

## D5 chosen against

- Prove the rows with the mutation runner before the push: the build starts no mutation run; a hand plant reads the same mutant with its failing assertion quoted, and CI's `mutation-rows` job proves every selected row again.
- Rows only on the lines the cures change: the parent, sibling and `cards_directory` presence copies sit on lines the cures leave as they are (`:298`, `:539`), so their reds would be held by nothing after this delivery.
- The two-push form: it is for a red that only CI can read, and none is here.
- A TLA+ model of two runs over one temporary directory: no run reads or removes another's entry, so a model would restate that the removal names its own path, which A1 already observes.

## D6. What stays out

The packs the runner runs and what each finds; the runner's other options and outputs (`--rev`,
`--post-status`, the scratch directory of the exported tree and its ledger, and a relative
`BOX_PACKS_OUT`, which resolves against the working directory and is never removed); the mutation
leg of #542's fifth item; a test that sends the runner a signal mid-run, and the window inside the
make; and the proof of every selected row in CI's `mutation-rows` job, which only the run after the
push can measure. SPEC-476 lists each with its issue.

## D6 chosen against

- Add a signal test now: it is not one of #542's items, and holding the runner at a known point needs a seam in the runner or a timing, which is its own decision.
- Leave the mutation leg unmentioned: #542 closes with this delivery, and its fifth item would then have no owner.

## Consequences

- A copy of the runner that removes more than the directory it made, or reads either variable by
  presence, or keeps only on `1`, fails a named test by an assertion, and CI's `mutation-rows` job
  proves it on every change that selects those rows.
- Every run of the runner from these tests starts in the test's own working directory.
- The runner's lines fit 100 columns, and its make is the last statement before the `try`; both are
  tests, so an edit that undoes either fails CI.
- A change to the runner is still proved by its hand rows alone until a leg mutates its Python.

## What would make this wrong

- A copy that removes another entry of the temporary directory by a name the decoy does not match:
  the decoy is one sibling, named as the issue names it.
- A test-only harness change that moves the runner's behaviour: the working directory reaches only
  relative paths, and every path the tests hand the runner is absolute.
- A Python whose parser keeps adjacent f-strings apart in the syntax tree: the wrap's equality would
  then be read on the output line instead.
