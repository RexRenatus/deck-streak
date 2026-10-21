---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Owner sessions live in a bounded in-memory store behind a __Host- cookie, and initData is fresh for one hour

## Context and Problem Statement

ADR-006 decides that the server validates Telegram `initData`, pins it to the owner, and then opens
"a short server session cookie" so later calls need not resend `initData`; it leaves the maximum
`auth_date` age to configuration "with a conservative default". It does not say where sessions are
kept, how long they last, or what that default is. SPEC-024 needs all three, and the answers decide
whether a personal table exists, what an erase must touch, and what the owner sees after a deploy.

## Decision Drivers

- One owner, a few devices; the API is one process on a small host.
- Every stored personal field must be declared, exported and erased (the privacy-gdpr pack).
- A session must end on the server when the owner logs out (the auth pack's
  `auth.logout-server-side`), rotate at every sign-in (`auth.session-rotated-on-login`) and expire
  when idle (cyber-pipeline's `cp.session-idle-timeout`).
- A leaked `initData` string must not stay usable for long (web-security's `ws.tg-init-data-fresh`).
- No new secret to manage.

## Considered Options (the alternatives it was chosen against)

- An in-memory store holding the SHA-256 of each 32-byte random session id, at most 8 live sessions, a 30-minute idle timeout and an 8-hour absolute lifetime; `initData` fresh for 3600 seconds by default — chosen: nothing personal is written to disk, logout and expiry act on the server, and a restart costs the owner one silent re-handshake.
- A SQLite session table — rejected because it adds a personal table (the privacy vocabulary names `sessions`) to declare, export and erase, for the one benefit of surviving a restart, which the Mini App's re-handshake already covers.
- A stateless signed cookie — rejected because it needs a signing key as a new credential and cannot be ended on the server before it expires.
- Validating `initData` on every request with no session — rejected because the raw `initData` would ride every call, and the freshness bound would end the owner's use mid-session after an hour.
- A 24-hour `initData` age, a common library default — rejected because a leaked launch string would open sessions for a day; one hour covers a Mini App launch and its re-handshakes.

## Decision Outcome

Chosen option as above. The session id is drawn from the operating system's generator
(`getrandom`), hex-encoded in the cookie, and only its SHA-256 is kept, so a memory dump holds no
usable id. The oldest session is evicted when a ninth is opened. `DECKSTREAK_INIT_DATA_MAX_AGE_SECONDS`
overrides the freshness bound; a future `auth_date` more than 60 seconds ahead is refused as clock
skew beyond tolerance. `getrandom` is admitted to `[workspace.dependencies]` (`hmac`, `sha2` and
`subtle` are ADR-006's); `tower` for tests is ADR-025's. `form_urlencoded`, which this record first
admitted too, is not: see "Decided at delivery". Handshakes are bounded at 30 a minute per process (web-security's `ws.rate-limit`): an owner opens
the Mini App a few times an hour, so the bound never touches real use, and it caps the HMAC work a
flood of forged payloads can buy.

### Decided at delivery (SPEC-024 §7)

The delivery measured what this record had assumed, and decided each of the following against its
alternatives:

- **The launch data is decoded by one strict decoder, and `form_urlencoded` is not admitted.**
  `form_urlencoded` 1.2.2's `parse` decodes bytes that are not UTF-8 lossily and keeps a malformed
  escape as text, so it cannot refuse a field that does not decode (SPEC-024 R1). Chosen against:
  - `form_urlencoded` alone, which would accept a payload whose hash covers the lossy reading;
  - `form_urlencoded` with a strict pre-check, which is two decoders of one input: a disagreement
    between the check and the reading the hash is verified against is a parser differential, the
    class of flaw a validator must not have.
  The decoder is twenty lines, and the malformed-payload test signs each case over what a lenient
  decoder reads, so only the strict reading refuses it.
- **The handshake's body limit is 16 KiB,** on the handler, inside the shell's 2 MiB (web-security's
  `ws.request-body-limit`). Chosen against the shell's limit alone, which lets each of 30 handshakes
  a minute buffer and parse up to 2 MiB of JSON for launch data of a few kilobytes, and against 4
  KiB, which a user object with long names and a photo address and a long start parameter can
  approach: a refused handshake locks the owner out until the next launch.
- **A handshake ends the session its request carried,** then opens a new one. Chosen against
  leaving the carried session live until it idles out: its id would keep working after a sign-in,
  which rotation exists to prevent (the auth pack's `auth.session-rotated-on-login`), and it would
  hold one of the eight places, so a device signing in again could evict another device's session.
- **The session store compares digests in constant time** (`subtle`, ADR-006) in a list of at most
  eight. Chosen against a map keyed by the digest, whose lookup compares the digest bytes in
  variable time; that leaks only a SHA-256 of a random id, but the constant-time compare costs
  nothing at eight entries and leaves no comparison to argue about.
- **hmac 0.13.0 and sha2 0.11.0,** RustCrypto's current generation (digest 0.11.3), whose
  `verify_slice` compares through `ctutils`. Chosen against hmac 0.12 and sha2 0.10, the generation
  sqlx-core 0.9 still compiles: sharing it would save compiling a second digest generation, and
  would pin the validator to the older line; cargo-deny reports the two generations as duplicate
  versions, which `deny.toml` warns on and does not refuse.

### Consequences

- Good, because the identity context stores nothing personal on disk, and an erase has nothing of
  it to reach.
- Good, because a leaked session id dies within 30 idle minutes, and a leaked `initData` within an
  hour.
- Bad, because every deploy and restart ends every session; the owner reopens the Mini App when the
  launch string is older than an hour.

### Confirmation

SPEC-024's A4, A10, A11 and A15; the auth and cyber-pipeline rows on the box.

## What would make this wrong

- The API runs as more than one process (sessions would need a shared store).
- The owner finds the hourly reopen intrusive in daily use (the bound is configuration; a longer
  default is an owner decision).

## More Information

ADR-006; SPEC-024; `docs/schematics/initdata-auth-sequence.md`; `docs/schematics/owner-session.md`;
the auth, web-security and privacy-gdpr packs.
