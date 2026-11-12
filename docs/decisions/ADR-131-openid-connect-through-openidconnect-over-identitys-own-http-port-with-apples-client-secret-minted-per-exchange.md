---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# OpenID Connect through `openidconnect` 4 over identity's own HTTP port, with Apple's client secret minted per exchange

## Context and Problem Statement

#58 asks for Google and Apple sign-in, linked to the owner's Telegram identity (ADR-006). Both
providers speak OpenID Connect: an authorization request with state, nonce and PKCE, a code
exchange, and an ID token whose signature, issuer, audience, expiry and nonce must all be checked.
Apple also needs a client secret that is itself a signed JWT. Which library carries the protocol,
how does it reach the providers, and how is Apple's secret made?

## Decision Drivers

- SECURITY class: every doubtful assertion is refused, and each refusal has a criterion and a
  hand-proved row (the W7 plan's binding decision 3).
- No test reaches a network: a killer runs against a test double (SPEC-039).
- One HTTP stack in the workspace where one will do, with redirects off for a token endpoint.
- No secret at rest that a shorter-lived one can replace.

## Considered Options (the alternatives it was chosen against)

- `openidconnect` 4 with its default features off, over identity's `ProviderHttp` port: chosen,
  because it verifies the ID token's signature, issuer, audience, expiry and nonce in one typed
  call, and with its built-in client off every request goes through the port the tests replace.
- `openidconnect` with its built-in reqwest client: rejected because the provider's HTTP would then
  bypass the port, so no refusal's killer could run against a double.
- The `oauth2` crate with a hand-written ID token check: rejected because signature, issuer,
  audience, expiry and nonce are exactly the checks a hand-written verifier gets wrong.
- A JWT library with hand-written discovery and key fetching: rejected because it leaves discovery,
  key rotation and the nonce binding to DeckStreak's own code.
- A hosted identity broker: rejected because a third service would hold the owner's sign-ins, and
  a Telegram-first identity needs none (ADR-006).
- A long-lived Apple client secret stored as a credential: rejected because a leaked secret would
  stay valid for months, where a secret minted per exchange lives 300 seconds.

## Decision Outcome

Chosen option: "`openidconnect` 4, default features off, over the `ProviderHttp` port", because it
is the only option that both verifies the token with a maintained, typed implementation and keeps
every provider call behind a port the tests control.

- **The adapter.** The daemon's wiring builds one adapter over `reqwest` (the 0.13 line the bot's
  client already brings, with rustls), redirects disabled and a 10-second bound per call, and hands
  it to every role that calls a provider.
- **Apple's secret.** An ES256 JWT signed with `p256` for each exchange (`kid`, `iss` the team id,
  `sub` the client id, `aud` Apple's issuer, `exp` 300 seconds after `iat`), never stored.
- **The credentials.** `google-client-secret`, `apple-signing-key` and `link-token-key` are read
  through the kernel's loader as optional (ADR-067): missing turns the provider off, empty refuses
  start by the credential's id. No unit names them until #347's delivery binds them.
- **The requests.** Google asks `scope=openid` alone, Apple asks for neither name nor email, and the
  email claim is never read, so no identity is joined by email.

### Consequences

- Good, because each of the callback's ten refusals is a typed branch that a test double can reach.
- Good, because no provider secret for Apple exists at rest.
- Bad, because the adapter is DeckStreak's code: its bound and its redirect policy are tested here,
  not by the library.

### Confirmation

SPEC-131 §3 (the flow and callback criteria, and the credentials' loads) and its rows in
`S13100-S13199`.

## What would make this wrong

- Apple stops answering `response_mode=query` for a request that asks no scope: the callback would
  become a cross-site `POST`, and the flow cookie's `SameSite=Lax` would need its own decision.
- A provider that cannot be reached through a plain HTTPS client (a mutual-TLS token endpoint): the
  port would gain a second adapter.

## More Information

SPEC-131, ADR-006, ADR-024, ADR-067, ADR-132, ADR-133, SPEC-066, RFC 9207 (the issuer parameter),
and the W7 schematic `docs/schematics/w7-sign-in-and-linking.md`.
