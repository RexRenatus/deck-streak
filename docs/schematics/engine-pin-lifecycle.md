# Schematic: the engine pin's lifecycle, from an upstream tag to a patched fork and back

Kind: component and data flow, then a state machine. Read at DeckStreak `dev` c0dbf2a
(`Cargo.toml`, `deny.toml`, `.github/workflows/ci.yml`, `.github/workflows/engine-measure.yml`,
`tools/parity-oracle/`) and at Anki's tags `26.05` and `26.09.3`. Decided by ADR-058, which amends
ADR-022's pin rule; built by SPEC-055.

## Where the engine comes from, and what reads the pin

```mermaid
flowchart LR
  subgraph upstream["ankitects/anki (upstream)"]
    tag["release tag 26.09.3"]
    main["main"]
    next["a later release tag that carries the fix"]
    main --> next
  end
  subgraph fork["RexRenatus/anki (the maintainer's fork)"]
    branch["a branch at 26.09.3 plus one commit, changing only rslib/io/src/lib.rs"]
    pinned["the pinned commit, tagged, never force-pushed while pinned"]
    branch --> pinned
  end
  tag -- "branched from" --> branch
  branch -- "upstream pull request, the maintainer's act (#233)" --> main
  subgraph ds["DeckStreak"]
    manifest["Cargo.toml: anki at upstream tag 26.09.3"]
    patch["Cargo.toml [patch] on the upstream URL: the fork, rev = the pinned commit"]
    lock["Cargo.lock: the engine's five packages from the fork"]
    deny["deny.toml: allow-git = the fork and ankitects/rust-url"]
    ingest["deck-streak-ingest: AnkiEngine adapter"]
    manifest --> patch --> lock --> ingest
    deny -. "cargo deny checks the graph's sources" .-> lock
  end
  pinned -- "rev" --> patch
  tag -- "tag" --> manifest
  subgraph ci["CI"]
    gate["ci.yml rust job: protoc 31.1, clippy, test, doctest, audit"]
    measure["engine-measure.yml: ADR-022's protocol on a pull request that changes Cargo.lock"]
  end
  lock --> gate
  lock --> measure
  goldens["the parity goldens, tools/parity-oracle/goldens"] --> ingest
```

The goldens do not depend on the engine: the generator never imports it (SPEC-055 §1).

## The pin's states

```mermaid
stateDiagram-v2
  [*] --> UpstreamTag
  UpstreamTag: pinned to an upstream tag (26.05), ADR-022
  ForkPinned: the upstream tag, patched by rev to the fork (the tag plus the fix), ADR-058
  Bump: an Anki bump while the fork is carried
  Removal: the removal delivery (#233)

  UpstreamTag --> ForkPinned: SPEC-055's delivery
  ForkPinned --> Bump: a new upstream tag
  Bump --> Removal: the new tag, unpatched, passes A2
  Bump --> ForkPinned: A2 fails unpatched, so the fix is cherry-picked and tagged, and the tag and rev move together
  ForkPinned --> Removal: an upstream release carries the fix
  Removal --> UpstreamTag: the patch entry deleted, allow-git back to upstream, ADR-058 superseded
```

Every transition into a pinned state re-runs ADR-022's protocol in `engine-measure.yml` and
SPEC-022's criteria.

## What each check guards

| check | when it runs | what it proves |
|---|---|---|
| SPEC-055 A1 | the gate's python stage | the dependency names the upstream tag, the `[patch]` entry points at a commit of the fork by `rev`, the lockfile's engine packages come from that commit, and `allow-git` is exactly the fork and `rust-url` |
| SPEC-055 A2 | the gate's python stage, against the workspace's own target | a second build of `deck-streak-ingest` compiles nothing: the pinned commit carries the fix. Unpatched, the same test is the removal check |
| SPEC-055 A3 | the gate's python stage | every advisory exception and allowed git source in `deny.toml` is still in the graph |
| SPEC-055 A5 | the gate's python stage | ADR-058 records the pinned commit, its one-file difference from the tag, and what it saves |
| SPEC-022 A1 | the gate's python stage | ADR-009's Confirmation names the pinned tag, and its latest record holds ADR-022's budgets |
| ADR-022's protocol | `engine-measure.yml`, on the pull request that changes `Cargo.lock` | every budget holds at the new pin |
| SPEC-022 A15, A16 | the gate's test stage | the recording server sees no upload and no local change at the new pin |
