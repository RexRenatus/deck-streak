---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The sync server is built in the release from the engine's pin, reads hashed users from credentials through a launcher, runs as its own unit, serves under its own path on the web origin, and is snapshotted by the daily backup

## Context and Problem Statement

ADR-340 (proposed, the owner's choice) moves the owner's sync server to the service's own host and
names what must exist first: a build at the tag, a unit with its own budget entry, a route on the
web app's origin, an offsite snapshot with a drilled restore, a review, a cutover and a rollback. It
leaves to the packaging delivery (SPEC-337) how each is done here: how the release builds the
server, how the unit gets its users with no secret in the tree, what the unit is called and how it
stops, what path and bounds the route has, what form the snapshot takes, and where the cutover's
runbook lives. This record decides those six. It decides no share of the host (ADR-032) and runs
nothing on it (#161).

## Decision Drivers

- One pin: the engine's fork and commit are written once, in the patch entry ADR-058 maintains, so
  the server and the engine cannot drift apart.
- The host never compiles; one attested tarball per tag is what the deploy verifies (CHARTER 3,
  ADR-062).
- No secret in the tree, a unit or a settings file; credentials come from the socket (ADR-010,
  ADR-038).
- The server reads its users from its process environment and nowhere else, and checks a hashed
  password only when `PASSWORDS_HASHED` is set (the fork's source at the pinned commit).
- Browser sync is same-origin and every client reaches the server over HTTPS (ADR-340); a client
  needs the URL's trailing slash when the server sits under a path (the server's documentation).
- The snapshot copies a live server consistently and restores to something a client can open.

## Considered Options (the alternatives it was chosen against)

D1, the build:

- A step in release.yml's one job, after the tag guard and the protobuf compiler, that reads the fork and the commit from `Cargo.toml`'s patch entry, refuses a commit that is not 40 hex, and runs `cargo install --locked` of the fork's `anki-sync-server` package at it, the binary placed in the same tarball as `bin/anki-sync-server` — chosen, because the pin stays one, and the server rides the release's one tarball, one manifest, one attestation and its draft-then-publish path.
- A second copy of the fork and the commit in the workflow: rejected because two pins drift, and the patch entry is the one ADR-058's lifecycle moves.
- A separate job or a second release asset: rejected because the deploy verifies one tarball's provenance and digests (ADR-062); a second asset needs a second verification path.
- The private rail provides the binary, as it provides the replicator's (ADR-064): rejected because the server is the engine's own code at the engine's own pin, and a binary built elsewhere carries neither the pin nor the release's provenance.
- Building on the host: rejected by CHARTER 3.
- Making the server's package part of DeckStreak's workspace: rejected because DeckStreak's lockfile would then re-resolve the server's dependencies, a build upstream never runs; `--locked` builds it with the fork's own lockfile.

D2, the users and their credentials:

- One `LoadCredential=` per sync user in the socket form, the owner's and ADR-344's staging user, each holding `name:<PHC pbkdf2-sha256 hash>`; a launcher in the release, `deploy/scripts/sync-server.sh`, reads them from `$CREDENTIALS_DIRECTORY`, refuses a missing, empty or unhashed entry, sets `SYNC_USER1`, `SYNC_USER2` and `PASSWORDS_HASHED` in its own environment and execs the server — chosen, because the server reads users nowhere else, the unit's text then holds no user, a hash and never a password is what the server receives, and a malformed hash fails the unit at start instead of failing the first login.
- `Environment=` lines in the unit: rejected because a credential in a unit is a secret in the tree (the durable-services secrets rows; `NoSecretInTheEnvironment`).
- An `EnvironmentFile=` the rail writes: rejected because a settings file holding a credential is what ADR-010 refuses.
- Plain passwords: rejected because ADR-340 stores hashed passwords only.
- A patch to the fork that reads users from a credentials file: rejected because the fork carries the engine's fixes only (ADR-058, ADR-336), and every patch is carried across each upstream move; the launcher is DeckStreak's own, small and tested.

D3, the unit:

- `deck-streak-sync-server.service`, `Type=exec`, `KillSignal=SIGINT`, the server on loopback, `SYNC_BASE` under its own state directory, the hardening every DeckStreak unit carries, and the watchdog waived as the replicator's is — chosen, because the server sends no readiness or watchdog notification and stops gracefully on an interrupt only, and the name says what runs.
- `deck-streak-sync.service`: rejected because the ingest's sync job is already `deck-streak-job@sync`, and the journal, the memory watch and the alerts name units by name.
- `Type=notify` with a watchdog: rejected because the server never calls `sd_notify`, so the unit would never be reported ready.

D4, the route:

- On the web app's origin under `/anki-sync/`: a `handle /anki-sync` that redirects to the slash form, and a `handle /anki-sync/*` that strips the prefix, answers the server's health route with 404 at the edge as the API's are (ADR-025), bounds the request body at the server's own payload limit, and proxies to a loopback upstream that render-caddy fills from a fourth key, reading with a larger buffer — chosen, because browser sync is same-origin, the block's TLS and headers then cover the server, and the bounds are the server's own.
- A subdomain or a second site: rejected because browser sync must be same-origin (ADR-340), and a second name needs its own certificate and header block.
- The server's port reached directly: rejected because every client reaches the server over HTTPS (ADR-340), which the edge terminates.
- The path `/sync/`: rejected because the server's own routes begin `/sync/` and `/msync/`, so every request would read `/sync/sync/`, which a reader of any log mistakes for an error.

D5, the snapshot and its drill:

- The daily backup (`backup.py`) also snapshots `SYNC_BASE`: for each user, SQLite's online backup of `collection.anki2` and `media.db`, each checked with `PRAGMA integrity_check`, and the `media/` files, as one archive with a manifest of sha256 digests, kept as the database copies are kept; the archive is copied to an offsite bucket named by configuration, which admits no public access and no listing, by the object-store command the rail provides and names in configuration, run with arguments and no shell; the weekly drill (`restore-drill.sh`) restores the newest archive into a scratch directory, checks the digests and both databases, and opens the collection — chosen, because SPEC-064's units already own copy, check, keep and drill, and a restore that opens the collection is the one ADR-340 asks for.
- The replicator on the collection: rejected because a user's data is two databases and a directory of media files that is no database, and a full upload replaces the collection file by a rename, so a replica of the collection alone would restore without its media and follow a file the server no longer holds.
- A parallel snapshot script and unit: rejected because it is a second schedule, alert and budget entry for what the backup's units already do.
- Copying the files while the server runs: rejected because a copy of a SQLite file being written is not a consistent database, and the online backup is (ADR-064's reason).
- Stopping the server for the snapshot: rejected because it is a daily window in which no client can sync.
- No offsite copy: rejected by ADR-340.

D6, the runbook:

- `docs/runbooks/sync-server-cutover.md`, beside the host scrub's (ADR-060) — chosen, because it is the owner's order of work, not a release file.
- `deploy/`: rejected because the release ships `deploy/` to the host, and a runbook is not something the host reads.
- `RELEASING.md`: rejected because that is the path of every release, and the cutover happens once.

## Decision Outcome

The chosen option of each of D1 to D6.

The unit's ceilings come from SPEC-337's measurement: one full upload of ADR-022's synthetic
collection peaks at 319 MiB resident, against 9.2 MiB idle. The share is fully allotted (four
long-running units and the largest job equal it in memory, and their quotas equal its processors),
so the unit's entry waits on the owner's decision on a resize, which ADR-340 names; this record
proposes no share.

ADR-064 keeps the ingest's collection copy out of every backup, and ADR-321 refused an off-host copy
of the owner's collection for a skip day's take (#108). The sync server's data is not that copy: it
is the owner's own server, which ADR-340 moves here and asks to snapshot offsite. Both rules stand
for what they name, and the snapshot enters the privacy record as a store in the part that builds
it (SPEC-337 section 7). While ADR-340 is proposed, so is this record.

### Consequences

- Good, because the server ships with the engine's pin and the release's provenance, and nothing on
  the host builds it.
- Good, because no user, password or hash appears in the tree or a unit, and a malformed hash stops
  the unit at start.
- Good, because the snapshot and its drill ride units that already page on failure.
- Bad, because the release builds the engine a second time, and the launcher is one more script to
  test and to hold mutation rows for.
- Bad, because the unit cannot be installed until the owner decides the share.

### Confirmation

SPEC-337's acceptance tests (A1 to A10, section 3), its rows
in S33700-S33799, the staging user's rehearsal of the cutover, and the security review before the
server faces the internet (ADR-340).

## What would make this wrong

- The fork's lockfile stops building with DeckStreak's pinned toolchain: the release fails at the
  tag, and a check on the pull request that moves the patch entry should catch it first.
- A client drops the path or the slash, or a full sync needs more than the body bound: the staging
  rehearsal reads it as a failed sync or the server's refusal.
- The server learns to read its users from a file: the launcher then retires.
- The owner keeps the share as it is: the unit cannot run here, and ADR-340's own "What would make
  this wrong" applies.

## More Information

ADR-340, ADR-058, ADR-336, ADR-344, ADR-032, ADR-064, ADR-062, ADR-060, ADR-025, ADR-010, ADR-038,
ADR-321; SPEC-337, SPEC-334, SPEC-064; `docs/schematics/sync-server-packaging-and-cutover.md`; #617,
#161, #167.

## Amendment: the unit's share (SPEC-337)

The share is moved: ADR-064's amendment of this delivery sets it to 1152 MiB, so the unit's entry
no longer waits. This amendment replaces the Decision Outcome's sentence that the entry waits on a
resize and the consequence that the unit cannot be installed until the share is decided; the rest
of that section stands.

D7, the share and the processors:

- `deck-streak-sync-server.service` takes `MemoryHigh=384M` and `MemoryMax=448M`, and the share's
  two processors are split among the five daemons, 200% in all: the API 75%, the sync server 75%,
  the bot 20%, the replicator 15% and the MCP server 15%. The five daemons' ceilings (768M) and the
  largest job's (384M) fill the share of 1152M exactly, and the share test asserts both figures
  EQUAL — chosen, because a full upload is the heaviest burst measured and a client waits on it,
  while the bot's traffic waits on the host and the replicator ships write-ahead-log frames, and
  because the measurement below held at about 75% of a processor. This line replaces the first
  split, which the same measurement refused (the 50% option below).
- Taking the sync server's room from the API: rejected because the API serves the clients' path.
- 25% of a processor for the sync server: rejected because a full upload is bound by the processor
  and the client waits on it.
- 50% for the sync server, with the bot, the replicator and the MCP server at 25% each (the first
  split): rejected because, measured at 50% of one processor, the client gave up at its 30-second
  stall limit while the server completed the upload and answered it with status 200 (SPEC-337 §6).
- 80% for the sync server, with the bot, the replicator and the MCP server at 15% each: rejected
  because no measurement lies above 75% short of 100%, and it takes the bot's headroom for no
  measured gain.
- 100% for the sync server, with a third processor in the share: rejected because the longest
  stall falls only from 14.1 s at about 75% to 12.7 s at 100%: it is the server's import and check
  of the received collection, mostly fixed, so a whole processor buys little. It is held in reserve
  as the share's next step: the cutover runbook's hold applies it at once, with no new decision,
  when the first reading under the real quota fails.

The measurement, ADR-022's shape (250,000 cards, 225,800,192 bytes) by one full upload, the server
held to one processor, read two ways, since a stall can be read at the client's socket by more
than one count:

| setting | the client | longest stall, written bytes | longest stall, acknowledged bytes only |
|---|---|---|---|
| 100% of one processor | succeeded | 12.7 s (a margin of about 17 s on 30 s) | not read |
| about 75% of one processor | succeeded | 14.1 s (a margin of about 16 s on 30 s) | not read |
| 50% of one processor | failed at its 30-second limit; the server answered 200 | not read | 14.1 s |

- Written bytes are the bytes the server acknowledged plus those still queued to send, read at the
  client's socket; it is the better reading, since acknowledged bytes alone count a full send queue
  as a stall the client is not in. The 50% reading took acknowledged bytes only, which is why a
  14.1 s figure sits beside a client that failed at 30 s.
- Each figure is a LOWER BOUND on the client's own stall, since the socket can be busy while the
  client still waits on the server's answer. The client's own result, success or its timeout, is
  the decisive reading.
- The about-75% run held the server to one processor beside one competing process at a lower
  scheduling priority, and the server took 73.2% of that processor; it is an approximation of a
  quota, not a run under `CPUQuota=`. The cutover runbook therefore takes the first reading under
  the unit's real `CPUQuota=75%` on the host, and a client failure or a stall over 20 s there stops
  the cutover (its hold).
- The margin holds for ADR-022's shape only: the runbook measures the full upload again before any
  full upload of a larger collection, under the same stop.

D8, where the credential ids live:

- The two credential ids, `sync-server-owner` and `sync-server-staging`, are shell constants in the
  launcher, `readonly SYNC_SERVER_OWNER=...`, which the deploy-template tests read as they read a
  Rust `pub const` — chosen, because the launcher is the one program that reads them.
- A Rust constant in a crate: rejected because no program would read it, so the census would hold
  a name nothing uses.

D9, what the server reads:

- The launcher clears every `SYNC_` variable, `PASSWORDS_HASHED` and `MAX_SYNC_PAYLOAD_MEGS` from
  its environment before it sets its own, so the server reads exactly two users, its state
  directory and its loopback address, and its payload limit stays its own default of 100 MiB, the
  bound the Caddy block holds the request body to — chosen, because the edge's bound and the
  server's must be the same number.
- Passing the environment through: rejected because a stray `SYNC_USER3` would add a user and a
  stray payload limit would let the server and the edge disagree.

## Amendment: the cutover by a full upload (SPEC-337)

The cutover's data path is decided here; it replaces the schematic's earlier sequence, in which a
snapshot of the old server's store was copied, restored and drilled before the new unit started.

D10, how the data moves:

- A final sync of every client against the old server, then desktop's full upload into the new
  server's EMPTY store, and a full download for every other client — chosen, because the full
  upload and download are the server's own documented path, and the old server stays untouched as
  the rollback.
- Copying the old server's store into the new one: rejected because the store's layout is the
  server's internal one, undocumented and coupled to the server's version.
- Uploading from a client while the old server still serves the others: rejected because two
  servers would then each look current to some client.

D11, a full upload the client reports as failed:

- Before any retry, the server's collection is read back on the host, read only, and its counts are
  matched with desktop's; the upload is repeated only when they differ — chosen, because the
  measured divergence (SPEC-337 §6) is an upload the server completed and the client reported as
  failed.
- Retrying at once on the client's failure: rejected because it repeats an upload that may have
  landed, and it hides the divergence the share must answer.
- Reading the collection back by a second client's full download: rejected because it adds a
  whole collection's download to the frozen window, and puts a client on the new server before
  the counts are known.

## Amendment: the snapshot in a stopped-server window (SPEC-337)

D5 chose SQLite's online backup of both databases while the server runs. Measured since (SPEC-337
§6): the server holds each user's `media.db` in SQLite's exclusive locking mode for its whole life,
so that backup is refused while the server runs, and D5's form cannot take the media index. D5's
rejection of stopping the server ("a daily window in which no client can sync") is outweighed by
that measurement: one owner, a window of seconds, and a client that syncs again at its next sync.
This amendment replaces D5's chosen form for the copy and that one rejection; the archive, its
digests, the offsite copy, the bucket's rules and the drill stand as D5 decided them. It also
replaces the Decision Outcome's sentence that the snapshot enters the privacy record in a later
part: this part enters it (`PRIVACY.md`; `privacy.json` is unchanged, as SPEC-337's file
manifest says).

D12, the snapshot's window:

- A oneshot unit of its own, `deck-streak-sync-snapshot.service`, which the daily backup's run pulls
  in and which runs before it. Its start stops the server first (`Conflicts=` and `After=` the
  server), copies each user's `collection.anki2` and `media.db` by the online backup with no busy
  wait (a progress callback refuses the first busy or locked step, so a process still holding a
  database fails the copy at once) and the `media/` files, all from the one stopped generation,
  into one file published by a rename only when it is whole. PID 1 starts the server again whether
  the copy succeeds or fails (`OnSuccess=` the server; `OnFailure=` the server and the alert). The
  start is bounded (`TimeoutStartSec=`), and a timeout is a failure, so it starts the server too.
  The integrity check, the archive, its digests and the offsite copy run on the copies in the
  backup's own run, after the restart. The window carries no condition and is gated by being
  enabled: the server's `[Install] Also=` names it and its `WantedBy=` is the backup, so enabling
  the server enables the window and disabling the server removes it. Every stop and start is a job
  PID 1 enqueues, so neither unit needs a grant and the rail is unchanged — chosen, because the
  measured lock leaves no consistent copy of `media.db` while the server runs, and the window adds
  no schedule, no grant and no rail change.
- The collection and the media files without `media.db`: rejected because the collection is held
  the same way after a normal sync, per the fork's source, and rebuilding the media index from the
  files alone is not measured.
- A patch to the engine fork that releases the lock: rejected because it widens the fork for an
  operations convenience, and ADR-336 keeps the fork's patches minimal.
- Taking the snapshot as a sync client: rejected because it puts a plaintext credential on the host
  and costs a full download every night.
- A condition on the window, such as a path the server's store must hold: rejected because a
  condition is checked when the start job runs, after the stop the same transaction already holds,
  and a skipped start fires neither `OnSuccess=` nor `OnFailure=`, which leaves the server stopped
  (systemd.unit(5), "Conditions and Asserts"); the model's witness `a-window-gated-by-a-condition`
  reaches exactly that state.
- The backup stopping and starting the server by `systemctl`: rejected because the service user
  would need a new grant to stop another unit, which is a new privilege and a change to the rail.
- A plain file copy of the stopped databases: rejected because a server that ended without a clean
  close leaves a write-ahead log beside its database, which a copy of the main file alone loses and
  the online backup reads through.

What else moves with it, each bounded to the snapshot:

- The backup's unit reaches the bucket for the offsite copy: `RestrictAddressFamilies=` gains
  `AF_INET` and `AF_INET6`, and it reads the settings file (`EnvironmentFile=`) for the copy's
  command and the bucket's name. It gains no other permission.
- The census of keys a paging unit may hold (`scripts/tests/_units.py`, `PAGING_KEYS`) admits
  `Also=` under `[Install]`, and its table of values (`PAGING_VALUES`) bounds it to the one value,
  the window's name; any other value is refused, by key and value.
- The window takes no batch priority and waives nothing for it: it runs with the server stopped,
  so it must not yield, and no timer starts it, so the durable lint's batch-priority rule, which
  judges the jobs a timer starts, does not reach it.

The model `formal/tla/SyncSnapshotWindow` holds the window's interleavings: the copy reads only
while the server is stopped, the archived pair comes from one generation, a client's sync in the
window is refused whole and never half-applied, and the server is up again after every window, a
failed copy included, under fairness on the start. Each property has a witness the check catches:
the copy not ordered after the stop, a generation published before it is whole, a session
committed step by step, a write that lands during the copy, a failed copy that starts nothing, and
a window gated by a condition.

What would make D12 wrong: the fork stops holding `media.db` for the server's life (the online
backup while it runs is then D5's again); the engine stops applying a sync in one transaction (the
refusal is then no longer whole, and the model's switch for it reads the change); or the copy
outgrows its bound, which the window's failure pages as it pages a failed copy.
