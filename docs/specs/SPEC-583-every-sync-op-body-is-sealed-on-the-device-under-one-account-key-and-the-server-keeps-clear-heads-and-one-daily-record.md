# SPEC-583: every sync op body is sealed on the device under one account key, and the server keeps clear heads and one daily record

- **Issue:** #1079. **Context(s):** the sync that replaces the stock-app path at the cut-over,
  `identity` (the passkeys an unlock rides), the nine contexts that read study data through
  `ingest`, `privacy` (export and erase), and `docs/`.
- **Decided by:** ADR-602 (every sync op body is sealed on the device under one account key that
  each device, each passkey and a recovery key unwrap); ADR-379 (the privacy page states today);
  ADR-371 (the XP rule); ADR-515 (the study day); ADR-374 and ADR-358 (today's sync key on each
  app).
- **Schematic:** `docs/schematics/sealed-sync-keys.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-583.md`; the design it records goes live at the sync cut-over and never
  before it. **Mutation band:** S58300-S58399. **Models:** none (ADR-602 D6).

## 1. The problem, measured

At the dev this change is cut from, DeckStreak's server can read everything a learner studies:

- The sync server stores each sync user's collection, media index and media files readable
  (`deploy/scripts/sync-server.sh:78-80`; `deploy/scripts/backup.py:50-52`), and the service keeps
  a private copy it reads (`crates/ingest/src/lib.rs:3-6`).
- Nine contexts depend on `ingest` (`docs/CONTEXT-MAP.md:16`, `:17`, `:19`, `:21`, `:24`, `:27`,
  `:30`, `:33`, `:40`), and DeckStreak's own schema holds 46 tables (`docs/CONTEXT-MAP.md:226-273`).
- The stock-app protocol cannot seal a body on the device, so sealing starts with DeckStreak's own
  sync at the cut-over.
- No account table exists (`migrations/035901_identity_passkeys.sql:12` keys passkeys by the
  Telegram user id); the web client posts each passkey ceremony with `extensions: {}`
  (`web/app/src/lib/passkeys.ts:140-170`); no file names the passkey PRF extension.

DeckStreak's own sync seals every op body on the device, and email never opens the data. This SPEC
records how:
the keys, the clear head, the sealed body, the daily record and the census of what moves to the
device, each held by a test.

## 2. Requirements

R1. The key hierarchy is the four keys of this table, each made, kept and wrapping as it says:

| key | made | kept | wraps | the server stores |
|---|---|---|---|---|
| account key | 256 random bits on the first device, before the first sync; a new one per epoch | in memory on each unlocked device | each op body key, and the previous epoch's account key | a sealed copy per slot, and a sealed chain record per epoch |
| device key | a hybrid X25519 and ML-KEM-768 key pair on each device | its private half in the device's own key store, this device only | nothing; it unwraps the account key | a public half, inside a sealed device-list version |
| passkey PRF slot | a hybrid key pair when a PRF passkey is set up | its private half sealed under a key the passkey's PRF output derives | nothing; it unwraps the account key | a public half, a random salt and a sealed private half |
| recovery key slot | a hybrid key pair derived from 256 random bits printed once | the printed key, by the learner | nothing; it unwraps the account key | a public half only |

R2. The server never stores a key in a form it can use: every key it holds is a public half or is
    sealed under a key it lacks. A PRF output, an account key and every private half stay on the
    device, and a ceremony still posts `extensions: {}`. Signing in by email opens the account's
    session and no key record.

R3. Every wrap seals the epoch's account key to one slot's public half by HPKE base mode (RFC 9180)
    with the X-Wing KEM, HKDF-SHA-256 and AES-256-GCM, its info string binding the account id, the
    slot id and the epoch. A device list version is sealed under the epoch's account key with its
    version number in the associated data; a device wraps only to the newest version it opened, and
    refuses one older than the newest it has seen.

