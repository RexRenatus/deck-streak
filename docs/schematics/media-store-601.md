# Media store (SPEC-601, ADR-621)

## Data flow: a face reads a media file

```mermaid
flowchart LR
    face["engine core face: Budget::rewrite"] --> grammar["media core: rewrite over the grammar"]
    face --> reader["Reader: StoreMedia on iOS, the test readers"]
    reader --> cas["collection: CasReader"]
    cas --> names["media_name: live name to hash"]
    names --> files["media_file: state and size"]
    files -->|state 0| blob["FsBlobStore::get(hash, limit)"]
    files -->|state 1 or 2| wanted["hash recorded wanted, nothing read"]
    blob --> check{"length equals size?"}
    check -->|yes| bytes["bytes to the face"]
    check -->|no| bad["file to bad/, row to state 1, hash wanted"]
    worker["web Worker, switched later"] --> handed["web core: handed files, admitted by MediaHash::of"]
```

## Sequence: a put

```mermaid
sequenceDiagram
    participant Caller
    participant Store as media store
    participant Blob as FsBlobStore
    participant Index as media_file
    Caller->>Store: put(bytes, kind, cap)
    Store->>Blob: put: stream into tmp/
    Blob->>Blob: hash while streaming
    Blob->>Blob: refuse past cap (and, for put_claimed, a hash other than the claimed one)
    Blob->>Blob: fsync the file
    Blob->>Blob: rename to sha256/blob path
    Blob-->>Store: hash, size
    Store->>Index: commit the state-0 row (the commit point)
    Store-->>Caller: hash, size
```

## Open-time pass

`tmp/` is emptied, and every hash path under `sha256/` without a state-0 row is deleted; no file is adopted.
