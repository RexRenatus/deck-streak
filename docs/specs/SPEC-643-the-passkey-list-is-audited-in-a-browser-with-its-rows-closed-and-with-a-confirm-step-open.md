# SPEC-643: the passkey list is audited in a browser with its rows closed and with a confirm step open

- **Issue:** #946 (a follow-up of #818: a browser accessibility audit of the sign-in methods screen
  with a passkey list shown, once with every row closed and once with a confirm step open).
  **Context(s):** the web client (`web/app/tests`, where the browser audits live, and one unit test
  under `web/app/src/lib/settings`). Nothing outside `web/app` and `docs` changes, except the
  changelog fragment.
- **Decided by:** ADR-666 (the audit's own spec file, the list it answers, the press that opens the
  step, axe's mode and tags, the shape test, and formal).
- **Amends:** none. SPEC-463 is unchanged: its section 5 already names this audit as #946's work.
- **Mutation band:** `S64300-S64399`, claimed, holding no rows file (section 9).
- **Status:** one pull request into dev; every path it writes is a test or a document, plus its
  changelog fragment.

## 1. The problem, measured

Every figure below was read at `36efa139f5d9dccdb5a4716267fec0ec2ca536fc` by the command beside it.

- The app's rendered audit visits `/sign-in-methods` in both of Telegram's palettes but answers no
  request for the list on it: `git show 36efa139:web/app/tests/a11y.spec.ts | grep -c
  'api/identities'` prints 0. The screen's list is read by `api.identities()`
  (`web/app/src/lib/settings/SignInMethods.svelte:46`), which reads `GET /api/identities`
  (`web/app/src/lib/api.ts:261`), so that audit reads the screen with no passkey row on it.
- The app's audit takes exactly one visit, the one to each route of the route table:
  `web/app/src/lib/a11y-coverage.test.ts:51` holds `page.goto(` to one match, and `git show
  36efa139:web/app/tests/a11y.spec.ts | grep -c -F 'page.goto('` prints 1. It has no place for a
  press that opens a step on one screen, nor for a list that only one screen reads.
- The rows and the open confirm step (SPEC-463, ADR-480) are read only by the component tests in
  `web/app/src/routes/sign-in-methods.test.ts`, whose docblock at line 2 reads
  `@vitest-environment jsdom`: `git show 36efa139:web/app/src/routes/sign-in-methods.test.ts | grep
  -c identities` prints 35 and `... | grep -c alertdialog` prints 1. That document computes no
  colours and no layout, so the contrast of a row, its Remove and the open step's buttons, and the
  size of each target, are not measured by any test.
- The voice picker's audit already shows the pattern this audit follows: a spec of its own beside
  the app's audit, which answers the page's requests in place and runs axe-core with the app's tags
  (`web/app/tests/voice-picker-a11y.spec.ts:129-158`), and whose shape a unit test holds
  (`web/app/src/lib/study/voice.test.ts:608-642`).

## 2. Requirements

- R1. A browser audit of `/sign-in-methods` answers `GET /api/identities` in the page with Telegram
  and two passkeys added at two instants, so the list shows three rows and two Removes, each named
  for its place.
- R2. With every row closed (no confirm step open), axe-core finds no WCAG 2.2 A or AA violation it
  can find, in each of Telegram's light and dark palettes, with the app's audit's tags.
- R3. With the first passkey's confirm step opened by a press of its Remove, the alert dialog named
  by its question showing and Keep holding the focus, axe-core finds no violation, in both palettes.
- R4. The audit answers the rest of the page as the voice picker's audit does (Telegram's script
  stand-in, the launch, the session, the study day and the wallet), and it makes one visit per
  test, to `/sign-in-methods`. The root layout's feed read (`/api/notifications/feed`) is answered
  by no route: it is left to the local preview server, as the voice picker's audit and the app's
  audit leave it, and no route is added for it.
- R5. A unit test holds the audit's shape for each state (the palettes, the list's answer, the one
  visit, the state's steps in order, the app's tags), and refuses a planted audit that lacks that
  state's step, since the test probe resolves no Playwright command (as `a11y-coverage.test.ts`
  and `voice.test.ts` hold theirs).
- R6. No product code changes; the app's audit, its route table and its coverage test are
  unchanged.