R4. A device joins by another device's check of a short code of its public key, by a PRF passkey,
    or by the recovery key, and then adds its own slot. A removal writes a list without the removed
    slot and starts the next epoch, wrapped to every remaining slot.

R5. An op's clear head holds exactly the fields of this table; every other byte of the op is
    sealed:

| head field | bytes | why the server needs it |
|---|---|---|
| version | 1 | which envelope layout the bytes follow |
| key id | 4 | the epoch whose key sealed the body |
| origin | 16 | the device that made the op, for deduplication |
| sequence | 8 | the origin's op number, for deduplication |
| clock | 12 | the hybrid clock key the judge orders by |
| class | 1 | append, register or tombstone, which the judge treats differently |
| register | 16 | a keyed hash of the entity and field, compared for equality; empty for an append |
| base revision | 8 | the register revision a write was based on |
| nonce | 12 | the AES-GCM nonce of the body |

R6. The body is the op's encoded body with one `0x80` byte and zeros appended to the next multiple
    of 256 bytes, sealed by AES-256-GCM under a per-op key that HKDF-SHA-256 derives from the
    epoch's body key with the origin and sequence as salt, under a random nonce, with the account
    id and every head byte as associated data. Media objects are padded by the Padmé rule and
    addressed by an HMAC-SHA-256 under a per-account media key.

R7. The daily record holds exactly these four fields, in this order, one record per account per
    study day, a later record for a date replacing the earlier:

| daily-facts field | type | server check |
|---|---|---|
| date | the study day, an ISO date | within one day of the learner's current study day |
| reviews | an unsigned count | at most the append heads pushed with a clock inside that study day |
| minutes | an unsigned count | at most 1440 |
| xp granted | an unsigned count | at most the XP rule's ceiling for that review count |

R8. The record feeds streaks, freezes, relights, the governor, coins, quests and push budgets; a
    refused record is kept out and the device is told why. Quotas read heads and ciphertext bytes.

R9. `docs/sealed-sync/device-census.md` lists every context the context map says depends on
    `ingest`, each with a verdict and its measured size, and every table of the map's register of
    DeckStreak's own tables exactly once with its owning context and a verdict; a register row that
    names the parked surface is counted, not listed. A verdict is one of `stays`, `moves`, `split`,
    `held`, `retires` and `unchanged`.

R10. The census lists, by path and line, every record whose sentence the sealing falsifies at the
     cut-over, and each later build item as a proposed issue.

R11. `docs/sealed-sync/privacy-at-the-cut-over.md` is the privacy text the cut-over installs, whole:
     it names notes, media, deck names, reviews, palace and tags as what the server can never read,
     and op heads, sizes, wrapped keys and the four fields as what it holds.

R12. `PRIVACY.md` makes no sealed-sync claim while the stock-app path's `crates/ingest/src/lib.rs` is
     in the tree: sealed sync goes live at the cut-over and never before it.

R13. One test module, `scripts/tests/test_sealed_sync_records.py`, holds every criterion below: each
     checker returns its findings, each refusal is planted over a fixture the test spells beside a
     control that reads green, and each enumeration reports and refuses an empty population.

## 3. Acceptance criteria of SPEC-583

