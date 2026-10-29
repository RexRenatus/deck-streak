# The collection store and ingest's read port: components (SPEC-512, ADR-530)

The collection crate holds the store and declares the read port; ingest holds the schema-18 adapter beside its live
readers. The live readers keep their own drivers and import the moved statements from the adapter's module.

```mermaid
flowchart TB
  subgraph ingest[deck-streak-ingest]
    live[live readers: reader, study_days, window, gate, structure, engine, skip_write]
    schema18[schema18.rs: the 19 moved statements and the schema-18 adapter]
  end
  subgraph collection[deck-streak-collection]
    store[store.rs: Store::open, open_reader, ForeignFile]
    migrate[migrate.rs: the embedded migrations]
    sql[sql.rs: every store statement and the register]
    read[read.rs: CollectionRead and the store adapter]
    rights[data_rights.rs: export, erase, the gate]
  end
  kernel[deck-streak-kernel: the foreign read-only sqlx connection]
  engine[Anki's engine: its own connection]
  copy[(the schema-18 copy)]
  file[(the store's file)]
  live -->|imports the statements| schema18
  live --> kernel
  live --> engine
  kernel --> copy
  engine --> copy
  schema18 -->|implements| read
  schema18 --> store
  store -->|ForeignFile: read-only, query_only| copy
  store --> migrate
  store --> file
  read --> sql
  rights --> sql
  read --> store
```

- `deck-streak-collection` depends on no workspace crate; `deck-streak-ingest` depends on the kernel and on it.
- Only `store.rs` builds a connection; every statement is a registered constant, prepared by the census.
