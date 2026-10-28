### Security

- The engine fork's own zstd moves from 0.13.3 to 0.14.3, so zstd 0.13.3 and zstd-safe 7.3.0
  leave the lock and thirteen RustSec advisories clear: RUSTSEC-2026-0391 and RUSTSEC-2026-0395
  to RUSTSEC-2026-0406. The engine patch entry moves to the fork's tag
  `deckstreak-pin-26.09.3-wasm32-rustsec`, two commits over the earlier pin.
- The engine fork's own lock, from which the release builds and audits the sync server, moves
  rustls, rustls-webpki, webpki-roots, quinn-proto, tokio, rand and ammonia to patched versions,
  past sixteen more RustSec advisories. This repository's lock already held patched versions of
  all seven.
- web-push-native comes from the maintainer's fork at the upstream change that moves it to
  p256 0.14, until a release carries it, and the workspace's p256 moves from 0.13.2 to 0.14.0.
  crypto-bigint 0.5.5 leaves the lock, which clears RUSTSEC-2026-0387. The push signer and its
  tests follow p256 0.14's renamed point encoding and key generation.
