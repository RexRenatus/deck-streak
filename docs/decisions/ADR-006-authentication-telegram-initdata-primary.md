---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Authentication: Telegram initData first, pinned to the owner, with linked sign-in methods

## Context and Problem Statement

DeckStreak serves one owner. The predecessor's safety boundary was the bot answering only the
owner's chat id. The Mini App is a web page anyone can load, so the server must prove who the
caller is on every request. The owner chose "Telegram primary, link others": Telegram identity
first, with Google, Apple and passkeys as linked secondary methods.

## Decision Drivers

- Owner-only gating is a safety property (the predecessor's rule 9), not a formality.
- The web-security pack's Telegram rows: validated server-side, constant-time compare, bounded `auth_date`.
- The owner's user id is configuration held in a secret, never in the repository.

## Considered Options (the alternatives it was chosen against)

- Validate `initData` in an axum extractor: HMAC-SHA256 with the key HMAC-SHA256("WebAppData", bot token), constant-time compare, a maximum `auth_date` age, then pin the user id to the owner; link other methods to that account — chosen: Telegram's documented scheme, no third-party trust, and the owner's decision.
- Trust `initDataUnsafe` in the client — rejected because anything the client reports can be forged.
- Validate with Telegram's third-party Ed25519 signature — rejected as the primary path because the bot token is already held and the HMAC path is simpler; it remains available if a third party must verify.
- A username and password — rejected because it adds a credential to store and phish, with no benefit to a single Telegram-native owner.

## Decision Outcome

Chosen option: `identity` owns an in-house extractor on RustCrypto `hmac`, `sha2` and `subtle`
(the house golden path `telegram-mini-app`), which rejects a missing, malformed, stale or forged
`initData` with 401 and a user who is not the owner with 403, and never logs `initData`. A
successful check opens a short server session cookie (`__Host-` prefix, Secure, HttpOnly,
SameSite=Strict) so later calls need not resend `initData`. Linking Google, Apple or a passkey
(W7) attaches a secondary credential to the same owner account and never creates a second
account; the auth pack judges that delivery. The bot's owner gate compares the update's user id
with the same configured owner id.

### Consequences

- Good, because one owner id, from one credential, gates both surfaces.
- Bad, because the maximum `auth_date` age trades convenience for replay safety; it is configuration with a conservative default.

### Confirmation

The web-security rows `ws.tg-init-data-verified`, `ws.tg-init-data-constant-time`, `ws.tg-init-data-fresh` on the box (reviewed by hand until train 84's fixes land); the extractor's own tests with a forged, a stale and a valid payload.

## What would make this wrong

- Telegram changes the `initData` scheme or deprecates the HMAC path (tracked by the telegram-platform pack's Bot API version notes).

## More Information

The auth pack (Telegram primary, linked methods); the web-security pack; docs/schematics/initdata-auth-sequence.md.
