---
status: "accepted"
decision-makers: "the owner, the DeckStreak architect"
---

# The passkey list's confirm step takes the focus on Keep inside an alert dialog named by its question, focus returns to a Remove or the list, and each Remove is named by its passkey's place and day

## Context and Problem Statement

#818 carries two accessibility follow-ups on the sign-in methods screen's passkey list, found in
the review of #782 (SPEC-385, ADR-399) and left outside its scope. Pressing Remove replaces the
focused button with the confirm step, so keyboard focus is lost, and the confirm question is in no
live region (WCAG success criteria 2.4.3 and 4.1.3). Every passkey row's button is named
"Remove", the same on every row (2.4.6). The issue is done when focus moves into the confirm step,
its question is announced, and each Remove carries a name that tells the rows apart, each pinned by
a test in the web client's suite; a person also checks that the focus order reads well.

Every `path:line` below was read at `36c4783a29c44d2ffafc987803ccb1742a2c2cfc`.

## Decision Drivers

- Focus is never lost: after every step of the flow it sits on an element the step leaves on the
  page (2.4.3).
- The question is heard once, when the step opens, by the mechanism that also moves the focus
  (4.1.3, 4.1.2).
- A name says which passkey a Remove removes, using only what the row already shows, and contains
  the button's visible word (2.4.6, 2.5.3).
- The step stays a pause: the gesture that opened it cannot also confirm it.
- The house's own confirm step and per-row names are reused, not reinvented.
- Every criterion is red first in the web unit suite, and every changed web file is proved by
  mutation.

## D1. The mechanism today, measured from the component and the unit suite

