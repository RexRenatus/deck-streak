# SPEC-463: the passkey list moves the focus into its confirm step, says its question, and names each Remove for its passkey

- **Issue:** #818 (two accessibility follow-ups on the web client's passkey list, found in the
  review of #782 and left outside its scope). **Context(s):** the web client (`web/app`: the sign-in
  methods screen's list, its wording in every locale, their unit tests). Nothing outside `web/app`
  and `docs` changes, except the changelog fragment.
- **Decided by:** ADR-480 (this SPEC's: the mechanism, the cure, production and tests, the shape,
  the tests and rows, what stays out), ADR-399 (the screen and its confirm step, D3).
- **Amends:** SPEC-385, by an insert-only new last section (§8): its wording gains one key and its
  confirm step takes the focus.
- **Mutation band:** `S46300-S46399`, claimed, holding no rows file (§7).
- **Status:** one pull request into dev; every path it writes is under `web/app` or `docs`, plus
  its changelog fragment.

## 1. The problem, measured

Every figure below was read at `36c4783a29c44d2ffafc987803ccb1742a2c2cfc` by the command beside it.

- The list is `web/app/src/lib/settings/SignInMethods.svelte` (130 lines), rendered by
  `web/app/src/routes/sign-in-methods/+page.svelte:11` under the route's heading (`:10`). The
  `{#if confirming === method.id}` at `SignInMethods.svelte:81` swaps a row's Remove (`:101-109`)
  for the confirm step: the question as a plain paragraph (`:82`), then Remove (`:84-91`) and Keep
  (`:92-99`).
- Focus is lost at every step: `git show 36c4783a:web/app/src/lib/settings/SignInMethods.svelte |
  grep -c -E 'focus\(|tick|bind:this|tabindex|aria-'` prints 0. The press unmounts the focused
  Remove and nothing places the focus; Keep (`:96`) and the end of a removal (`:60`) unmount the
  focused button again. Where a browser then puts the focus is not measured by these reads; the
  unit suite's document reads its body.
- The question is announced by nothing: `... | grep -c -E 'role="(status|alert)"'` prints 4, and
  the four are the loading status (`:67`), the two alerts that replace the list (`:69`, `:71`) and
  the refusal alert (`:118`). The question at `:82` is in no live region and labels nothing.
