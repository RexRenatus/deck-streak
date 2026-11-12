# Schematic: a gated Rust job is skipped only when the change touches nothing it reads

Kind: data flow (the map, the change list and the event into the decision and the admission, and
the Lean port tied to the script by vectors). Read at dev at the cut (the land of #1196). Decided
by ADR-658; built by SPEC-634.

## 1. The decision and the admission

`read_map` admits the map or refuses it; a refused map is None. `decide` answers for one job from
the event, the change list and the map, through `reads`, `reads_entry` and `prefix_of`. `admit`
judges each gated job's result by the path-gate job's success and its output. The caller, the
path-gate job and the `ci` aggregate, is a later delivery's, drawn dashed.

```mermaid
flowchart TD
  text[("scripts/read-map.json")] --> reader["read_map: the map, or None with why"]
  tracked[("the tree's tracked paths")] --> reader
  reader -->|"map or None"| decide["decide(event, job, changed, m)"]
  event["the event: name, base ref, release flag"] --> decide
  changed["changed: the merge commit against its first parent, renames split, or None"] --> decide
  decide --> readsfn["reads(m, job, q)"]
  readsfn --> entry["reads_entry(e, q)"]
  entry --> prefix["prefix_of(d, q)"]
  decide -->|"run or skip"| out["the gate's output for the job"]
  out --> admitfn["admit(needs, gate_ok, gate_out)"]
  needs["needs: each job and its result"] --> admitfn
  gateok["gate_ok: the path-gate job succeeded"] --> admitfn
  admitfn -->|"refused"| fail["the aggregate fails"]
  admitfn -->|"rest"| loop["the aggregate's existing loop"]
  caller["the path-gate job and the ci aggregate"] -.->|"a later delivery"| decide
  caller -.->|"a later delivery"| admitfn
```

## 2. The proof and its tie

`lean/PathGate` ports the five functions and proves two theorems over them. The vectors writer
prints the port's answers; the formal checker writes them to the vectors file, and the test reads
each one against the script.

```mermaid
flowchart TD
  script["scripts/path_gate.py"] -->|"five covers, each digest a span"| port["formal/lean/Formal/PathGate.lean"]
  port --> exact["the_gate_skips_exactly_the_changes_no_read_reaches"]
  port --> admits["a_skipped_job_passes_only_on_the_gates_own_skip"]
  port --> writer["formal/lean/Formal/PathGateVectors.lean"]
  writer -->|"the formal checker writes"| vectors[("formal/vectors/path-gate.jsonl")]
  vectors --> test["scripts/tests/test_path_gate.py"]
  script --> test
  test -->|"each vector is the gate's decision"| tie["O18 tied"]
```