| id | criterion | decided by |
|---|---|---|
| A1 | the key table names exactly the four keys, and every server cell names a sealed or public form only (R1, R2) | `test_sealed_sync_records.py` |
| A2 | a planted key table whose server cell holds a key in the clear is refused (R2) | `test_sealed_sync_records.py` |
| A3 | the head table names exactly the nine head fields (R5) | `test_sealed_sync_records.py` |
| A4 | a planted head table that carries a body field is refused (R5) | `test_sealed_sync_records.py` |
| A5 | the daily table names exactly the four fields, in order (R7) | `test_sealed_sync_records.py` |
| A6 | a planted daily table with a fifth field, or its fields reordered, is refused (R7) | `test_sealed_sync_records.py` |
| A8 | the census names every `ingest` context and every register table with its owner and a verdict, and counts the parked rows (R9) | `test_sealed_sync_records.py` |
| A9 | a planted census that drops a table, adds one, gives a wrong owner or an unknown verdict is refused (R9) | `test_sealed_sync_records.py` |
| A10 | the staged privacy text names the six sealed classes and the four fields, and a planted text missing one is refused (R11) | `test_sealed_sync_records.py` |
| A11 | `PRIVACY.md` makes no sealed-sync claim while the stock-app path is in the tree; a planted claim is refused and the same claim with the path gone is accepted (R12) | `test_sealed_sync_records.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_key_table_names_four_keys_and_the_server_holds_none_in_the_clear
A2: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_a_key_held_in_the_clear_by_the_server_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_clear_head_holds_exactly_the_judges_fields
A4: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_a_body_field_in_the_head_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_daily_record_holds_exactly_four_fields
A6: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_a_fifth_or_reordered_daily_field_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_census_covers_every_ingest_context_and_every_table
A9: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_a_census_that_drops_adds_or_misjudges_a_table_is_refused
A10: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_the_staged_privacy_text_names_what_is_sealed_and_what_is_kept
A11: python3 -m unittest discover -s scripts/tests -p test_sealed_sync_records.py -k test_privacy_md_claims_no_sealed_sync_while_the_stock_path_is_in_the_tree
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-583-every-sync-op-body-is-sealed-on-the-device-under-one-account-key-and-the-server-keeps-clear-heads-and-one-daily-record.md` | records | new: this SPEC |
| `docs/decisions/ADR-602-every-sync-op-body-is-sealed-on-the-device-under-one-account-key-that-each-device-each-passkey-and-a-recovery-key-unwrap.md` | records | new: D1 to D8 |
| `docs/schematics/sealed-sync-keys.md` | records | new: the keys, an op's path and the daily record's path |
| `docs/sealed-sync/device-census.md` | census | new: R9, R10 |
| `docs/sealed-sync/privacy-at-the-cut-over.md` | privacy | new: R11 |
| `scripts/tests/test_sealed_sync_records.py` | tests | new: A1 to A11 |
| `docs/red-first/SPEC-583.md` | tests | new: the red-first record |
| `scripts/mutation-rows.d/S58300-S58399.json` | rows | new: S58301 to S58314 |
| `changelog.d/sealed-sync-keys-583.md` | records | new: the fragment |

The tests read two files this delivery leaves as they are: the context map, for its `ingest`
dependants and its register of DeckStreak's own tables (A8, A9), and the privacy page (A11).

## 5. What this does NOT cover

- The recovery key's printable flow before the first sync, on both apps (#1087).
- The sealing core: the envelope, the padding, the slots, the HPKE wraps and their test vectors, with the crate choice it needs (#1080).
- The key hierarchy's client code on the iPhone and iPad app (#1081).
- The key hierarchy's client code on the web (#1082).
- The relay that stores sealed bodies, slots, list versions, media objects and device checkpoints (#1083).
- The daily record's route, its checks and the features it feeds (#1084).
- Each context's move to the device, item by item in the census (#1095).
- The reading context's move, held while its surface is parked (#1098).
- Export and erase over sealed data (#1085).
- The cut-over itself: the stock-app path's end, the staged privacy text installed and the records it retires (#1086).
- The at-rest keys and the backups' design (#1088).
- AI over sealed data, on the device or by a grant (#1089).
- Any change to the engine, the scheduler, a web screen, the sign-in or the sync server (#1079).

## 6. Risks

- A later change to the context map adds a table or an `ingest` dependant with no census row: A8
  refuses it by name, and the census gains the row with its verdict.
- A client serializer that starts posting `credential.toJSON()` would carry a PRF output: the web
  key item pins the posted JSON to `extensions: {}` (#1082).
- The sync core lands with head fields other than R5's: the sealing core maps them and amends this
  SPEC's table, and A3 holds the table.
- A learner who loses every device, PRF passkey and printed key loses the data: the recovery flow
  makes the printed key a step before the first sync (#1087).
