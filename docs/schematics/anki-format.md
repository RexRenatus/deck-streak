# Schematic: the format crate's components

Kind: component. Read at DeckStreak `dev` at the commit this pull request branches from (SPEC-602, ADR-622,
ADR-623).

A foreign file passes through the defensive opener before any reader sees a row. The opener copies the source's
bytes to a private file under a bound, opens the copy read-only with every defensive setting, registers `unicase`,
checks the copy, and refuses before any row is read. The readers answer DeckStreak's own row types; the blob views
decode through the wire walk and keep their raw bytes. Ingest imports the statements and re-exports the walk.

```mermaid
flowchart TD
  source["the source file (never opened by SQLite)"]
  subgraph format[deck-streak-anki-format]
    opener["opener.rs: -wal check, bounded private copy (mode 600), sha256"]
    open["read-only open: defensive on, trusted schema off, query_only, cell_size_check, mmap_size 0, limits"]
    collate["unicase registered (ADR-623)"]
    check["quick_check, then the refusals: col.ver, table set, user_version, views, triggers, virtual tables"]
    read["read.rs: the 14 tables as row types, a count per table"]
    blob["blob.rs: 8 typed views plus raw bytes"]
    wire["wire.rs: the wire walk (moved from ingest)"]
    fields["fields.rs: the field codec, U+001F"]
    statements["statements.rs: the 19 statements (moved from ingest)"]
  end
  subgraph ingest[deck-streak-ingest]
    schema18["schema18.rs: its adapter, importing the statements"]
    reexport["lib.rs: wire re-exported"]
  end
  source --> opener --> open --> collate --> check --> read
  read --> blob --> wire
  read --> fields
  statements --> schema18
  wire --> reexport
```

Every refusal names its check and the object kind or table, never a value. The store's `ForeignFile` in
`deck-streak-collection` is unchanged and is not drawn: it is the opener left to re-point when the read port
switches to this crate.
