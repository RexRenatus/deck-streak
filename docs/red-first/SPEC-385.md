# Red-first record: SPEC-385

The SPEC, ADR-399 and the web passkey schematic were committed first (929dd94e) and the SPEC
re-installed with the startapp route test admitted (46416b6d). Every criterion's test was then
committed with one stub per production file (bd2efd4b), before the code that turns them green. A
stub keeps every input but the missing behaviour, so each criterion is red for the reason section 3
of SPEC-385 states. Each red below is the first failing line of that criterion's test in the whole
suite (`pnpm run test` in `web/app`) at bd2efd4b, where exactly these 19 tests failed and the other
550 passed.

## The fence, line by line

Each of the 19 lines of SPEC-385 section 3's fence resolves to one test, named by the `-t` filter.

| # | criterion | test file | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/src/routes/link.test.ts` | added |
| 2 | A2 | `web/app/src/routes/signin.test.ts` | added |
| 3 | A3 | `web/app/src/routes/sign-in-methods.test.ts` | added |
| 4 | A4 | `web/app/src/routes/sign-in-methods.test.ts` | added |
| 5 | A5 | `web/app/src/lib/api.test.ts` | added |
| 6 | A6 | `web/app/src/lib/passkeys.test.ts` | added |
| 7 | A7 | `web/app/src/lib/passkeys.test.ts` | added |
| 8 | A8 | `web/app/src/routes/link.test.ts` | added |
| 9 | A9 | `web/app/src/routes/link.test.ts` | added |
| 10 | A10 | `web/app/src/routes/signin.test.ts` | added |
| 11 | A11 | `web/app/src/lib/passkeys.test.ts` | added |
| 12 | A12 | `web/app/src/routes/signin.test.ts` | added |
| 13 | A13 | `web/app/src/lib/passkeys.test.ts` | added |
| 14 | A14 | `web/app/src/lib/passkeys.test.ts` | added |
| 15 | A15 | `web/app/src/routes/layout.test.ts` | added |
| 16 | A16 | `web/app/src/routes/sign-in-methods.test.ts` | added |
| 17 | A17 | `web/app/src/routes/sign-in-methods.test.ts` | added |
| 18 | A18 | `web/app/src/routes/link.test.ts` | added |
| 19 | A19 | `web/app/src/lib/passkey-messages.test.ts` | added |

## Why each red is the criterion's

- A1, A8: the stub link page posts `/api/link/redeem?code=<code>` on load, so the first line that
  fails is the census of requests sent before the owner's tap, and it names the code in a URL.
- A2: the stub sign-in page offers its passkey button where the browser has no WebAuthn.
- A3: the stub methods screen offers removal on the Telegram row. A4: it calls the authenticator in
  place. A16: it posts the handshake and mints again. A17: it deletes on the first tap.
- A5: outside Telegram the client answers before any request is sent.
- A6, A7, A11, A13, A14: the stub ceremony client returns its input unconverted, never signals,
  hands any options to the authenticator, maps every refusal to nothing, and posts with no content
  type.
- A9: the stub link page registers nothing. A18: it keeps the code in session storage.
- A10: the stub sign-in page stays on the page after a sign-in. A12: it posts the finish twice.
- A15: the shell calls the wallet on every route. A19: no locale has the keys.

```red-first
A1: red at bd2efd4b: AssertionError: expected [ Array(1) ] to deeply equal [] (received [ "POST /api/link/redeem?code=c0de-C0DE_" ], link.test.ts:110)
A2: red at bd2efd4b: AssertionError: expected <button type="button" …(1)></button> to be null (signin.test.ts:107)
A3: red at bd2efd4b: AssertionError: expected [ <button type="button"></button> ] to deeply equal [] (sign-in-methods.test.ts:94)
A4: red at bd2efd4b: AssertionError: expected [ 1, +0 ] to deeply equal [ +0, +0 ] (sign-in-methods.test.ts:123)
A5: red at bd2efd4b: AssertionError: expected { kind: 'reopen' } to deeply equal { kind: 'ok', …(1) } (api.test.ts:808)
A6: red at bd2efd4b: AssertionError: expected [ '-', '-', '-', '-' ] to deeply equal [ 251, 239, 190 ] (passkeys.test.ts:165)
A7: red at bd2efd4b: AssertionError: expected [] to deeply equal [ [ { rpId: …, …(2) } ] ] (passkeys.test.ts:208)
A8: red at bd2efd4b: AssertionError: expected [ 'POST /api/link/redeem?code=c0de' ] to deeply equal [] (link.test.ts:130)
A9: red at bd2efd4b: AssertionError: expected [ 'POST /api/link/redeem?code=c0de' ] to deeply equal [ 'POST /api/link/redeem', …(2) ] (link.test.ts:147)
A10: red at bd2efd4b: AssertionError: expected [] to deeply equal [ [ '/' ] ] (signin.test.ts:131)
A11: red at bd2efd4b: AssertionError: expected "vi.fn()" to not be called at all, but actually been called 1 times (passkeys.test.ts:231)
A12: red at bd2efd4b: AssertionError: expected [ …(4) ] to deeply equal [ …(2) ] (received a second sign-in start and finish, signin.test.ts:152)
A13: red at bd2efd4b: AssertionError: expected [ 401, 'not_linked', …(1) ] to deeply equal [ 401, 'not_linked', …(1) ] (received wording null, expected "signin_not_linked", passkeys.test.ts:289)
A14: red at bd2efd4b: AssertionError: expected [ '/api/link/redeem', …(2) ] to deeply equal [ '/api/link/redeem', …(2) ] (received headers {}, expected the JSON content type, passkeys.test.ts:329)
A15: red at bd2efd4b: AssertionError: expected [ '/link', 1, 1 ] to deeply equal [ '/link', +0, +0 ] (layout.test.ts:83)
A16: red at bd2efd4b: AssertionError: expected [ 'POST /api/link/code', …(2) ] to deeply equal [ 'POST /api/link/code' ] (received "POST /api/session" and a second mint, sign-in-methods.test.ts:141)
A17: red at bd2efd4b: AssertionError: expected [ 'DELETE /api/identities/4' ] to deeply equal [] (sign-in-methods.test.ts:156)
A18: red at bd2efd4b: AssertionError: expected "setItem" to not be called at all, but actually been called 1 times (link.test.ts:182)
A19: red at bd2efd4b: AssertionError: expected [ 'en', 'signin_title', false ] to deeply equal [ 'en', 'signin_title', true ] (passkey-messages.test.ts:56)
A6: green at bdc244de
A7: green at bdc244de
A11: green at bdc244de
A13: green at bdc244de
A14: green at bdc244de
A19: green at bdc244de
A5: green at 4b82077c
A15: green at 4b82077c
A1: green at 77c313c0
A2: green at 77c313c0
A8: green at 77c313c0
A9: green at 77c313c0
A10: green at 77c313c0
A12: green at 77c313c0
A18: green at 77c313c0
A3: green at 1d657022
A4: green at 1d657022
A16: green at 1d657022
A17: green at 1d657022
```