- Every Remove has one name: `... | grep -n -F '{m.methods_remove()}'` prints lines 90 (the step's
  Remove) and 108 (the row's Remove), and neither carries another name, so each is named "Remove"
  (`web/app/messages/en.json:314`). `git show 36c4783a:web/app/src/routes/sign-in-methods.test.ts |
  grep -c -F "name: 'Remove' }"` prints 8.
- A row shows the word "Passkey" (`:79`) and the day it was added (`:80`, formatted at `:39-42`
  with the page's locale and `dateStyle: 'medium'`), and the client's passkey carries only an id, a
  kind and that day (`web/app/src/lib/passkeys.ts:34-36`): two passkeys added on one day show the
  same row.
- No browser suite reaches the rows: `git show 36c4783a:web/app/tests/a11y.spec.ts | grep -c
  identities` prints 0, so the rendered audit's visit to `/sign-in-methods`
  (`web/app/src/lib/routes.ts:31`) answers no passkey list of its own; what the screen shows there
  is not measured by these reads.

## 2. Requirements

- R1. Pressing a passkey row's Remove opens its confirm step as one inline element with
  `role="alertdialog"`, labelled through `aria-labelledby` by the question's paragraph, whose id is
  built from the passkey's id. The question, the step's two buttons and their visible words are
  those of today.
- R2. When the step opens, the focus moves to its Keep.
- R3. Keep puts the question away, sends nothing, and gives the focus to that row's Remove. Escape,
  pressed anywhere on the page while a step is open, does what Keep does, by pressing that Keep, so
  while the step's removal is pending (Keep held) Escape changes nothing.
- R4. When a removal ends and the list is read again, the focus moves to the Remove of the passkey
  that took the removed row's place; when the removed row was the last, to the Remove of the
  passkey before it; when no passkey remains, to the list, which is named by `methods_title` and
  takes the focus only from script (`tabindex="-1"`).
- R5. A removal refused with a wording closes the step, shows the wording in the alert, and gives
  the focus to that row's Remove.
- R6. No live region is added for the question; the dialog's name is how it is announced.
- R7. Each passkey row's Remove, and the Remove of its open step, carries the accessible name
  `methods_remove_passkey`, with `position` the passkey's 1-based place among the passkey rows as
  shown and `date` the text the row shows for its day. Its visible word stays `methods_remove`. No
  name carries the passkey's id or anything else the row does not show.
- R8. Every locale carries `methods_remove_passkey`, non-empty, holding `{position}` and `{date}`
  and containing that locale's own `methods_remove` text. English is "Remove passkey {position},
  added {date}". SPEC-385 §8 adds the key to its wording, and SPEC-385's A19 test carries it.
- R9. Nothing a sighted learner sees changes, except a focus ring where the focus now lands.

## 3. Acceptance criteria of SPEC-463

| # | criterion | decided by |
|---|---|---|
| A1 | pressing a passkey's Remove moves the focus to that step's Keep (R2) | `sign-in-methods.test.ts`; red: the base leaves the focus on the document's body |
| A2 | the confirm step is an alert dialog whose name is its question, and the focus is inside it (R1, R6) | `sign-in-methods.test.ts`; red: the base renders no alert dialog |
| A3 | Keep puts the question away, sends nothing, and gives the focus to that row's Remove (R3) | `sign-in-methods.test.ts`; red: the base leaves the focus on the body |
| A4 | Escape while a step is open does what Keep does, and while its removal is pending changes nothing (R3) | `sign-in-methods.test.ts`; red: the base ignores Escape and keeps the question |
| A5 | after a removal the focus moves to the next passkey's Remove, else the previous one's, else the list named "Sign-in methods" (R4) | `sign-in-methods.test.ts`; red: the base leaves the focus on the body |
| A6 | a refused removal shows its wording and gives the focus to that row's Remove (R5) | `sign-in-methods.test.ts`; red: the base leaves the focus on the body |
| A7 | every Remove on the list, the open step's included, is named for its passkey's place and day, and no two names are alike when two passkeys share a day (R7) | `sign-in-methods.test.ts`; red: the base names every Remove "Remove" |
| A8 | every locale carries `methods_remove_passkey` with `{position}`, `{date}` and its own Remove word, and English reads as R8 states (R8) | `passkey-messages.test.ts`; red: no locale has the key |

```acceptance
A1: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "pressing remove moves the focus to keep in the confirm step"
A2: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "the confirm step is an alert dialog named by its question"
A3: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "keep puts the question away and gives the focus back to the row's remove"
A4: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "escape keeps the passkey as keep does, and changes nothing while the removal is pending"
A5: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "after a removal the focus moves to the next remove, the previous one, or the list"
A6: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "a refused removal gives the focus back to the row's remove"
A7: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "each remove names its passkey by place and day, and no two names are alike"
A8: pnpm exec vitest run web/app/src/lib/passkey-messages.test.ts -t "every locale names a passkey's remove with its place, its day and its own remove word"
```

A1 to A6 find each button by its place in its row, never by a name, so each is red for the focus
and not for the names A7 decides. Each asserts the element that holds the focus by identity, never
only that the body does not. A5 enumerates its three removals (the middle row, the last row, the
only row) and prints `examined 3 of 3 removals`; A7 prints `examined N of N` over the Remove
buttons it read and refuses unless every passkey row's reached its assertion; A8 prints
`examined 7 of 7 locales`. A7's fixture holds two passkeys added at one instant, and its goldens
are literal English names whose day is built from the fixture as the existing day test builds it
(`sign-in-methods.test.ts:218-219`), never by the component's formatter. A4 asserts the open step's
Escape first and the pending step's second, so its red is the ignored Escape.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-463-the-passkey-list-moves-focus-into-its-confirm-step-says-its-question-and-names-each-remove-for-its-passkey.md` | document | new |
| `docs/decisions/ADR-480-the-passkey-confirm-step-takes-focus-on-keep-inside-an-alert-dialog-named-by-its-question-and-each-remove-is-named-by-place-and-day.md` | document | new |
| `docs/specs/SPEC-385-the-web-client-registers-and-signs-in-with-a-passkey-pinned-to-the-owner.md` | document | a new last section (§8) only; nothing above it changes |
| `docs/schematics/passkey-sign-in-on-the-web.md` | document | a new last section (§12) only; nothing above it changes |
| `docs/red-first/SPEC-463.md` | document | new |
| `changelog.d/passkey-remove-focus-463.md` | document | new |
| `web/app/src/lib/settings/SignInMethods.svelte` | web client | changed |
| `web/app/src/routes/sign-in-methods.test.ts` | web client | changed: eight new tests, and its eight `name: 'Remove'` queries restated to the new names |
| `web/app/src/lib/passkey-messages.test.ts` | web client | changed: one new test, and its table of SPEC-385's keys grown by one |
| `web/app/messages/en.json` | web client | one key inserted |
| `web/app/messages/es.json` | web client | one key inserted |
| `web/app/messages/fr.json` | web client | one key inserted |
| `web/app/messages/ja.json` | web client | one key inserted |
| `web/app/messages/ko.json` | web client | one key inserted |
| `web/app/messages/zh-Hans.json` | web client | one key inserted |
| `web/app/messages/zh-Hant.json` | web client | one key inserted |
| `scripts/mutation-equivalent.d/miniapp.json` | web client | only if a StrykerJS survivor is proved equivalent: one record with its reason |

No `.rs`, `.swift`, workflow, edge or content-security file changes, and `web/app/package.json`
and the lockfile do not change.

## 5. What this does NOT cover

- The person's check that the focus order reads well, by keyboard and by a screen reader on a phone and on a computer: the pull request's test plan names it as a manual step, #818.
- The native clients' sign-in and any list of their methods: #58.
- The settings screen that embeds this list: #57.
- A native speaker's review of the six new non-English names: the build writes them, and their review is tracked with SPEC-385's, #627.
- The focus when the list's re-read after a removal fails, which leaves it where the browser puts it: #945.
- A browser audit of the passkey list's rows and its open confirm step: #946.

## 6. Risks

- The seven catalogs are also changed by other open pull requests (#778, #842), so a merge can
  conflict on them; the build re-measures them at its cut and before its push, and a conflict keeps
  both sides' keys.
- The unit suite's document is not a browser: it moves the focus and computes names and roles by
  the same rules, but a browser's focus ring and a screen reader's speech are met only in the
  person's check (#818).
- A screen reader that does not name an alert dialog as the focus enters it would not say the
  question; the person's check detects it, and ADR-480's "What would make this wrong" names the
  cure.
- A name's place changes when a passkey above it is removed; the day does not (ADR-480 D2).
- The question's id is built from the passkey's id, so a page that rendered the list twice would
  repeat it; the settings screen's embedding (#57) re-reads it.
- The six translations are written by the build and not yet reviewed (§5).

## 7. Formal, rows and mutation

- **Formal: NOT APPLICABLE by surface.** The delivery is TypeScript, Svelte and JSON under
  `web/app`, which no `@phx covers` line names and which the formal registry cannot cover (a
  covered path is Rust, Python or shell). Its state, the open step and the pending removal, is one
  page's and changes one event at a time; a pending removal already holds every button
  (`SignInMethods.svelte:87`, `:95`, `:105`, `:125`), and the new Escape acts only through the
  step's held Keep. The focus target after a removal is a total choice over the re-read list,
  closed by A5's fixture-defined goldens and by StrykerJS.
- **Rows:** none. No rows table runs a vitest killer, so the band `S46300-S46399` is claimed and
  holds no file.
- **Mutation:** StrykerJS over `web/app/src/lib/settings/SignInMethods.svelte`, with the
  repository's own configuration at a break of 100; each survivor is killed by a test or recorded
  as equivalent with its reason.
