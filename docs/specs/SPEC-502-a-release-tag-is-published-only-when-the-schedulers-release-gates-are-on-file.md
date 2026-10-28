# SPEC-502: a release tag is published only when the scheduler's release gates are on file

Path:
`docs/specs/SPEC-502-a-release-tag-is-published-only-when-the-schedulers-release-gates-are-on-file.md`.

- **Issue:** refers to #822, the release pull request from `dev` into `main`; closes nothing.
- **Context(s):** the release workflow's `release` job (`.github/workflows/release.yml`), one new
  script `scripts/release_gates.py` with its test, one new Lean entry under `formal/lean/`, and the
  formal ledger (`formal/obligations.json` and its test). No crate, web, deploy or iOS change.
- **Decided by:** ADR-520 (the record, why it is a public aggregate, the decision and where the
  workflow runs it).
- **Schematic:** `docs/schematics/release-gates-502.md` (data flow: the tag, the record, the
  decision and the publish step).
- **Mutation band:** `S50200-S50299`, reserved; this delivery adds no rows file (section 9).
- **Red-first record:** `docs/red-first/SPEC-502.md`.
- **Scope:** every release tag after `v0.3.0`'s. The delivery lands after `v0.3.0` is tagged.
- **Status:** delivered by the pull request that adds this file, with its tests, its Lean entry,
  its vectors and its red-first record.

## 1. The problem, measured

Every figure below was read at the wave base `d3b09d5e`.

- The release job publishes in its last step, `publish, last` (`release.yml:161-164`,
  `gh release edit "$GITHUB_REF_NAME" --draft=false`), after the tag guard, the builds, the draft,
  the attestation and the uploads. No step reads any record of the scheduler's evaluation:
  `grep -c 'release_gates' .github/workflows/release.yml` reads 0. So a tag is published whether
  or not the scheduler's release gates hold.
- The formal ledger names the property and no artifact for it:
  `formal/obligations.json`'s O17, "The release gates admit a release only when every gate holds",
  is `planned`, its one artifact `lean/ReleaseGates` is `planned`, and it has no tie.
  `formal/lean/Formal/ReleaseGates.lean` is absent (`test -e` reads 1).
- `phxd formal check --kind lean --gate` at the base reads `FORMAL OK exit=0` over 16 entries and
  45 properties.

## 2. Requirements

R1. **The record.** The release gates are on file as one JSON object at the repository root,
`release-gates.json`, read at the tag's own commit. Its keys:

| key | type | meaning | the gate holds when |
|---|---|---|---|
| `model` | string, 64 lowercase hex | `scheduler_weights_sha256`: the sha256 of the scheduler's weights file the figures were measured on | it is on file |
| `dev.logloss` | `[number, number]` | log loss on the full development set, as [the scheduler, FSRS-7] | the scheduler's is below FSRS-7's |
| `dev.rmse_bins` | `[number, number]` | RMSE(bins) on the full development set, as [the scheduler, FSRS-7] | the scheduler's is below FSRS-7's |
| `sealed_holds` | boolean | the development result holds on the sealed set | true |
| `worse_cells` | integer | the scorecard cells on which the scheduler is worse than FSRS-7 | 0 |
| `simulator_win` | boolean | the scheduler wins the offline simulator | true |
| `permission_on_file` | `{"value": boolean, "date": "YYYY-MM-DD" or null}` | written permission to ship the trained weights is on file, and since when | `value` is true |
| `scheduler_gate_done` | boolean | the scheduler's own final release check has run | true |

Each pair is [the scheduling companion, FSRS-7]. A number is a finite JSON number; a boolean is
never a number and a number is never a boolean. The record at its documented shape, every gate on
file and none holding:

```json
{
  "model": "0000000000000000000000000000000000000000000000000000000000000000",
  "dev": { "logloss": [0.0, 0.0], "rmse_bins": [0.0, 0.0] },
  "sealed_holds": false,
  "worse_cells": 0,
  "simulator_win": false,
  "permission_on_file": { "value": false, "date": null },
  "scheduler_gate_done": false
}
```

R2. **The decision.** A release is published exactly when the record is on file and every gate of
    R1 holds: the model is on file, `dev.logloss[0] < dev.logloss[1]`, `dev.rmse_bins[0] <
    dev.rmse_bins[1]`, `sealed_holds`, `worse_cells = 0`, `simulator_win`,
    `permission_on_file.value` and `scheduler_gate_done`.
