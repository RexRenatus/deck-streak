---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The sync server moves to the service's own host, behind HTTPS, with hashed passwords and an offsite snapshot

## Context and Problem Statement

Desktop Anki and AnkiMobile sync to the owner's own sync server (the owner's answer ANK-08), which
runs today on the current third-party host. DeckStreak's ingest reads a private copy from it
(CHARTER 4). The owner chose to move it to the service's own host, so the service lives on one
host (SRV-01), behind HTTPS with hashed passwords (ANK-07). The new clients add two needs: the
browser client syncs from the web app's page, so the sync server must answer on the same origin,
and a one-way upload must first find an offsite snapshot of the server's data (SPEC-334 R8). How
does the sync server move without losing data, and how is it run once it has?

## Decision Drivers

- No review is lost: a snapshot that has been restored once exists before anything moves.
- The host never compiles; binaries are built elsewhere and deployed from a SemVer tag on `main`
  (CHARTER 3).
- Every DeckStreak unit runs within its own entry of the host budget (ADR-032).
- Browser sync is same-origin, and every client reaches the server over HTTPS.
- A change to the host's reverse proxy or firewall is the owner's go (`RELEASING.md`, #161).
- The old server stays reachable as the rollback until the owner retires it.

## Considered Options (the alternatives it was chosen against)

- Move it to the service's own host, behind its HTTPS reverse proxy on the web app's origin, with hashed passwords, an offsite snapshot and a drilled restore, in one cutover — chosen because the owner chose it (SRV-01), it leaves one host to run and back up, browser sync is same-origin, and ingest reads the server on the same host.
- Keep it on the current third-party host and add HTTPS there — rejected because the owner chose one host for the service, two hosts must each be run and backed up, and browser sync would cross origins.
- A new host for the sync server alone — rejected because it adds a host to run and pay for, and the owner chose the service's own host.
- Anki's hosted sync service — rejected because DeckStreak reads a self-hosted sync server only (the PRD's non-goals, #172).

## Decision Outcome

Proposed option: the move to the service's own host, behind HTTPS.

- **The build.** The release workflow builds the sync server from the pinned fork (ADR-058, as
  ADR-336 extends it) on a SemVer tag on `main`, as it builds every DeckStreak binary, and the
  deploy installs that artifact.
- **The unit.** The sync server runs as its own unit, with its own entry in the host budget
  (ADR-032), sized from a measurement of the host's memory and disk taken before the move. The
  unit's data directory is the one the snapshot copies.
- **The route.** The host's reverse proxy serves the sync server on the web app's origin, under
  its own path, with TLS. The server stores hashed passwords only. The route, the unit and any
  firewall change are host changes, made on the owner's go (#161).
- **The snapshot and the drill.** Before the move, an offsite snapshot of the sync data exists,
  and its restore has been drilled into a scratch directory and opened. The offsite copy admits
  no public access. The same snapshot is what a one-way upload checks for (SPEC-334 R8).
- **The review.** A security review of the server, the browser's sync credential and the route
  runs before the server faces the internet; any host finding goes to #167, never into public
  text.
- **The cutover.** One window: a final sync on every client; a freeze; the snapshot and the copy
  of the sync data; the server started on the service's own host; desktop repointed, then
  AnkiMobile, then the app; and a new sync password set. A staging sync user, holding a scrubbed
  copy, rehearses the cutover first, and it is the user that dev builds of the app sync to
  (ADR-344). Repointing the owner's own devices is the owner's act.
- **The rollback.** The current third-party host keeps its server, untouched, until the owner
  retires it; retiring it is the owner's act, asked at the time.

### Consequences

- Good, because the service and its sync server live on one host, behind one HTTPS origin, and
  ingest reads the synced copy locally.
- Good, because a drilled snapshot exists before any one-way upload, from the first day.
- Bad, because the host carries one more unit within its budget, and the snapshot is one more
  backup to keep and drill.
- Bad, because the cutover is a window in which no client may sync.

### Confirmation

- The packaging delivery's tests: the unit and the route render from the deploy templates, and
  the host budget names the sync unit.
- The restore drill's record, and the security review's verdict before the first deploy of the
  unit.
- After the cutover, a sync from desktop, AnkiMobile and the app against the new route.

## What would make this wrong

- The host's measured memory or disk cannot take the sync server within the budget, which the
  measurement before the move reads; a resize is then the owner's decision.
- The sync server cannot serve on a path of the web app's origin, which the packaging delivery
  checks before the cutover.

## More Information

- SPEC-334 (row 1.3; R8, R9).
- ADR-058 and ADR-336 (the fork), ADR-032 (the host budget), ADR-010 (deployment, backup and
  secrets), ADR-344 (the staging sync user), `RELEASING.md`, #161, #167.
