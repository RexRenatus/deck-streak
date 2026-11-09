---
status: "accepted"
decision-makers: "the DeckStreak architect"
---

# A failed re-read after a passkey removal moves the focus to the alert in the list's place, and a failed first read moves none

## Context and Problem Statement

#945, a follow-up to #818, asks for a decided place for the focus when the sign-in methods list's
re-read after a removal fails, read by a test. SPEC-463 and ADR-480 placed the focus at every
other step of a removal: on Keep as the confirm step opens, back on a Remove after Keep, Escape or
a refusal, and after a removal on the next Remove, the previous one, or the list. They left this
one end as later work (SPEC-463 section 5, ADR-480 D6).

Every `path:line` below is in `web/app/src/lib/settings/SignInMethods.svelte`, read at `36efa139`,
unless it names another file. After a removal, `removeNow` reads the list again (`:120`), awaits
the render (`:121`) and places the focus only when that read answers `ok` (`:122`). A `reopen` or
`unavailable` answer replaces the list with an alert paragraph (`:131-134`) that nothing focuses,
so the focused button leaves the page with the list and the focus falls to the document's body
(WCAG success criterion 2.4.3). SPEC-463's mutation-coverage test at
`web/app/src/routes/sign-in-methods.test.ts:523-545` pins the body as the focus there.

## Decision Drivers

- The focus never rests on nothing: after every end of a removal it sits on an element the page
  still shows (2.4.3).
- What the learner hears is what the page shows: the alert already says why, in the wording the
  first read shows.
- The list's own conventions are kept: an element that is not a control takes the focus from
  script only, as the list does when no passkey is left (SPEC-463 R4, ADR-480 D2, `:138`).
- The screen's first read stays as it is: nothing the learner had focused left the page there.
- The criterion is red first in the web unit suite, and the changed component is proved by
  mutation.

## D1. After a removal whose re-read fails, the focus moves to the alert in the list's place

After a removal whose re-read answers `reopen` or `unavailable`, the focus moves to the alert that
takes the list's place. The alert keeps its role and its wording and takes the focus only from
script (`tabindex="-1"`), as the list itself does when no passkey is left (SPEC-463 R4, ADR-480
D2, `:138`). The component binds the alert (`bind:this`) and focuses it in `removeNow` where it
now places none, after the render it already awaits. A keyboard learner's next Tab then starts
where the list stood.

## D1. Chosen against

- The route's heading: it belongs to the route (`web/app/src/routes/sign-in-methods/+page.svelte:10`), not the component, and the settings screen will embed the component under its own heading (#57); ADR-480 D2 chose against it for the same reason.
- Leaving the focus where the browser puts it: the focused button leaves the page, so the focus rests on nothing, which 2.4.3 refuses.
- Dropping the alert role while the alert holds the focus: the list's two failures would then differ between its first read and its re-read, and the role is how the first read's alert is announced.
- A new named region around the screen's states: it adds a region to every state of the screen for one failure.
- The "Link a passkey" button: it shows only inside Telegram, and pressing it is another act.
- A new live region or a new message: the alert already says why, and a region announced while the focus also moves is heard twice or cut off (ADR-480 D2).

## D1. Consequences

- Good: no end of a removal leaves the focus on nothing, and the alert is read where the focus
  lands.
- Bad: some screen reader and browser pairs may say the message twice, the alert announcing itself
  and then the focus landing on it.

## D2. The first read moves no focus

When the screen's first read of the list answers `reopen` or `unavailable`, the alert shows as it
does today and the focus does not move: nothing the learner had focused left the page, and an
alert announces itself without taking the focus.

## D2. Chosen against

- Focusing the alert on the first read too: the learner's focus is still on the page, and moving it unasked as the screen opens would take it from wherever the learner left it.

## D2. Consequences

- Good: a screen that opens on a failure behaves as it does today.
- Bad: the two failures differ in where the focus goes, by design: one follows a removal, the other
  follows nothing.

## D3. One criterion red by assertion, one guard, and StrykerJS before the push

One criterion is red by assertion at the base in vitest (A1) and one is a `not red` guard (A2).
SPEC-463's mutation-coverage test that read the focus on the body after this failure states the
behaviour this delivery changes, so the red commit removes it and A1 takes its place, keeping its
fixture and its no-rejection check. StrykerJS over the component runs before the push, at the
repository's break of 100. One push.

## D3. Chosen against

- Keeping SPEC-463's test beside A1: it asserts the focus on the body, the behaviour this delivery cures, so it would fail at the cure.
- A browser test of the failed re-read: it would need a stand-in server for the list's two reads and its delete in the browser suite, for one assertion the unit suite already makes; a browser audit of the list is #946.

## D3. Consequences

- Good: the red is the focus itself, and the removed test's other checks survive in A1.
- Bad: a screen reader's speech is still met only in the person's check (#945).

## D4. Formal not required by surface

Formal is not required by surface (SPEC-642 section 7): TypeScript and Svelte under `web/app`,
which no `@phx covers` line names and the formal registry cannot cover. The change adds one focus
target to one page's existing removal path, one event at a time.

## D4. Chosen against

- A state model of the list's focus: the change adds one target to a path that already holds every button while a removal is pending, and the unit tests and StrykerJS decide it.

## D4. Consequences

- Good: the delivery stays one component and one test file.
- Bad: none beyond SPEC-642's risks.

## Consequences

- A keyboard or screen-reader learner whose removal ends in a failed re-read finds the focus on
  the message that says why, where the list stood.
- The list's look does not change, except a focus ring where the focus now lands.
- Some screen reader and browser pairs may say the message twice.

## What would make this wrong

- A person's check that hears the message twice and judges it a fault: the cure keeps the focus target and moves the alert role to an element inside it, so that the focus move and the announcement are one.
- A settings screen that embeds the list beside another alert (#57): the focus would then land on one of two alerts, and the embedding re-reads which one says why.
