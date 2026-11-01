---
status: "accepted"
---

# ADR-436: the release run is named by a fail-closed predicate, and its Python legs pass the share only up to one signed ceiling, dealt over two matrices of at most 256

Decides SPEC-422. Path:
`docs/decisions/ADR-436-the-release-run-is-named-by-a-fail-closed-predicate-and-its-python-legs-pass-the-share-only-up-to-one-signed-ceiling-over-two-matrices.md`.
Amends ADR-373 and ADR-434 insert-only, and SPEC-362 and SPEC-290 through SPEC-422.

## Context

A release pull request, from `dev` into `main`, is judged on its merge diff: every Python mutant
`dev` gained since the last release, in one run (ADR-373, SPEC-362). Every run deals its Python
legs under one share, `PYTHON_MAX_SHARDS` 32, which no signature may raise past
`PYTHON_SHARE_CAP` 128 (`scripts/mutation-verdict.py:125`, `:127`; SPEC-420, ADR-434). A listing
whose legs pass the share is refused whole, by name, never capped: by the count rule when no listed
file is priced (`python_shards`, `:1052`; SPEC-415 R2) and by the cost deal when one is
(`python_by_cost`, `:1161`; SPEC-418 R4). So a release whose listing needs more legs than the
share admits cannot be judged at all, and the next release's range only grows.

An owner-signed ruling admits one exception and no other: the release pull request's mutation run
alone may deal Python legs above `PYTHON_SHARE_CAP`, up to a ceiling, with every mutant run, every
shard counted and no skipped-mutant record. The ceiling is the figure measured on the release's
final union, never an intermediate one, and a later run above it needs a new signature. Pull
requests into `dev` keep the share 32 and the cap 128.

Two facts of the tree shape the answer:

- A matrix generates at most 256 jobs. `RUN_JOBS` 256 (`scripts/tests/test_dispatch_shards.py:218`)
  holds that figure, and ADR-373 D1 read it per run, so that a second matrix in the same run would
  share the first one's budget (ADR-373, Considered Options). If it is counted per matrix instead,
  a second matrix adds legs. Nothing in the tree measures which reading holds.
- The applying delivery's own lines in `scripts/mutation-verdict.py` add Python mutants to the
  release's listing, so any figure measured before its last code commit is stale by construction.

## Decision Drivers

- Every listed mutant runs once, in a leg the verdict counts; nothing is sampled, skipped or capped.
- Only the release pull request's run changes. A pull request into `dev`, a push, a schedule, a
  dispatch and the weekly battery keep the share, the cap and both refusals byte for byte.
- An unreadable or ambiguous context is not the release run: the predicate fails closed.
- One constant carries the ceiling, and the tree cites the signed ruling file by path.
- The share, the cap, the leg bound, `RUN_JOBS`, `LEG_CEILING` and the Rust ceiling do not move.
- No required context changes, and the aggregate `ci` still reads every need's result.
- A public decision rests on a public measurement this repository makes and commits.

## Decisions

**D1. The release run is named by a fail-closed predicate in the plan.** A new function
`release_run(event, base_ref, head_ref, head_repo, repo) -> bool`, beside `scope_of`
(`scripts/mutation-verdict.py:195`), is true only when the event is `pull_request`, the base ref is
`main`, the head ref is `dev`, the repository's name is not empty and the head repository's name
equals it: the condition `base-is-dev` holds a pull request into `main` to
(`.github/workflows/ci.yml:944`). The plan step passes the head ref, the head repository's full
name and the repository's full name by `env`, never into the command's text, as `--head-ref`,
`--head-repo` and `--repo`: `--head-repo` and `--repo` default to the empty string and `--head-ref`
to no value, which SPEC-422 R1 reads as false exactly as it reads an empty one. `plan_diff` writes a
top-level `"release": true` only when the
predicate holds, and writes no key otherwise, so every existing plan and fixture stays byte for
byte. `shards` reads `plan.get("release") is True`: an absent key, `false`, a string or a number is
not the release run.

**D2. One ceiling constant, at the share cap until a signed measurement sets it.**
`PYTHON_RELEASE_CEILING = 128` follows `PYTHON_SHARE_CAP` (`:127`), with one `#:` comment line
directly above it. A helper returns the ceiling only for a release plan whose ceiling exceeds
`PYTHON_SHARE_CAP`, and None otherwise; under None the run takes the pull request path byte for
byte, the share 32 and both refusals. At the placeholder value the release run therefore admits no
leg past the share.
- A range check runs before any deal, as `python_share_refused` (`:1033`) does for the share: a
  ceiling that is not an `int`, is below `PYTHON_SHARE_CAP`, or is above
  `PYTHON_MATRIX_LEGS * PYTHON_MATRICES` is refused by name.
