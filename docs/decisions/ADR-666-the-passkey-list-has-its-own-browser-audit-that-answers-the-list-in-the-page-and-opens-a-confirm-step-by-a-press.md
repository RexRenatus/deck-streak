---
status: "accepted"
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-666: the passkey list has its own browser audit, which answers the list in the page and opens a confirm step by a press

Decides SPEC-643.

## Context and Problem Statement

#946, a follow-up of #818, asks for a browser accessibility audit of the sign-in methods screen
with a passkey list shown, once with every row closed and once with a confirm step open. The app's
rendered audit (`web/app/tests/a11y.spec.ts`) visits `/sign-in-methods` in both of Telegram's
palettes but answers no `GET /api/identities`, so it reads the screen with no passkey row on it.
The rows and the open confirm step that SPEC-463 and ADR-480 shaped are read only by the component
tests, whose document computes no colours and no layout. SPEC-463 names this audit as #946's work,
and ADR-480 names it as not measured. How should the rows and the open step be audited in a
browser, without changing the screen or the app's audit?

## Decision Drivers

- The audit measures what only a rendered page shows: colour pairs in each palette, target sizes,
  and the roles and names of the open step as a browser computes them.
- The app's audit keeps its one visit per route and its route table (`a11y-coverage.test.ts`
  holds both), and every other route's setup stays as it is.
- The open step is reached the way a person reaches it, so the audit reads the screen a person
  sees.
- The audit's shape is held by a test the test probe resolves, red before the audit exists.
- No product code changes.

## Considered Options

### D1. Where the audit lives

- Chosen: a spec file of its own, `web/app/tests/sign-in-methods-a11y.spec.ts`, beside the voice
  picker's audit, because it can answer the list and press a Remove for this one screen alone.
- Rejected: answering the list in `a11y.spec.ts`'s shared setup, because its coverage test holds
  it to one visit and no list of its own, so the open step would need a press inside the
  every-route loop, and the answer would change every route's setup.
- Rejected: axe over the component in jsdom, because that document computes no colours and no
  layout, so contrast and target size would go unmeasured.

### D2. The list the audit answers

- Chosen: Telegram and two passkeys added at two instants, answered at `**/api/identities` in the
  page, because two passkeys put two Removes with distinct names on the screen.
- Rejected: one passkey, because a single Remove cannot show that each Remove is named for its own
  place.
- Rejected: leaving the list to the local preview server, because it serves the built app and no
  list, so the screen would show no row.

### D3. How the open step is reached

- Chosen: a press of the first passkey's Remove, as a person reaches it; axe runs once the alert
  dialog named by its question shows and Keep holds the focus.
- Rejected: rendering the step open by a state planted in the page, because the screen would then
  not be the one a press shows, and the focus move SPEC-463 decides would go unread.

### D4. Axe's mode and tags

- Chosen: axe in its standard mode with the app's audit's tags, because the screen holds no
  frame.
- Rejected: the legacy mode the review's sandboxed card frame needs, because it exists for a frame
  that cannot run axe's script, and this screen has none.

### D5. How the audit's shape is held

- Chosen: a unit test, `web/app/src/lib/settings/sign-in-methods-a11y.test.ts`, as
  `a11y-coverage.test.ts` and `voice.test.ts` hold theirs, with a planted audit lacking each
  state's step refused; it is committed first and is red while the audit is absent.
- Rejected: a criterion on the Playwright spec itself, because the test probe resolves no
  Playwright command, so such a criterion would decide nothing a repository test runs.

### D6. Formal

- Chosen: not required, because the delivery is tests and documents only.
- Rejected: a formal entry for the audit, because it adds no state, protocol or concurrency, and no
  file a formal entry covers changes.

## Decision Outcome

Chosen: D1 to D6 as above. The sign-in methods screen gets its own browser audit beside the voice
picker's. Each test answers Telegram's script, the launch, the session, the study day, the wallet
and the list in the page, makes its one visit to `/sign-in-methods`, and waits for the three rows
and the two named Removes. The first test then sees no alert dialog and runs axe; the second
presses the first passkey's Remove, sees the alert dialog named by its question with Keep focused,
and runs axe. Both run in each of Telegram's light and dark palettes with the app's tags, and allow
no violation. A unit test holds the shape of both.

## Consequences

- Good: the rows, their Removes and the open step are measured for contrast, target size, roles and
  names in a browser, in both palettes.
- Good: the app's audit, its route table, its coverage test and the screen are unchanged.
- Bad: the audit repeats the voice picker's stand-in for Telegram's script and its answers for the
  session, the study day and the wallet, so a change to that setup is made in three specs.
- Bad: the root layout's feed read is answered by no route; it is left to the local preview
  server, as both existing audits leave it.
- Bad: the browser legs run only in CI, so a violation they find is seen at the pull request head.

## Confirmation

The shape test's two criteria, SPEC-643 A1 and A2, are red before the audit exists and green with
it, with the shape test unchanged; the record is `docs/red-first/SPEC-643.md`. The browser audit's
two tests run in both palettes in the end-to-end suite of CI's `web` job.

## More Information

- SPEC-463 and ADR-480 shaped the list, its names and its confirm step; they are unchanged.
- The schematic is `docs/schematics/sign-in-methods-browser-audit.md`.
- A violation axe reports on the screen is a product finding, cured by its own change to the
  screen, never by loosening this audit.
