---
status: "proposed"
---

# ADR-602: every sync op body is sealed on the device under one account key that each device, each passkey and a recovery key unwrap, and the server keeps clear heads and one daily record

Decides SPEC-583 (issue #1079). It supersedes the earlier sync scope, never recorded in this
tree, under which only palace events and tags were sealed and reviews and notes stayed readable
by the server: from this record on, every sync op body is sealed on the device. It widens the
sealed relay that was designed for palace events alone to every op body. Sealed sync goes live at
the sync cut-over, the change that retires the stock-app sync path and starts DeckStreak's own
operation-log sync, and never before it. No line of an earlier SPEC or ADR changes here (D5).

## Context and Problem Statement

DeckStreak's server reads its learner's study data today, and DeckStreak's own sync is to replace
that path with one in which the server orders and stores ops and computes no Rex state. In it,
every op body (notes, media, deck names, reviews, palace and tags) is sealed on the device, the
server sees only op heads, sizes and one declared record per study day, and a learner who loses a
device gets the data back through a passkey, another device or a printed key, never through email.
This record decides how. Measured at the dev this change is cut from:

- The sync server stores each sync user's collection, media index and media files readable, under
  two logins fixed at start (`deploy/scripts/sync-server.sh:78-80`;
  `deploy/systemd/deck-streak-sync-server.service:30-31`; `deploy/scripts/backup.py:50-52`).
- The server keeps a private copy of the collection and reads it (`crates/ingest/src/lib.rs:3-6`;
  `crates/ingest/src/settings.rs:26`). Nine contexts depend on `ingest`: analytics, progression,
  curriculum, quests, discipline, the reading context, insights, coordination and the daemon
  (`docs/CONTEXT-MAP.md:16`, `:17`, `:19`, `:21`, `:24`, `:27`, `:30`, `:33`, `:40`).
- DeckStreak's own schema is 46 tables (`docs/CONTEXT-MAP.md:226-273`): 36 exported and erased, 7
  reset in place and 3 exempt.
- There is no account table: the one identity table, `passkeys`, is keyed by the Telegram user id
  (`migrations/035901_identity_passkeys.sql:10-20`). The web client posts every passkey ceremony
  with `extensions: {}` (`web/app/src/lib/passkeys.ts:140-170`), so no extension result reaches
  the server, and no file names the passkey PRF extension.
- The web client keeps today's sync key as an AES-GCM sealed record in IndexedDB, opened only in
  its Worker (ADR-374 D1, `:60`); the iPhone and iPad app keeps it in one Keychain item, this
  device only (`ios/App/Sources/SyncCredentialStore.swift:38`).
- The workspace already depends on `aes-gcm`, `hkdf`, `sha2`, `hmac`, `p256` and `getrandom`
  (`Cargo.toml:110-113`, `:159-163`); the lock holds no X25519, ML-KEM or Ed25519 crate.
- The charter has Rex run on the device and the server only relay (`CHARTER.md:133-134`), and
  the leaderboard ranks a learner against their own past only (`crates/progression/src/board.rs:1-2`),
  so no feature compares learners' reviews.
- The sync core that holds the op head, the op body and the merge judge is not on dev: it arrives
  in an open sibling change. Its judge decides by the op's origin, sequence number and hybrid
  clock, the register it writes and the revision that write was based on, and treats a field's
  value as opaque. This record states what the head must carry; the sealing core maps it (D3).
- The stock-app protocol stores the collection on the server and has no client-side sealing, so
  sealed sync cannot start before the cut-over.

## Decision Drivers

- The server never holds a key in a form it can use, so it can never open an op body.
- Nothing the merge judge needs is sealed.
- Email opens the account and never the data; no key is escrowed.
- Both apps seal through one implementation and one set of test vectors.
- A crate the tree lacks is a question, never a silent addition.
- Sealed sync goes live at the cut-over and never before it.
- Public text states the product change only.

## D1, the surfaces, measured: what it was chosen against

The census `docs/sealed-sync/device-census.md` lists every surface the sealing changes, each by
path and line at the dev this change is cut from: the sync path, the private copy, the nine contexts and the 46
tables, the export and erase paths, the backups, the identity and passkey surfaces an unlock
rides, the privacy page's statements, and the tests and rows that pin them.

- Chosen: every surface measured at dev by path and line, because a later build item cuts from dev and needs line citations dev holds
- The planning measurement taken on the sibling change's branch: rejected because the sync core and the per-user store it cites are not on dev, so its line numbers name files dev lacks

## D2, the key hierarchy: what it was chosen against

- **The account key** is 256 random bits from the operating system's generator, made on the
  learner's first device at setup, before the first sync. It is the one root of the learner's
  data, and it has epochs: each removal starts a new one. The account key of each epoch seals the
  previous epoch's key in a chain record, so a device that joins later reads older bodies.
- **Each device key** is a hybrid key-encapsulation key pair, X25519 with ML-KEM-768 joined by the
  X-Wing combiner, made on the device. On the iPhone and iPad its private half is one Keychain
  item, available after first unlock and this device only, never synchronised; on the web it is
  kept in the Worker's IndexedDB, sealed under a non-extractable AES-GCM key. Its public half sits
  in the device list.
- **Each passkey slot** is a hybrid key pair made when the slot is set up. Its private half is
  sealed under a key derived by HKDF-SHA-256 from the passkey's PRF output, with the slot's random
  salt and the label `deckstreak passkey slot v1`. The client adds the PRF input to the ceremony
  itself, keeps the output in memory, and still posts `extensions: {}`, so the output never
  reaches the server.
- **The recovery key's slot** holds a hybrid public key derived from 256 random bits that are
  shown once to be printed. Only the public half is stored. This record fixes the slot's place;
  the printable key and its flow on both apps are the next delivery.
- **Every wrap** seals the epoch's account key to one slot's public half by HPKE base mode
  (RFC 9180) with the X-Wing KEM, HKDF-SHA-256 and AES-256-GCM, its info string binding the account
  id, the slot id and the epoch.
- **The device list** is a sequence of versions, each sealed under the epoch's account key with
  its version number in the associated data. A device wraps only to the slots of the newest version
  it has opened, and it refuses a version older than the newest it has seen. The server stores
  versions and never writes one.
- **A new device joins** in one of three ways: an existing device checks a short code of the new
  device's public key and wraps the account key to it; a PRF passkey opens its slot; or the
  recovery key opens its slot. The joined device then adds its own slot in a new list version.
- **A device is removed** by a remaining device, which writes a list without it, makes the next
  epoch's account key, and wraps it to every remaining slot.
- **The server stores wrapped records only:** each slot's public half, each sealed account-key
  record, each passkey slot's salt and sealed private half, the sealed list versions and the chain
  records. Signing in by email opens the account's session and reaches none of them.

- Chosen: one random account key with epochs, wrapped to a hybrid public key per slot, because any one device can re-wrap every slot at a rotation with no passkey or printed key present
- A wrap per passkey from its PRF output alone: rejected because a rotation would then need every passkey present, so a removed device's epoch could not be closed
- A key derived from a password: rejected because sign-in is passwordless, so it adds a secret the learner must remember
- Escrow by the server or by email: rejected because email must open the account and never the data, and an escrowed key makes every sealed body readable by the server side
- Device keys with no recovery slot: rejected because a learner who loses every device and passkey would then lose the data with no way back
- X25519 alone: rejected because sealed bodies are kept for years, and an exchange recorded today could be opened by a later quantum attack, which the hybrid resists
- ML-KEM-768 alone: rejected because it rests on one young assumption, and the hybrid holds while either half holds
- A P-256 and ML-KEM-768 hybrid that reuses the tree's `p256` crate: rejected as the default because no specified combiner names that pair for HPKE, while X-Wing is specified; it stays an option of the crate question
- A signing key per device for the device list: rejected because a list sealed under the account key is already authenticated to every holder of that key, and the server holds none

## D3, the sealed op body and the clear head: what it was chosen against

- **The clear head** carries exactly: the envelope version (1 byte); the key id, the epoch number
  (4 bytes); the origin device id (16 bytes); the origin's sequence number (8 bytes); the hybrid
  clock key (12 bytes); the class, append, register or tombstone (1 byte); the register address
  (16 bytes, empty for an append); the base revision (8 bytes); and the AES-GCM nonce (12 bytes).
  The register address is a keyed hash of the entity and field under an address key derived from
  the account's first epoch key, so the judge compares addresses for equality and learns no card,
  note or field id.
- **The sealed body** is the op's encoded body, padded and sealed by AES-256-GCM under a per-op
  key derived by HKDF-SHA-256 from the epoch's body key, with the origin and sequence number as
  salt and the label `deckstreak op body v1`, under a random nonce. The associated data is the
  account id and every head byte, so the server cannot move a body to another head.
- **Padding** appends one `0x80` byte and zeros up to the next multiple of 256 bytes, the rule the
  palace relay already had. Media objects travel beside the log, padded by the Padmé rule and
  addressed by an HMAC-SHA-256 under a per-account media key, never by the plaintext hash.
- **The op format:** the sync core's op head keeps its origin, sequence number and hybrid clock and
  gains the version, key id, class, register address, base revision and nonce; the op body's bytes
  become the envelope's ciphertext; the judge reads the head alone. The server-built snapshot is
  replaced by a sealed checkpoint a device makes, and the server keeps the ops after the newest.

- Chosen: a clear head of exactly the judge's inputs plus the envelope's version, key id and nonce, because the judge decides by origin, sequence, clock, register and base revision alone, and binding the head as associated data keeps each body on its head
- Register addresses in the clear: rejected because the judge needs only their equality, and a keyed hash keeps equality while hiding which card, note or field changed
- The op kind in the clear: rejected because the judge needs only the class, and the kind would tell the server which feature each op touched
- A random nonce under the epoch key alone: rejected because a restored device that repeats a sequence number would then rest on nonce chance alone, while a per-op key also needs a repeated nonce to fail
- XChaCha20-Poly1305: rejected because AES-256-GCM is already a workspace dependency, and the per-op key removes the nonce-length reason for the longer nonce
- AES-GCM-SIV: rejected because it adds a crate for a misuse the per-op key and the random nonce already cover
- No padding: rejected because a body's exact length tells a review from a note edit
- Padmé for op bodies: rejected because nearly every op body is under 256 bytes, so one 256-byte class hides more; Padmé is chosen for media, whose sizes span megabytes
- A plaintext SHA-256 media address: rejected because equal hashes tell the server that two learners hold one file

## D4, the daily-facts record and the device rework: what it was chosen against

- **The record** is one clear row per account per study day: the date (the study day), the review
  count, the minutes and the XP granted. A later record for the same date replaces the earlier.
- **The server checks** that it has exactly the four fields; that the date is within one day of
  the learner's current study day by the `study-day` rule (`docs/CONTEXT-MAP.md:45`); that the
  review count is at most the number of append heads the learner pushed with a clock inside that
  study day; that the minutes are at most 1440; and that the XP granted is at most the ceiling the
  `xp` crate's rule gives for that review count (`docs/CONTEXT-MAP.md:44`). A record that fails is
  refused and kept out, and the device is told why.
- **It feeds** streaks, freezes, relights and the governor (the date and a non-zero review count),
  coins and quests (the counts, minutes and XP), and push budgets (whether the learner studied that
  day). Quotas read the heads and the stored ciphertext bytes, never the record. XP is reported by
  the device and bounded by the server.
- **The device rework** is censused in `docs/sealed-sync/device-census.md`: the nine contexts with
  their measured sizes, and DeckStreak's 46 tables, 44 listed with a verdict (22 stay, 9 move, 3
  split, 2 held, 3 retire, 5 unchanged) and 2 that name the parked surface counted by name only.

