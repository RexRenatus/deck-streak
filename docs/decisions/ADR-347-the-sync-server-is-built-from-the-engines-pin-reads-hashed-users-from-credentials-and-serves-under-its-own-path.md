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

SPEC-337's acceptance tests (A1, A2 in this part; A3 to A9 in the parts section 7 names), its rows
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
  two processors are split among the five daemons, 200% in all: the API 75%, the sync server 50%,
  the bot 25%, the replicator 25% and the MCP server 25%. The five daemons' ceilings (768M) and the
  largest job's (384M) fill the share of 1152M exactly, and the share test asserts both figures
  EQUAL — chosen, because a full upload is the heaviest burst measured and a client waits on it,
  while the bot's traffic waits on the host and the replicator ships write-ahead-log frames.
- Taking the sync server's room from the API: rejected because the API serves the clients' path.
- 25% of a processor for the sync server: rejected because a full upload is bound by the processor
  and the client waits on it.

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
