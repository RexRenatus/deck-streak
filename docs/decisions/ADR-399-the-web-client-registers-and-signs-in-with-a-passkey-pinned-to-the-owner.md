---
status: "proposed"
decision-makers: "the owner, the DeckStreak architect"
---

# The web client registers and signs in with a passkey pinned to the owner: the browser's own ceremony API, a binding through the link code, the browser as the only ceremony surface, and one delivery

## Context and Problem Statement

#627 asks for passkey sign-in pinned to the one owner account, the web first. The server half is
on dev (SPEC-359, ADR-370): the link code, the `link` and `linked` sessions, the strict ceremony
cookie, the bound, the counter's compare-and-swap and every refusal code. ADR-132 settles the
library, the relying party (the web client's host) and that every ceremony's state stays on the
server. What does not exist is the web client's half: no file under `web/app` calls the browser's
credential API, `api.ts` answers `reopen` before any request when there is no launch data, and no
screen links, signs in or lists the methods. SPEC-359 §7 named those screens and their tests and
left them to a delivery of their own.

## Decision Drivers

- The gate stays on the server: the client opens no session, holds no challenge and never decides
  who the owner is.
- One owner, Telegram first (ADR-006): a passkey is only ever a second way in to the account
  Telegram already identifies.
- The Mini App's frame is granted no WebAuthn, and the browser is.
- The smallest client that the server's own tests already describe, with no new runtime
  dependency on the path that decides sign-in.
- Every criterion red first, and every changed web file proved by mutation.

## D1. The web ceremony client is the browser's own API, through a codec the app owns

`web/app/src/lib/passkeys.ts` calls `navigator.credentials.create` and `navigator.credentials.get`
directly. A small unpadded base64url codec turns the server's options (under `publicKey`: the
challenge, the user id, each excluded or allowed credential id) into bytes, and the
authenticator's answer into the JSON the server parses (`id`, `rawId`, `type`, `response`, and
`extensions` as `{}`). The challenge travels server to client inside the options and client to
server only inside the authenticator's signed client data; the flow id travels in the strict
ceremony cookie, and the session in the session cookie, both HttpOnly. The client posts only to
same-origin relative paths, as JSON, once per finish, and keeps nothing: no storage write, no log
of a code, an id, a handle or a response.

## D1. Chosen against

- A pinned helper library such as `@simplewebauthn/browser`: it would be the app's first runtime dependency, on the one path that decides sign-in, for a conversion of a handful of fields.
- The browser's own JSON helpers (`parseCreationOptionsFromJSON`, `parseRequestOptionsFromJSON`): not every browser offers them, so a fallback codec would still be needed, and two paths are two surfaces to test.
- The `api.ts` call loop for ceremony posts: its one renewal re-posts the handshake on a 401, and a ceremony's 401 is a refusal to show, never a reason to sign in again.
- Forwarding the client's extension results: the server reads none of them, and its own tests post `{}`.

## D1. Consequences

- Good: no dependency, and the codec is pinned by literal goldens (A6).
- Good: the client cannot be made to carry a challenge or a flow id; R2 and A14 hold it.
- Bad: the codec is the app's own code to keep; StrykerJS holds every branch of it.

## D2. The passkey binds through the server's link code, and a session opens only from the server

Inside Telegram, the methods screen mints a code from a telegram session and opens the link page in
the browser with the code in the fragment. The link page redeems it in a JSON body on the owner's
tap, which opens a `link` session, and registers inside it. A mint or removal refused
`reauth_required` asks the owner to reopen DeckStreak from Telegram; the client never posts launch
data again to refresh it, and sends each mint and each removal once.

On the client, a second account's passkey meets `not_owner`, an unknown credential meets
`not_linked` or `passkey_invalid`, and a replayed or stale ceremony meets `challenge_invalid` or
`challenge_expired`. Each maps to one message (SPEC-385 §2a), none leaves the page, and a refused
finish is never posted again. Options naming another relying party never reach the authenticator.

Outside Telegram, `api.ts` sends owner calls with the session cookie and no handshake. A 401 asks
for sign-in once (the shell opens `/signin`) and answers `reopen` to its screen. The shell makes no
owner call on `/link` or `/signin`. A sign-in that succeeds goes to Today at the fixed path `/`.

## D2. Chosen against

- An automatic re-handshake on `reauth_required`: launch data stays valid for an hour, so posting it again would stretch the server's five-minute rule for a mint or a removal to the launch data's bound.
- A fourth answer kind (`signin`) in `api.ts`: `reopen` is matched across about a dozen screens, so a fourth kind would touch every one of them for a wording the sign-in page already carries.
- Registering straight from the telegram session, with no code: the frame is granted no WebAuthn, and the server registers only inside a `link` session.
- A redirect target read from the URL after sign-in: a `next` parameter is an open redirect, and Today is the one destination.

## D2. Consequences

- Good: the five-minute rule for linking and removal stays the server's, unbent by the client.
- Good: a refusal never opens a session on the client, because the client never opens one.
- Bad: a 401 outside Telegram shows the screen's `reopen` wording for the moment before `/signin`
  opens.