- A release listing whose legs pass the ceiling is refused whole, by name, with `EXIT_FAIL`. Its
  text names `PYTHON_RELEASE_CEILING` and says that a larger figure needs a new owner-signed
  ruling. The pull request refusal's text is byte-identical.
- The count rule (`python_shards`, a listing with no priced file) keeps the share on every run.
- The one commit that sets the constant also edits its `#:` line to cite the owner-signed ruling
  file by its path under `docs/rulings/`. A test holds the pair: at the cap no ruling path is
  cited; above it exactly one is, the file is in the tree, and its first line states the figure and
  the word `release`. A set ceiling therefore reads red until the signed ruling is in the branch.
- No test and no mutation row pins the constant's value, and the value is the measured figure with
  no headroom above it.
- This exception narrows SPEC-415 R2 and SPEC-418 R4, which refuse a listing past the share on
  every run, for the release run under a set ceiling only. Neither SPEC is amended; this decision
  and SPEC-422 name both by id.

**D3. Matrices of at most 256: the release run's legs past 256 go to a second matrix job.** Two
constants, `PYTHON_MATRIX_LEGS = 256` and `PYTHON_MATRICES = 2`, each with its `#:` line. For a
deal of n Python legs the plan writes four outputs: `python_shards` n, the total;
`python_matrix` the shards 0 to min(n, 256) - 1; `python_release_matrix` the shards 256 to n - 1,
or `[]`; and `python_release_legs` max(0, n - 256).
- A new `ci.yml` job, `mutation-python-release`, is `mutation-python` with two changes: the
  job-level condition `if: ${{ needs.mutation-plan.outputs.python_release_legs != '0' }}` and the
  axis `fromJSON(needs.mutation-plan.outputs.python_release_matrix)`. Each leg runs the same
  `--shard "$SHARD/$SHARDS"` over the total and keeps its report as `mutation-python-shard-<k>`.
- The verdict's merged download of `mutation-python-shard-*` and `python_reports`' count of shards 0
  to n - 1 (`:1839`) read both matrices unchanged; `scripts/mutation_python.py` needs no edit.
- `mutation-verdict` and `ci` each need the new job. `ci` admits its skip once, beside
  `mutation-rust` and `mutation-rows`, and the verdict's `legs` step passes
  `--python-release-leg`. `legs` (`:2186`) gains an optional arm: the leg is owed exactly when the
  plan's Python count passes 256; owed and judged `skipped`, or owed with no argument, reads VOID by
  name; not owed and no argument prints nothing, so every existing output stays byte for byte.
- The required contexts do not change, and no ruleset is edited.
- On every run but the release run the new job generates no leg, so the job-budget test counts its
  axis at max(0, `PYTHON_MAX_SHARDS` - 256), that is 0, and `LEG_CEILING` stays
  `{"ci": 190, "battery": 234}`. A second test holds each Python matrix of the release run within
  `RUN_JOBS` and `PYTHON_MATRIX_LEGS` equal to it.
- The per-matrix reading rests on this delivery's own measurement: a new dispatch-only workflow,
  `.github/workflows/matrix-limit.yml`, run once at 256 legs a matrix. Its `count` job records each
  matrix's legs, each leg's `strategy.job-total`, every job's result and the run's job count; the
  record is committed as `scripts/tests/fixtures/matrix-limit-<run>/`, and a test reads it. ADR-373
  D1's per-run reading is amended insert-only on that record. A record that shows the limit per run
  fails that test, and this decision fails with it: the delivery does not land.

**D4. What stays.** `PYTHON_MAX_SHARDS` 32 (`:125`), `PYTHON_SHARE_CAP` 128 (`:127`),
`PYTHON_LEG_BOUND_SECONDS` 1800 (`:130`), `RUN_JOBS` 256, `LEG_CEILING` (`:925`) and the Rust
ceiling stay byte for byte. A need to change any of them is not this decision's to meet.

**D5. The figure moves with the delivery, and the constant's commit moves no figure.** The
delivery's own new lines add Python mutants, so the release's figure is read on its final union,
after its last code commit. The commit that sets the ceiling edits two lines and no other: the
constant's value and its `#:` line. The listing tool makes one integer-literal mutant of a constant
line, its value plus one, at 128 and at any other figure alike, and a `#:` comment line yields
none, so that commit adds no mutant, and a re-measure after it reads the figure unchanged.

**D6. The runbook says the method, never the figure.** `RELEASING.md` gains, insert-only, one
sentence after the share and cap paragraph's last line (`:73`) and one paragraph between the scoped
price paragraph (ending `:98`) and "**Cut a release before its range reaches the ceiling.**"
(`:100`). They name `PYTHON_RELEASE_CEILING`, `mutation-python-release` and
`PYTHON_MATRIX_LEGS`, say a release listing past the ceiling is refused whole, by name, never
capped, and say a larger figure needs a new measurement and a new signature. They state no figure
and no number outside a SPEC or ADR id, and cite no ruling number and no private record.

