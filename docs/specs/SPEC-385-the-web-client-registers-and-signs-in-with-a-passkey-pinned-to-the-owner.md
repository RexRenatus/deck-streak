# SPEC-385: the web client registers and signs in with a passkey pinned to the owner, through the link page, the sign-in page and the sign-in methods screen

- **Issue:** #627 (passkey sign-in pinned to the one owner account, the web first; SPEC-334 R19
  and its stretch row 2.1, a passkeys-only slice of #58). **Context(s):** the web client
  (`web/app`: the ceremony client, the session gate, the link page, the sign-in page, the sign-in
  methods screen, their wording in every locale). The server half is on dev and is not edited.
- **Decided by:** ADR-399 (this SPEC's: the ceremony client, the binding, the surfaces, the tests,
  the shape), ADR-370 (the server half: the ceremony cookie, the counter, sign-in over the owner's
  own credentials, the bound, removal), ADR-132 (passkeys through `webauthn-rs`, the web client's
  host as the relying party, every ceremony's state on the server; this delivery flips its status
  to accepted) and ADR-006 (Telegram first, pinned to the owner, other methods linked).
- **Delivers from SPEC-359:** §7's web criteria A37 to A44 under SPEC-359's own test names, as
  this SPEC's A1 to A8. SPEC-359 is not edited; its §7 A45 stays with the deploy rail (§5).
- **Mutation band:** `S38500-S38599`, claimed, holding no rows file (§7).
- **Status:** one pull request. Every path it writes is under `web/app` or `docs`, plus its
  changelog fragment.

## 1. The problem, measured

Every `path:line` was read at `e7ecf10d6b796eb1f86fe6544e04a96a0583c791`.

- The server half is built. `crates/api/src/linking_routes.rs:44-58` serves `POST /api/link/code`,
  `POST /api/link/redeem`, `POST /api/passkeys/register/start` and `/finish`,
  `POST /api/passkeys/sign-in/start` and `/finish`, `GET /api/identities` and
  `DELETE /api/identities/{id}`. A start answers the options as the library serializes them,
  under `publicKey` (`crates/identity/src/passkeys.rs:102-107`, `linking_routes.rs:361`); a
  registration finish answers 201 and the row id (`linking_routes.rs:182`); a sign-in finish
  answers the accepted credential ids and the user handle beside the session cookie
  (`linking_routes.rs:208-217`). The JSON the server parses is the shape its own tests post
  (`crates/identity/tests/support/mod.rs:366-397`).
- The web client has no passkey code. A `git grep` for the browser's credential calls over the
  whole tree, fenced, finds them in no file under `web/app`. `web/app/package.json` declares
  development dependencies only.
- Outside Telegram the client cannot use a session. `web/app/src/lib/api.ts:141-173` answers
  `reopen` before any request when there is no launch data, and `api.ts:113-128` refuses the
  handshake without it. `api.ts:43-46` holds three answers: `ok`, `reopen`, `unavailable`.
- "Inside Telegram" is not `telegram.inside`. `web/app/src/app.html:6` loaded Telegram's script on
  every page, so `web/app/src/lib/telegram.svelte.ts:92` read true in a plain browser too; the
  launch data (`telegram.svelte.ts:95`) is null there. SPEC-400 since loads the script only on a
  launch (`web/app/src/lib/telegram-launch.ts`), so `inside` now names whether the script ran,
  which is still not the launch data the server's handshake reads.
- No screen exists for linking or signing in. `web/app/src/lib/routes.ts:10-27` lists 16 routes and
  none of `/link`, `/signin`, `/sign-in-methods`; `web/app/src/lib/a11y-coverage.test.ts:37-41` holds
  that list equal to the screens.
- The wording does not exist. `web/app/messages/en.json` holds 227 keys and each of the six other
  locales 127; no key names a passkey.
- The server half's ceremony store and counter are modelled (`formal/tla/PasskeyOnce/PasskeyOnce.tla:2-4`
  covers `passkeys.rs` and `linking.rs`); no `@phx covers` line names a path under `web/`.

## 2. Requirements

- R1. The ceremony client is `web/app/src/lib/passkeys.ts`, calling the browser's
  `navigator.credentials.create` and `navigator.credentials.get` directly. It decodes the server's
  base64url fields (the challenge, the user id, every excluded or allowed credential id) to bytes,
  and encodes the authenticator's response back to unpadded base64url JSON in the shape the
  server parses (`support/mod.rs:366-397`), with `extensions` sent as `{}`. `web/app/package.json`
  gains no dependency.