## 3. Acceptance criteria of SPEC-643

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | the audit answers the list in the page, makes its one visit, shows three rows and two named Removes with every row closed and no alert dialog, then runs axe with the app's tags and allows no violation, in both palettes; a planted audit without the closed-row step is refused (R1, R2, R4, R5) | the base has no such audit: the shape test's existence assertion fails | `web/app/src/lib/settings/sign-in-methods-a11y.test.ts` "the passkey list audit runs axe over the list with every row closed in both colour schemes" (added), holding `web/app/tests/sign-in-methods-a11y.spec.ts` (added) |
| A2 | the audit answers the list in the page, makes its one visit, presses the first passkey's Remove, sees the alert dialog named by its question with Keep focused, then runs axe with the app's tags and allows no violation, in both palettes; a planted audit without the press is refused (R1, R3, R4, R5) | the base has no such audit: the shape test's existence assertion fails | `web/app/src/lib/settings/sign-in-methods-a11y.test.ts` "the passkey list audit runs axe over the list with a confirm step open in both colour schemes" (added), holding `web/app/tests/sign-in-methods-a11y.spec.ts` (added) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/settings/sign-in-methods-a11y.test.ts -t "the passkey list audit runs axe over the list with every row closed in both colour schemes"
A2: pnpm exec vitest run web/app/src/lib/settings/sign-in-methods-a11y.test.ts -t "the passkey list audit runs axe over the list with a confirm step open in both colour schemes"
```

Each line is a Vitest test in the `node` environment that reads the browser audit's text: the
browser audit itself is a Playwright spec, which the test probe does not resolve, so its shape is
held here, as `a11y-coverage.test.ts` holds the app's audit and `voice.test.ts` the voice picker's.
Each test first asserts that the audit exists, then finds each of its state's steps inside that
state's own test, in order, and the shared marks across the file; each prints `examined <n> marks`
over the marks it judged and refuses zero. Each also runs its checks over a planted audit that
lacks that state's own step (the closed-row count for A1, the press for A2) and over one whose list
is answered at another path, and requires both to be refused. The browser legs themselves run in
CI (section 7).

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-643-the-passkey-list-is-audited-in-a-browser-with-its-rows-closed-and-with-a-confirm-step-open.md` | docs | added |
| `docs/decisions/ADR-666-the-passkey-list-has-its-own-browser-audit-that-answers-the-list-in-the-page-and-opens-a-confirm-step-by-a-press.md` | docs | added |
| `docs/schematics/sign-in-methods-browser-audit.md` | docs | added |
| `docs/red-first/SPEC-643.md` | docs | added |
| `changelog.d/passkey-list-browser-audit-643.md` | docs | added |
| `web/app/src/lib/settings/sign-in-methods-a11y.test.ts` | miniapp | added |
| `web/app/tests/sign-in-methods-a11y.spec.ts` | miniapp | added |

No other file changes.

## 5. What this does NOT cover

- A removal in flight (every button held) and a refused removal's wording in its alert: #946 names
  the two states this audit reads, the rows closed and a confirm step open.
- The focus after a list re-read that fails after a removal: #945.
- A real screen reader's speech as the focus enters the step, which only a person's check meets:
  the person's check of #818.
- The native clients' sign-in and any list of their methods: #58.

## 6. Risks

- A violation axe reports at the pull request head is a product finding on the screen. It is
  handled by its own change to the screen, never by loosening the audit (no tag removed, no rule
  disabled, no element excluded); the browser legs in CI (section 7) detect it.
- The day each passkey row shows is formatted with the page's locale and time zone. The audit
  matches each Remove by its place and an `added ` prefix, never by the formatted day, so the
  browser's time zone and locale data cannot move the names it matches.
- The list's answer is the shape `parseMethods` reads (`web/app/src/lib/passkeys.ts:174-187`). If
  that shape changes, the parser refuses the answer and the screen shows no rows, so the audit's
  three-row count fails before axe runs, and the drift is reported, never passed over.
- The root layout's feed read is left to the local preview server, as both existing audits leave
  it; the audit waits for the network to settle before it counts the rows.

## 7. What only CI proves

| # | what | where |
|---|---|---|
| C1 | the type check of the browser audit and its shape test | the `web` job's svelte-check |
| C2 | the browser audit's two tests, the rows closed and a confirm step open, in both palettes | the end-to-end suite in the `web` job |

## 8. Formal

None. The delivery adds a browser test, a unit test and documents; no state, protocol or
concurrency is added, and no file a formal entry covers changes.

## 9. Mutation rows

None. The band `S64300-S64399` is claimed and holds no file: the shape test and the browser audit
are test files, which the web mutation plan classes `other`.

## 10. Red first

The shape test is committed before the audit and is red for both criteria by its existence
assertion, an assertion failure on the absent audit. The audit's commit greens both with the shape
test unchanged. The browser legs are read in CI. The record is `docs/red-first/SPEC-643.md`.