- Chosen: four clear fields per study day, bounded by what the server can count from the heads and by the XP rule's own ceiling, because the server features that stay need no more, and a record this short can be stated whole on the privacy page
- A clear due count for push reminders: rejected because the record holds the four fields the change names, and the device schedules its own reminders
- The device's XP unbounded: rejected because a ceiling the server computes costs nothing and keeps a malformed record from paying out
- XP recomputed on the server: rejected because the server cannot read the reviews the XP rule reads
- A census over the predecessor's 64-table register: rejected because dev's schema is DeckStreak's own 46 tables, and the predecessor's import follows each moved table to the device
- Every context moved to the device: rejected because streaks, the economy, quests and the router need only the daily record, and a pure relay would end the server's nudges

## D5, records: what it was chosen against

This record supersedes the earlier scope, widens the palace relay to every op body, and fixes
that sealed sync goes live at the cut-over. No SPEC or ADR is amended now: every sentence that
says the server reads the collection, the notes, the reviews, the deck names or the media
describes the stock-app path and stays true until the cut-over, and the census lists each by path
and line for the cut-over item (#1086). `PRIVACY.md` is unchanged, as ADR-379 has it
state what happens today: the text the cut-over installs is written whole now in
`docs/sealed-sync/privacy-at-the-cut-over.md`, and a test refuses a sealed-sync claim in
`PRIVACY.md` while the stock-app path's `ingest` crate is in the tree.

- Chosen: no amendment now, each falsified sentence listed for the cut-over, and the privacy text written whole and staged, because each sentence stays true until the cut-over and the privacy page states what happens today
- Insert-only amendments now: rejected because they would describe as live a change that starts only at the cut-over
- A new section in `PRIVACY.md` now: rejected because the page would describe a capability the app lacks (ADR-379), and an open sibling change edits the same file
- New `privacy.json` categories now: rejected because its checks refuse a store no migration creates, and the slot, op-log and daily-record tables arrive with their build items

## D6, shape, tests and rows: what it was chosen against

One documents delivery: SPEC-583, this record, the schematic `docs/schematics/sealed-sync-keys.md`,
the census, the staged privacy text, the changelog fragment, the red-first record, one test module
`scripts/tests/test_sealed_sync_records.py` and the rows band S58300-S58399 (rows S58301 to
S58314). Every red runs locally in Python over planted fixtures, so the delivery has one push.
FORMAL: not applicable here, since no code changes; the sealing core carries a TLA+ model of a
rotation racing a push and a join, and a Lean proof of the padding rule and the record's bounds.

- Chosen: one test module whose checkers read the new documents against the context map, each refusal planted beside a green control, because every fact the delivery adds is a table or a sentence a test can read
- A test module per document: rejected because the census and the SPEC's tables are read against one map, and one module keeps one reader of it
- Two pushes: rejected because no red here is read only in CI

## D7, not this round: what it was chosen against

The recovery key's flow on both apps (#1087); the at-rest keys and the backups
(#1088); AI over sealed data, on the device or by a grant (#1089); the sealing core
(#1080); the key hierarchy's client code on the iPhone and iPad app (#1081)
and on the web (#1082); the relay that stores sealed bodies (#1083); the daily
record (#1084); each context's move (the census's items); the export and erase over sealed
data (#1085); and the cut-over itself (#1086). The engine, the scheduler,
the web screens and the sign-in are unchanged.

- Chosen: the design and its records only, because the sealing code needs the crate answer and the sync core it extends is not on dev
- The sealing core in this delivery: rejected because its crates are an open question and its op head is not on dev to extend

## D8, public text: what it was chosen against

- Chosen: every document states the product change, the keys, the head, the record and what moves, because that is what a reader of the repository needs
- Recounting the planning that led here: rejected because the public record states the change, not its private reasons

## Decision Outcome

D2, D3 and D4 are the design SPEC-583 records; D5 and D6 shape this delivery; D1, D7 and D8 bound
it. Each later build item cites this record.

### Consequences

- Good, because a reader of the server's store, its backups or its logs sees ciphertext, heads,
  sizes, wrapped keys and four numbers a day, and never a note, a review or a deck name.
- Good, because the judge's conflict rule is unchanged: it never read a value.
- Good, because a removed device is shut out of every op sealed after its removal.
- Bad, because a learner who loses every device, every PRF passkey and the printed key loses the
  data for good.
- Bad, because the web app's sealing code is served by DeckStreak's own site: on the web the seal
  holds against the store, the backups and the logs, and trusts the released app the site serves.
- Bad, because XP becomes device-reported, bounded but not proved, and cross-learner media dedup
  ends.
- Bad, because the heads still show when a learner studies, how many ops a day, and which
  addresses repeat.

### Confirmation

SPEC-583's acceptance criteria A1 to A11, its red-first record, its rows S58301 to S58314, and CI's
scripts-tests stage at the pull request's head.

## What would make this wrong

- A PRF output or an unwrapped key that reaches the server through any posted field.
- A platform on which a device key cannot be held for this device only.
- A weakness found in the X-Wing combination or in ML-KEM-768 and X25519 together.
- A judge rule that needs a field this record seals.
- A feature that needs another learner's reviews.

## More Information

Built by SPEC-583; drawn in `docs/schematics/sealed-sync-keys.md`; censused in
`docs/sealed-sync/device-census.md`. CHARTER item 22 (Rex on the device), ADR-374 (today's web sync
key), ADR-358 (today's iPhone and iPad Keychain item), ADR-370 (passkey sign-in), ADR-379 (the
privacy page states today), ADR-371 (the XP rule) and ADR-515 (the study day).