- R2. Every ceremony call is a same-origin `POST` to a relative path, with
  `credentials: 'same-origin'` and `content-type: application/json`. A start sends `{}`; a finish
  sends only the authenticator's response. The client never makes, posts or keeps a challenge
  beyond the one call, and never reads the flow id or a session id (both HttpOnly cookies).
- R3. The client writes nothing to `localStorage`, `sessionStorage`, IndexedDB or Telegram's
  storage, and logs no link code, credential id, user handle or ceremony response.
- R4. Options whose relying party (the registration's `rp.id`, the sign-in's `rpId`) is absent or
  differs from the page's host are refused before the authenticator is asked.
- R5. A finish is posted at most once per start. A refused finish is never posted again; another
  attempt is a new start.
- R6. A passkey is bound to the owner only through the server's link code. Inside Telegram the
  methods screen mints a code and opens `<the page's origin>/link#<code>` through the Mini App's
  link opener (`telegram.svelte.ts:137-142`); no ceremony runs inside Telegram's frame. The link
  page redeems the code, in a JSON body, only on the learner's tap and only where the browser
  offers WebAuthn; it clears the fragment once the redeem has answered, and registers inside the
  `link` session the redeem opened.
- R7. A mint or a removal refused `reauth_required` asks the owner to reopen DeckStreak from
  Telegram. The client never posts launch data again to refresh it, and sends each mint and each
  removal once.
- R8. Outside Telegram the sign-in page offers "Sign in with a passkey". After a sign-in the page
  signals the accepted credentials where the browser offers the signal (feature-detected, the
  relying party taken from the request options) and goes to Today at the fixed path `/`. It reads
  no redirect target from the URL.
- R9. Outside Telegram `api.ts` sends owner calls with the session cookie and no handshake. A
  call refused 401 asks for sign-in, which opens `/signin`, and answers `reopen` to its screen;
  the answer kinds stay three. The shell makes no owner call on `/link` or `/signin`. Inside
  Telegram the handshake and its one renewal are unchanged.
- R10. The surface test is `telegram.launchData !== null`, never `telegram.inside`.
- R11. Every refusal the ceremony routes answer maps to one message key (§2a), and no refusal
  opens a session or leaves the page.
