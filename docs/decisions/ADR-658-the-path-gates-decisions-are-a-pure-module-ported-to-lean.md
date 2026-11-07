---
status: "accepted"
---

# ADR-658: the path gate's decisions are a pure module ported to Lean

## Status

Accepted, with SPEC-634.

## Context

`ci.yml` runs every job on every pull request, whatever the change touches: its triggers carry no
paths filter (`ci.yml:14-19`), and the `ci` job needs 21 jobs (`ci.yml:1415`). The path gates will
skip a gated Rust job (`rust`, `engine`, `release` or `census`) on a pull request into dev when the
change touches nothing the job reads, and the aggregate must then admit that skip. A skip decided
wrongly passes a change no job built, so the decision decides a gate verdict, and a gate verdict
takes a Lean proof. The aggregate today admits a skipped result only by count, one admission for
each skipped mutation leg (`ci.yml:1462-1476`), so a skipped gated job would read as a failure, or,
on a run where a leg was skipped, could use that leg's admission.

## Decision

- D1. The decisions are pure functions in one standard-library module, `scripts/path_gate.py`:
  `prefix_of`, `reads_entry`, `reads`, `decide` and `admit`, each a top-level function, beside the
  map's reader. The module has no command line and no caller; the delivery that wires the path
  gates adds both. Each function is ported branch for branch to `lean/PathGate` over text read as
  characters, covered by its anchor, and tied to the script by vectors the formal checker writes and
  the script's test reads.
- D2. The map's meaning is stated declaratively, as `InMap`, from the map's schema alone: an entry
  reads a path when it is `*`, the path itself, or ends in `/` and begins the path; `always` joins
  every job's reads; a job with no entry reads every path; a refused map has no member. It calls no
  port function, so a port that drops `always`, ignores `*` or loses the `/` boundary no longer
  proves.
- D3. The gate's claim is exact, one theorem with two conjuncts: `reads` answers exactly `InMap`,
  and `decide` skips exactly a pull request into dev whose change list is readable and non-empty, on
  a parsed map, when no changed path is read. Soundness, a skip only when the change is disjoint
  from the map, is the left-to-right half; the right-to-left half is merged into the same claim
  because the base, which runs every job, already satisfies soundness alone, and a claim the base
  satisfies has no witness.
- D4. The admission is decided per gated job: a result passes when it is `success`, or `skipped`
  while the path-gate job succeeded and its output for that job reads `skip`. It replaces the count
  for the gated jobs; every other job goes on to the existing loop with its result.
- D5. The reader refuses a key named twice in one object, so the script's map (the last value
  wins) and the port's association list (the first wins) never differ on an admitted map, and it
  refuses a prefix written without its `/` by the tree it is read in: an entry names a directory
  when some tracked path begins with it and `/`.

## Options rejected

| option | why not |
|---|---|
| a `paths` filter on each gated job | a filtered-out required job never reports, so the aggregate cannot tell a skip from a job that never ran, and no test or proof holds the filter |
| `InMap` written through the ported `reads` | the theorem would then prove the port equal to itself; a port that dropped `always` would still prove |
| soundness alone as the theorem | the base already satisfies it (it never skips), so it has no witness, and the registry refuses a theorem without one |
| keeping the count-based admission for a gated skip | a gated skip would fail the aggregate, or use an admission that belongs to a skipped mutation leg |
| refusing an entry without `/` by its spelling | no spelling tells a directory `tools` from a file such as `LICENSE`, and a directory can carry a dot |

## Consequences

The decision and the admission are one small module with a Lean proof, so a later edit that widens
either one fails the vectors test or stales a cover. The proof is of the decision against the map:
the map's completeness and the workflow wiring are held by the later deliveries' census and tests,
and the ledger's O18 names both as gaps. Read with SPEC-502 and ADR-520, whose entry this one
follows.

## What would make this wrong

- A gated job reads a path the map does not name, and no census refuses the map.
- The path-gate job computes `changed` from anything but the merge commit against its first
  parent with renames split, so a renamed path's old side is missed.
- A later edit calls `decide` with an event the three strings do not name, such as a merge queue
  event, and the event test reads it as a pull request into dev.