**D7. The tests are structural and red first, and the formal stamps move by rule.** Each
acceptance criterion of SPEC-422 has a test read red before its code exists, and none asserts a
wall-clock time. The rows are S42200 to S42218, on the workflow lines and printed texts SPEC-422
section 9 names; none sits on the ceiling constant or its comment. The red tests are pushed alone
first, the code and rows next, and the measurement record after the dispatched run; the
constant's commit is a later, separate step after the final union is measured, and the delivery
lands only after the signed ruling file is on `dev`. The formal surface is
`formal/tla/EveryLegCounted`'s `shards` and `python_by_cost` spans: each hunk is re-read against
the model and recorded in its comments, and the two digests are re-stamped by the digest rule with
the base as its control. The second matrix job adds no action: its legs are the model's per-leg
actions for shards 256 and up, and an owed leg skipped is a leg with no report, which the model
already reads as void. No new model, and no model checker run.

**D8. One delivery, and insert-only amendments.** SPEC-422 and this ADR record the change. ADR-373
(D1, read per matrix on D3's record), SPEC-362 (R3 and section 5's "no second matrix"), SPEC-290
(R8's admitted legs) and ADR-434 (D1's per-matrix question, decided for the release run alone) are
amended insert-only, each after its last line. `docs/schematics/mutation-testing-python.md` gains
section 9. SPEC-420, SPEC-421 and ADR-435 are not amended, since no claim of theirs is
contradicted, and SPEC-415 and SPEC-418 are named by id, not amended (D2).

## What it was chosen against

### D1

- A condition written in `ci.yml` alone: rejected because the listing tool mutates the Python
  predicate and a table test reads it, where a workflow expression is reached by hand rows only.
- Reading the event file the runner writes: rejected because it is a second source beside the
  arguments, with its own failure arm (an absent file, unreadable JSON) to test and to fail closed.
- Folding the release case into `scope_of`: rejected because it changes a return shape that pinned
  callers and their tests read.
- Making `mutation-plan` need `base-is-dev`: rejected because a failed guard would skip the plan and
  cascade skips through every leg that the `ci` admission was never written to read; the predicate
  repeats the guard's condition instead.

### D2

