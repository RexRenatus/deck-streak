---
status: "accepted"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Passkeys through `webauthn-rs` 0.5, with the Mini App's host as the relying party and every ceremony's state kept on the server

## Context and Problem Statement

#58 asks for passkeys beside Google and Apple, each linked to the owner's Telegram identity
(ADR-006). A passkey needs a WebAuthn relying party: a registration that stores a public key and a
counter, and an assertion that checks the challenge, the origin, the relying party's id, the user
verification flag and the counter. Which library verifies the ceremonies, and which host is the
relying party?

## Decision Drivers

- SECURITY class: a replayed, expired, cross-origin or unverified ceremony is refused, and each
  refusal has a criterion and a hand-proved row (the W7 plan's binding decision 3).
- A test runs against a software authenticator, never a device.
- The relying party must be a host DeckStreak already serves over HTTPS, so the browser binds the
  credential to it.

## Considered Options (the alternatives it was chosen against)

- `webauthn-rs` 0.5 and its `Passkey` type: chosen, because it enforces user verification for a
  passkey, keeps each ceremony's state as a server-side value, and reports the counter to check.
- `webauthn_rp`: rejected because it is younger with a far smaller public record, and a
  SECURITY-class path prefers the library with the longer one.
- The client-and-authenticator passkey crates: rejected because they implement the client's and the
  authenticator's sides, and DeckStreak needs the relying party's.
- Hand-written verification over COSE keys and `p256`: rejected because the CBOR parsing, the flags,
  the relying party's id hash and the counter are the checks a hand-written verifier gets wrong.
- No passkeys, providers only: rejected because #58 asks for passkeys, and a passkey needs no
  third party at all.
- A separate host as the relying party: rejected because it would need its own certificate and
  origin, while the Mini App's host already serves the API over HTTPS (ADR-007).

## Decision Outcome

Chosen option: "`webauthn-rs` 0.5, relying party the host of `DECKSTREAK_PUBLIC_ORIGIN`", because
it is the maintained relying-party implementation whose passkey type makes user verification a
requirement rather than an option.

- **Registration** runs inside a `link` session, asks user verification `required` and attestation
  `none`, and uses a user handle that is a random version-4 UUID (16 bytes, the `Uuid` webauthn-rs
  0.5's
  `start_passkey_registration` takes as `user_unique_id`) minted once for the owner, with a display
  name
  that carries no personal data.
- **State.** Each ceremony's state is kept on the server under the flow id, never sent to the
  browser, and lives 300 seconds.
- **The counter.** A counter that does not advance, when either value is non-zero, is refused
  `counter_regressed`; a verified assertion stores the new counter and backup state.

### Consequences

- Good, because the ceremonies' refusals are reachable from a software authenticator in the tests.
- Bad, because `webauthn-rs` links OpenSSL, so OpenSSL joins the build and the dependency audit.
- Bad, because a passkey is bound to the configured host: a new domain (#168) means registering
  passkeys again.

### Confirmation

SPEC-131 §3 (the passkey criteria) and its rows in `S13100-S13199`.

## What would make this wrong

- A relying party on a domain other than the Mini App's (#168 choosing separate hosts): related
  origins would need their own decision.
- An authenticator population that reports no user verification: the refusal would lock such
  devices out, which is the intended outcome for a SECURITY-class sign-in.

## More Information

SPEC-131, ADR-006, ADR-007, ADR-131, ADR-133, and the W7 schematic
`docs/schematics/w7-sign-in-and-linking.md`.
