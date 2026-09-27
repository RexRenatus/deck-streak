# Schematic: the parity oracle, from the predecessor's functions to a Rust assertion

Kind: data flow. Read at DeckStreak `main` e05dfa5 (ADR-012, `tools/parity-oracle/generate.py`,
`tools/parity-oracle/test_generate.py`), and at the predecessor's `27ee2bc` for the first function
it drives (`analytics.py:study_day`, `types.py:CollectionConfig`). Decided by ADR-029.

```mermaid
flowchart LR
  subgraph private[the owner's machine: never in public CI]
    v9[(predecessor checkout)] -->|import by name| adapters
    registry[registry/spec_NNN.py: function, adapter or constants] --> gen[generate.py]
    adapters[adapter glue in the module: build args, patch sleep, CALL] --> gen
    seed[random.Random of SEED] --> registry
  end
  gen -->|strict JSON, sort_keys, no NaN, epoch ms and epoch days| goldens[(goldens/*.json)]
  gen -. records .-> prov[source_commit, generator_sha256, registry_sha256, seed, inputs: synthetic]
  goldens --> tg[test_goldens.py: schema, digests, seed, no date strings]
  goldens --> reader[golden.rs, included by #path]
  reader --> tests[crates/ctx/tests/*.rs: port output equals golden output, examined N]
  tg -->|public CI, python stage| gate{{scripts/check.sh}}
  tests -->|public CI, test stage| gate
```

| step | what crosses | guard |
|---|---|---|
| registry to generator | a registration of one kind | a golden name registered twice exits 2 naming both modules |
| predecessor to golden | outputs of the predecessor's own code over seeded synthetic inputs | a registry module may not read a file, a database or the network (static read of its syntax tree) |
| generator to repository | the golden with its provenance | `generator_sha256` and `registry_sha256` stale only what changed |
| golden to Rust | typed cases | wrong schema, no case or a missing field is refused; zero examined panics |

The oracle never runs in public CI; public CI runs only what reads the committed goldens.