R3. **Each gate refuses by its name.** A gate the record lacks, or holds in another type than R1's,
    refuses as `missing`; a gate on file that does not hold refuses as `failed`. Every refusing
    gate is named, one line each, in R1's order, never only the first:
    `release-gates: REFUSED: <gate> (<missing|failed>)`, then the exit is 1.
R4. **A missing record refuses.** A record that is absent, is not UTF-8 JSON, holds a key twice in
    one object, or is not a JSON object refuses as `record (missing)`, with the cause, and the exit
    is 1. No gate is judged without a record.
R5. **The script.** `scripts/release_gates.py` uses the standard library alone, reads only the
    record at `<root>/release-gates.json` (`--root` defaults to the checkout that holds the
    script), writes nothing and starts no process. On admission it prints
    `release-gates: PASS: every gate holds for the scheduler's weights <model>` and exits 0; a
    usage error exits 2.
R6. **The publish step is unreached on a refusal.** `release.yml`'s `release` job gains one step
    directly before `publish, last`: `python3 scripts/release_gates.py`, one line, with a name and
    a run and nothing else (no `if:`, no `continue-on-error`, no `env`, no token). A refusal fails
    the job there, so the publish step never runs; the draft and its uploads stay a draft.
R7. **Every other release step is unchanged.** The workflow's diff against the base adds lines only,
    all in that one step, and the release's existing tests pass unchanged.
R8. **Lean.** `formal/lean/Formal/ReleaseGates.lean` ports `verdict`, `refused` and the four
    per-gate helpers branch for branch, covers each by its anchor and `read_gates` as the input it
    reads, and proves two theorems at `ramp=report` (section 8). Its vectors,
    `formal/vectors/release-gates.jsonl`, are written by the formal checker, never by hand, and
    the release lane's test reads every one against the script.
R9. **The ledger.** O17 reads `tied`: its two theorems are present artifacts, and one vectors tie
    names the test of R8. `lean/ReleaseGates` gains its row (`keep`, linked to O17), and the
    ledger's test follows: 44 rows, O17's present names and its tie.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the record's schema: the documented record above is read with every gate on file, a passing record admits, and each value of another type than R1's reads as that gate missing (R1, R5) | `test_release_gates.py` `TheRecordSchema` |
