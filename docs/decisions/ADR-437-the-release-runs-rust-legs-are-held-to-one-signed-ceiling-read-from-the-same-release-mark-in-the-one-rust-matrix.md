---
status: "accepted"
---

# ADR-437: the release run's Rust legs are held to one signed ceiling, read from the same release mark, in the one Rust matrix

Decides SPEC-423. Path:
`docs/decisions/ADR-437-the-release-runs-rust-legs-are-held-to-one-signed-ceiling-read-from-the-same-release-mark-in-the-one-rust-matrix.md`.
Amends ADR-373 and ADR-436 insert-only, and SPEC-362 and SPEC-422 through SPEC-423. Read at dev
`a3a052b2`.

## Context

A release pull request, from `dev` into `main`, is judged on its merge diff in one run (ADR-373,
SPEC-362). Every run sizes its Rust legs with `fewest_shards` up to one ceiling,
`LEG_CEILING["ci"]` 190 (`scripts/mutation-verdict.py:961`), and a listing that needs more is
refused whole, by name, never capped (`:1322-1329`). #822's own run did exactly that: its plan read
9385 Rust mutants, projected at 1252726 s serially, and refused them because they need more than 190
legs within 10800 s each, with the pull request's text. Its Python half raised no refusal, since its
legs are held to `PYTHON_RELEASE_CEILING` (ADR-436).

`LEG_CEILING["ci"]` is 256 less the most jobs every other `ci.yml` job can generate (SPEC-362 R3).
That derivation read the job limit per run. SPEC-422 measured it per matrix and committed the record
(`scripts/tests/fixtures/matrix-limit-<run>/`, read by SPEC-422 A19), so one matrix may hold 256
legs whatever else the run holds. The same release listing, sized with no ceiling, needs more than
190 legs and fewer than 256: one matrix holds it.

An owner-signed ruling admits one exception and no other: the release pull request's run alone
may size its Rust legs above `LEG_CEILING["ci"]`, up to one ceiling, the figure measured on the
release's final union, with every mutant run, every leg counted and no skipped-mutant record.
Every other run keeps 190. The delivery's own lines add Python mutants to the release's Python
listing, so the Python figure is read on the same final union.

## Decision Drivers

- Every listed mutant runs once, in a leg the verdict counts; nothing is sampled, skipped or capped.
- Only the release pull request's run changes. A pull request into `dev`, a push, a schedule, a
  dispatch and the weekly battery's `size` keep `LEG_CEILING` and the Rust refusal byte for byte.
- An unreadable or ambiguous context is not the release run, and an unset ceiling admits nothing.
- One constant carries the figure, and the tree cites the signed ruling file by path.
- `LEG_CEILING`, `SHARD_BOUND_SECONDS`, the price tables, `fewest_shards`, `RUN_JOBS`, the Python
  constants and every required context do not move.
- No new job, no new check-run name, and no `ci.yml` change.
- The delivery adds few Python mutants, and the commit that sets the figure adds none.

## Decisions

**D1. The release run is SPEC-422's, read from the same mark.** No second predicate: the plan's
top-level `"release": true`, written only when `release_run` holds (SPEC-422 R1, R2;
`scripts/mutation-verdict.py:230`, `:825-832`), is the one release reading, and `shards` reads it as
`plan.get("release") is True` exactly as `python_release_ceiling` does (`:1100`; SPEC-422 R3). A
fork named `dev`, an empty repository name, a push, a schedule, a dispatch and a pull request into
`dev` write no `release` key, so their Rust legs keep `LEG_CEILING["ci"]`. The weekly battery's
`size` (`:1006`) reads `LEG_CEILING["battery"]` and no plan, and is not edited.

**D2. One ceiling constant, at the pull request ceiling until a signed measurement sets it.**
`RUST_RELEASE_CEILING = 190` directly follows the `LEG_CEILING` line, with one `#:` line directly
above it. A helper, `rust_release_ceiling(plan)`, returns it only for a release plan whose
ceiling is above `LEG_CEILING["ci"]`, and None otherwise; under None `shards` sizes, refuses and
prints the Rust legs exactly as for a pull request into `dev`. At 190 the release run therefore
admits no leg past 190.
- A range check, `rust_ceiling_refused()`, runs on every `shards` run before any deal, after the
  two Python checks (`:3713-3720`): a ceiling that is not an `int`, is below `LEG_CEILING["ci"]`, or
  is above `RUST_MATRIX_LEGS` is refused by name, exit 1.
- A release listing that needs more legs than the ceiling is refused whole, by name, exit 1. The
  text is the pull request refusal's with its last clause naming `RUST_RELEASE_CEILING` and saying
  a larger figure needs a new measurement and a new owner-signed ruling. The pull request refusal's
  lines are not edited.
