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

The third push carries this fence, the four Firefox declarations named below and the mutation
band. The changelog fragment was carried by the first push (7e5571eb), not by the third as the
first paragraph says.

The second push's `card-sandbox` run (job 114037978095) printed three census lines:
`examined 28 planted cards, 28 pairs, 112 variants in chromium`, the same line `in webkit` and
the same line `in firefox`. In Chromium and WebKit every planted test passed. In Firefox the
reference frame reached no listener for four cards, `preconnect`, `shadow-link`, `ping` and
`webrtc`, and each is now declared in `planted.ts`'s `UNOBSERVABLE` table with its measured
reason and in section 3 of the schematic (ADR-412 D8). In Firefox the meta policy (W2) holds
five of the `img` card's seven forms alone, and its two image-set forms, `srcset` on an `img`
and on a `picture` source, beside W1 and W4 (ADR-412 D6). The third push's `card-sandbox` run
reads the single-layer assertion, the same in every engine, over the strip that removes every
`srcset` (#771, ADR-412 D7).

The second commit, 04a5ab17, edited a test file between A1 to A4's red and green: it added the
`NOT_WORKFLOW_READS` entry naming `card_projects` to `scripts/tests/test_ci_workflows.py`, a named
read for the census test that refuses an unlisted file read (ADR-412 D8), and the two Firefox
declarations every engine makes to `planted.ts` (ADR-412 D2). It changed no assertion of A1 to
A4.

```red-first
A1: red at 7e5571eb: planted-coverage.test.ts: AssertionError: expected [ 'chromium', 'webkit' ] to deeply equal [ 'chromium', 'webkit', 'firefox' ]
A1: green at 04a5ab17
A2: red at 7e5571eb: CI run 37992746454 job 114030862686 hygiene: AssertionError: 0 != 1 : the job does not install Firefox once
A2: green at 04a5ab17
A3: red at 7e5571eb: CI run 37992746454 job 114030862686 hygiene: AssertionError: False is not true : the card-sandbox job's timeout is 20, not 30 to 45 minutes
A3: green at 04a5ab17
A4: red at 7e5571eb: planted-coverage.test.ts: AssertionError: firefox dns-prefetch: expected [] to include 'dns-prefetch'
A4: green at 04a5ab17
A5: not red: the planted suite's verdict in Firefox is a browser engine's behaviour; the second push's card-sandbox run (job 114037978095) is its first reading, and the third push's run, over the strip (#771), is the reading named above
```
