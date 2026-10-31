# The collection store's tables: data flow (SPEC-512, ADR-530)

One file per user holds every table below, on the server and on each device. The collection crate is the one writer
of each; nothing live writes or reads them yet (the store is dormant). Dashed arrows are writes later changes make.

```mermaid
flowchart LR
  subgraph replicated[replicated tables]
    study_rule[(study_rule)]
    notetypes[(notetypes)] --> fields[(fields)]
    notetypes --> templates[(templates)]
    notetypes --> notes[(notes)]
    notes --> note_tags[(note_tags)]
    notes --> note_field_history[(note_field_history: append-only)]
    presets[(presets)] --> decks[(decks)]
    notes --> cards[(cards)]
    decks --> cards
    review_log[(review_log: append-only)] --> review_retraction[(review_retraction: append-only)]
    review_log --> review_client[(review_client: append-only, the client record)]
  end
  subgraph local[device-local tables]
    store_meta[(store_meta: role, writer id, slot)]
    erase_gate[(erase_gate: closed)]
    outbox[(outbox)]
    applied_ops[(applied_ops)]
    undo_journal[(undo_journal)]
    schedule[(schedule)]
    device_setting[(device_setting)]
  end
  opener[Store::open: migrations, store_meta] --> store_meta
  opener --> erase_gate
  rights[data_rights.rs: export, erase] --> erase_gate
  rights -.->|erase under the open gate| review_log
  later[later changes: answers, notes, operations] -.-> review_log
  later -.-> notes
  reader[store adapter: CollectionRead] --> decks
  reader --> cards
  reader --> review_log
  reader --> notes
```

- `review_log` rows never change; a row, a retraction, a client record or a history row leaves only while
  `erase_gate` reads open, and only `data_rights.rs` opens it.
- A review row has no foreign key to its card, so a deleted card keeps its history; a retraction and a client record
  each name one review.
- On the server a review row needs its `server_seq`; a device's row may hold none.
