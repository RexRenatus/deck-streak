# Runbook: the sync server's cutover, by a full upload into an empty store

The owner's Anki data moves from the old sync server to DeckStreak's own (ADR-340) by the server's
documented path (ADR-347 D10): every client syncs once against the old server, desktop's full
upload fills the new server's empty store, and every other client takes a full download from it.
The old server's store is never copied and never written: it is the rollback.
`docs/schematics/sync-server-packaging-and-cutover.md` draws the sequence, and this runbook holds
one step per state of it, in its order, each marked as the schematic's table marks it. Every host
step waits for the owner's go, and its exact command is the one the seat records in #161 before any
run; the host's names and addresses are in the maintainer's private gate packet, and this runbook
names none.

## What lives where

| what | where |
|---|---|
| the new server | `deck-streak-sync-server.service` on a loopback address, behind the Caddy block's `/anki-sync/` route (SPEC-337 R2, R4) |
| its store | the unit's own state directory, one directory per sync user, each holding `collection.anki2`, `media.db` and `media/` |
| its two users | the owner's and the staging user's (ADR-344), read from the credential socket as a user name and a pbkdf2-sha256 hash each (ADR-347 D2) |
| the old server | untouched by every step; the rollback points every client back at it |

## The hold

The sync server's processor share is decided: `CPUQuota=75%` (ADR-347 D7). Its measurement was an
approximation of that quota, so the cutover's full upload is held to three conditions in place of
a larger share (SPEC-337 §6):

- (a) The first full upload under the unit's real `CPUQuota=75%` on the host is the reading that
  counts. It is taken in `rehearsed`, before the owner's collection moves, by the staging user's
  full upload of a collection of ADR-022's shape, when that needs no change to the deploy;
  otherwise the cutover's own full upload in `uploaded` is that reading.
- (b) Either way, the step records the client's result and its longest stall, read at the client's
  socket as the bytes it has written, those the server acknowledged plus those still queued
  (ADR-347 D7).
- (c) A client failure, or a longest stall over 20 seconds, is a STOP. No step after it starts
  except `read_back`, which is read only, and `rolled_back`, and the share's reserve in ADR-347 D7,
  a third processor with the sync server's quota at 100%, applies at once, with no new decision,
  before any full upload is tried again.

The margin holds for ADR-022's shape only: before any full upload of a larger collection, the
full upload is measured again as (a) and (b) say, and (c) applies. `started` and `rehearsed` move
none of the owner's data; no step from `final_sync` on starts until (a)'s reading has passed, or is
to be the cutover's own.

## Before the window

- The release that carries `bin/anki-sync-server` is deployed from its tag (`RELEASING.md`).
- The security review's verdict on the server, the route and the browser's credential is in
  (ADR-340, #167).
- The owner has chosen the window. From `final_sync` until `mobile`, no client syncs or studies
  except as a step says.

## The steps

### `started`: the unit started over an empty store

A host step, on the owner's go (#161). On the host: the unit's two credentials go into the
credential store the rail reads (ADR-038), the Caddy block is rendered with its fourth key and
installed, and the unit is started. Its store holds no collection: no user has synced. Check that
the unit is active and that a login of the staging user through the route is answered. A failure
here moves nothing of the owner's: `rolled_back` is only the unit stopped.

### `rehearsed`: the staging user's rehearsal

A host step, on the owner's go (#161). The whole sequence below, once, as the staging user and with
a scrubbed collection, on two scratch desktop profiles pointed at the new server: the full upload
from the first, the read-back on the host as `read_back` does it, the full download into the second,
then one sync each way. Record the upload's client-side result beside the read-back, and what
desktop asks at its next sync if the client reported the upload as failed. When it needs no change
to the deploy, the staging user's full upload of a collection of ADR-022's shape is the hold's
reading (a), recorded as (b) says; (c) applies to it. A failed rehearsal stops the cutover here:
nothing of the owner's has moved.

### `final_sync`: every client synced once against the old server

This step is the owner's own act. Once the hold's reading (a) has passed, or is to be the cutover's
own, and the window opens, each client syncs
against the old server, one at a time, each finished before the next starts: AnkiMobile, then the
app, then desktop, so that desktop's sync brings every client's studies.

### `frozen`: no client syncs

This step is the owner's own act. From here until its own step, no client syncs or studies. Desktop
makes one backup of its own collection (File, Create Backup), and its totals of cards and notes are
read from the Browse window over the whole collection and kept for `read_back`.

### `uploaded`: desktop's full upload into the empty store

This step is the owner's own act, under the hold's conditions (the hold, above). Desktop's sync
address is set to the new server's, under the web app's origin at `/anki-sync/`, and it logs in as
the owner. It finds an empty server and asks for a one-way sync: choose the upload. Record what
desktop reports. Whatever it reports, nothing is uploaded again before `read_back`.

### `read_back`: the server's collection read back

A host step, on the owner's go (#161). On the host, as DeckStreak's user and read only, the owner's
collection in the new store is opened and its cards and notes counted. A full upload closes the
collection on the server, and a read-only read of it succeeds while the server runs (SPEC-337 §6).

- The counts equal desktop's from `frozen`: the upload landed, whatever desktop reported. Do not
  upload again (ADR-347 D11). A failure desktop reported is the divergence SPEC-337 §6 names:
  record it for #617. When this upload was the hold's reading, a failure or a stall over 20 seconds
  is the hold's STOP (c), whatever the counts say.
- The counts differ, or the store holds no collection: desktop uploads once more, and the read-back
  is repeated. A second mismatch is `rolled_back`.

### `mobile`: AnkiMobile repointed

This step is the owner's own act. AnkiMobile's sync address is set to the new server's, it logs in
as the owner, and when it asks for a one-way sync it takes the download. One review on it, a sync,
and a sync on desktop shows the review: both now read the new server.

### `app`: the app repointed

This step is the owner's own act. The app is pointed at the new server and syncs once, as its own
sync allows (SPEC-334 R8, R9, #617).

### `rekeyed`: a new sync password on the new server

A host step, on the owner's go (#161). A new password's pbkdf2-sha256 hash, made on the maintainer's
machine, replaces the owner's entry in the credential store, the unit is restarted, and each client
logs in again with the new password. The old server keeps the old password, so this is the last
state: a rollback before it never meets a password the old server lacks. The cutover is then live,
and the old server is kept until the owner retires it.

### `rolled_back`: every client back on the old server

A host step, on the owner's go (#161). Any step from `started` to `app` that fails rolls back: the
new unit is stopped on the host, and each client that was repointed is pointed back at the old
server, desktop last, and syncs once. The old server holds the state of the freeze, so nothing
studied before it is lost. A study made on the new server after `uploaded` is not on the old server:
the runbook rolls back before any study on the new server, or not at all.

## After the cutover

- The memory watch reads the server's real peak after the first syncs (SPEC-031), against the
  figure SPEC-337 measured with ADR-022's shape.
- Until the snapshot's part lands (SPEC-337 §6), the copies of the owner's collection are the
  clients' own, desktop's backup from `frozen`, and the old server's untouched store.