## D3. The Mini App mints and opens; the browser registers and signs in; the native half stays out

The surface test is `telegram.launchData !== null`. Inside Telegram the owner meets the sign-in
methods screen (`/sign-in-methods`, a component the settings screen embeds later, #57): Telegram
first with no removal, each passkey with Remove behind a confirm step, and "Link a passkey", which
opens `<the page's origin>/link#<code>` through the Mini App's link opener. No ceremony runs in the
frame. In a plain browser, `/link` checks for WebAuthn first: with none, it keeps the code unspent
and asks for the browser; with it, a tap redeems and registers. `/signin` offers "Sign in with a
passkey", with the wording of SPEC-385 §2b in all seven locales. The native half of #627 (associated
domains and a native sign-in) is out of scope and stays on #627.

## D3. Chosen against

- `telegram.inside` as the surface test: at `e7ecf10d` the script loaded on every page, so it read true in a plain browser where no launch data exists; since SPEC-400 it names only whether the script ran, still not the launch data the handshake reads.
- A ceremony inside Telegram's frame: the frame is a cross-origin embed with no grant for the credential features, whose default allowlist is the embedding page's own origin, so the ceremony answers `NotAllowedError`.
- A link URL from an origin the server sends: it would need a Rust change for a value the page already holds while one host serves both.
- The native half in this delivery: it needs associated domains, a native token and a server route the server half does not have, so it is a delivery of its own.
- Redeeming the code on load: a browser with no WebAuthn would spend it, and the owner would need a new one.

## D3. Consequences

- Good: one ceremony surface, the browser, with one relying party and one origin.
- Bad: linking is two steps across two apps; the wording names each step.
- Bad: a second host (#168) would split the page's origin from the relying party (below).

## D4. Tests are vitest, red first per criterion; StrykerJS is the mutation proof; formal is not applicable by surface

Each of SPEC-385's nineteen criteria is a vitest test that is red against a stub keeping every
input but the missing behaviour, then green. SPEC-359 §7's eight keep their test names. The two
enumerating tests print what they examined and refuse an empty population. The band
`S38500-S38599` is claimed and holds no rows file; StrykerJS over every changed production file,
at a break of 100, is the mutation proof. The formal decision is NOT APPLICABLE by surface: the
change is TypeScript and Svelte, which no cover names and the registry cannot cover, and its new
actor, a browser tab issuing the server's existing requests, takes no step `PasskeyOnce` lacks.

## D4. Chosen against

- Hand rows in the band: no rows table runs a vitest killer, so a row could not be killed by the tests that hold the behaviour.
- A model or a proof of the web client: the registry covers Rust, Python and shell paths only, and the new actor adds no step to the ceremony store the model already holds.
- New rows in the threat model's web table: the web's identity routes belong to the whole-product model (#60), and another open change edits that schematic.
- A browser end-to-end ceremony with a virtual authenticator: the web suite has no such harness, and the owner's device proof (#637) is the end-to-end record.

## D4. Consequences

- Good: every criterion is decided by a test in the tree; every changed line is held by a mutant.
- Bad: the real authenticator is met only on the owner's device.

## D5. One delivery under SPEC-385 and ADR-399, carrying ADR-132's flip

SPEC-385 delivers SPEC-359 §7 (A37 to A44, under their test names) and the criteria this decision
adds, with its own band, red-first record and changelog fragment. SPEC-359 and ADR-370 are not
edited. ADR-132's front matter flips from `proposed` to `accepted` in the first documents commit,
and nothing else in it changes.

## D5. Chosen against

- An insert-only amendment of SPEC-359 and ADR-370: moving §7's rows into SPEC-359's fence edits text that already stands, and §7 lacks the registration, replay, gate and locale criteria added here.
- A status note in SPEC-359 pointing here: a changed SPEC is judged again whole, and this SPEC's header already names what it delivers.
- Two deliveries, the ceremony client then the screens: the client has no caller without the screens, so its delivery would prove nothing a learner meets.

## D5. Consequences

- Good: the band, the red-first record and the changelog are this delivery's own, as ADR-370 D7
  reasoned for the server half.
- Bad: SPEC-359 §7 reads as undelivered on its own page; SPEC-385's header is the pointer.

## Consequences

- The owner can link a passkey from the Mini App and sign in to the web client in a browser, and
  every refusal says what to do next.
- No session is opened by the client; the server's gate, bound and counter stand as built.
- The six non-English translations are written by the build and await a native review.

## What would make this wrong

- A second host (#168): the link URL is built from the page's own origin, which must then be the
  relying party's host.
- An edge Permissions-Policy that omits the credential features: every ceremony would answer
  `NotAllowedError`.
- Telegram granting its frame the credential features: a ceremony in place would then be possible,
  and D3's two-step link could be revisited.
- A server change to the options' or the finish's JSON shape: the codec follows the server's own
  tests, so they move together.
