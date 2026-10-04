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
| its user | the system user and group `deck-streak-sync`, which the server, its window, its archive and its sync drill run as, and no other unit (SPEC-340 R2; ADR-351 D1) |
| its snapshot | `deck-streak-sync-snapshot.service`, a stopped-server window the daily backup's run pulls in: the server is stopped for a few seconds, its store is copied, and the server is started again whatever the copy does; `deck-streak-sync-archive.service` then checks the copy, archives it and copies the archive offsite, and `deck-streak-sync-restore-drill.service`, which the restore drill pulls in, restores the newest (ADR-347 D12; ADR-351 D1) |
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
- The rail has created the system user and group `deck-streak-sync`, with no login shell, no home
  and no other group (the owner's go, #161).
- The offsite bucket exists with no public access and no listing, and the host's identity may only
  create objects in it. Its lifecycle rule deletes each object `P30D` after it is written, the
  period `PRIVACY.md` states, and the owner checks the rule before the window (the owner's go,
  #161).
- The settings file names `DECKSTREAK_SNAPSHOT_COPY` and `DECKSTREAK_SNAPSHOT_BUCKET`, and the copy
  command is checked under the backup unit's sandbox (the owner's go, #161).
- Each unit's `CPUQuota=` equals ADR-347 D7's split (the owner's go, #161).
- The security review's verdict on the server, the route and the browser's credential is in
  (ADR-340, #167).
- The owner has chosen the window. From `final_sync` until `mobile`, no client syncs or studies
  except as a step says.

## The steps

### `started`: the unit started over an empty store

A host step, on the owner's go (#161). Each of the unit's two entries, the owner's and the staging
user's, is made on the maintainer's machine by the standard library's command, which takes a
16-byte salt from `os.urandom(16)`, reads the password with `getpass` and never echoes it, and
derives a 32-byte digest in 600000 rounds, the house's shape (`l=32`; SPEC-340 R10):

```
python3 -c 'import base64, getpass, hashlib, os; s = os.urandom(16); d = hashlib.pbkdf2_hmac("sha256", getpass.getpass().encode(), s, 600000, 32); b = lambda raw: base64.b64encode(raw).decode().rstrip("="); print(f"owner:$pbkdf2-sha256$i=600000,l=32${b(s)}${b(d)}")'
```

For the staging user, `staging` takes the place of `owner`. Each entry goes straight into the
credential store, and is never printed to a log, a file in a repository or a chat (ADR-347 D13).
On the host: the unit's two credentials go into the credential store the rail reads (ADR-038), its
listen address and the rail's drop-in are installed, the Caddy block is rendered with its fourth
key and installed, and the unit is enabled and started, which enables its snapshot window, its
archive and its sync drill as well. Its store holds no collection: no user has synced.
Check that the unit is active and that a login of the staging user through the route is answered.
The edge validates the rendered block before its reload, and one line of the sync route is read
from the edge's journal: it holds the client's address, the method, the path and the status, and no
request header and no `k` parameter (SPEC-340 R6). The rail installs the ban filter from
`deploy/fail2ban/`, checks it with the ban service's own regex tool against the journal line of one
refused login of the staging user, and only then installs and starts the jail (SPEC-340 R5). The
backup runs once by hand, so the first sync drill finds an archive.
A failure here moves nothing of the owner's: `rolled_back` is only the unit stopped.

### `rehearsed`: the staging user's rehearsal

A host step, on the owner's go (#161). The whole sequence below, once, as the staging user and with
a scrubbed collection, on two scratch desktop profiles pointed at the new server: the full upload
from the first, the read-back on the host as `read_back` does it, the full download into the second,
then one sync each way. Record the upload's client-side result beside the read-back, and what
desktop asks at its next sync if the client reported the upload as failed. When it needs no change
to the deploy, the staging user's full upload of a collection of ADR-022's shape is the hold's
reading (a), recorded as (b) says; (c) applies to it. Then one run of the daily backup, after which
the server is active again and the archive unit archived a generation and copied it offsite, and
one run of the restore drill, whose sync drill restores the newest archive. A failed rehearsal stops
the cutover here: nothing of the owner's has moved.

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

A host step, on the owner's go (#161). On the host, as the sync server's user, `deck-streak-sync`,
and read only, the owner's collection in the new store is opened and its cards and notes counted. A full upload closes the
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
machine by the same command as `started` (`os.urandom(16)`, `getpass`, 600000 rounds, `l=32`),
replaces the owner's entry in the credential store, the unit is restarted, and each client logs in
again with the new password. The old server keeps the old password, so this is the last state: a
rollback before it never meets a password the old server lacks. The cutover is then live, and the
old server is kept until the owner retires it.

The server makes each client's key from the user name and the stored hash, so the hash is a secret
of the password's class, and the key has no lifetime of its own (ADR-347 D13). This step is taken
again, on the owner's go, whenever one of its two triggers holds:

- a synced device is lost: any new hash retires every key, so a new hash, with a new salt, is
  enough, and every other client logs in again;
- the credential store may have been exposed: a new password as well, since the hash is made from
  the password.

The new entry goes straight into the credential store, and is never printed to a log, a file in a
repository or a chat.

### `rolled_back`: every client back on the old server

A host step, on the owner's go (#161). Any step from `started` to `app` that fails rolls back: the
new unit is stopped and disabled on the host, which removes its snapshot window, its archive and
its sync drill too, and each client that was repointed is pointed back at the old server, desktop last, and
syncs once. The old server holds the state of the freeze, so nothing
studied before it is lost. A study made on the new server after `uploaded` is not on the old server:
the runbook rolls back before any study on the new server, or not at all.

## After the cutover

- The memory watch reads the server's real peak after the first syncs (SPEC-031), against the
  figure SPEC-337 measured with ADR-022's shape.
- The daily backup's run takes the snapshot in its stopped-server window (ADR-347 D12). For the
  window's few seconds no client can sync: a sync in flight is refused whole, and the client syncs
  again at its next sync. A failed copy, or a window that outlasts its bound, starts the server
  again and pages. Disabling the server removes the window.
- Until the first archive is in the offsite bucket, the copies of the owner's collection are the
  clients' own, desktop's backup from `frozen`, and the old server's untouched store.
- The jail bans an address after five refused sync logins within ten minutes, for one hour
  (SPEC-340 R5). An address banned by mistake, the owner's own client after mistyped passwords
  included, is released before its hour by `fail2ban-client set deck-streak-sync unbanip
  <address>`, on the owner's go (#161).
