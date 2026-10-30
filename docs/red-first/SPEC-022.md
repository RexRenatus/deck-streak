# Red-first record: SPEC-022

The delivery ran in two phases, as the SPEC orders. **The spike** came first: its tests were committed
(50d6469) beside an `AnkiEngine` port whose adapter was a stub that resolved no queue and downloaded
nothing, and ADR-009 held no measured table; the adapter followed (f295b95), and ADR-009 recorded
`engine-measure.yml` run 36357990387 (479f91d). **The sync** came once the kernel had merged: its
tests were committed (608c9b8) beside a syncer, a collection lock, a run record, settings and a
data-rights port whose public API was in place and whose behaviour was stubbed: the syncer reported
`engine_failed` without asking the engine anything, the lock took no lock, the record kept nothing,
the settings accepted a missing endpoint, the port declared no table, and the constants were
placeholders. The implementation followed (8689d21). Between each red commit and its green one, no
test changed what it asserts: the sync's tests took lint fixes only (helpers that panic with a
message instead of `expect`, `Duration::from_hours`, a saturating subtraction, `write!` for a
header).

Each criterion was run with the SPEC's own fenced command and failed by assertion for its own
criterion, not by a compile error, a missing fixture or an empty selection. A12's precondition is
itself the criterion's subject: with no lock and a stub that syncs nothing, no first sync ever held
the collection while a second waited for it.

The census (A15) measured the requests the engine sends through the recording layer, per scenario:
a full-sync demand sent `hostKey, meta, hostKey, download` (the port logs in once for the normal
sync that learns of the demand and once for the download); a normal sync that pulled another
client's reviews sent `hostKey, meta, start, applyChanges, chunk, applyChunk, sanityCheck2, finish`,
with `applyChanges`, `applyChunk` and `start` carrying no local change; a sync with nothing new sent
`hostKey, meta`; and an empty server was asked `hostKey, meta` on each of three attempts, then
refused as `full_upload_required`. No scenario sent `upload`.

```red-first
A1: red at 50d6469: AssertionError: Lists differ: ["ADR-009's Confirmation holds no table of measure, budget, measured, verdict"] != []
A1: green at 479f91d
A2: red at 50d6469: assertion `left == right` failed: the queue is resolved for both top-level decks; left: 0, right: 2
A2: green at f295b95
A3: red at 50d6469: assertion `left == right` failed: the copy holds every card of the server's; left: 0, right: 250000
A3: green at f295b95
A4: red at 608c9b8: assertion `left == right` failed: the first sync's outcome; left: Err(EngineFailed), right: Ok(())
A4: green at 8689d21
A5: red at 608c9b8: the first sync downloads the collection: Ran { outcome: Err(EngineFailed), attempts: 0, full_download: false, .. }
A5: green at 8689d21
A6: red at 608c9b8: a copy that does not exist yet is downloaded: Ran { outcome: Err(EngineFailed), attempts: 0, full_download: false, .. }
A6: green at 8689d21
A7: red at 608c9b8: assertion `left == right` failed; left: Err(EngineFailed), right: Err(FullUploadRequired)
A7: green at 8689d21
A8: red at 608c9b8: assertion `left == right` failed: the attempts for {"failures":0,"jitter_draws":[0.9056396761745208,0.6862541570267026],"kind":"error"}; left: 0, right: 1
A8: green at 8689d21
A9: red at 608c9b8: assertion `left == right` failed: constants.SYNC_RETRY_ATTEMPTS: the port holds 0, the predecessor 3
A9: green at 8689d21
A10: red at 608c9b8: assertion `left == right` failed: one run, one record: []; left: 0, right: 1
A10: green at 8689d21
A11: red at 608c9b8: assertion failed: the run is recorded as network_unreachable: Ran { outcome: Err(EngineFailed), attempts: 0, .. }
A11: green at 8689d21
A12: red at 608c9b8: the first sync never held the collection lock inside the engine
A12: green at 8689d21
A13: red at 608c9b8: assertion `left == right` failed: sync_runs is the owner's data: exported and erased (CHARTER 13); left: None, right: Some(ExportAndErase)
A13: green at 8689d21
A14: red at 608c9b8: a missing DECKSTREAK_SYNC_ENDPOINT must refuse start by name, not Ok(SyncSettings { endpoint: SyncEndpoint(..), state: StateDirectory("/state") })
A14: green at 8689d21
A15: red at 608c9b8: a full-sync demand sent no hostKey: []
A15: green at 8689d21
A16: red at 608c9b8: assertion `left == right` failed: the first scheduled run's outcome; left: Err(EngineFailed), right: Ok(())
A16: green at 8689d21
A17: red at 608c9b8: assertion `left == right` failed; left: Ran { outcome: Err(EngineFailed), .. }, right: Debounced { last: SyncRun { .. } }
A17: green at 8689d21
```