The list is `web/app/src/lib/settings/SignInMethods.svelte`, rendered by
`web/app/src/routes/sign-in-methods/+page.svelte:11` under the page's own heading (`:10`). One
state, `confirming` (`SignInMethods.svelte:18`), holds the passkey whose step is open. Each passkey
row shows the word "Passkey" (`:79`) and the day it was added (`:80`, formatted at `:39-42` with the
page's locale and `dateStyle: 'medium'`); the client's passkey carries only an id, a kind and that
day (`web/app/src/lib/passkeys.ts:34-36`). The `{#if confirming === method.id}` at `:81` swaps the
row's Remove (`:101-109`, whose click sets `confirming`) for the step: the question as a plain
paragraph (`:82`), then Remove (`:84-91`) and Keep (`:92-99`, whose click clears `confirming`).

- **Focus.** The component holds no focus call, no `tick`, no `bind:this`, no `tabindex` and no
  `aria-` attribute (0 matching lines). So the press unmounts the focused button and nothing places
  the focus; Keep (`:96`) and the end of a removal (`removeNow`, `:55-63`, which clears `confirming`
  at `:60`) unmount the focused button again. Where a browser then puts the focus is not measured
  by these reads; the unit suite's document reads its body, which the reds observe.
- **Announcement.** The question at `:82` is in no live region and labels nothing. The component's
  four live regions are the loading status (`:67`), the two alerts that replace the list (`:69`,
  `:71`) and the refusal alert (`:118`). What a screen reader says at the swap is not measured.
- **Names.** The row's Remove (`:108`) and the step's Remove (`:90`) both render `methods_remove`
  ("Remove", `web/app/messages/en.json:314`) with no other name, so every Remove on the list is
  named "Remove"; the unit suite queries it by that exact name 8 times
  (`web/app/src/routes/sign-in-methods.test.ts`).
- **Holds.** A pending mint or removal holds every button (`disabled={working}`, `:87`, `:95`,
  `:105`, `:125`), and the list is keyed by passkey id (`:74`), so a re-read keeps the rows that
  remain.

## D1. Chosen against

- A browser trace of the flow: no browser suite reaches the passkey rows, because the rendered audit (`web/app/tests/a11y.spec.ts:303-305`) opens `/sign-in-methods` without answering `GET /api/identities`.
- A screen-reader recording at the base: no repository test can hold one, so it is the person's check of #818, made on the cure.

## D1. Consequences

- Good: every step of the cure below starts from a cited line.
- Bad: what a browser and a screen reader do at the base is named as not measured.

## D2. Focus moves to Keep inside an alert dialog named by its question; it returns to a Remove, or to the list; each Remove is named by its passkey's place and day

- **Opening.** The step becomes one inline `role="alertdialog"` element labelled through
  `aria-labelledby` by its question's paragraph, which takes an id built from the passkey's id. The
  focus moves to Keep when the step mounts. This is the house's own confirm step:
  `web/app/src/lib/study/ReviewScreen.svelte:236-241` (an inline alert dialog labelled by its
  title), `:108` and `:298` (Keep bound to an element), `:175` (focus on Keep when it mounts).
  A screen reader names the dialog, its question, as the focus enters it; no live region is added.
- **Keep, and Escape.** Keep closes the step, sends nothing, and gives the focus back to that row's
  Remove. Escape, pressed anywhere on the page while a step is open, does what Keep does, through
  Keep itself, so a pending removal (whose Keep is held) is not put away by it. This is the house's
  Escape (`ReviewScreen.svelte:164-168`, `:199-200`).
- **Confirm.** When a removal ends and the list is read again, the focus moves to the Remove of the
  passkey that took the removed row's place; when the removed row was the last, to the Remove of
  the passkey before it; when no passkey is left, to the list itself, which is named by the
  screen's existing title key `methods_title` ("Sign-in methods") and focusable only by script
  (`tabindex="-1"`), as the house names its lists (`web/app/src/lib/records/RecordsScreen.svelte:16`).
- **Refusal.** A removal refused with a wording closes the step, as today (`:60`), shows the
  wording in the alert (`:118`), and gives the focus back to that row's Remove.
- **Names.** The row's Remove and the step's Remove carry one accessible name, through
  `aria-label`, from a new key `methods_remove_passkey`: English "Remove passkey {position}, added
  {date}". The place is the passkey's 1-based position among the passkey rows as shown; the day is
  the same text the row shows. The visible word stays "Remove", and every locale's name contains
  that locale's `methods_remove` text. This is the house's per-row name
  (`web/app/src/lib/study/MappingScreen.svelte:150`, `:160`, from `en.json:232-233`).

## D2. Chosen against

- Focus on the step's Remove: a second press of Enter or a second tap, the gesture that opened the step, would remove the passkey without the pause the step exists for.
- Focus on the question paragraph made focusable: a paragraph's role takes no name, so what each reader says depends on its own heuristics, and Tab would then reach Remove before Keep.
- A live region holding the question: a region inserted with its content is not reliably announced, and one announced while the focus also moves is heard twice or cut off.
- A persistent polite region beside the list, like the shell's (`web/app/src/routes/+layout.svelte:51`): it speaks without moving the focus, so the focus would still be lost, which 2.4.3 refuses.
- A labelled `role="group"` in place of the alert dialog: a group's name on entry is read less consistently than a dialog's, and the product's other confirm step is an alert dialog.
- No Escape: the product's other confirm step keeps on Escape, and a dialog that ignores Escape breaks the convention its role announces.
- After a removal, focus on the page's heading: the heading belongs to the route (`+page.svelte:10`), not the component, and the settings screen will embed the component under its own heading (#57).
- After a removal, focus always on the list: a learner removing several passkeys would walk back down the list after each one.
- Names by creation day alone, the issue's example: two passkeys added the same day, as on a first day that links a phone and a computer, would still share one name.
- Names by place alone: they drop the day the row shows, the one detail a learner can match to a device.
- Names by the passkey's own name: the client's passkey carries none (`passkeys.ts:34-36`).
- Names composed from the row's own text through `aria-labelledby`, with no new key: "Remove", "Passkey" and the row's added-day line, read in a row, keep no place, so same-day rows collide, and English word order is forced on every locale.
- The place shown in the row ("Passkey 2"): it changes what every reader sees to fix what assistive technology reads, and changes an existing key's parameters in every catalog.

## D2. Consequences

- Good: no step leaves the focus on nothing, the question is heard as the focus enters its
  dialog, and every Remove says which passkey it removes, also when two share a day.
- Good: both confirm steps of the product behave alike, Keep first and Escape keeping.
- Bad: a name's place changes when a passkey above it is removed; the day does not.
- Bad: one new key enters all seven catalogs, which other open changes also edit.

## D3. The cure is production code and one key, held by tests in the web unit suite

Production: `SignInMethods.svelte` (the dialog, the three focus moves, Escape, the names, the
list's name) and the seven catalogs under `web/app/messages/` (one key each, inserted after
`methods_remove_keep`). Tests: `web/app/src/routes/sign-in-methods.test.ts` (the criteria's new
tests, and the eight `name: 'Remove'` queries restated to the new names in the commit that changes
them) and `web/app/src/lib/passkey-messages.test.ts` (the new key in every locale, and its table
of SPEC-385's keys grown from 25 to 26). No rows table runs a vitest killer, so the band
`S46300-S46399` is claimed and holds no file; StrykerJS over the changed component at a break of
100 is the mutation proof.

## D3. Chosen against

- A test-only change: both defects are in the component, and a test can only pin what the component does.
- Existing keys only: no existing key carries a passkey's place, and borrowing another screen's numbered key ties the name to a translation made for that screen.
- A shared focus helper module: one component uses it, and the house's pattern lives in its component.
- Hand rows in the band: no rows table runs a vitest killer, so a row could not be killed by the tests that hold the behaviour (ADR-399 D4).

## D3. Consequences

- Good: every changed line is held by a test in the tree and by a mutant.
- Bad: the six non-English names are written by the build and await a native speaker's review.

## D4. One delivery, SPEC-385 amended by a new last section, the passkey schematic by a new section

SPEC-463 and this ADR decide one pull request, with the band, the red-first record and the
changelog fragment its own. SPEC-385 gains an insert-only new last section that adds the key to its
wording and says its confirm step now takes the focus; nothing above it changes. The passkey
schematic (`docs/schematics/passkey-sign-in-on-the-web.md`, whose section 7 owns the component)
gains a new last section. ADR-399 is not edited: its D3, "each passkey with Remove behind a
confirm step", stays true.

## D4. Chosen against

- Two deliveries, focus and announcement first, then names: both edit one component, one test file and the same queries, the second would rebase on the first, and each would need its own CI run.
- An edit inside SPEC-385's wording table: an accepted SPEC changes by insertion, and a new last section keeps every line above it byte for byte.
- An amendment of ADR-399: nothing it decides changes.
- A new schematic: the passkey schematic's section 7 already owns the component, so a second page would split its one owner.

## D4. Consequences

- Good: #818 closes with one pull request.
- Bad: SPEC-385's wording table reads 25 keys on its own page; its new last section is the pointer.

## D5. Eight criteria, each red by assertion at the base in vitest; one push; formal not applicable by surface

Each criterion of SPEC-463 is one vitest test, red at the base for the criterion's reason and
green with the cure. A focus test finds its buttons by their place in the row, never by the new
names, so it is red for the focus and not for a name; the names test holds every name to an
English golden over a fixture with two passkeys of one day, and the catalog test reads every
locale. The eight restated queries land with the cure, so the red commit fails exactly the eight
new tests. Every red is read locally, so the delivery is pushed once. The formal decision is NOT
APPLICABLE by surface: the change is TypeScript, Svelte and JSON under `web/app`, which no
`@phx covers` line names and the formal registry cannot cover; the step's state is one page's,
changed by one event at a time, and a pending removal already holds every control the cure adds a
path through.

## D5. Chosen against

- A browser test of the flow: its red is read only in CI, a second push, while the unit suite already places the focus and computes names and roles the way the criteria read them.
- Restating the eight queries in the red commit: they would be red for the names too, reds that are no criterion of their own.
- A TLA+ model: no second actor shares the step's state; a pending removal holds Keep, Remove and every other button (`:87`, `:95`, `:105`, `:125`).
- A Lean proof of the focus target: it decides no verdict, ledger, money or release, and its test's fixture-defined goldens (the next row, the previous row, the list) with StrykerJS close it.

## D5. Consequences

- Good: the red commit shows the defect and the cure commit shows it gone, with no CI read between them.
- Bad: a real browser's focus and a real screen reader's speech are met only in the person's check.

## D6. The person's check, the native clients and the rendered audit of the open step stay out

The person's walk of the focus order is a manual step in the pull request's test plan, citing #818,
never a criterion. The native clients' sign-in stays with #58. A browser audit of the rows and the
open step, and the focus when a list re-read after a removal fails, are each a later change.

## D6. Chosen against

- The person's check as a criterion: no repository test decides how an order reads to a person.
- Moving the focus when the re-read after a removal fails: the list is then replaced by an alert that announces itself, and placing the focus there is a change to the list's failure states, which #818 does not name.

## D6. Consequences

- Good: the delivery stays inside #818.
- Bad: a removal whose re-read fails still leaves the focus where the browser puts it.

## Consequences

- A keyboard or screen-reader learner who removes a passkey keeps the focus at every step, hears
  the question as the step opens, and knows which passkey each Remove removes.
- The list's look does not change.
- The six non-English names await a native review.

## What would make this wrong

- A server that lists a passkey's own name or device: the name could then carry it, if the row shows it.
- The settings screen embedding the list twice on one page (#57): the question's id, built from the passkey's id, would then repeat.
- A screen reader that does not name an alert dialog as the focus enters it: the question would then need a description on Keep as well.
