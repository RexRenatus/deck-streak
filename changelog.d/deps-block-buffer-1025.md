### Security

- block-buffer below 0.12.1 leaves both Rust locks, which clears GHSA-qwgh-2vcv-g2f7. The seven
  sqlx crates come from the maintainer's fork at the published sqlx 0.9.0 with sha2 moved to 0.11,
  until a sqlx release carries it, so digest 0.10 and block-buffer 0.10.4 leave this repository's
  lock. The engine patch entry moves three commits along the engine fork, to the tag
  `deckstreak-pin-26.09.3-wasm32-blockbuf`: in the fork's own lock, from which the release builds
  and audits the sync server, block-buffer moves from 0.12.0 to 0.12.1, sha1 to 0.11 and pbkdf2 to
  0.13, axum-extra and the linkchecker member leave, and pulldown-cmark moves from 0.13.0 to
  0.13.5, which clears RUSTSEC-2026-0414.
- A lock check refuses any block-buffer below 0.12.1 and any digest below 0.11.0. The sync
  server's release audit runs it over the engine fork's lock before its advisory check.
