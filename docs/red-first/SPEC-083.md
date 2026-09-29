# Red-first record: SPEC-083

The order of work: SPEC-083 promoted and the recorder's control (3a0961c); the goldens
(f8d584a); the record's tests beside a stub store and stub pure functions, with the two migrations
(2808dca); their implementation, the rights port, the registrations and the rows (8075b52).

The goldens were generated from the predecessor's own functions at `27ee2bc`, and the predecessor's
checkout was left as it was.

The control A33 is disclosed as not red: its red was measured, not committed. With the classifier
stubbed to return nothing, the test failed by the assertion "the classifier marks an upload", on
commit 3a0961c, so every proof that rests on the recorder's zero rests on a layer shown to see a
planted upload.

Each other criterion was run at its red commit with the SPEC's own fenced command, selecting one
test, and failed by assertion for its own criterion. The stubs compiled: a store that recorded
nothing and refused nothing, a day spec of the bounds as given, a search that was not wrapped or
held, a summary of zeros, and a port that declared no skip table. A2's red is the migration
without its key.

```red-first
A33: not red: measured red at 3a0961c with the classifier stubbed to return None: the classifier marks an upload; committed green only
A1: red at 2808dca: a second take for the day is refused: Ok(SkipId(0))
A1: green at 8075b52
A2: red at 2808dca: the key refuses a second pending or applied skip for one study day
A2: green at 8075b52
A3: red at 2808dca: a skip to undo: NothingToUndo
A3: green at 8075b52
A4: red at 2808dca: assertion `left == right` failed: only an applied skip not undone covers its day (R4); left: []
A4: green at 8075b52
A7: red at 2808dca: assertion `left == right` failed; left: "2-2", right: "2"
A7: green at 8075b52
A8: red at 2808dca: assertion `left == right` failed; left: 1, right: 0
A8: green at 8075b52
A23: red at 2808dca: skip_days is the owner's data: exported and erased (CHARTER 13); left: None
A23: green at 8075b52
```
