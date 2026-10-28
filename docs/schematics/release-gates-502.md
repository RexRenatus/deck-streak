# Schematic: a release tag is published only when the scheduler's release gates are on file

Kind: data flow (the release job from the tag to the publish step, before and after the gate), with
the decision drawn as a flowchart. Read at DeckStreak `d3b09d5e` (`.github/workflows/release.yml`).
Decided by ADR-520; built by SPEC-502.

## 1. Before: every tag is published

At `d3b09d5e` the `release` job proves the tag (`release.yml:47-67`), builds, assembles the tarball
(`:127-149`), creates a draft (`:150-153`), attests (`:154-156`), uploads (`:157-160`) and publishes
in its last step (`:161-164`). Nothing between the upload and the publish reads any record.

```mermaid
flowchart TD
  tag["a SemVer tag on main"] --> guard["the tag guard"]
  guard --> build["the builds and the tarball"]
  build --> draft["gh release create --draft"]
  draft --> attest["the build-provenance attestation"]
  attest --> upload["gh release upload"]
  upload --> publish["publish, last: gh release edit --draft=false"]
```

## 2. After: the record at the tag decides the publish

One step joins directly before the publish. It reads `release-gates.json` from the tag's own
checkout and exits 0 only when the record is on file and every gate holds; a non-zero exit fails
the job, so GitHub runs no later step and the draft stays a draft.

```mermaid
flowchart TD
  tag["a SemVer tag on main"] --> guard["the tag guard"]
  guard --> build["the builds and the tarball"]
  build --> draft["gh release create --draft"]
  draft --> attest["the build-provenance attestation"]
  attest --> upload["gh release upload"]
  upload --> gate["python3 scripts/release_gates.py"]
  record[("release-gates.json at the tag's commit")] --> gate
  gate -->|"exit 0: every gate holds"| publish["publish, last: gh release edit --draft=false"]
  gate -->|"exit 1: REFUSED lines, one per gate"| stop["the job fails; the release stays a draft"]
```

## 3. The decision

`read_gates` turns the record into each gate's value, or none when the record lacks it or holds it
in another type. `verdict` then names every refusing gate in the record's order.

```mermaid
flowchart TD
  read["read_record: release-gates.json"] -->|"absent, not JSON, a key twice, not an object"| norecord["REFUSED: record (missing)"]
  read -->|"a JSON object"| gates["read_gates: each gate's value or none"]
  gates --> each{"for each gate, in order: model, dev.logloss, dev.rmse_bins, sealed_holds, worse_cells, simulator_win, permission_on_file, scheduler_gate_done"}
  each -->|"none"| missing["REFUSED: gate (missing)"]
  each -->|"on file, does not hold"| failed["REFUSED: gate (failed)"]
  each -->|"holds"| next["the next gate"]
  missing --> next
  failed --> next
  next --> done{"any refusal?"}
  done -->|"no"| pass["PASS, exit 0"]
  done -->|"yes"| refuse["exit 1"]
```

The Lean entry `lean/ReleaseGates` ports `verdict` and the per-gate helpers; its vectors,
`formal/vectors/release-gates.jsonl`, are read by `scripts/tests/test_release_gates.py`.
