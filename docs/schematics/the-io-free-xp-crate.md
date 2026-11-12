# Schematic: the I/O-free XP crate, its callers, the parity oracle and the census

Kind: component and data flow. Read at DeckStreak `dev` `b32a40b4` (`crates/progression/src/review_xp.rs`,
`crates/progression/src/economy_config.rs`, `crates/coordination/src/recompute/xp.rs`,
`crates/coordination/src/progression/law_tiers.rs`, `crates/progression/Cargo.toml`,
`crates/kernel/Cargo.toml`, `tools/parity-oracle/README.md`, `scripts/check.sh`,
`docs/CONTEXT-MAP.md`). Everything named `deck-streak-xp`, `ReviewFacts`, `ReviewXpTable`,
`table()` and `test_xp_crate_graph.py` is what SPEC-360 adds; everything else is on `dev` at
that commit. A solid arrow is a Cargo dependency, drawn by this delivery or already present; a
dashed arrow is an edge drawn later by #639.

## Components

```mermaid
flowchart TB
  subgraph clients["clients, later callers (#639)"]
    ffi["deck-streak-ffi: the umbrella FFI crate"]
    webengine["deck-streak-web-engine: the engine on wasm32"]
  end
  subgraph server["the server"]
    coordination["coordination: recompute/xp.rs:110, progression/law_tiers.rs:83"]
    progression["progression: review_xp, the translation at its edge"]
    ingest["ingest: Review, Tier, is_study_event"]
    kernel["kernel: Db, sqlx, tokio"]
  end
  subgraph bottom["bottom layer, beside the kernel"]
    xp["deck-streak-xp: ReviewFacts, Tier, is_study_event, review_xp, table()"]
  end
  economy[("economy.json: the xp section")]
  subgraph oracle["the parity oracle"]
    goldens[("goldens: review_xp.json 265, study_event.json 64, progression.constants.json 24")]
    reader["golden.rs, the one reader, included by path"]
  end
  subgraph ci["CI gates"]
    rust["test stage: cargo nextest run --workspace"]
    python["python stage: test_xp_crate_graph.py, test_goldens.py"]
    mutation["mutation jobs: cargo-mutants on the diff, and the hand rows"]
  end

  coordination --> progression
  progression --> xp
  progression --> ingest
  progression --> kernel
  ingest --> kernel
  ffi -.-> xp
  webengine -.-> xp
  xp -- "include_str! at build time" --> economy
  progression -- "include_str! at build time, day-level constants" --> economy
  reader --> goldens
  rust --> reader
  python --> goldens
  python -. "reads manifests, Cargo.lock, the map and crates/xp/src" .-> xp
  mutation --> xp
  mutation --> progression
```

The XP crate depends on nothing in the workspace, and its one external dependency is serde_json
with `float_roundtrip`. No arrow leaves it toward the kernel, ingest, an adapter or the engine.

## Flow 1: a review's XP, computed on the server through the crate

```mermaid
flowchart LR
  rev["ingest Review: ease, interval, last_interval, kind, ..."] --> adapter
  tier["Option of ingest Tier, chosen by coordination for a law card only (recompute/xp.rs:80)"] --> adapter
  adapter["progression::review_xp::review_xp: builds ReviewFacts from ease, interval and kind; maps the tier by an exhaustive match"] --> crateCall
  crateCall["deck_streak_xp review_xp: ReviewFacts and an Option of the crate Tier"] --> guard{"is_study_event(kind, ease): a type 0 to 3 and an ease of 1 or more"}
  guard -- "no" --> zero["0"]
  guard -- "yes" --> product["base x ease x maturity x type x tier, left to right in f64, from table()"]
  product --> round["round_ties_even, as u32"]
  round --> back["u32 back to coordination: the day's review XP, or the law-tier readout"]
  table[("table(): one static, parsed once from economy.json's per-review keys")] --> product
```

Coordination's call sites and their arguments do not change; only the body behind progression's
`review_xp` moves.

## Flow 2: the parity oracle compares both paths with the recorded vectors

```mermaid
flowchart LR
  g1[("review_xp.json: 265 cases")] --> t1
  g1 --> t3
  g2[("study_event.json: 64 cases")] --> t2
  g2 --> t4
  g3[("progression.constants.json: 24 names")] --> t5
  t1["xp crate, tests/review_xp.rs: ReviewFacts from ease, ivl, rtype; tier from T1 to T4 or none"] --> v1{"equal for every case?"}
  t2["xp crate, tests/study_event.rs: is_study_event(rtype, ease)"] --> v2{"equal for every case?"}
  t3["progression, tests/xp_review.rs, unchanged: an ingest Review through the translation"] --> v3{"equal for every case?"}
  t4["ingest, tests/scope.rs, unchanged: ingest's own copy"] --> v4{"equal for every case?"}
  t5["progression, tests/xp_constants.rs: eight names from the crate's table, sixteen from progression"] --> v5{"equal for every name?"}
  v1 & v2 & v3 & v4 & v5 -- "each prints examined N, and refuses zero" --> pass["the server's XP is unchanged, and the crate equals the predecessor"]
```

No golden is regenerated and no registry module changes: the tests read the committed files.

## Flow 3: the census refuses an I/O dependency

```mermaid
flowchart TB
  start["python stage: test_xp_crate_graph.py"] --> m["crates/xp/Cargo.toml, by tomllib"]
  start --> l["Cargo.lock: the closure from deck-streak-xp"]
  start --> s["crates/xp/src/*.rs, each file counted"]
  start --> f["docs/CONTEXT-MAP.md's fence, and every member's manifest"]
  m --> m1{"[dependencies] exactly serde_json, with float_roundtrip; dev at most serde and serde_json; no build script, features, target table or build dependencies?"}
  l --> l1{"every package in the closure on the allow-list: serde_json, serde, serde_core, serde_derive, proc-macro2, quote, syn, unicode-ident, itoa, memchr, zmij?"}
  s --> s1{"economy.json embedded once; no std::fs, std::net, std::io, std::time, std::env, std::process, std::thread, SystemTime, Instant::, env! or another include?"}
  f --> f1{"the crate's line says nothing; the lines naming xp equal the members naming deck-streak-xp, which are progression alone?"}
  m1 & l1 & s1 & f1 -- "no" --> refuse["refused, naming the package, the token or the member: for example deck-streak-kernel in the closure, or std::fs in src/table.rs"]
  m1 & l1 & s1 & f1 -- "yes, and each planted control refused" --> ok["green, with each examined count printed"]
```

A planted manifest, lockfile, source text and map are judged beside the real ones in each test, so
a census that went blind fails on its own controls.
