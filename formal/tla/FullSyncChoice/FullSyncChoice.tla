---------------------------- MODULE FullSyncChoice ----------------------------
\* @phx covers crates/engine-core/src/full_sync.rs anchor=between digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/full_sync.rs anchor=confirm digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/full_sync.rs anchor=backed_up digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/full_sync.rs anchor=download_ready digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/full_sync.rs anchor=snapshot_found digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/full_sync.rs anchor=rechecked digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/full_sync.rs anchor=at_write digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #631, #620, #660
\* @phx property BackupBeforeReplace ramp=report
\* @phx property SnapshotBeforeUpload ramp=report
\* @phx property CountsCoverTheUpload ramp=report
\* @phx property AWindowSyncIsNotSilent ramp=report
\* @phx property NoReviewLost ramp=report
\* @phx witness witness/a-download-before-its-backup.cfg kills=BackupBeforeReplace
\* @phx witness witness/an-upload-with-no-snapshot-found.cfg kills=SnapshotBeforeUpload
\* @phx witness witness/an-upload-with-no-re-check.cfg kills=CountsCoverTheUpload
\* @phx witness witness/an-upload-that-keeps-the-servers-schema.cfg kills=AWindowSyncIsNotSilent
\* @phx witness witness/a-write-that-does-not-re-read-the-device.cfg kills=NoReviewLost
\*
\* The full-sync choice (SPEC-357, ADR-368). When the engine answers a normal sync with a full sync,
\* the owner chooses a direction: an upload replaces the server's collection with the device's, a
\* download replaces the device's with the server's. Before either write the core counts what each
\* side loses by id, takes a backup of the side the write replaces, and, for an upload, finds the
\* offsite snapshot and re-reads the server; at the write it re-reads the device. Meanwhile this
\* device keeps studying (study is not held during the choice, ADR-368 D9), and a second client
\* keeps reviewing and making normal syncs against the server (#620, #660). The rule both clients
\* share is `crates/engine-core/src/full_sync.rs` (#631).
\*
\* What the model abstracts, and why (ADR-368 D8):
\* - Every collection is a set of review ids. Cards and notes follow the same rule as reviews, so
\*   one kind of row stands for all three. The modified stamp the re-check also compares is folded
\*   into the ids: a server copy that changed is a server copy whose ids changed.
\* - The choice is the program counter `pc`; each of its steps, and the adapter's read or fetch that
\*   feeds it, is one step. A second client's normal sync is atomic, and a write is one step.
\* - The second client's own full sync is outside the model: `needFull` records that it is forced,
\*   not what it chooses.
\* - The snapshot is the server's rows at some moment, sealed offsite (ADR-340), taken at any time;
\*   `NONE` marks that none exists yet. It is a set no snapshot can be, since no review has id 0.
\* - `held` is the ids the choice carries for the write's last check: the device side the counts
\*   were taken over, and the backup's ids once a backup is accepted (`full_sync.rs::backed_up`).
\* - `MaxReviews` review ids bound every set; each id is made once, on one device.
\*
\* The switches are the chosen design when all five are TRUE: `BackupFirst` is a download's backup
\* before its write, `SnapshotFirst` an upload's found snapshot before its write, `Recheck` an
\* upload's re-read of the server before its write, `SchemaBump` the upload's schema change that
\* forces every other client into a full sync, and `VerifyAtWrite` a download's re-read of the
\* device at its write.
\*
\* The action-to-code map, by `file::item`:
\* - Count, Recount -> `full_sync.rs::Counted`'s show over the core's id reads of the device and of
\*   the scratch server copy; `full_sync.rs::between` is each direction's loss.
\* - Confirm(d) -> `full_sync.rs::confirm`, refused for a direction not offered.
\* - Backup -> the adapter writes the backup; `full_sync.rs::backed_up` accepts it only when it
\*   holds every id of the side the write replaces.
\* - DownloadReady -> `full_sync.rs::download_ready`.
\* - SnapCheck -> `full_sync.rs::snapshot_found`; a snapshot not found refuses the upload.
\* - Rechecked -> `full_sync.rs::rechecked`, over a fresh server copy.
\* - AtWriteRefused, Write -> `full_sync.rs::at_write`, then the write it admits.
\* - Cancel -> the owner's Cancel at any step before the write.
\* - AReview, BReview, BSync -> a review on this device, a review on the second client, and the
\*   second client's normal sync.
\* - Snapshot -> the daily window's sealed snapshot of the server.

EXTENDS Integers

CONSTANTS BackupFirst, SnapshotFirst, Recheck, SchemaBump, VerifyAtWrite, MaxReviews

Reviews == 1..MaxReviews
NONE == {0}
Directions == {"upload", "download"}
Places == {"conflict", "counted", "confirmed", "backedup", "checked", "ready", "done", "refused"}

VARIABLES local, server, other, pending, snap, made,
          backup, copy, held, pc, dir, counted, snapFound,
          window, lostUncounted, lostUnbacked, needFull, uploadedNoSnap

world == <<local, server, other, pending, snap, made>>
choice == <<backup, copy, held, pc, dir, counted, snapFound>>
history == <<window, lostUncounted, lostUnbacked, needFull, uploadedNoSnap>>
vars == <<world, choice, history>>

TypeOK ==
    /\ local \in SUBSET Reviews
    /\ server \in SUBSET Reviews
    /\ other \in SUBSET Reviews
    /\ pending \in SUBSET Reviews
    /\ snap \in (SUBSET Reviews) \cup {NONE}
    /\ made \in SUBSET Reviews
    /\ backup \in SUBSET Reviews
    /\ copy \in SUBSET Reviews
    /\ held \in SUBSET Reviews
    /\ pc \in Places
    /\ dir \in {"none"} \cup Directions
    /\ counted \in [upload : SUBSET Reviews, download : SUBSET Reviews]
    /\ snapFound \in BOOLEAN
    /\ window \in SUBSET Reviews
    /\ lostUncounted \in SUBSET Reviews
    /\ lostUnbacked \in SUBSET Reviews
    /\ needFull \in BOOLEAN
    /\ uploadedNoSnap \in BOOLEAN

\* A schema conflict: the device and the server hold any rows, the second client has synced, and
\* the newest snapshot, if one exists, is of the server's rows.
Init ==
    /\ local \in SUBSET Reviews
    /\ server \in SUBSET Reviews
    /\ other = server
    /\ pending = {}
    /\ snap \in {NONE, server}
    /\ made = local \cup server
    /\ backup = {}
    /\ copy = {}
    /\ held = {}
    /\ pc = "conflict"
    /\ dir = "none"
    /\ counted = [upload |-> {}, download |-> {}]
    /\ snapFound = FALSE
    /\ window = {}
    /\ lostUncounted = {}
    /\ lostUnbacked = {}
    /\ needFull = FALSE
    /\ uploadedNoSnap = FALSE

\* A review on this device: study is not held while a choice is open (ADR-368 D9).
AReview(r) ==
    /\ r \notin made
    /\ local' = local \cup {r}
    /\ made' = made \cup {r}
    /\ UNCHANGED <<server, other, pending, snap>>
    /\ UNCHANGED choice
    /\ UNCHANGED history

\* A review on the second client, not yet synced.
BReview(r) ==
    /\ r \notin made
    /\ other' = other \cup {r}
    /\ pending' = pending \cup {r}
    /\ made' = made \cup {r}
    /\ UNCHANGED <<local, server, snap>>
    /\ UNCHANGED choice
    /\ UNCHANGED history

\* The second client's normal sync, refused once it must make a full sync. A row it syncs after an
\* upload's re-check and before its write is in the window.
BSync ==
    /\ ~needFull
    /\ server' = server \cup pending
    /\ other' = other \cup server'
    /\ pending' = {}
    /\ window' = IF pc = "ready" /\ dir = "upload" THEN window \cup pending ELSE window
    /\ UNCHANGED <<local, snap, made>>
    /\ UNCHANGED choice
    /\ UNCHANGED <<lostUncounted, lostUnbacked, needFull, uploadedNoSnap>>

\* The daily window seals a snapshot of the server's rows offsite.
Snapshot ==
    /\ snap' = server
    /\ UNCHANGED <<local, server, other, pending, made>>
    /\ UNCHANGED choice
    /\ UNCHANGED history

\* The counts, read over the device's ids and a fresh scratch copy of the server's
\* (`full_sync.rs::between`): an upload loses the server's rows the device lacks, a download the
\* device's rows the server lacks.
Recount ==
    /\ copy' = server
    /\ counted' = [upload |-> server \ local, download |-> local \ server]
    /\ held' = local
    /\ snapFound' = FALSE
    /\ pc' = "counted"

Count ==
    /\ pc = "conflict"
    /\ Recount
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, dir>>
    /\ UNCHANGED history

\* The offer (SPEC-342 F1): the upload when the device holds rows, the download when the server
\* copy does (`full_sync.rs::confirm` refuses a direction not offered).
Offered(d) == IF d = "upload" THEN local # {} ELSE copy # {}

Confirm(d) ==
    /\ pc = "counted"
    /\ Offered(d)
    /\ dir' = d
    /\ pc' = "confirmed"
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, copy, held, counted, snapFound>>
    /\ UNCHANGED history

\* The owner's Cancel, at any step before the write: nothing is written.
Cancel ==
    /\ pc \in {"counted", "confirmed", "backedup", "checked", "ready"}
    /\ pc' = "done"
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, copy, held, dir, counted, snapFound>>
    /\ UNCHANGED history

\* The backup holds the side the write replaces: the device's collection for a download, the
\* scratch server copy for an upload (`full_sync.rs::backed_up`).
Backup ==
    /\ pc = "confirmed"
    /\ backup' = IF dir = "download" THEN local ELSE copy
    /\ held' = backup'
    /\ pc' = "backedup"
    /\ UNCHANGED world
    /\ UNCHANGED <<copy, dir, counted, snapFound>>
    /\ UNCHANGED history

\* A download is ready from its backup (`full_sync.rs::download_ready`); without `BackupFirst`, from
\* its confirm.
DownloadReady ==
    /\ dir = "download"
    /\ \/ pc = "backedup"
       \/ pc = "confirmed" /\ ~BackupFirst
    /\ pc' = "ready"
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, copy, held, dir, counted, snapFound>>
    /\ UNCHANGED history

\* An upload's snapshot check (`full_sync.rs::snapshot_found`): a found snapshot moves it on, none
\* refuses it. Without `SnapshotFirst`, an upload may move on without looking.
SnapCheck ==
    /\ pc = "backedup"
    /\ dir = "upload"
    /\ \/ /\ snap # NONE
          /\ pc' = "checked"
          /\ snapFound' = TRUE
       \/ /\ snap = NONE
          /\ pc' = "refused"
          /\ UNCHANGED snapFound
       \/ /\ ~SnapshotFirst
          /\ pc' = "checked"
          /\ UNCHANGED snapFound
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, copy, held, dir, counted>>
    /\ UNCHANGED history

\* The re-check over a fresh server copy (`full_sync.rs::rechecked`): unchanged, the upload is
\* ready; changed, the choice returns to new counts. Without `Recheck`, it is ready unread.
Rechecked ==
    /\ pc = "checked"
    /\ \/ /\ (server = copy \/ ~Recheck)
          /\ pc' = "ready"
          /\ UNCHANGED <<copy, held, counted, snapFound>>
       \/ /\ Recheck
          /\ server # copy
          /\ Recount
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, dir>>
    /\ UNCHANGED history

\* The write's last check (`full_sync.rs::at_write`): a download whose device side gained a row the
\* choice does not hold returns to new counts.
AtWriteRefused ==
    /\ pc = "ready"
    /\ dir = "download"
    /\ VerifyAtWrite
    /\ ~(local \subseteq held)
    /\ Recount
    /\ UNCHANGED world
    /\ UNCHANGED <<backup, dir>>
    /\ UNCHANGED history

\* The upload: the server's rows the device lacks are removed, and the upload changes the server's
\* schema, so every other client must make a full sync of its own.
WriteUpload ==
    /\ pc = "ready"
    /\ dir = "upload"
    /\ lostUncounted' = lostUncounted \cup ((server \ local) \ counted.upload)
    /\ lostUnbacked' = lostUnbacked \cup ((server \ local) \ (backup \cup window))
    /\ server' = local
    /\ needFull' = (needFull \/ SchemaBump)
    /\ uploadedNoSnap' = (uploadedNoSnap \/ ~snapFound)
    /\ pc' = "done"
    /\ UNCHANGED <<local, other, pending, snap, made>>
    /\ UNCHANGED <<backup, copy, held, dir, counted, snapFound>>
    /\ UNCHANGED window

\* The download: the device's rows the server lacks are removed, admitted only when the device holds
\* no row the choice does not (`full_sync.rs::at_write`).
WriteDownload ==
    /\ pc = "ready"
    /\ dir = "download"
    /\ VerifyAtWrite => local \subseteq held
    /\ lostUnbacked' = lostUnbacked \cup ((local \ server) \ backup)
    /\ local' = server
    /\ pc' = "done"
    /\ UNCHANGED <<server, other, pending, snap, made>>
    /\ UNCHANGED <<backup, copy, held, dir, counted, snapFound>>
    /\ UNCHANGED <<window, lostUncounted, needFull, uploadedNoSnap>>

Write == WriteUpload \/ WriteDownload

\* A finished or refused choice stutters, so its end is not a deadlock.
Finished ==
    /\ pc \in {"done", "refused"}
    /\ UNCHANGED vars

Next ==
    \/ \E r \in Reviews : AReview(r) \/ BReview(r)
    \/ BSync
    \/ Snapshot
    \/ Count
    \/ \E d \in Directions : Confirm(d)
    \/ Cancel
    \/ Backup
    \/ DownloadReady
    \/ SnapCheck
    \/ Rechecked
    \/ AtWriteRefused
    \/ Write
    \/ Finished

Spec == Init /\ [][Next]_vars

\* Every row a write removes is in a backup, or is a row the second client synced after the
\* re-check ("the backup holds every id of the side the write replaces", `full_sync.rs::backed_up`;
\* `full_sync.rs::download_ready`).
BackupBeforeReplace == lostUnbacked = {}

\* No upload is written unless the snapshot was found first ("an answer whose found is false is
\* refused", `full_sync.rs::snapshot_found`).
SnapshotBeforeUpload == ~uploadedNoSnap

\* Every row an upload removes that the counts did not show was synced by the second client after
\* the re-check ("equal ids and modified stamp, else a new count over the fresh copy",
\* `full_sync.rs::rechecked`; `full_sync.rs::between`).
CountsCoverTheUpload == lostUncounted \subseteq window

\* A row the second client synced is on the server, or that client must make a full sync: the
\* upload's schema change makes a sync in the window loud, never silent (SPEC-357 section 8).
AWindowSyncIsNotSilent == \A r \in window : r \in server \/ needFull

\* Every review ever made is on the device, the server, the second client, a backup, the server
\* copy or the snapshot ("a download refuses a device id its backup lacks",
\* `full_sync.rs::at_write`; `full_sync.rs::confirm`).
NoReviewLost ==
    made \subseteq (local \cup server \cup other \cup backup \cup copy
                        \cup (IF snap = NONE THEN {} ELSE snap))

=============================================================================