- The one commit that sets the figure also edits the `#:` line to cite the owner-signed ruling file
  by its path under `docs/rulings/`. The path is fixed when the unsigned ruling is cut, before that
  commit. A test holds the pair: at 190 no ruling path is cited; above it
  exactly one is, the file is in the tree, and its first line holds the figure as a whole word,
  `release` and `Rust`. A set figure therefore reads red until the signed ruling is in the branch.
- No test and no mutation row pins the constant's value, and the value is the measured figure with
  no headroom above it.

**D3. One matrix: `mutation-rust` holds the release run's legs, at most 256.** The existing job's
`shard` axis is the plan's `matrix` output (`.github/workflows/ci.yml:468-469`), which `announce`
writes as `0..N-1` (`:1256`). On SPEC-422's per-matrix record that one matrix may hold 256 legs, so
the figure needs no new job, no new output, no new verdict arm and no new check-run name. A second
constant, `RUST_MATRIX_LEGS = 256` with its `#:` line, is the range check's top, and a test in
`scripts/tests/test_dispatch_shards.py` holds it equal to `RUN_JOBS` and holds exactly one
`ci.yml` job's axis on the plan's `matrix` output. A figure above 256 is refused by D2's range
check, by name, and never capped: it needs a second matrix, which is a new decision. On every run
`mutation-rust` remains the one leg job `LEG_CEILING`'s derivation sets apart, so that derivation
(`test_dispatch_shards.py:323-338`) reads `{"ci": 190, "battery": 234}` unchanged.
- What pins 190 or reads 256 per run, and how each stands: `LEG_CEILING`'s `#:` lines (`:959-960`)
  and `RUN_JOBS`'s (`test_dispatch_shards.py:217-218`) state the pull request run's rule, which
  still holds there, and are not edited; the new `#:` lines below `LEG_CEILING` state the release
  exception. The tests that read 190 (`test_dispatch_shards.py:283-300`, `:364-382`, `:399-410`;
  `test_mutation_verdict.py:1084`, `:1112`) size plans with no `release` key and stay green
  unedited. `ci.yml`'s plan comment (`:370-375`) states `LEG_CEILING`'s rule and is not edited.
  Rows S36201 and S36202 sit on `LEG_CEILING`'s values and S03913 on the pull request refusal's
  last line; none moves.
- The four tests that patch `LEG_CEILING["ci"]` to 193 and run `shards` through `main`
  (`scripts/tests/test_mutation_python_shard_binding.py:439`, `:667`, `:2008`, `:2199`) would read
  the placeholder 190 as below the range. Each gains one line holding `RUST_RELEASE_CEILING` at the
  same 193, beside its `LEG_CEILING` patch, every old line kept.

**D4. What stays.** `LEG_CEILING` (`:961`), `SHARD_BOUND_SECONDS` 10800 (`:958`),
`SECONDS_PER_MUTANT` (`:931`), `BASELINE_SECONDS` (`:948`), `CENSUS_SECONDS` (`:954`),
`MUTANT_TIMEOUT_SECONDS` 2300 and `TIMEOUT_MARGIN` (`:964-965`), `fewest_shards`'s scan (`:993`),
`size`, `RUN_JOBS` 256, `PYTHON_MAX_SHARDS`, `PYTHON_SHARE_CAP`, `PYTHON_LEG_BOUND_SECONDS`,
`PYTHON_MATRIX_LEGS`, `PYTHON_MATRICES`, `ci.yml` and every required context stay byte for byte.
`PYTHON_RELEASE_CEILING` keeps its signed figure unless D5's case holds. A need to change any of
them is not this decision's to meet.

**D5. Both figures move with the delivery, and the commit that sets them moves neither.** The
delivery's lines in `scripts/mutation-verdict.py` are in the release's merge diff, so they add
Python mutants to the release's listing. Counted by the listing tool's operators (one mutant for
each integer, string or boolean constant outside an f-string, each comparison operator, each
boolean pair, each arithmetic operator and each return of a value, on a changed line inside the
node's span; an f-string and a comment yield none), they add 29: the two constants 2,
`rust_ceiling_refused` 7, `rust_release_ceiling` 7, `main`'s return 1, and in `shards` 12 (the
ceiling's `or` and its key 2, the refusal's condition 3 and its return 1, the release line's
condition 1, the share's sum 1, and the budget line's four arithmetic nodes). The figures are
therefore read on the release's final union, after the delivery's last code commit. The commit that
sets the Rust figure edits two lines and no other: the constant's value and its `#:` line. If the
Python figure on that union reads above its signed figure, the same commit also edits
`PYTHON_RELEASE_CEILING`'s value and its `#:` line, and one new ruling names both figures. An
integer constant's line yields one mutant at any value and a `#:` line yields none, so that commit
adds no mutant, and a re-measure after it reads both figures unchanged. The delivery adds no Rust
mutant: it changes no `crates/*/src` path.

