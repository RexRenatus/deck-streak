# Opening the collection store: open, check, migrate, refuse (SPEC-512, ADR-530)

`Store::open(path, role)` reads the file's header before it changes anything, so a file it refuses is left byte for
byte as it was.

```mermaid
sequenceDiagram
  participant caller
  participant opener as Store::open
  participant file as the file
  caller->>opener: open(path, role)
  opener->>file: busy timeout 30,000 ms
  opener->>file: read application_id, user_version, schema count
  alt application_id is not DSTC, and the file is not empty
    opener-->>caller: refused: a file of another application
  else user_version above the newest migration
    opener-->>caller: refused: written by a newer app
  else a store, or an empty file
    opener->>file: foreign_keys, secure_delete, WAL (native), synchronous FULL (server) or NORMAL (device)
    loop each pending migration n
      opener->>file: BEGIN IMMEDIATE#59; re-read user_version
      opener->>file: the migration#59; PRAGMA user_version = n#59; COMMIT
    end
    opener->>file: BEGIN IMMEDIATE#59; store_meta written once, then read
    alt store_meta names another role, writer id or slot
      opener-->>caller: refused: another role
    else
      opener-->>caller: the store
    end
  end
```

- `Store::open_reader(path)` takes the same first read, opens read-only with `query_only`, and also refuses a store
  below head.
- `ForeignFile::open_read_only(path)` takes the same first read and refuses the store's own `application_id`.
