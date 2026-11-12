# Sealed sync keys: the account key and its slots, an op's clear head and sealed body, and the daily record's path

A data-flow schematic of SPEC-583 (ADR-602). Every `path:line` citation was read at the dev this change is cut from;
the new paths are SPEC-583's. The design goes live at the sync cut-over and never before it.

## 1. The key hierarchy: from the account key to each slot

```mermaid
flowchart TB
  GEN["the device's random generator, at setup, before the first sync"] --> AKE["account key of epoch e, 256 random bits"]
  AKE -->|seals| AKP["account key of epoch e minus 1, in a chain record"]
  AKE -->|derives by HKDF| BODYK["the epoch's body key"]
  AKE -->|seals| LIST["device-list version v, its number in the associated data"]
  DEVK["each device key: X25519 and ML-KEM-768 joined by X-Wing, private half this device only"] -->|public half listed in| LIST
  PRF["a PRF passkey's output, kept in memory, never posted"] -->|derives by HKDF with the slot salt| PKEK["the passkey slot's sealing key"]
  PKEK -->|seals| PSLOT["the passkey slot's private half"]
  PRINT["the recovery key: 256 random bits, printed once"] -->|derives| RSLOT["the recovery slot's key pair"]
  AKE -->|HPKE to each slot's public half| WRAPS["one sealed account-key record per slot"]
  WRAPS --> STORE["the server's store: public halves, sealed records, salts, sealed list versions, chain records"]
  PSLOT --> STORE
  LIST --> STORE
  EMAIL["an email sign-in"] -->|opens the session only| SESSION["the account's session: no key record is reachable from it"]
```

A new device joins by one of three doors, and a removal starts the next epoch:

```mermaid
flowchart LR
  NEW["a new device makes its key pair"] --> CODE["an existing device checks a short code of its public key"]
  NEW --> PASS["a PRF passkey opens its slot"]
  NEW --> RECOVER["the recovery key opens its slot"]
  CODE --> JOINED["the account key of the newest epoch is wrapped to the new device"]
  PASS --> JOINED
  RECOVER --> JOINED
  JOINED --> ADDED["the device adds its own slot in a new list version"]
  REMOVE["a remaining device removes a slot"] --> NEXT["it makes the next epoch's account key and wraps it to every remaining slot"]
```

## 2. One op, from one device through the server to another

```mermaid
flowchart LR
  OP["an op made on device A: its encoded body"] --> PAD["pad: one 0x80 byte and zeros to the next multiple of 256"]
  PAD --> SEAL["AES-256-GCM under a per-op key from the body key, origin and sequence, random nonce"]
  HEAD["the clear head: version, key id, origin, sequence, clock, class, register, base revision, nonce"] -->|associated data, with the account id| SEAL
  SEAL --> PUSH["push: the head and the ciphertext"]
  PUSH --> JUDGE["the server's judge: dedup by origin and sequence, order by clock, compare register and base revision"]
  JUDGE --> LOG["the stored op: head in the clear, body as ciphertext"]
  LOG --> PULL["pull by device B"]
  PULL --> OPEN["device B opens the body with the epoch's key and checks the head as associated data"]
```

## 3. The daily record's path to the features it feeds

```mermaid
flowchart LR
  DAY["device: one study day's review count, minutes and XP granted"] --> REC["the daily record: date, reviews, minutes, XP granted"]
  REC --> CHECK["server checks: four fields; the date within a day; reviews within the day's append heads; minutes at most 1440; XP within the rule's ceiling"]
  CHECK -->|accepted| FEED["streaks, freezes, relights, the governor, coins, quests, push budgets"]
  CHECK -->|refused| TOLD["kept out, and the device is told why"]
  HEADS["the stored heads and ciphertext bytes"] --> QUOTA["quotas"]
```

## 4. Each step, its proof, its red and its row

| step | proved by | red | rows |
|---|---|---|---|
| the four keys, and the server holds none in a usable form (section 1) | A1, A2 | A1 red at the red commit; A2 planted | S58301, S58302, S58303 |
| the clear head holds exactly the judge's fields (section 2) | A3, A4 | A3 red at the red commit; A4 planted | S58304, S58305 |
| the daily record holds exactly four fields, in order (section 3) | A5, A6 | A5 red at the red commit; A6 planted | S58306, S58307 |
| every context and table that sealing touches has a verdict | A8, A9 | A8 red at the red commit; A9 planted | S58308, S58309, S58310 |
| the privacy text the cut-over installs names what is sealed and what is kept | A10 | A10 red at the red commit, its refusals planted | S58311, S58312 |
| the privacy page claims no sealed sync before the cut-over | A11 | not red: the page claims none at dev; its rows are the reds | S58313, S58314 |

Every proof is in `scripts/tests/test_sealed_sync_records.py`. The sealing core, the clients and
the relay that build these steps carry their own tests and models (SPEC-583 section 5).

## 5. What stays as it is

Until the cut-over, the stock-app sync path is unchanged: the sync server's store and its
snapshots (`deploy/scripts/sync-server.sh:78-80`; `deploy/scripts/backup.py:50-52`), the private
copy `ingest` reads (`crates/ingest/src/lib.rs:3-6`), today's sealed web sync key (ADR-374) and
the app's Keychain item (`ios/App/Sources/SyncCredentialStore.swift:38`). The context map, the
privacy page and every table keep their text and schema; the census records what each becomes.