- Splitting the release: rejected because a release is every change `dev` carries since the last,
  so a split needs a second head into `main` or history `dev` never had (ADR-034's model).
- A one-time admission past the gate: rejected because it judges one release by another gate than
  the next, leaves no constant a test reads, and an admitted leg is a weakening.
- A global cap raise: rejected because it raises the ceiling of every run, and pull requests into
  `dev` keep the share 32 and the cap 128.
- Headroom above the measured figure: rejected because headroom admits legs no measurement read.
- A placeholder of None, 0 or 32: rejected because None needs an arm the range check must
  special-case, and 0 and 32 sit below the cap, which the range check refuses; at 128 the helper's
  "above the cap" test alone keeps the pull request path.
- A string constant holding the ruling's path: rejected because until the ruling is committed it
  names a file that does not exist, a placeholder with no meaning and a second line to edit.
- Reading the ruling file at plan time: rejected because the plan would then depend on a
  document's prose at run time, with a new failure arm, and the figure would live in two places.
- A test or row pinning the constant's value: rejected because the signed figure would then change
  two lines, the value and its pin.

### D3

- One matrix past 256 legs: rejected because a matrix generates at most 256 jobs, which `RUN_JOBS`
  holds per matrix.
- Raising `RUN_JOBS` or `LEG_CEILING`: rejected because both stay byte for byte (D4), and the pull
  request's and the battery's shares are not this decision's.
- Several runs counted by one verdict: rejected for ADR-373's own reasons: a job in another run
  cannot be a need of `ci`, and the pull request's token cannot start runs.
- A third or further matrix job now: rejected because two matrices hold `PYTHON_MATRIX_LEGS * 2`,
  the range check refuses a ceiling past it, and a test holds `PYTHON_MATRICES` equal to the
  workflow's Python matrix jobs.
- A second artifact family for the release legs: rejected because the merged download and
  `python_reports` already read `mutation-python-shard-<k>` for every k, and a second pattern is a
  second thing to keep in step.
- Requiring the new job by name: rejected because a matrix job reports one check per leg, never its
  bare name, and `ci` already carries every need's result.
- Citing a measurement this repository did not make: rejected because a public decision rests on a
  run whose record is committed beside the test that reads it.

### D4

- Raising the share for every run: rejected because pull requests into `dev` keep it, and the
  job-budget test reads red above 32 unless the Rust share falls.
- Lengthening `PYTHON_LEG_BOUND_SECONDS` to need fewer legs: rejected because a leg's bound is half
  its job (SPEC-418 R1), and a longer bound lengthens the leg instead of sizing it.

### D5

- Setting the constant in the build's first commit from an earlier measurement: rejected because
  the delivery's own lines add mutants, so that figure is stale before the delivery lands.
- Carrying the ruling's path in a second constant: rejected because it is a second value line for
  the setting commit to edit, with no meaning until the ruling exists (D2).

### D6

- Rewriting the runbook's share, refusal and cap lines: rejected because insert-only keeps every
  earlier byte and every test that reads them.
- Stating the figure in the runbook: rejected because the figure lives in one constant and one
  signed file, and a third copy drifts from both.

### D7

- A wall-clock assertion: rejected because a time read on a loaded host proves nothing, and the
  plan's legs, outputs and texts are read structurally.
- Red and green in one push: rejected because a red is read only in CI, so the red tests must run
  there alone first.
- Setting the constant within the build's own pushes: rejected because the figure is read on the
  final union after the last code commit (D5).
- A new model, or a model checker run: rejected because the model's actions and properties do not
  change; only the two spans' digests move.

### D8

- Amending SPEC-415 and SPEC-418 as well: rejected because their own amendments touched only the
  release owners, and naming both rules by id here carries the narrowing.
- A new SPEC that supersedes SPEC-362 and ADR-373: rejected because insert-only keeps every earlier
  byte, pin and row, and only D1's reading changes.

## Consequences

- The release pull request's run deals its Python legs up to `PYTHON_RELEASE_CEILING` once the
  constant is set, in at most two matrices, and refuses past it whole, by name.
- Until the constant is set, the release run is the pull request run, so landing this delivery's
  code admits nothing by itself.
- A fork whose branch is named `dev`, a push, a schedule, a dispatch and a pull request into `dev`
  are not the release run, and read exactly as before.
- `ci` gains one need and one admitted skip; the required contexts are unchanged.
- A later release that needs more legs than the signed ceiling is refused and waits for a new
  measurement and a new signature.
- The head ref is read from the event and passed by `env`, never placed into a command's text, so
  a branch name cannot inject a command.

## Confirmation

SPEC-422's acceptance criteria A1 to A21, each read red then green in CI, or recorded not red with
its reason. The rows in `scripts/mutation-rows.d/S42200-S42299.json`. The committed record of the
measurement run under `scripts/tests/fixtures/matrix-limit-<run>/`. The formal entry SPEC-422
section 8 names. The release run's own plan line, which must reproduce the signed figure.

## What would make this wrong

- The 256-job limit holds per run. Then a second matrix adds no leg, the measurement record fails
  its test, and the release needs another route.
- A release whose final union needs more legs than two matrices hold. The range check refuses such
  a ceiling, and a third matrix job is a new decision.
- The pull request event no longer carries the head ref or the head repository. The predicate then
  reads false, and the release run is the pull request run, which fails closed.
- A matrix whose axis is empty and whose condition is false being reported as a failure. Then the
  new job's skip on every other run reads red, and the condition, not the axis, must carry it.

## More Information

SPEC-422 (the requirements and criteria); ADR-373 and SPEC-362 (one run, its job budget);
SPEC-290 (a leg with nothing to examine is not started); SPEC-415 R2 and SPEC-418 R4 (the
refusals past the share, narrowed for the release run by D2); SPEC-420 and ADR-434 (the share and
its cap); ADR-034 (only `dev` reaches `main`). Issue #822 is the release; #825 tracks the Python
legs' fit.

## Amendment: D4's Rust ceiling, and the release run's Rust legs (ADR-437)

Insert-only; every earlier byte is kept. D4 kept `LEG_CEILING` and the Rust ceiling byte for byte,
and that still holds: `LEG_CEILING` stays `{"ci": 190, "battery": 234}` and every run but the
release run reads it. ADR-437 meets the release run's Rust need apart from this decision: a
constant of its own, `RUST_RELEASE_CEILING`, read from D1's `"release": true` mark with no second
predicate, in the one `mutation-rust` matrix, refused past it whole and by name, and cited to an
owner-signed ruling as D2 cites this decision's. D5's rule holds for both figures: the delivery
that adds the Rust constant adds Python mutants, so both figures are read on the release's final
union, and the one commit that sets them adds none.

Chosen against:

- Widening this decision's ceiling to hold the Rust legs too: rejected because the Python and Rust legs are dealt by different rules into different jobs, and one constant cannot carry two figures.
- Leaving D4 unamended: rejected because a reader of D4 would find the Rust ceiling unchanged and no pointer to the release run's own Rust constant.
