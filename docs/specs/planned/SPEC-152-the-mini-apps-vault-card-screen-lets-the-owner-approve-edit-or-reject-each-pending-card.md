# SPEC-152: the Mini App's vault-card screen lets the owner approve, edit or reject each pending card

- **Wave:** W9. **Issue:** #65 (the vault to Anki flashcard bridge, `SB-U19`) (epic #10); the owner's
  questions are #379. **Context(s):** the Mini App (`web/app`: the vault-card screen, its client and
  its route, token and strings). It calls SPEC-150's routes and adds no server code.
- **Decided by:** ADR-006 (owner-only gating is a safety property), ADR-150 (the stored GUID) and
  ADR-152 (both surfaces decide through one use case, and the package comes from the bot). No new
  decision is taken here, so this SPEC has no ADR of its own.
- **Prerequisites:** SPEC-024, SPEC-028 and SPEC-150 (planned, W9). **Mutation band:**
  `S15200-S15299`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-152.md` (ADR-016).

## 1. The problem, measured

- **The Mini App has three screens.** At `dev` a4036b3 `ROUTES` (`web/app/src/lib/routes.ts`) is
  `/`, `/about` and `/score`; `a11y-coverage.test.ts` holds it equal to the folders under
  `src/routes`, and `tests/a11y.spec.ts` audits each route in both Telegram colour schemes.
- **A route needs a token.** `startapp.ts` maps the tokens `today`, `about` and `score` to routes,
  and `startapp.test.ts` ("every destination is a screen of the route table") holds the tokens'
  destinations equal to `ROUTES`, so a new screen changes both files.
- **The client only reads.** `api.ts`, the Mini App's one API client, opens the session and reads
  `/api/me` and `/api/score`; SPEC-150's decision routes are `POST`s, which SPEC-024 R9 refuses
  unless they are `application/json` and same-site. The client gains a JSON post.
- **The strings are in seven locales** (`web/app/messages/`), and every screen's text is a message.

## 2. Requirements

R1. The route `/vault-cards` (`web/app/src/routes/vault-cards/+page.svelte`) joins `ROUTES`, and the
    startapp token `vaultcards` opens it.
R2. The screen reads `GET /api/vault-cards` and lists the pending cards oldest first, each with its
    front, its back, its note's file name, the duplicate line when the flag is `found`, and three
    actions: Approve, Edit and Reject. It shows the approved count. With nothing pending it says so.
R3. Approve and Reject post their decision once to SPEC-150's route and then show the list without
    that card. Edit opens two text fields holding the current sides; Save posts both sides
    unchanged, as typed. A refused edit (422) shows the reason's words and keeps both fields; a card
    already decided elsewhere (409) or gone (404) is dropped from the list with a status line; a
    failed request says so and changes nothing. While a request runs, its card's actions are
    disabled, so one tap posts once.
R4. A Scan action posts `POST /api/vault-cards/scan` once, shows the report's counts, refusals
    (up to 10, as the bot shows them), and a line when the walk was capped or duplicates were
    unchecked, then reads the list again.
R5. The screen says the bot's /vaultpack sends the package to import, and never says a card is in
    Anki (CHARTER 10: no claim the sync cannot honour). No package is downloaded in the Mini App
    (ADR-152).
R6. `api.ts` gains the vault-card calls: the list, the scan and the three decisions, each a
    `POST` with `content-type: application/json` on the session, and each answer typed by name
    (`not_pending`, `unknown`, `refused` with its reason). No other client is added.
R7. Every string is a message in all seven locales; the screen follows the accessibility pack:
    each action's label is in its accessible name, a decision's result is announced as a status
    message, every action is keyboard-operable, and nothing moves when the owner prefers reduced
    motion.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the screen lists each pending card oldest first with its note, its duplicate line and three actions, and the approved count | `the screen lists each pending card with its note, its flag and three actions` |
| A2 | Approve and Reject post their decision once, with the actions disabled while it runs, and the card leaves the list | `approve and reject post the decision once and the card leaves the list` |
| A3 | Save posts both sides as typed, and a refused edit shows the reason and keeps both fields | `edit posts both sides as typed and a refused edit keeps both fields` |
| A4 | a card decided elsewhere or gone leaves the list with a status line, and a failed request changes nothing | `a card decided elsewhere leaves the list with a status line` |
| A5 | Scan posts once and shows the report's counts, its refusals and its capped and unchecked lines | `scan posts once and shows the report` |
| A6 | the screen names the bot's /vaultpack and never says a card is in Anki | `the screen points to the bot for the package and claims nothing of Anki` |
| A7 | with nothing pending the screen says so and shows the approved count | `with nothing pending the screen says so` |
| A8 | a vault-card decision is a JSON post on the session, and 409, 404 and 422 are answered by name | `a vault-card decision is a JSON post answered by name` |
| A9 | the token `vaultcards` opens `/vault-cards`, and every token's destination is a route | `every destination is a screen of the route table` |
| A10 | the accessibility audit covers `/vault-cards` in both colour schemes | `the accessibility audit covers every route in both colour schemes` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "the screen lists each pending card with its note, its flag and three actions"
A2: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "approve and reject post the decision once and the card leaves the list"
A3: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "edit posts both sides as typed and a refused edit keeps both fields"
A4: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "a card decided elsewhere leaves the list with a status line"
A5: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "scan posts once and shows the report"
A6: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "the screen points to the bot for the package and claims nothing of Anki"
A7: pnpm exec vitest run web/app/src/lib/vault-cards/VaultCards.test.ts -t "with nothing pending the screen says so"
A8: pnpm exec vitest run web/app/src/lib/api.test.ts -t "a vault-card decision is a JSON post answered by name"
A9: pnpm exec vitest run web/app/src/lib/startapp.test.ts -t "every destination is a screen of the route table"
A10: pnpm exec vitest run web/app/src/lib/a11y-coverage.test.ts -t "the accessibility audit covers every route in both colour schemes"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The accessibility pack stays enforced; no check is
deferred or lifted for this delivery, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `web/app/src/routes/vault-cards/+page.svelte` and `web/app/src/lib/vault-cards/`: the screen passes the accessibility audit in both Telegram colour schemes, each action's label is in its name, a decision's result is a status message, every action is keyboard-operable, and nothing moves under reduced motion | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/routes/vault-cards/+page.svelte` | miniapp | added: the screen's route |
| `web/app/src/lib/vault-cards/VaultCards.svelte` | miniapp | added: the list, the actions, the edit fields and the scan |
| `web/app/src/lib/vault-cards/vaultCards.ts` | miniapp | added: the screen's types and the answers' reading |
| `web/app/src/lib/vault-cards/VaultCards.test.ts` | miniapp | added: A1 to A7 |
| `web/app/src/lib/api.ts` | miniapp | changed: the vault-card calls and a JSON post |
| `web/app/src/lib/api.test.ts` | miniapp | changed: A8 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /vault-cards joins `ROUTES` |
| `web/app/src/lib/startapp.ts` | miniapp | changed: the token `vaultcards` |
| `web/app/src/lib/startapp.test.ts` | miniapp | changed: A9 holds the new token |
| `web/app/tests/a11y.spec.ts` | miniapp | changed: the vault-card routes are mocked for the audit |
| `web/app/messages/en.json`, `es.json`, `fr.json`, `ja.json`, `ko.json`, `zh-Hans.json`, `zh-Hant.json` | miniapp | changed: the screen's strings, in each locale |
| `docs/specs/SPEC-152-the-mini-apps-vault-card-screen-lets-the-owner-approve-edit-or-reject-each-pending-card.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-152.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It adds no route, table or rule on the server; SPEC-150 does (#65).
- It downloads no package; the bot's /vaultpack sends it (#65).
- It adds no settings entry for the card tag or the deck (#57).
- It shows no approved card's history or a rejected card again: a rejected text stays rejected until
  the note's text changes (#65).

## 6. Risks

- **A double decision** from a double tap. Prevented by disabling the card's actions while its
  request runs, and on the server by SPEC-150's conditional update; detected by A2.
- **An edit changed on its way.** Prevented by posting both sides as typed; detected by A3.
- **A claim the product cannot honour**, that a card is in Anki. Prevented by R5's copy; detected by
  A6.
- **A route without a token, or a token without a route.** Detected by A9 and A10.

## 7. Parity goldens

None: the predecessor had no such screen (SPEC-150 §1).

## 8. Tables and the v9 import

None: this SPEC adds no table.

## 9. Mutation rows

None. The row runner (`scripts/mutation_rows.py`) takes a cargo or a unittest killer and never a
vitest one, and this screen holds no gate, cap, refusal or grant of its own: each is SPEC-150's, in
its rows `S15001` to `S15023`.