**D6. What the run prints.** On the release run under a set ceiling, three lines read it:
- a Rust release line, printed directly after SPEC-422 R6's Python release line and on every
  release run whose Rust ceiling is set, whether or not the Python legs were dealt by cost:
  `mutation: shards: release run: {count} rust leg(s) of ceiling {ceiling} (RUST_RELEASE_CEILING):
  {count} in mutation-rust, at most {RUST_MATRIX_LEGS} a matrix`;
- the Rust shards line's `legs N of ceiling C`, whose C is the release ceiling;
- the run-legs line, whose share, runner-minute budget and Rust denominator read the release
  ceiling in place of `LEG_CEILING["ci"]`; its Python denominator stays the share, as SPEC-422 left
  it.

SPEC-422 R6's Python release line is byte for byte, so #822's run shows both readings by name on
two adjacent lines. Every other run's lines and outputs are byte for byte; a test compares them
with literal golden lines at a set Rust ceiling.

**D7. The runbook and the tests.** `RELEASING.md` gains, insert-only, one sentence after the Rust
headroom paragraph's last line (`:63`) and one paragraph after the release-cutting paragraph's last
line (`:116`), before section 3. They name `RUST_RELEASE_CEILING`, `mutation-rust` and
`RUST_MATRIX_LEGS`, say a release listing past the ceiling is refused whole, by name, never
capped, and say a larger figure needs a new measurement and a new owner-signed ruling. They state
no figure and no number outside a SPEC or ADR id. The paragraph does not sit before
"**Cut a release before its range reaches the ceiling.**", since SPEC-422 A20 reads the lines
between the scoped-price paragraph and that heading as one paragraph. Each acceptance criterion of
SPEC-423 has a test read red before its code exists, structural and never a wall-clock time, in a
new module the mutation map names for `scripts/mutation-verdict.py`, so its tests judge the new
lines' mutants. The rows are S42300 to S42308, on printed texts, the runbook and two test-side
lines; none sits on a ceiling constant or its `#:` line. The red tests are pushed alone first, the
code, rows and red-first record next; the figure's commit is a later round after the final union is
measured, and the delivery lands only after the signed ruling file is on `dev`. The formal surface
is `formal/tla/EveryLegCounted`'s `shards` span: its hunks are re-read against the model and
recorded in its comments, and its one digest is re-stamped by the digest rule with the base as its
control. No new model and no model checker run; this repository's CI has no formal job.

