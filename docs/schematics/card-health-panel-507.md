# The card health panel: data flow (SPEC-507, ADR-525)

The panel's figures come from two fixed reads the engine core holds and from the scheduler's readout. The core
builds one record of integers and closed enums; the web adapter answers it as JSON over the Worker's one
`card-health` operation; the review opens it in a dialog and stops its answer clock while it is open.

```mermaid
flowchart LR
  cards[(cards: SNAPSHOT_SQL)] --> core
  revlog[(revlog: CARD_REVIEWS_SQL)] --> core
  readout[scheduler readout, or its state] --> core
  core[engine core: Dispatcher::card_health and health.rs CardHealth] --> adapter
  adapter[web adapter: card_health export, JSON] --> worker
  worker[Worker: card-health operation] --> client
  client[page: EngineClient.cardHealth] --> review
  review[Review: overlay pause of the answer clock] --> dialog
  dialog[CardHealthPanel dialog: counts, capped times, recall rows, provenance]
```

- Nothing is written: both reads are selects, and the Worker serves the operation over the collection already open.
- The recall rows show a number only for a Ready readout; every other state reads its line.
- The answer clock: opening the dialog starts an overlay pause; closing it ends the pause; a grade sends the time
  shown minus every pause.
