---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner, through the maintainer at gate 6), the DeckStreak architect"
---

# Credentials come from the secret manager at each unit start, and are never stored on the host

## Context and Problem Statement

ADR-010 chose to store each credential on the host encrypted with `systemd-creds` and to load it
with `LoadCredentialEncrypted=`. At gate 6 the owner, by the maintainer's decision, approved
reusing the Anki login on condition that its values are read from the secret manager at run time
on the host, and are never on disk, in the public repository, in CI or in a log. An encrypted blob
on the host's disk does not meet that condition. How do DeckStreak's units receive their
credentials?

## Decision Drivers

- No credential value on disk, encrypted or not, and none in an environment variable, a command
  line or a log (CHARTER).
- The kernel's loader already reads `$CREDENTIALS_DIRECTORY/<id>` (SPEC-020 R11). The delivery
  mechanism should not change the code.
- The grant that lets the host read the secrets is an owner gate (gate 2), so the mechanism must
  name exactly what it reads.

## Considered Options (the alternatives it was chosen against)

- A credential socket fed from the secret manager: chosen, because the value travels from the
  secret manager into systemd's credentials directory at each start, and nothing is written to disk.
  Each unit declares `LoadCredential=<id>:/run/deck-streak-credentials/socket`, and a
  socket-activated fetch helper answers each connection. It serves only systemd, and only the
  (unit, credential) pairs of the private rail's map. The public templates name only DeckStreak's
  own credential ids and the socket path.
- `systemd-creds encrypt` with `LoadCredentialEncrypted=` (ADR-010): rejected because the
  ciphertext is on the host's disk, which the gate's condition excludes.
- Fetch into a file under `/run` in an `ExecStartPre=`: rejected because `/run` is tmpfs, whose
  pages can be swapped to disk, and a second copy outlives the unit.
- Read the secret manager from the application at start: rejected because it would spread
  secret-manager access through the code, and because every role would need the client library
  and the grant. The socket keeps one small reader.

## Decision Outcome

Chosen option.
- **The templates.** The unit templates (SPEC-032, SPEC-031) use
  `LoadCredential=<id>:/run/deck-streak-credentials/socket` for every credential, and never
  `LoadCredentialEncrypted=`, `Environment=` or a literal.
- **The socket serves only systemd.**
  - The socket unit is root-only: `SocketUser=root`, `SocketGroup=root`, `SocketMode=0600`, and
    `DirectoryMode=0700`, so its directory is root's alone. No service user can reach the path, or
    replace the socket to feed systemd a forged credential.
  - `Accept=yes` gives each connection its own helper instance.
- **The helper answers only mapped pairs.**
  - It reads the peer's credentials (`SO_PEERCRED`) and serves only uid 0.
  - It reads the peer's name with `getpeername(2)`. systemd binds its end of the connection to the
    abstract address NUL, random, `/unit/`, the unit's name, `/`, the credential id (systemd.exec(5),
    `LoadCredential=`).
  - It answers only a (unit, credential id) pair in the private rail's map, and closes the
    connection on anything else: a non-abstract peer, an unmapped unit, or a mapped unit asking for
    another row's id. A refusal is logged with the unit and the id, never a value.
- **Where the pieces live.** The fetch helper, its socket unit and the map live in the private deploy
  rail (#41), outside this repository. The helper reads the secret manager with the host's own
  identity. The grant that allows it, on each named secret only, is decided at gate 2 (#161).
- If a read fails, the unit does not start, and its `OnFailure=` alert names the credential id,
  never the value.

This supersedes ADR-010's credential storage (`systemd-creds encrypt` and
`LoadCredentialEncrypted=`). The rest of ADR-010 stands.

### Consequences

- Good, because no credential value is ever at rest on the host's disk, and a secret rotated in the
  secret manager reaches each unit at its next start.
- Bad, because systemd backs the credentials directory with non-swappable memory only when it can
  (ramfs), and falls back to tmpfs otherwise, whose pages can reach swap. The rehearsal measures
  which one it is. tmpfs on a host with swap is a finding for the owner (gate 8), not a pass.
- Good, because the kernel's loader and its tests are unchanged (SPEC-020).
- Bad, because a unit cannot start while the secret manager is unreachable. The alert says which
  credential failed, and the service's restart policy retries.

### Confirmation

- SPEC-032's template census: every credential line has the socket form, and no unit carries
  `LoadCredentialEncrypted=`.
- W2's rehearsal on the host (gate 2), each proof recorded:
  1. a unit starts and its credential is present under `$CREDENTIALS_DIRECTORY`;
  2. `find / -xdev` finds no copy of it;
  3. a non-root process's `connect()` to the socket is refused;
  4. a root connection without systemd's abstract address gets nothing;
  5. a unit asking for a credential id outside its row gets nothing;
  6. `findmnt /run/credentials/<unit>` shows the filesystem type (ramfs, or a finding);
  7. `systemctl --version` on the host confirms the peer-address format in its own
     `systemd.exec(5)` before anything relies on it.

## What would make this wrong

- The host's systemd cannot load a credential from a socket, or names the peer differently. The
  rehearsal's first check reads the host's own manual, and the socket is not deployed until the
  format matches.
- The secret manager's availability becomes a start-up risk the owner will not accept. A
  memory-only cache in the helper, never on disk, would then be the next step.

## More Information

ADR-010 (partly superseded), ADR-037, SPEC-020 R11, SPEC-031, SPEC-032; systemd.exec(5)
`LoadCredential=` (AF_UNIX sockets) and the systemd credentials documentation.

Amendment (2026-09-28): one passage stating a version of the host's software, in the decision
outcome, was redacted under the public-prose rule (ADR-059).
