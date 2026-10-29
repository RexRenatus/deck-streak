---
status: "accepted"
---

# ADR-513: the push sender takes web-push-native from a fork until upstream releases its p256 0.14 move

RUSTSEC-2026-0387 names crypto-bigint 0.5.5, fixed in 0.7.6. In this workspace that crate comes
only through elliptic-curve 0.13.8 under p256 0.13.2, and p256 stays at 0.13 because
web-push-native 0.5.0, its newest release, requires `p256 ^0.13.2` while the web push sender hands
a `p256::PublicKey` to `web_push_native::encrypt`. This decision takes web-push-native from the
maintainer's fork, at the upstream change that moves it to p256 0.14, by a `[patch.crates-io]`
entry, until a release carries that change.

## Context and Problem Statement

- The Rust advisory audit fails on RUSTSEC-2026-0387. In `Cargo.lock`, crypto-bigint 0.5.5 has one
  dependent, elliptic-curve 0.13.8, which comes with p256 0.13.2.
- p256 0.14.0 depends on elliptic-curve 0.14, whose newest release requires crypto-bigint 0.7.8 or
  later.
- web-push-native 0.5.0 requires `p256 ^0.13.2`, and `crates/push` passes it a `p256::PublicKey`,
  so the workspace's one p256 requirement cannot move while web-push-native stays.
- Pull request 13 on leotaku/web-push-native moves web-push-native to p256 0.14.0 and aes-gcm
  0.11.1, and ece-native to aes-gcm 0.11.1, hkdf 0.13.0 and sha2 0.11.0, in one commit, `71900ba`,
  over master, which is release 0.5.0 plus one commit that marks the example crate unpublished. It
  keeps the version at 0.5.0. It is open, and no release carries it.

## Decision Drivers

- The advisory audit is a gate: an advisory is fixed, never ignored. The exceptions in `deny.toml`
  are unmaintained notices only.
- The upstream change its maintainer reviews is preferred over a bump written here.
- Cargo reads patches from the root manifest only, and `deny.toml` admits no git source it does not
  name.
- The entry must leave when upstream releases.

## Decision

- D1. The root manifest's `[patch.crates-io]` takes `web-push-native` from
  `https://github.com/RexRenatus/web-push-native.git` by `rev`, the full id of `71900ba`. The fork's
  branch `deckstreak-p256-0.14` keeps that commit fetchable. The crate's version there is 0.5.0, so
  the workspace requirement `0.5.0` still matches it.
- D2. ece-native comes from the same commit through web-push-native's own path dependency, so it
  takes no patch entry: an entry nothing uses is recorded as unused, which
  `scripts/tests/test_engine_pin.py` refuses.
- D3. The workspace's p256 moves to 0.14.0. The push signer and its tests follow its API:
  `to_sec1_point` replaces `to_encoded_point`, and keys and secrets come from the `Generate` trait.
- D4. `deny.toml`'s `allow-git` gains the fork, and the exact list of allowed git sources in
  `scripts/tests/test_engine_pin.py` gains it too, so the list stays exact.

## What it was chosen AGAINST

- Waiting for the release: RUSTSEC-2026-0387 stays open, and the advisory audit stays red.
- An advisory exception: it would admit a vulnerability, where every exception is a notice.
- A p256 bump written here on the fork: upstream's change is the one its maintainer reviews, and
  the fork stays one upstream commit.
- Copying the crate into the workspace: more code to own than one patch line.

## Decision Outcome

- `Cargo.lock`: p256 0.14.0 is the only p256, every crypto-bigint is 0.7.6 or later,
  web-push-native and ece-native come from the fork's commit, and no unused patch is recorded.
- The advisory audit no longer reports RUSTSEC-2026-0387.
- `deny.toml` admits four git sources: the engine fork, rust-url, fsrs-rs and the web-push-native
  fork.

### Consequences

- The workspace takes web-push-native and ece-native from a git source until a release carries
  the change, so a clean build fetches the fork's commit.
- `deny.toml` and the exact list in `scripts/tests/test_engine_pin.py` name four git sources; a
  fifth fails that test until both name it.
- When upstream releases, the entry, its `allow-git` line and its place in the test's list go
  together (More Information).

### Confirmation

The Rust advisory audit in `ci` reports no RUSTSEC-2026-0387; `scripts/tests/test_engine_pin.py`
checks the `allow-git` list exactly and refuses an unused patch; and `Cargo.lock` holds one p256,
0.14.0, with web-push-native and ece-native from the fork's commit.

## What would make this wrong

- Upstream rejects or reworks pull request 13, so its release differs from the pinned commit.
- The fork's branch or the fork goes, so the commit can no longer be fetched.

## More Information

- Removal: when a web-push-native release requires p256 0.14, the `[patch.crates-io]` entry, the
  `allow-git` entry and its place in `scripts/tests/test_engine_pin.py` go, the requirement moves to
  that release, and a note marks this ADR superseded.
- Read with ADR-058 and ADR-338, whose git-source rules this extends, and ADR-354, under which the
  push senders take web-push-native.