**D8. One delivery, and insert-only amendments.** SPEC-423 and this ADR record the change.
SPEC-362 (R3's refusal, narrowed for the release run), ADR-373 (D1's ceiling, narrowed the same
way), SPEC-422 (its tests read the Python ceiling alone, and R6's line keeps its bytes) and ADR-436
(D4's Rust ceiling stays `LEG_CEILING`, and the release exception is this decision's) are amended
insert-only, each after its last line. `docs/schematics/mutation-testing.md` gains section 11,
insert-only.

## What it was chosen against

### D1

- A Rust-only predicate: rejected because it could disagree with the Python reading on the same run.
- Reading the event in `shards`: rejected because the plan already carries the one reading, and a second source needs its own failure arms.
- Applying the ceiling to the weekly battery's `size` too: rejected because the battery reads no plan and keeps `LEG_CEILING["battery"]`.

### D2

- Splitting the release: rejected because a release is every change `dev` carries since the last, so a split needs a second head into `main` or history `dev` never had (ADR-034's model).
- A one-time admission past the gate: rejected because it judges one release by another gate than the next, leaves no constant a test reads, and an admitted leg is a weakening.
- A global ceiling raise: rejected because it raises `LEG_CEILING["ci"]` for every run, and pull requests into `dev` keep 190.
- Headroom above the measured figure: rejected because headroom admits legs no measurement read.
- A placeholder of None: rejected because the range check would need an arm for it, and setting the figure would turn a line with no mutant into one with a mutant.
- A placeholder below 190: rejected because the range check refuses it on every run, which would stop every pull request's plan until the figure is set.
- A placeholder written as `LEG_CEILING["ci"]`: rejected because the cite test reads the constant as one integer assignment, as SPEC-422 A16 reads the Python one.
- A string constant holding the ruling's path: rejected because until the ruling is cut it names a file that does not exist, and it is a second line to set.
- A range floor fixed apart from `LEG_CEILING["ci"]`: rejected because it restates 190, a second copy of the pull request ceiling.
- A test or row pinning the constant's value: rejected because the signed figure would then change two lines, the value and its pin.

### D3

- A second matrix job holding the legs past 190, `[190, 2]`: rejected because it adds a job, an owed-leg reader, its verdict and `ci` wiring and more Python mutants, only to keep the first matrix at a figure the release run already passes on the per-matrix record.
- Raising `LEG_CEILING["ci"]` or `RUN_JOBS`: rejected because both stay byte for byte (D4), and the pull request's and the battery's ceilings are not this decision's.
- Reading `PYTHON_MATRIX_LEGS` as the Rust range's top: rejected because a Python name would then bound the Rust matrix, and the two constants answer to different jobs.
- A literal 256 in the range check: rejected because it restates the matrix limit outside a named constant a test holds equal to `RUN_JOBS`.
- Capping a figure above 256 to one matrix: rejected because a run is never capped; such a figure is refused by name and needs a new decision.
- Editing `ci.yml`'s plan comment: rejected because it states `LEG_CEILING`'s rule, which still holds for every other run, and every `ci.yml` edit is a path three open pull requests share.

### D4

- Lengthening `SHARD_BOUND_SECONDS` to need fewer legs: rejected because the bound is half the leg's job (SPEC-362 R2), and a longer bound lengthens the leg instead of sizing it.
- Lowering a price to need fewer legs: rejected because a price is read from recorded runs (SPEC-362 R10), never set to fit.

### D5

- Setting the constant in the build's first commit from an earlier measurement: rejected because the delivery's own lines add Python mutants, so that figure is stale before the delivery lands.
- A second ruling for the Python figure: rejected because both figures are read on one union, and one ruling takes one signature.
- Writing logic inside an f-string's fields to avoid mutants: rejected because the listing tool never mutates an f-string, so that logic would leave the population untested.

### D6

- Appending the Rust figure to SPEC-422 R6's own line: rejected because SPEC-422 R6 states that line byte for byte, its test helper asserts it whole up to its newline, and it prints only when the Python legs are dealt by cost, so a release dealt by the count rule would name no Rust figure.
- Leaving the run-legs line at 190: rejected because a release run would read `rust N of 190` with N past 190, which states a ceiling the run did not hold.
- Rewriting the pull request refusal's lines with a chosen clause: rejected because leaving them unedited keeps every other run's text byte for byte by construction.

### D7

- A paragraph before "**Cut a release before its range reaches the ceiling.**": rejected because SPEC-422 A20 reads that span as one paragraph with no blank line.
- The tests in `scripts/tests/test_dispatch_shards.py` alone: rejected because the mutation map does not name it, so its tests kill no mutant of the new lines.
- The tests in `scripts/tests/test_release_python_ceiling.py`: rejected because that module and its classes are SPEC-422's, and two SPECs' criteria in one module blur which delivery owns which pin.
- A wall-clock assertion: rejected because a time read on a loaded host proves nothing.
- Red and green in one push: rejected because a red is read only in CI, so the red tests run there alone first.

### D8

- A new SPEC that supersedes SPEC-362 and ADR-373: rejected because insert-only keeps every earlier byte, pin and row, and only the release run's ceiling changes.
- Leaving SPEC-422 and ADR-436 unamended: rejected because this delivery edits SPEC-422's test helper and adds the Rust exception ADR-436 D4 kept out, and each record must say so where a reader looks.

## Consequences

- The release pull request's run sizes its Rust legs up to `RUST_RELEASE_CEILING` once it is set,
  in the one `mutation-rust` matrix, and refuses past it whole, by name.
- Until the constant is set the release run's Rust legs are the pull request run's, so landing the
  code admits nothing by itself.
- Every other run reads exactly as before, the weekly battery included.
- A later release that needs more legs than the signed figure is refused and waits for a new
  measurement and a new signature; one that needs more than one matrix waits for a new decision.
- `ci`, the required contexts and every job name are unchanged.

## Confirmation

SPEC-423's acceptance criteria, each read red then green in CI, or recorded not red with its
reason. The rows in `scripts/mutation-rows.d/S42300-S42399.json`. The formal entry SPEC-423
section 8 names. The release run's own Rust and Python release lines, which must reproduce the
signed figures.

## What would make this wrong

- The matrix limit counted per run after all. Then SPEC-422 A19 fails on its record, one matrix
  past 190 cannot start, and the release needs another route.
- A release whose final union needs more legs than one matrix holds. The range check refuses such
  a figure, and a second Rust matrix is a new decision.
- A leg that waits past the platform's queue or run limits on a run this large. The verdict reads
  it VOID by name and `ci` reads red: closed, never a silent pass.
- The pull request event no longer carrying the head ref or repository. `release_run` then reads
  false and the release run is the pull request run, which refuses past 190: closed.

## More Information

SPEC-423 (the requirements and criteria); SPEC-422 and ADR-436 (the release mark, the Python
ceiling and the per-matrix record); SPEC-362 and ADR-373 (one run, its job budget); SPEC-290 (a
leg with nothing to examine is not started); ADR-034 (only `dev` reaches `main`). Issue #822 is
the release; #825 tracks the legs' fit; #691 the verdict across runs.
