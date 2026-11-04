# The harness report reads the planted suite bundle (SPEC-608)

Each test step writes one result bundle; the report reads every bundle through `cases_in`, writes each case's
details beside the report, and lists each case with its result on each simulator through `per_simulator`. In the
sibling job, a bundle the other job measures is named by `elsewhere`, from the owners map.

```mermaid
flowchart LR
  subgraph harness
    D[Debug tests step] --> DB[debug.xcresult]
    CP[planted suite step, iPhone then iPad] --> CB[card-probe.xcresult]
  end
  subgraph release-and-app
    R[Release, app, review and review actions steps] --> RB[release, app, review, review-actions bundles]
  end
  DB --> CI[cases_in label]
  CB --> CI
  RB --> CI
  CI --> J[label-tests.json and details-label-id.json]
  CI --> PS[per_simulator: each Device child's name and result]
  PS --> ROW["| Card probe test id | iPhone: result; iPad: result |"]
  O[owners map: card-probe is harness] --> E[elsewhere]
  E --> SIB["sibling row: | Card probe: no test result was read | in harness |"]
```

The two report bodies are one text: `harness` reads `card-probe.xcresult` and lists its cases, and
`release-and-app`, which holds no such bundle, writes the one row that names `harness`.