| A2 | each missing or failed gate refuses by its name, alone or all together, in R1's order, exit 1 (R2, R3) | `test_release_gates.py` `EachGateRefusesByName` |
| A3 | a missing, unreadable or non-object record refuses as `record`, exit 1 (R4) | `test_release_gates.py` `AMissingRecordRefuses` |
| A4 | the Lean entry checks clean, and its vectors, read by the lane's test, equal the script's verdicts; the ledger reads O17 tied (R8, R9) | `test_release_gates.py` `TheLeanVectors`; `test_formal_obligations.py`; `phxd formal check --root . --base <base> --kind lean --gate --format json` |
| A5 | a planted failing or missing record leaves the publish step unreached, and a passing one reaches it (R6) | `test_release_gates.py` `AFailingRecordLeavesPublishUnreached` |
| A6 | every other release step is unchanged (R7) | `test_release_gates.py` `EveryOtherReleaseStepIsUnchanged`; `test_release_workflow.py`; `git diff --numstat <base> -- .github/workflows/release.yml` reads added lines only |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_gates.py -k TheRecordSchema
A2: python3 -m unittest discover -s scripts/tests -p test_release_gates.py -k EachGateRefusesByName
A3: python3 -m unittest discover -s scripts/tests -p test_release_gates.py -k AMissingRecordRefuses
A4: python3 -m unittest discover -s scripts/tests -p test_release_gates.py -k TheLeanVectors
A4: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py
A5: python3 -m unittest discover -s scripts/tests -p test_release_gates.py -k AFailingRecordLeavesPublishUnreached
A6: python3 -m unittest discover -s scripts/tests -p test_release_gates.py -k EveryOtherReleaseStepIsUnchanged
A6: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py
```

Each test prints the population it examined as `examined N`, and N is above 0.

## 4. File manifest

| path | change |
|---|---|
| `scripts/release_gates.py` | new: the reader, the decision and the command; standard library only |
| `scripts/tests/test_release_gates.py` | new: A1 to A6 |
| `scripts/mutation-python.json` | one entry: `scripts/release_gates.py` and its test module |
| `.github/workflows/release.yml` | one step added directly before `publish, last` |
| `formal/lean/Formal/ReleaseGates.lean` | new: the entry (section 8) |
| `formal/lean/Formal/ReleaseGatesVectors.lean` | new: its vectors writer |
| `formal/lean/Formal/Vectors.lean` | one import and one arm |
| `formal/vectors/release-gates.jsonl` | new: written by the formal checker, never by hand |
| `formal/obligations.json` | O17's artifacts present, its tie, its status `tied`; the `lean/ReleaseGates` row |
| `scripts/tests/test_formal_obligations.py` | the ledger's expectations: O17 tied and present, its tie, a committed Python tie, 44 rows |
| `docs/specs/SPEC-502-a-release-tag-is-published-only-when-the-schedulers-release-gates-are-on-file.md` | new: this file |
| `docs/decisions/ADR-520-a-release-is-published-only-from-a-passing-public-aggregate-record-at-its-tag.md` | new |
| `docs/schematics/release-gates-502.md` | new |
| `docs/red-first/SPEC-502.md` | new |
| `changelog.d/release-gates-502.md` | new fragment |

## 5. What this does NOT do

- It commits no record. Once this lands, every release tag is refused at its publish step until a
  passing `release-gates.json` is on file at the tag's commit (#822).
- It does not admit the record only from an owner-signed commit. That is a later requirement: the
  record would be read only when the commit that last changed it carries a signature the owner's
  signers file names. Until then the record is trusted as committed (#822).
- It gates only the publish step. The tag guard, the builds, the draft, the attestation and the
  uploads run as before, so a refused tag leaves a draft (#822).
- It changes no other workflow: the Apple and TestFlight lanes are untouched (#822).
- No hosted run proves the step: A5 runs the gate and publish steps cut from `release.yml` under
  bash, with `gh` stubbed, in a planted tree (#822).

## 6. Risks

- **A record that misstates its figures** publishes. The record is an aggregate the owner files; the
  signed-commit requirement of section 5 is the remedy, and until then review of the commit that
  changes `release-gates.json` is the control.
- **A refused tag stays a draft.** The record is read at the tag's commit, so a re-run of the same
  tag reads the same record; the release that follows a filed record is a new tag.
- **A non-finite figure** (`NaN`, `Infinity`, which Python's JSON reader accepts) is no number, so
  its gate reads missing and refuses, never compares.
- **A key spelled otherwise** reads as that gate missing and refuses; it never admits.

## 7. What only CI proves

The step's place in a hosted run, and the refusal stopping a real publish, are proved by no run of
this delivery: A5 runs the two steps' own text, cut from `release.yml`, under GitHub's default bash
with a stub `gh` that records its arguments, and asserts the stub never receives `--draft=false`
after a refusal.

## 8. Formal

The function decides a release, so a Lean entry, `formal/lean/Formal/ReleaseGates.lean`, ports
`release_gates.py`'s `verdict`, `refused`, `present_refusal`, `pair_refusal`, `flag_refusal` and
`count_refusal` branch for branch, and covers each and `read_gates` by its anchor. The figures are
of any type with the `<` the script applies, so each claim holds for every instance of it. Its
theorems, each entering at `ramp=report`:

- `a_release_is_published_only_when_every_gate_holds`: the verdict is empty exactly when the record
  is on file and every gate of R2 holds.
- `each_refusal_names_its_gate_and_why`: each gate is named at most once, and a gate is named
  exactly when it refuses (only `record` when no record is on file), as `missing` exactly when it
  is not on file in its type.

Each witness ports the base's behaviour: at `d3b09d5e` the publish step (`release.yml:161-164`)
runs for every tag, so the base's verdict is always empty, and with no record on file it publishes
and names nothing. The vectors cover a missing record, a passing one, each gate missing and failing
alone, the boundary of an equal pair, a negative count, every gate missing and every gate failing.

## 9. Mutation rows (S50200-S50299)

None in this delivery. The generated mutants of `scripts/release_gates.py` run in the Python
population (`scripts/mutation-python.json` maps it to `test_release_gates`), and the workflow step
has no line a row would mutate that A5 and A6 do not already refuse: the step's run, its place, and
the absence of `if:` and `continue-on-error`.
