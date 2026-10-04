---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The sync server's family runs as its own user, its archive is sealed and dated, and its route is logged, bounded and kept apart from the web session

## Context and Problem Statement

SEC-01's review (#618) recorded eleven findings on #647's sync server, SEC01-F01 to SEC01-F11.
Ruling 235 routes all eleven into SPEC-340, which gates the first deploy (#628). ADR-347 decided
the packaging. This record decides the cures that are new decisions; F01's is ADR-347's amendment
D13, "the sync key and its hash".

## Decision Drivers

- No cure changes the window's order or the three functions the model stamps (ADR-347 D12,
  `formal/tla/SyncSnapshotWindow`).
- The share stays as ruled: no cure moves a memory or processor figure of a daemon.
- Every host change is a recorded step on the owner's go (#161).
- `backup.py` stays standard library only.

## Considered Options (the alternatives it was chosen against)

D1, the sync family's user (SEC01-F02, the sync family's service user):

- The sync server, its window, a new archive unit and a new sync drill unit run as the system user
  and group `deck-streak-sync`, which the rail creates with no login shell, no home and no other
  group. The window's `StateDirectory=deck-streak-sync-snapshots deck-streak-sync-server` keeps its
  order, so `backup.py` reads the snapshots' root first and the store second, as it does now. The
  archive (`backup.py --sync-archive`) and the drill (`restore-drill.sh --part sync`) are oneshots
  pulled in by the backup and by the restore drill, and no unit of either user names the other's
  directory — chosen, because then no directory is shared between the two users and the window's
  body is unchanged.
- A group-readable outbox, the sync user writing sealed archives the backup's user copies offsite:
  rejected because it adds a marker file and a directory two users write to.
- `SupplementaryGroups=` on the backup to read the sync store: rejected because it is a new
  privilege, and it changes the window's modes and so the model's stamped bodies.
- `DynamicUser=`: rejected because its user id is not fixed. A change re-owns the whole store at a
  start inside the window, and the read-back step has no stable owner to read as.
- Keeping one user: rejected by the finding.

D2, the offsite archive's encryption (SEC01-F11, the offsite snapshot's encryption):

- `archive()` seals the archive and its manifest with the command `DECKSTREAK_SNAPSHOT_SEAL` names,
  a command the rail provides with a public recipients file, run as `[*seal, "--output", sealed,
  plain]` with no shell, to the owner's offline public key. It refuses a sealed file that does not
  begin with `age-encryption.org/v1`, copies only the sealed files, and removes them after the
  copy. An unset seal command fails the run with nothing copied. The host keeps its own plain
  archives for the drill — chosen, because the key that opens an offsite copy is then never on the
  host or in the bucket, and the check is one header read.
- The bucket's own encryption alone: rejected because the bucket's holder then holds the key.
- A symmetric key on the host: rejected because the key would sit beside the data it protects.
- A crypto library in `backup.py`: rejected because the backup is standard library only.
- OpenPGP tooling: rejected because its key handling is larger than this one recipient needs.

D3, the login bound (SEC01-F03, the sync route's login bound):

- A ban filter and a jail, shipped as templates under `deploy/fail2ban/`, read the edge's access
  log of the sync route through the journal. They ban an address after five refused logins at
  `/anki-sync/sync/hostKey` within ten minutes, for one hour. The rail installs them on the
  owner's go — chosen, because it needs no custom edge build and no fork change, and it reads the
  log D4 writes anyway.
- The edge's rate-limit module: rejected because it needs a custom build of the edge.
- A fork patch limiting logins: rejected because ADR-336 keeps the fork's patches minimal.
- No bound, relying on the hash's work factor: rejected because the work factor sets the cost of
  one verify, and not how many verifies the server performs.

D4, the edge's log of the sync route (SEC01-F04, the sync route's client address in the record;
SEC01-F05, the sync key kept out of every log; ruling 235's Q2 (b)):

- One site-level `log` in JSON, with `request>headers` deleted and the `k` query parameter deleted
  from `request>uri`, and `log_skip` for every path outside `/anki-sync/`. It names no output, so it
  reaches the journal and its retention — chosen, because only the edge sees the client's address,
  and deleting every header holds against any client header that carries a key.
- The server's client-address header setting: rejected because a direct loopback client without
  the header is then refused by the server.
- Deleting the one `Anki-Sync` header only: rejected because a header added by a later client would
  be logged.
- Logging every route: rejected because only the sync route has no record of its own.

D5, the server's dependency audit (SEC01-F06, the sync server's dependency audit):