- R12. The sign-in methods screen (`/sign-in-methods`, a component the settings screen embeds
  later, #57) lists `GET /api/identities`: Telegram first with no removal, then each passkey with
  its date and a Remove behind a confirm step that sends one `DELETE`. Today links to it.
- R13. The wording is §2b's English, carried by all seven locales.
- R14. `routes.ts` lists `/link`, `/signin` and `/sign-in-methods`.
- R15. ADR-132's front matter reads `status: "accepted"`; nothing else in it changes.

### 2a. Each refusal and what it meets

| route | status and code | message key | what the page does |
|---|---|---|---|
| sign-in start | 401 `not_linked` | `signin_not_linked` | asks the authenticator nothing |
| any finish | 401 `challenge_invalid`, `challenge_expired` | `passkey_start_again` | offers a new start; the finish is not posted again |
| any finish | 401 `origin_mismatch`, `uv_required`, `passkey_invalid`, `counter_regressed`; 403 `not_owner` | `passkey_refused` | stays; no session |
| any start or finish | 429 `too_many_ceremonies` | `passkey_too_many` | stays |
| any ceremony route, mint | 404 `linking_off` | `passkey_off` | starts no ceremony |
| any ceremony route | 403 `cross_site_request`, `not_json` | `passkey_refused` | stays |
| redeem | `link_code_invalid`, `link_code_expired` | `link_code_spent` | the code is cleared |
| registration start | 401 `no_session` | `link_code_spent` | starts no ceremony |
| registration finish | 409 `already_linked` | `link_already` | stays |
| mint, removal | 401 `reauth_required` | `methods_reopen` | no launch data is posted |
| removal | 404 `identity_unknown` | none | the list is read again |
| the authenticator | `NotAllowedError` | `passkey_cancelled` | offers a new start |
| any other answer | any | the screen's existing unavailable wording | stays |

### 2b. The wording, in English

| key | English |
|---|---|
| `signin_title` | Sign in |
| `signin_with_passkey` | Sign in with a passkey |
| `signin_no_webauthn` | This browser can't use passkeys. Open DeckStreak in a browser that can, or reopen it from Telegram. |
| `signin_not_linked` | No passkey is linked yet. Link one from DeckStreak in Telegram. |
| `passkey_refused` | That passkey can't sign in to DeckStreak. |
| `passkey_start_again` | That attempt expired or was already used. Start again. |
| `passkey_too_many` | Too many attempts. Wait a minute, then start again. |
| `passkey_off` | Passkeys aren't available here. |
| `passkey_cancelled` | No passkey was used. You can start again. |
| `link_title` | Link a passkey |
| `link_create` | Create a passkey |
| `link_open_in_browser` | Open this link in your browser to create a passkey. |
| `link_no_code` | This link has no code. Get a new one from DeckStreak in Telegram. |
| `link_code_spent` | This link expired or was already used. Get a new one from DeckStreak in Telegram. |
| `link_done` | Passkey created. You can now sign in to DeckStreak in this browser. |
| `link_already` | This passkey is already linked. |
| `methods_title` | Sign-in methods |
| `methods_telegram` | Telegram |
| `methods_passkey` | Passkey |
| `methods_added` | Added {date} |
| `methods_link_passkey` | Link a passkey |
| `methods_remove` | Remove |
| `methods_remove_confirm` | Remove this passkey? You can still sign in with Telegram. |
| `methods_remove_keep` | Keep |
| `methods_reopen` | Reopen DeckStreak from Telegram, then try again. |

## 3. Acceptance criteria of SPEC-385

| # | criterion | decided by |
|---|---|---|
| A1 | the link page reads the code from the fragment, posts it in a body, never in a URL, and clears it from the address once the redeem answered (SPEC-359 A37) | `link.test.ts`; red: the stub posts the code in the URL and keeps the fragment |
| A2 | outside Telegram the sign-in page offers a passkey where the browser offers WebAuthn, and says to open a browser that can where it does not (SPEC-359 A38) | `signin.test.ts`; red: the stub offers a passkey where the browser has no WebAuthn |
| A3 | the sign-in methods screen lists the methods, Telegram first, and offers no removal for Telegram (SPEC-359 A39) | `sign-in-methods.test.ts`; red: the stub offers removal on every row |
| A4 | inside Telegram, "Link a passkey" mints a code and opens `<origin>/link#<code>` through the link opener, and no ceremony runs in the frame (SPEC-359 A40) | `sign-in-methods.test.ts`; red: the stub calls the authenticator in place |
| A5 | outside Telegram a call is sent with the session cookie, and a 401 asks for sign-in once instead of answering before any request (SPEC-359 A41) | `api.test.ts`; red: the call sends nothing and asks for nothing |
| A6 | ceremony options and responses round-trip through unpadded base64url, against literal goldens (`----` is bytes `fb ef be`, `____` is `ff ff ff`, `AA` is `00`) (SPEC-359 A42) | `passkeys.test.ts`; red: the stub returns its input unconverted |
| A7 | after a sign-in the page signals the accepted credentials where the browser offers it, with the request's relying party, and calls nothing where it does not (SPEC-359 A43) | `passkeys.test.ts`; red: the stub never signals |
| A8 | where the browser offers no WebAuthn, the link page redeems nothing and asks the owner to open the link in the browser, keeping the code unspent (SPEC-359 A44) | `link.test.ts`; red: the stub redeems on load |
| A9 | the link page registers a passkey after the code is redeemed: redeem, start, create with decoded options, one finish in the server's shape, then the done wording | `link.test.ts`; red: the stub registers nothing |
| A10 | a passkey sign-in posts one finish in the server's shape and opens Today at `/` | `signin.test.ts`; red: the stub stays on the page after a sign-in |
| A11 | options naming another relying party, or none, never reach the authenticator | `passkeys.test.ts`; red: the stub passes them to the authenticator |
| A12 | a refused finish is never posted again, opens no session, and offers a new start | `signin.test.ts`; red: the stub posts the finish twice |
| A13 | every refusal of §2a maps to its message key, and an unknown code maps to the refusal wording | `passkeys.test.ts`; red: the stub maps every refusal to nothing |
| A14 | every ceremony call is a same-origin JSON post to a relative path | `passkeys.test.ts`; red: the stub posts with no content type |
| A15 | the open routes `/link` and `/signin` make no owner call, while Today makes its own | `layout.test.ts`; red: the shell calls the wallet on every route |
| A16 | a mint refused `reauth_required` asks for a reopen, posts no launch data and opens no link | `sign-in-methods.test.ts`; red: the stub re-posts the handshake |
| A17 | removing a passkey asks first, Keep sends nothing, and Remove sends one delete | `sign-in-methods.test.ts`; red: the stub deletes on the first tap |
| A18 | a whole registration writes nothing to browser storage | `link.test.ts`; red: the stub keeps the code in session storage |
| A19 | every locale carries every key of §2b, non-empty | `passkey-messages.test.ts`; red: no locale has the keys |

```acceptance
A1: pnpm exec vitest run web/app/src/routes/link.test.ts -t "the link page posts the code from the fragment"
A2: pnpm exec vitest run web/app/src/routes/signin.test.ts -t "outside telegram the shell offers sign-in"
A3: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "the sign-in section offers no unlink for telegram"
A4: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "link a passkey opens the link page in the browser"
A5: pnpm exec vitest run web/app/src/lib/api.test.ts -t "outside telegram a refused call asks for sign-in"
A6: pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "ceremony options and responses round-trip through base64url"
A7: pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "a sign-in signals the accepted credentials only where the browser offers it"
A8: pnpm exec vitest run web/app/src/routes/link.test.ts -t "without webauthn the link page keeps the code unspent"
A9: pnpm exec vitest run web/app/src/routes/link.test.ts -t "the link page registers a passkey after the code is redeemed"
A10: pnpm exec vitest run web/app/src/routes/signin.test.ts -t "a passkey sign-in opens today"
A11: pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "options for another relying party never reach the authenticator"
A12: pnpm exec vitest run web/app/src/routes/signin.test.ts -t "a refused finish is never posted again"
A13: pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "every ceremony refusal maps to its message"
A14: pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "every ceremony call is a same-origin json post"
A15: pnpm exec vitest run web/app/src/routes/layout.test.ts -t "the open routes make no owner call"
A16: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "a stale telegram session asks for a reopen before a link"
A17: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "removing a passkey asks first and sends one delete"
A18: pnpm exec vitest run web/app/src/routes/link.test.ts -t "a registration writes nothing to browser storage"
A19: pnpm exec vitest run web/app/src/lib/passkey-messages.test.ts -t "every locale carries the passkey keys"
```

A13 and A19 enumerate: each prints `examined N of N` over its fixed table and refuses unless every
member reached its assertion. A6 compares with literal goldens, never with the codec's own output.
A12, A16 and A18 assert the forbidden act first and the positive artifact (the one finish, the
reopen wording, the posted finish) second, so a page that did nothing cannot pass.

## 3a. Proved on a device by the owner

| # | what | who, when, record |
|---|---|---|
| D1 | a passkey is created from the link the Mini App opens, and signs in to the web client in the system browser, on a phone and on a computer | the owner, after the deploy; recorded on #637 beside SPEC-359's D1 |

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-385-the-web-client-registers-and-signs-in-with-a-passkey-pinned-to-the-owner.md` | document | new |
| `docs/decisions/ADR-399-the-web-client-registers-and-signs-in-with-a-passkey-pinned-to-the-owner.md` | document | new |
| `docs/decisions/ADR-132-passkeys-through-webauthn-rs-with-the-mini-apps-host-as-the-relying-party.md` | document | its status line only |
| `docs/schematics/passkey-sign-in-on-the-web.md` | document | sections appended; nothing above them changes |
| `docs/red-first/SPEC-385.md` | document | new |
| `changelog.d/web-passkey-385.md` | document | new |
| `web/app/src/lib/passkeys.ts` | web client | new |
| `web/app/src/lib/passkeys.test.ts` | web client | new |
| `web/app/src/lib/passkey-messages.test.ts` | web client | new |
| `web/app/src/lib/api.ts` | web client | changed |
| `web/app/src/lib/api.test.ts` | web client | changed |
| `web/app/src/lib/routes.ts` | web client | changed |
| `web/app/src/lib/startapp.test.ts` | web client | `BY_PATH` gains the three paths, plus one comment line citing R6, R8 and R12 |
| `web/app/src/lib/settings/SignInMethods.svelte` | web client | new |
| `web/app/src/routes/+layout.svelte` | web client | changed |
| `web/app/src/routes/layout.test.ts` | web client | A15's test added beside its two existing tests, which do not change |
| `web/app/src/routes/+page.svelte` | web client | Today's link to the methods screen |
| `web/app/src/routes/link/+page.svelte` | web client | new |
| `web/app/src/routes/link.test.ts` | web client | new |
| `web/app/src/routes/signin/+page.svelte` | web client | new |
| `web/app/src/routes/signin.test.ts` | web client | new |
| `web/app/src/routes/sign-in-methods/+page.svelte` | web client | new |
| `web/app/src/routes/sign-in-methods.test.ts` | web client | new |
| `web/app/messages/en.json` | web client | changed |
| `web/app/messages/es.json` | web client | changed |
| `web/app/messages/fr.json` | web client | changed |
| `web/app/messages/ja.json` | web client | changed |
| `web/app/messages/ko.json` | web client | changed |
| `web/app/messages/zh-Hans.json` | web client | changed |
| `web/app/messages/zh-Hant.json` | web client | changed |

No `.rs`, `.swift`, edge, content-security or workflow file changes.

## 5. What this does NOT cover

- No native passkey sign-in, no associated-domains file and no native bearer token in the device's key store: those are the native half of #627, which stays open for them.
- No discoverable credential and no conditional UI: sign-in runs over the owner's own credentials (ADR-370 D1), #627.
- No ceremony inside Telegram's frame: the frame is granted no WebAuthn, so the Mini App opens the link page in the browser, #627.
- No change to the server half, its routes, its bound or its refusal codes: SPEC-359 holds them, #627.
- SPEC-359 §7's A45, the deploy-time library check, stays with the private deploy rail, #627.
- No settings screen: the methods screen is a component it embeds later, #57.
- No other linked sign-in method: the rest of #58.
- No public origin of a second host: the link URL is built from the page's own origin, #168.
- No commands with stakes for a `linked` session: the server refuses them, and their gate is #638.
- No bot notice when a passkey is linked or removed, #653.
- No new row in the threat model's web table: the web's identity routes belong to the whole-product model, #60.
- No translation review by a native speaker of the six non-English locales: the build writes them, and their review is tracked on #627.
- No on-device proof by a test: D1 is the owner's, recorded on #637.

## 6. Risks

- Telegram's script keeps the URL's hash parameters for the tab's session
  (`telegram.svelte.ts:4-5`), so the link code may sit in that tab's session storage. It is
  single-use, lives 600 seconds and is spent at the redeem. A18 holds the app's own code to no
  storage write; this copy is the script's. Since SPEC-400 the script loads only on a launch, so a
  `/link` tab opened from the methods screen, whose fragment carries no launch parameter, no
  longer loads it; the risk stands only for a tab that is a launch.
- A later edge Permissions-Policy that omits the credential features would make every ceremony
  answer `NotAllowedError`. Detected by D1, and by ADR-370's "What would make this wrong".
- A second host (#168) splits the page's origin from the relying party; the link URL built from
  the page's origin then lands on the wrong host. ADR-399 D3 names it.
- A 401 outside Telegram renders the screen's `reopen` wording for the moment before `/signin`
  opens. A5 holds that sign-in is asked; the wording is the existing screens'.
- The six translations are written by the build and not yet reviewed (§5).
- A shared path with an open pull request (the locale files and `routes.ts`) can conflict at
  merge; the build re-measures them at its cut and before its push.

## 7. Formal, rows and mutation

- **Formal: NOT APPLICABLE by surface.** The delivery is TypeScript and Svelte under `web/app`,
  which no `@phx covers` line names and which the formal registry cannot cover (a covered path is
  Rust, Python or shell). It edits no covered file. Its new actor is a browser tab issuing the
  server's existing requests; `PasskeyOnce` already lets any request task take any id, and a take
  of an id no live entry has is its stuttering step, so two tabs sharing one ceremony cookie take
  no step the model lacks and end in a refusal.
- **Rows:** none. No rows table runs a vitest killer, so the band `S38500-S38599` is claimed and
  holds no file.
- **Mutation:** StrykerJS over every changed production file under `web/app/src`, with the
  repository's own configuration at a break of 100; each survivor is killed by a test or recorded
  as equivalent with its reason.
