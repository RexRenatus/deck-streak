---
status: "accepted"
---

# ADR-583: the sqlx crates come from a fork until a sqlx release moves them to sha2 0.11

GHSA-qwgh-2vcv-g2f7 names block-buffer `< 0.12.1`, first patched 0.12.1. In this workspace
block-buffer 0.10.4 comes only through digest 0.10.7, and after the engine fork's own moves the one
digest 0.10 user left is sha2 0.10.9, which sqlx-core 0.9.0 (feature `migrate`) and sqlx-macros-core
0.9.0 require (`sha2 ^0.10.0`). This decision takes all seven sqlx crates from the maintainer's
fork, at one commit that moves those two crates to sha2 0.11, by `[patch.crates-io]` entries, until
a sqlx release carries that move. It also records why a patched copy of block-buffer 0.10.4 is
refused, and why the engine fork's workspace drops its link checker (SPEC-565).

## Context and Problem Statement

- The workspace's lock at 4e600dbb holds block-buffer 0.10.4 under digest 0.10.7, used by hmac
  0.12.1, pbkdf2 0.12.2, sha1 0.10.7 and sha2 0.10.9. The engine brings the first three and
  axum-extra's `headers` (sha1 0.10); sqlx-core and sqlx-macros-core bring sha2 0.10.
- No sqlx release moves: the crates.io index lists 0.9.0 as the newest sqlx-core and
  sqlx-macros-core, each on `sha2 ^0.10.0`, while sqlx-postgres and sqlx-mysql 0.9.0 already take
  sha2 0.11.0.
- The call sites are the same algorithms under sha2 0.11: the migration checksum
  (`Sha384::digest`), the test database name (`Sha512`) and the query cache key (`Sha256` into
  `hex::encode`). So the checksums sqlx stores in `_sqlx_migrations` do not change, which a hold
  guard checks (SPEC-565 A6).
- sqlx-macros-core takes sqlx-sqlite as a path dependency of its own workspace, so a git source
  for sqlx-macros-core alone would add a second sqlx-sqlite 0.9.0 beside the registry's.
- The engine fork's lock, which the release builds the sync server from, also held block-buffer
  0.12.0 and 0.10.4. Three of its 0.10 paths came only from the fork's docs member `linkchecker`:
  mdbook 0.4.52 (sha2 0.10.9), axum's `ws` feature that mdbook asks for (sha1 0.10.6), and pest_meta
  2.8.1 under handlebars (sha2 0.10.9).

## Decision Drivers

- The advisory audit is a gate: an advisory is fixed, never ignored, and the fix must be red first.
- Cargo reads patches from the root manifest only, and `deny.toml` admits no git source it does not
  name.
- The lock must keep one copy of each sqlx crate.
- The entries must leave when a sqlx release carries the move.

## Decision

- D1. The root manifest's `[patch.crates-io]` takes sqlx, sqlx-core, sqlx-macros, sqlx-macros-core,
  sqlx-mysql, sqlx-postgres and sqlx-sqlite from `https://github.com/RexRenatus/sqlx.git`, all by
  `rev`, the full id of one commit. That commit sits on the fork's branch `deckstreak-sha2-0.11`
  over 003b698e, the commit the published 0.9.0 crates name in their `.cargo_vcs_info.json`, and
  changes two manifest lines: sha2 `0.10.0` becomes `0.11` in sqlx-core and in sqlx-macros-core. The
  version stays 0.9.0, so the workspace requirement still matches it.
- D2. All seven come from the same rev, so sqlx-macros-core's path dependency on sqlx-sqlite
  resolves to the same package the workspace takes, and the lock keeps one of each.
- D3. `deny.toml`'s `allow-git` gains the fork, and the exact list of allowed git sources in
  `scripts/tests/test_engine_pin.py` gains it too, so the list stays exact.
- D4. One rule guards both locks, `scripts/lock-digest-line.py`: no block-buffer below 0.12.1 and
  no digest below 0.11.0. It reads this workspace's lock in a test, and the engine fork's lock at
  the pinned commit both in a test and in the release audit, before cargo deny.
- D5. The engine fork's workspace drops `rslib/linkchecker` (L-a below), in the fork commit the
  engine pin moves to (ADR-058's note).

## What it was chosen AGAINST

- A patched copy of block-buffer 0.10.4 from a fork. There is no defect in 0.10.4 to patch: its
  position is a separate field written only after every closure returns, so no test can be red for
  the advisory there. And cargo-deny 0.20.2 matches an advisory only to a crate from the advisory's
  source, crates.io when none is given (`matches_rustsec`, src/lib.rs:210-219, and
  src/advisories/helpers/db.rs:218-223), so a git-sourced copy would clear the audit by its source
  alone, whatever it contains: an exception under another name.
- An advisory exception or ignore entry: it would admit the advisory, where every exception is an
  unmaintained notice.
- Patching sqlx-core and sqlx-macros-core alone: sqlx-sqlite would come twice (D2).
- Waiting for a sqlx release: the advisory and the audit stay open.
- For the fork's link checker, L-b, moving mdbook, pest and axum to digest-0.11 releases: within
  linkcheck's `mdbook ^0.4.21`, mdbook 0.4.46 to 0.4.52 require sha2 ^0.10.8, and every axum release
  through 0.8.9 requires sha1 ^0.10 under `ws`, so only the pest path would close.
- For the fork's link checker, L-c, a patched copy of linkcheck that inlines mdbook's one function:
  a third forked repository, a licence boundary (mdbook is MPL-2.0), and an upstream pull request
  owed, against three deleted files and four manifest lines for L-a. The link checker's one test
  runs only online and checks the Anki manual's links, which this workspace never runs, and nothing
  in the fork's build names it.

## Decision Outcome

- `Cargo.lock`: no block-buffer below 0.12.1 and no digest below 0.11.0; the seven sqlx crates at
  0.9.0 from the fork's commit, one each.
- The engine fork's lock at the pinned commit: block-buffer 0.12.1 and digest 0.11.2 only.
- `deny.toml` admits five git sources: the engine fork, rust-url, fsrs-rs, the web-push-native fork
  and the sqlx fork.

### Consequences

- The workspace takes the sqlx crates from a git source until a release carries the move, so a
  clean build fetches the fork's commit.
- `deny.toml` and the exact list in `scripts/tests/test_engine_pin.py` name five git sources; a
  sixth fails that test until both name it.
- A later engine pin whose lock brings back block-buffer below 0.12.1 or digest 0.10 fails the
  guard test and stops the release audit before cargo deny.

### Confirmation

`scripts/tests/test_lock_digest_line.py` and `scripts/tests/test_fork_lock_guard.py` read both locks
GREEN; `scripts/tests/test_audit_sync_server.py` holds the audit's guard; the migration checksum
guard holds; and `scripts/tests/test_engine_pin.py` checks the `allow-git` list exactly.

## What would make this wrong

- A sqlx release moves sqlx-core and sqlx-macros-core to sha2 0.11 and the entries stay.
- The fork's branch or the fork goes, so the commit can no longer be fetched.
- Upstream changes the hash a migration checksum uses, so the stored checksums would differ.

## More Information

- Removal: when a sqlx release requires sha2 0.11 in sqlx-core and sqlx-macros-core, the seven
  `[patch.crates-io]` entries, the `allow-git` entry and its place in
  `scripts/tests/test_engine_pin.py` go, the requirement moves to that release, and a note marks
  this ADR superseded. The guard stays.
- The sqlx change is offered upstream to launchbadge/sqlx once this delivery is green.
- Read with ADR-058, whose note moves the engine pin, and ADR-513, whose git-source rule this
  extends.
