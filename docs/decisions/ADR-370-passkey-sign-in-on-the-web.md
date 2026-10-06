---
status: "proposed"
date: "2026-10-06"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Passkey sign-in on the web: a strict ceremony cookie, a counter advanced by compare-and-swap, sign-in over the owner's own credentials, a bound of its own, and removal that ends its sessions

## Context and Problem Statement

#627 brings passkey sign-in to the web client, linked to the one owner account whose primary
identity stays Telegram `initData` (ADR-006). ADR-132 settles the library (`webauthn-rs` and its
`Passkey`), the relying party (the web client's host), user verification `required`, attestation
`none`, a random version-4 UUID as the user handle, server-kept ceremony state living 300 seconds,
and the `counter_regressed` refusal. SPEC-131 (planned) settles the link code, the `link` and
`linked` proofs, the refusal names, removal's fresh-Telegram rule and `last_method`, and the
tables' rights. ADR-024 settles the session store and its bounds.

They leave open how a ceremony's id reaches the browser and back, how the counter is checked
without a race, which sign-in ceremony runs, how the ceremony routes are bounded, what removal
does to a live session, where the owner sees and removes a passkey, how this slice is numbered
against SPEC-131, and what a verification failure SPEC-131 does not name answers. `webauthn-rs`
leaves two of these to its caller by its own documentation: the authentication state must be kept
on the server, and when the presented counter is above zero the application must assert that it
exceeds the stored one.

## Decision Drivers

- SECURITY class: every doubtful assertion is refused by name, and each refusal has a criterion and
  a mutation row.
- No unbounded work, and no secret on anything public (CHARTER 10).
- One owner: no route creates an account, and every way in resolves to the owner's Telegram
  identity.
- A test runs against a software authenticator and a fixed clock, never a device.

## D1. The sign-in ceremony runs over the owner's own credentials

## D1. Options considered

- Chosen: a ceremony whose allowed credentials are the owner's stored passkeys, because the
  service has one account, the library's plain passkey flow needs no extra feature, and a sign-in
  with no passkey held is refused `not_linked` before any ceremony starts.
- Discoverable credentials with conditional UI: rejected because it needs the library's
  conditional-UI feature and resident keys at registration, and its account-picker affordance
  serves many accounts where DeckStreak has one.
- A username step before the ceremony: rejected because there is no username to type; the owner is
  configuration.

## D1. Consequences

- The start route names the owner's credential ids to a caller who holds no session. They are
  random, scoped to this relying party and carry no personal data; D5 bounds the route.

## D2. A ceremony's id travels in a strict cookie of its own

SPEC-131 R10 keeps the ceremony's state "on the server under the flow id" and does not say how a
passkey ceremony's id reaches the browser. Its flow cookie (R6) is built for an OpenID Connect
redirect.

## D2. Options considered

- Chosen: `__Host-deckstreak_ceremony` (`Path=/`, `Secure`, `HttpOnly`, `SameSite=Strict`,
  `Max-Age=300`), holding a 32-byte id from the operating system's generator that the server keeps
  as its SHA-256, cleared by the finish, because both ceremony calls are same-origin `fetch` calls
  that a strict cookie serves, and the id never enters script or a body.
- Reusing `__Host-deckstreak_flow`: rejected because it is `SameSite=Lax` and lives 600 seconds,
  which a top-level cross-site callback needs and a same-origin ceremony does not; a shared name
  would also let a passkey ceremony and a provider flow overwrite each other's id.
- The id in the response body, posted back by the page: rejected because script could read and
  replay it, and every other secret of this kind stays in an `HttpOnly` cookie (SPEC-024 R4).
- The state inside the `link` session's record: rejected because a sign-in ceremony has no session
  to hang it on, and two carriers for one thing is the drift this avoids. A registration ceremony
  still records its `link` session and finishes in no other.

## D3. The counter is decided in code and advanced by compare-and-swap

## D3. Options considered

- Chosen: a pure function decides the rule (accepted when both counters are zero or the presented
  one is greater), and the store writes with `UPDATE ... WHERE id = ? AND counter = ?` on the
  value it read, refusing `counter_regressed` when no row matched, because the rule is then a
  plain function a test table and a mutation row can hold, and two concurrent assertions carrying
  one counter cannot both pass.
- The whole rule as the `UPDATE`'s predicate: rejected because a predicate inside a compile-checked
  query cannot be hand-mutated (its mutant has no offline cache entry and reads VOID), so the rule
  would stand unproved.
- Trusting the library's own counter handling: rejected because its documentation leaves the
  assertion to the caller.
- Read, decide, then write with no guard: rejected because two assertions racing on one counter
  could both pass.

### Storage, decided with it

- Chosen: one `passkeys` row per credential, holding the serialized `Passkey`, the counter and the
  backup state as columns, and the user handle as a column every row of the owner shares, because
  the handle is then exported and erased with the rows that use it, and a fresh handle is minted
  only when none is held.
- A one-row table for the handle: rejected because it would outlive every passkey, owe its own
  rights declaration, and hold an identifier with nothing to identify.
- A handle derived from the Telegram user id: rejected because a derived handle links the
  credential to an identity, which ADR-132 and the auth pack refuse.
- Removal names a passkey by its integer row id: chosen over the credential id in the path, because
  a path reaches the edge's access log and the credential id never should (R13).

## D4. The ceremony routes have a bound and a cap of their own

## D4. Options considered

- Chosen: the two ceremony starts and the link redeem share a fixed window of 30 per minute per
  process, refused 429 `too_many_ceremonies` with `Retry-After`, checked after the cross-site
  bound and before the body is read; at most 8 ceremonies are live and a ninth evicts the oldest,
  because the handshake's bound has the same shape (ADR-024) and SPEC-131 caps codes and flows at
  8.
- Sharing the handshake's bound: rejected because a flood of passkey starts would then lock the
  owner out of the Telegram way in, which is also the recovery path.
- A per-address bound: rejected because the API sits behind the edge on loopback and sees one
  address.
- No cap on live ceremonies, bounded by their 300-second life alone: rejected because the bound
  admits 150 starts in that life, and CHARTER 10 refuses unbounded work.

## D5. Removing a passkey ends the sessions it opened

## D5. Options considered

- Chosen: a `linked` session records the row that opened it, and removal ends every such session,
  because removal is how the owner answers a lost or stolen authenticator.
- Leaving them to their idle and absolute bounds: rejected because a lost device could keep the
  owner's session for up to 8 hours after the owner removed its passkey.
- Ending every session: rejected because it would also end the Telegram session the owner is
  removing from.

## D6. The owner's methods get a screen of their own

## D6. Options considered

- Chosen: a sign-in methods screen hosting one component that lists the methods, removes a passkey
  and offers "Link a passkey", which the settings screen embeds when it lands (#57), because the
  removal route needs a surface and the settings screen is unlanded.
- Waiting for the settings screen: rejected because the slice would then ship a removal route with
  no surface.
- Removal through the API alone: rejected because the owner would have no way to answer a lost
  authenticator without a tool.

## D7. The slice is a SPEC of its own, under SPEC-131's test names

## D7. Options considered

- Chosen: SPEC-359 delivers SPEC-131's passkey slice under SPEC-131's own test names, and adds
  a status note to SPEC-131 naming what it took, because #627 is the campaign's row and its band,
  red-first record and changelog are its own.
- Moving SPEC-131 whole: rejected because its providers wait on the owner's action (#347) and its
  revocation job on its own prerequisites.
- Building it as SPEC-131's first part: rejected because a planned SPEC's band and red-first record
  would then be shared across deliveries decided apart.

## D8. A verification failure SPEC-131 does not name answers `passkey_invalid`

## D8. Options considered

- Chosen: `passkey_invalid` (401) for a signature or a response that does not verify, beside
  SPEC-131's six names, and 401 for every refusal of an assertion, 403 `not_owner`, 409
  `already_linked` and `last_method`, 404 `linking_off` and `identity_unknown`, because a forged
  signature then reads as what it is.
- Folding it into `challenge_invalid`: rejected because a forged signature would read as a stale
  ceremony, which hides the attack class from the audit events.

## Consequences

- Good: each doubtful assertion is refused by name, and the counter rule, the cookie's flags and
  the bounds each have a criterion and a mutation row.
- Good: the owner can remove a lost authenticator and end its sessions from the Mini App.
- Bad: the sign-in start discloses the owner's credential ids, without personal data (D1).
- Bad: ceremonies live in memory, so one in flight across a restart is refused and restarted.

## What would make this wrong

- A second account on the service: D1's allowed-credentials flow would then need a username step
  or discoverable credentials.
- A Permissions-Policy at the edge: it would have to grant `publickey-credentials-create` and
  `publickey-credentials-get` to `self`, or every ceremony stops.
- More than one API process behind the edge: the in-memory ceremonies, codes and bounds would need
  a shared store.