- `scripts/audit-sync-server.sh` reads the fork and the commit from `Cargo.toml`'s patch entry, as
  the release step does, and refuses a commit that is not 40 hex. It fetches that commit and runs
  the RustSec advisory check with the house `deny.toml` on `rslib/sync/Cargo.toml`, under the
  fork's lockfile (`--locked`). It runs in a release job, `audit-sync-server`, with
  `permissions: contents: read`, and the release job `needs:` it — chosen, because the shipped
  binary's graph is audited at the moment it is built, and the job that holds write and id-token
  permissions gains nothing.
- A step inside the release job: rejected because it adds a tool and a network fetch to the job
  that holds write and id-token permissions.
- Auditing the fork's whole workspace: rejected because it judges crates the server never builds.
- An audit on pull requests only: rejected because an advisory published after the merge would
  reach the release unread. A pull-request check is #649's.

D6, the server's peers (SEC01-F07, the sync server's network peers):

- `IPAddressAllow=localhost` and `IPAddressDeny=any` on the server's unit — chosen, because the
  server serves the loopback edge and reaches nothing.
- A host firewall rule: rejected because it is a rail change for what the unit can state itself.
- The address families alone: rejected because `RestrictAddressFamilies=` limits families, not
  peers.

D7, the web cookie and the sync route (SEC01-F08, the web session cookie on the sync route):

- In the sync route's `reverse_proxy`, `header_up -Cookie` and `header_down -Set-Cookie` — chosen,
  because the sync server needs no web cookie and must set none on the web origin.
- A path on the session cookie: rejected because a `__Host-` cookie must carry `Path=/`, and the
  edge's removal holds whatever the cookie's attributes are.
- A second origin for the sync route: rejected by ADR-340's same-origin rule.

D8, the hash's shape (SEC01-F09, the sync credential's hash parameters):

- The launcher's entry pattern admits `i=` from 600000 to 999999, an optional `,l=32`, a 16-byte
  salt and a 32-byte digest, each in canonical unpadded standard base64. The runbook names the
  standard-library command that makes it: a 16-byte salt from `os.urandom`, `hashlib.pbkdf2_hmac`
  with sha256, 600000 rounds and a 32-byte digest, the password read without echo — chosen,
  because the pattern alone holds both the floor and a ceiling on the time one verify takes.
- An arithmetic floor with no ceiling: rejected because a hash with tens of millions of rounds would
  slow every login by that much.
- Trusting the runbook's command alone: rejected because nothing at start would refuse a hash of
  another shape.

D9, the offsite archives' period (SEC01-F10, the offsite snapshot's retention statement):

- The bucket's lifecycle rule deletes each archive `P30D` after it is written. `PRIVACY.md` states
  that period, and the runbook checks the rule before the window — chosen, because the policy then
  states a period (GDPR Art. 13(2)(a)), and the host's identity keeps its create-only permission.
- Deletion by the archive unit: rejected because the host's identity would need delete permission
  on the bucket.
- The retention left as the owner's open choice: rejected by the finding.

## Decision Outcome

The chosen option of each of D1 to D9. The share is unchanged. The new units take the backup's
and the drill's own ceilings, and the largest job stays the job template's.

### Consequences

- Good, because a fault in the sync server reaches no file of the API, the bot or the database.
- Good, because an offsite archive opens only with a key that is never on the host.
- Bad, because the rail gains a user, a jail, a seal command and a recipients file, each on the
  owner's go.
- Bad, because a ban acts on the address, and a mistyped password on the owner's own client can
  ban it; the runbook's unban step answers it.

### Confirmation

SPEC-340's acceptance tests A1 to A15, its rows in S34000-S34099, and the `started` step's checks
on the host (#161).

## What would make this wrong

- The fork reads its users from a file, or gives keys a lifetime: D13's exception and the launcher
  move with it.
- The edge's log no longer records the original path: the jail's pattern misses, and the `started`
  check catches it.
- The seal's format changes its header: the archive unit fails closed and pages.

## More Information

SPEC-340; ADR-347 (its amendment D13 and the sentences this record replaces); ADR-340, ADR-336,
ADR-058, ADR-064, ADR-032; #618, #628, #649, #161.
