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
- The web lock moves four development-only packages to fixed releases inside their parents'
  declared ranges, past sixteen advisories: dompurify 3.4.16 to 3.4.17 (GHSA-cg2g-xh64-v4q7,
  GHSA-mrv2-3pqr-j5jr, GHSA-c6gr-qvv4-5v3f), undici 8.11.2 to 8.12.3 (GHSA-4qv8-2p9j-2685,
  GHSA-f72v-6w45-q472, GHSA-xjj6-f33q-929c, GHSA-4w7w-5xrc-9h26, GHSA-2m5v-pch6-74hx,
  GHSA-qrxm-fqhp-jwvm, GHSA-9fm3-97px-rjf6, GHSA-55wc-3pg8-jfw6), fast-uri 3.1.8 to 3.1.9
  (GHSA-w74x-v6f9-j3xg, GHSA-xhq7-xmqj-769h, GHSA-wjr9-j795-jxcr, GHSA-333x-w4r6-6crp) and zod
  4.6.5 to 4.6.7 (GHSA-mrhx-c7xc-m3w5). The `qs` override's floor rises to 6.16.1, the first
  release that fixes GHSA-6pq8-jhm2-2vcw, and the lock takes qs 6.17.0. Three low advisories stay
  open: GHSA-pfhg-q3qv-9q6r and GHSA-27mw-82g8-phjj in SvelteKit 2, fixed only in SvelteKit 3,
  and GHSA-gcqx-w48m-w2h4 in deepmerge 4.3.1, which has no fixed release.
