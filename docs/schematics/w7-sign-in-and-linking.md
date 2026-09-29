# Schematic: linked sign-in, from the link code to each refusal

Kind: sequence, decision flow and state machine. Read at DeckStreak `dev` 026d1f3 (ADR-006,
ADR-024, `docs/schematics/owner-session.md`, `docs/schematics/initdata-auth-sequence.md`,
`docs/schematics/data-rights-export-and-erase.md`, `docs/schematics/cron-fire-ledger-and-catch-up.md`).
Added by the W7 architect turn for SPEC-131 (ADR-131, ADR-132, ADR-133). It rewrites none of them:
the Telegram handshake of `initdata-auth-sequence.md` and the session store of `owner-session.md`
stand as drawn. What it adds is the second way in, and every point where a doubtful assertion is
refused. The identity model's declaration is SPEC-131's delivery (`docs/auth/identity-model.md`),
written when the methods ship, so this schematic declares nothing.

## The link: a fresh Telegram session hands a one-time code to the browser

```mermaid
sequenceDiagram
  participant M as Mini App in Telegram
  participant A as api role
  participant B as the owner's browser
  participant P as Google, Apple or the passkey authenticator
  M->>A: POST /api/link/code with a telegram session at most 300 s old
  A-->>M: a 128-bit code, once, kept only as its SHA-256, living 600 s
  M->>B: openLink to the origin's /link page with the code in the fragment
  B->>A: POST /api/link/redeem with the code
  A-->>B: the request's session ends, a link session starts, living 600 s
  B->>A: start a provider flow or a passkey registration, purpose link
  A->>P: the ceremony, bounded at 10 s per call
  P-->>A: an assertion
  A->>A: every check below, in order
  A->>A: one row for the configured owner's Telegram user id, or already_linked
```

No route creates an account: a link attaches to the owner's Telegram identity or is refused. The
email claim is never read, so no identity is joined by email.

## The OpenID Connect callback: ten refusals, in order

```mermaid
flowchart TD
  cb["GET the provider's callback"] --> f1{"flow cookie names a live flow"}
  f1 -->|no| r1["state_invalid"]
  f1 -->|yes| rm["the flow is removed, so a replay meets the first check"]
  rm --> f2{"flow at most 599 s old"}
  f2 -->|no| r2["state_expired"]
  f2 -->|yes| f3{"state equal, compared in constant time"}
  f3 -->|no| r3["state_invalid"]
  f3 -->|yes| f4{"the flow's provider path and issuer parameter"}
  f4 -->|no| r4["issuer_mismatch"]
  f4 -->|yes| ex["code exchange with the PKCE verifier"]
  ex --> f5{"an ID token came back"}
  f5 -->|no| r5["token_missing"]
  f5 -->|yes| f6{"signature verifies against the issuer's keys"}
  f6 -->|no| r6["token_invalid"]
  f6 -->|yes| f7{"iss is the provider's issuer"}
  f7 -->|no| r7["wrong_issuer"]
  f7 -->|yes| f8{"aud holds the client id, no other party"}
  f8 -->|no| r8["wrong_audience"]
  f8 -->|yes| f9{"exp not passed on the kernel's clock"}
  f9 -->|no| r9["token_expired"]
  f9 -->|yes| f10{"nonce is the flow's"}
  f10 -->|no| r10["nonce_mismatch"]
  f10 -->|yes| purpose{"purpose"}
  purpose -->|link, inside a link session| link["insert the row, or already_linked"]
  purpose -->|link, outside one| rr["reauth_required"]
  purpose -->|sign_in| look{"issuer and subject are linked"}
  look -->|no| nl["not_linked, nothing written"]
  look -->|to another Telegram user| no["not_owner"]
  look -->|to the owner| rot["session rotated, proof linked, redirect to an allow-listed target"]
```

The start route admits `provider` from `google` and `apple` only (`provider_unknown`) and
`return_to` from `/` and `/settings` only (`redirect_refused`). Each refusal consumes nothing but
the flow.

## The passkey ceremony

```mermaid
flowchart TD
  pk["a registration inside a link session, or an assertion at sign-in"] --> c1{"a live, unused ceremony held on the server"}
  c1 -->|no| p1["challenge_invalid"]
  c1 -->|yes| c2{"at most 299 s old"}
  c2 -->|no| p2["challenge_expired"]
  c2 -->|yes| c3{"origin and relying party are the configured ones"}
  c3 -->|no| p3["origin_mismatch"]
  c3 -->|yes| c4{"user verified"}
  c4 -->|no| p4["uv_required"]
  c4 -->|yes| c5{"credential known, at sign-in"}
  c5 -->|no| p5["not_linked"]
  c5 -->|yes| c6{"signature counter advances, when either is non-zero"}
  c6 -->|no| p6["counter_regressed"]
  c6 -->|yes| ok["counter and backup state stored, then the owner check and rotation"]
```

## What each session proof admits

```mermaid
stateDiagram-v2
  [*] --> telegram: the initData handshake
  [*] --> link: a redeemed link code
  [*] --> linked: a verified provider or passkey sign-in
  telegram --> telegram: mints a link code or unlinks, while its handshake is at most 300 s old
  link --> [*]: 600 s, admitted by the linking routes alone
  linked --> [*]: the store's idle and absolute bounds
  telegram --> [*]: the store's idle and absolute bounds
```

| proof | owner routes | mint a link code | unlink | linking routes |
|---|---|---|---|---|
| `telegram` | yes | yes, at most 300 s after its handshake | yes, at most 300 s after its handshake | yes |
| `link` | no | no | no | yes |
| `linked` | yes | no (`reauth_required`) | no (`reauth_required`) | no |

A linked identity never grants the owner's gate by itself: the sign-in reaches `linked` only when
the stored row's Telegram user id is the configured owner's.

## Unlink and erase: Apple's token is withdrawn at Apple first

```mermaid
flowchart LR
  req["unlink Apple, or either erase path"] --> open["open the sealed refresh token"]
  open --> rev{"Apple's revocation, bounded at 10 s, with the role's credential"}
  rev -->|accepted| del["delete the local row"]
  rev -->|failed, timed out or no credential| q["the sealed token, its issuer and token id move to identity_revocations, due in 1 h"]
  q --> del
  q --> job["job link_revocation, hourly at minute 41"]
  job --> try{"each due row, tried once"}
  try -->|accepted| gone["the row is deleted"]
  try -->|failed| wait["an attempt counted, the wait doubles: 1, 2, 4, 8, 16 h"]
  wait --> fifth{"fifth failed attempt"}
  fifth -->|yes| page["the row is deleted, the job exits with the page code"]
  fifth -->|no| job
```

A queued row never blocks an unlink, a sign-in or an erase. The erase order is the one
`data-rights-export-and-erase.md` draws, with the revocation step before the engine: when SPEC-137
has landed, its withdrawal of the public page runs first (`w7-publishing-and-unpublishing.md`).

## With no configuration

| missing | what happens |
|---|---|
| the public origin | every linking route answers 404 `linking_off`; Telegram's handshake is unchanged |
| a provider's credential, or the credentials directory | that provider answers `provider_off`; the others work |
| the token key or Apple's signing key in an erasing role | Apple's revocation is queued, never skipped |
| an empty credential | the role refuses to start, by the credential's id (ADR-067) |
