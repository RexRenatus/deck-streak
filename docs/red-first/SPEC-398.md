# Red-first record: SPEC-398

The delivery takes three pushes. The first carries SPEC-398, ADR-412, the schematic's section 9, the
two insert-only pointer amendments and the red tests alone, with no wiring: `planted.ts` names the
`firefox` engine, the coverage test lists it, and the card configuration and the `card-sandbox` job
still run two engines. The second carries the wiring: the card configuration's `firefox` project,
the job's Firefox install step, its comment, its suite step's name and its bound, and the two
Firefox declarations every engine makes. The third carries the Firefox readings that the second
push's `card-sandbox` run measured, this record's fence, the mutation band and the changelog
fragment.

Where each criterion's red was read:

- A1 and A4 are Vitest tests in `planted-coverage.test.ts`; their red is read on the box, from the
  whole file, at the first push. A1 fails by its first assertion (the card configuration's projects
  are two, not three) and A4 fails by `firefox dns-prefetch` (no Firefox declaration yet).
- A2 and A3 are unittest cases in `scripts/tests/test_ci_workflows.py`, which this box does not run;
  their red is read in CI at the first push, in the `hygiene` job that runs that module. A2 fails by
  its first assertion (the job does not install Firefox once) and A3 by the bound sitting below the band.
- A5 is not red: the planted suite's verdict in Firefox is a browser engine's behaviour, and the
  second push's `card-sandbox` run is its first measurement. That run's three census lines, one per
  engine, and each Firefox declaration it names are recorded at the third push.
