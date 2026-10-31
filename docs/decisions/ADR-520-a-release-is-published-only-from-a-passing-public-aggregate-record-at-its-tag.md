# ADR-520: A release is published only from a passing public aggregate record at its tag

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-502 |
| builds on | ADR-062 (the release workflow publishes last; none amended), ADR-514 (the formal ledger; its O17 row moves to `tied`) |
| measured at | `d3b09d5e` |

## Context

DeckStreak's releases are cut from SemVer tags on `main` by `release.yml`, which builds once,
uploads to a draft and publishes in its last step (`release.yml:161-164`). Once the scheduling
companion schedules the owner's deployment, a release must wait until the scheduler's release gates
are met: it beats FSRS-7 on the full development set by log loss and by RMSE(bins), the result holds
on the sealed set, no scorecard cell is worse, it wins the offline simulator, written permission to
ship its trained weights is on file, and its own final release check has run. If any gate fails,
the release is delayed, not shipped. At `d3b09d5e` nothing in the tree records those outcomes and
nothing reads them: every tag is published.

## Considered Options (the alternatives it was chosen against)

- **A record in the tree, read by a standard-library script in one step directly before the
  publish** (chosen): the gates are judged from the tag's own commit, the decision is a pure
  function a test and a Lean entry can hold, and only the publish is refused.
- **A protected environment with a required reviewer on the publish**: lost, because the decision
  would be a person's click with no record in the tree, nothing a test could fail on, and nothing
  the public could read.
- **The gate before the draft or the build**: lost, because a refused release would then build
  nothing, while the design keeps the lane building and uploading so that only its publish waits.
- **The record as a release asset, a variable or a secret**: lost, because it would not be at the
  tag's commit, would not be public, and could change after the tag was cut.

## D1. The record and its schema

The record is one JSON object at the repository root, `release-gates.json`, read at the tag's
commit. Its keys and types are SPEC-502 R1's: `model` is `scheduler_weights_sha256`, the sha256 of
the scheduler's weights file the figures were measured on (64 lowercase hex); `dev.logloss` and
`dev.rmse_bins` are pairs [the scheduling companion, FSRS-7] of finite numbers; `sealed_holds`,
`simulator_win` and `scheduler_gate_done` are booleans; `worse_cells` is an integer; and
`permission_on_file` is `{"value": boolean, "date": "YYYY-MM-DD" or null}`. A value of another
type is not on file.

## D2. Why the record is a public aggregate

The record holds aggregate figures only: two log losses, two RMSE(bins) values, one count and four
booleans, plus the digest that names the weights file and one date. It holds no review, card, deck,
user, device or timing, and no figure from which one could be recovered; the weights file itself is
named by its digest and never enters the tree. A public evaluation summary of aggregate figures is
what this repository may publish, and it lets anyone check from the tag alone why a release was or
was not published.

## D3. The decision, and each refusal by its name

A release is published exactly when the record is on file and every gate holds (SPEC-502 R2).
The decision names every refusing gate, not the first: a gate the record lacks, or holds in another
type, refuses as `missing`, and a gate on file that does not hold refuses as `failed`, in the
record's order. A missing record (absent, not UTF-8 JSON, a key twice in one object, or not an
object) refuses as `record` and no gate is judged. A non-finite number is no number, so it refuses
as missing and never compares. The reader (`read_gates`) and the decision (`verdict`) are separate
functions, so the decision is a pure function of each gate's value or its absence.

## D4. One step directly before the publish

The step is `python3 scripts/release_gates.py`, one line with a name and a run and nothing else.
It has no `if:` and no `continue-on-error`, holds no token and sets no variable, so a refusal fails
the job at that step and GitHub runs no later step: the publish is unreached, and the draft with
its uploads and attestation stays a draft. No other step of the workflow changes.

## D5. Lean holds the decision; its vectors tie it to the script

The decision decides a release, so `lean/ReleaseGates` ports `verdict`, `refused` and the four
per-gate helpers and proves that the verdict is empty exactly when every gate holds, and that each
refusal names its gate once with why. The figures are of any type with the `<` the script applies.
The vectors the formal checker writes are read by `scripts/tests/test_release_gates.py`, which runs
each vector's record through the script and compares the refusals; that test is O17's tie in the
formal ledger.

## D6. Later: the record admitted only from an owner-signed commit

The record should be read only when the commit that last changed it is signed by a key the owner's
signers file names, as the owner's rulings are. That requirement is recorded and not built here;
until it is, the record is trusted as committed and reviewed like any change to `main`.

### Consequences

- Good, because a tag can no longer be published past a failing or absent gate, and the reason is
  printed by name in the run's log and readable by anyone in the tag's tree.
- Good, because the decision is one small pure function with a Lean proof and vectors the test
  reads, so a later edit that widens it fails a check.
- Bad, because every tag is refused until a passing record is filed, so the first release after
  this lands waits on that filing.
- Bad, because a refused tag leaves a draft behind, and its release needs a new tag once the record
  is filed.

### Confirmation

`scripts/tests/test_release_gates.py` (SPEC-502 A1 to A6), `scripts/tests/test_release_workflow.py`
unchanged, `scripts/tests/test_formal_obligations.py` (O17 tied), and `phxd formal check --kind lean
--gate` over `lean/ReleaseGates`.
