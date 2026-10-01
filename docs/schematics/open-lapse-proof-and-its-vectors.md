# Schematic: the open lapse walk's proof and its vectors

Kind: data flow and component. Read at DeckStreak `dev` 530a3d6 (`crates/streaks/src/lapse.rs`
`open_lapse`, `crates/kernel/src/study_day.rs` `StudyDay`, `config/formal.json`). Decided by
ADR-305; the property is #472's.

The first Lean entry adds one component, the Lake package under `formal/lean`, and one data flow:
the port's answers reach the Rust test through a committed vectors file, never through a shared
library. The checker reads every input at the head commit.

```mermaid
flowchart LR
  subgraph rust[crates/streaks]
    lapse["src/lapse.rs open_lapse"]
    test["tests/formal_vectors_open_lapse.rs"]
  end
  subgraph lean[formal/lean, the Lake package]
    entry["Formal/OpenLapse.lean: port, rule, claims, theorems, witnesses"]
    writer["Formal/OpenLapseVectors.lean: the axes and the line format"]
    main["Formal/Vectors.lean: main, one arm per entry"]
  end
  vectors[("formal/vectors/open-lapse.jsonl")]
  config[("config/formal.json: axioms, budgets")]
  checker{{"the formal check"}}
  entry -- "covers anchor=open_lapse digest" --> lapse
  writer -- "imports the port" --> entry
  main -- "dispatches OpenLapse" --> writer
  main -- "lean --run writes" --> vectors
  checker -- "builds, audits axioms, judges witnesses" --> entry
  checker -- "byte-compares the writer's output" --> vectors
  checker -- "reads" --> config
  checker -- "digests the covered span" --> lapse
  test -- "reads every line" --> vectors
  test -- "derives the same population from the axes" --> test
  test -- "answers each input" --> lapse
```

## What crosses each boundary

| from | to | what | caught when |
|---|---|---|---|
| `lapse.rs` | the entry | the span of `open_lapse`, by sha256 | the span moves: every theorem reads `STALE` until the port is re-read and the digest moves |
| the entry | the checker | three theorems, three witnesses, `#print axioms` of each | a hole (`sorry`, a new axiom, an axiom outside the allow-list), or a witness that does not build |
| the writer | the vectors file | a header naming the covered item and its digest, then one line per input | the committed file differs from the writer's output by one byte: a `DERIVED_DRIFT` finding names the first line that differs, and each theorem reads unclean |
| the vectors file | the test | each input and the port's answer | an input the test's own axes do not derive, a count that differs, or an answer the Rust function does not give |

## The port's states (`RangeInclusive::next_back`)

```mermaid
stateDiagram-v2
  [*] --> Open: start = window's first day, end = today
  Open --> Done: start above end (an empty window)
  Open --> Open: start below end, yield end, end steps back by one
  Open --> Exhausted: start equals end, yield it
  Exhausted --> Done
  Done --> [*]
```

The walk's own exits, at each yielded day: a study review ends it with the threshold's answer; at
the smallest day the type admits it answers none; a `Done` answers by the threshold.
