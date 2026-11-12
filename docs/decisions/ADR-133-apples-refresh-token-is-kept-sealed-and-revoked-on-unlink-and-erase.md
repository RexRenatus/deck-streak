---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Apple's refresh token is kept sealed, and revoked at Apple on unlink and erase through a bounded retry

## Context and Problem Statement

Sign in with Apple expects an app that lets a user delete their account, or unlink Apple, to
revoke the grant at Apple, and revocation needs a token from the original exchange. DeckStreak's
erase must also withdraw what it created elsewhere (SPEC-021's symmetry). Keeping a token is keeping
a credential at rest. Does DeckStreak keep Apple's refresh token, and if so, how?

## Decision Drivers

- SECURITY class: no live credential readable from a copy of the database.
- Export and erase stay symmetric, and an erase withdraws what it made at a provider.
- A revocation that fails must never block an unlink, a sign-in or an erase.
- No unbounded work: every retry has an end.

## Considered Options (the alternatives it was chosen against)

- Keep the refresh token sealed with XChaCha20-Poly1305 under `link-token-key`: chosen, because
  revocation stays possible and a copy of the database holds only ciphertext bound to its row.
- Defer Apple: rejected because the auth pack's revoke row would stay red with no plan.
- Keep no token: rejected because revocation at Apple would be impossible, so an unlink or an erase
  would leave the grant standing there.
- Keep the token in plain text: rejected because a copy of the database would carry a live
  credential.
- Seal with AES-256-GCM: rejected because a random 96-bit nonce leaves less margin than
  XChaCha20's 192-bit one, and nothing here needs AES hardware.

## Decision Outcome

Chosen option: "keep it sealed", because it is the only option that keeps revocation possible
without a live credential at rest.

- **The seal.** A random 24-byte nonce per seal, and `(issuer, token id)` as associated data, the
  token id kept beside the sealed token, so the retry
  opens it with no subject and a sealed token moved to another row does not open. The token is
  stored in
  `linked_identities.sealed_refresh_token`; no other provider token is kept.
- **The revocation.** Unlinking Apple and either erase path open the token and call Apple's
  revocation first, bounded at 10 seconds, then delete the local row either way.
- **The retry.** A revocation that fails, times out or has no credential moves the sealed token to
  `identity_revocations` (with its issuer and token id; no subject, no user id), tried hourly by the
  job `link_revocation` with
  waits of 1, 2, 4, 8 and 16 hours; the fifth failure deletes the row and pages once.
- **Rights.** `identity_revocations` is exempt from export and erase: each row is the erase's own
  withdrawal, holding nothing that names the owner.

### Consequences

- Good, because an unlink or an erase withdraws the grant at Apple, or says that it could not.
- Bad, because `link-token-key` is one more credential; missing, Apple's revocation is queued and
  the job pages when it gives up.

### Confirmation

SPEC-131 §3 (the seal, the revocation and the retry criteria) and its rows in `S13100-S13199`.

## What would make this wrong

- Apple accepts revocation by the client's own credentials alone: the token would no longer be
  needed, and the column would go.
- A second provider that also needs a kept token: the seal would serve it under its own associated
  data.

## More Information

SPEC-131, SPEC-021, SPEC-066, ADR-067, ADR-131, and the W7 schematic
`docs/schematics/w7-sign-in-and-linking.md`.
