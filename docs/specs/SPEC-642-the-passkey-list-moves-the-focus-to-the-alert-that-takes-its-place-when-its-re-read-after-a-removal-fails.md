# SPEC-642: the passkey list moves the focus to the alert that takes its place when its re-read after a removal fails

- **Issue:** #945 (follow-up to #818: a decided place for the focus when the list's re-read after a
  removal fails, read by a test). **Context(s):** the web client (`web/app`: the sign-in methods
  list and its unit tests). Nothing outside `web/app` and `docs` changes, except the changelog
  fragment.
- **Decided by:** ADR-665 (this SPEC's: the focus target, the first read, the tests and the
  mutation run, formal), and ADR-480 (the list's focus, D2).
- **Amends:** `docs/schematics/passkey-sign-in-on-the-web.md`, by an insert-only new last section
  (§13). SPEC-463 and ADR-480 are unchanged.
- **Mutation band:** `S64200-S64299`, claimed, holding no rows file (§7).
- **Status:** one pull request into dev; every path it writes is under `web/app` or `docs`, plus
  its changelog fragment.

## 1. The problem, measured

Every figure below was read at `36efa139` by the command beside it; `...` stands for
`git show 36efa139:web/app/src/lib/settings/SignInMethods.svelte`.

- The list is `web/app/src/lib/settings/SignInMethods.svelte` (206 lines, `... | wc -l`), rendered
  by `web/app/src/routes/sign-in-methods/+page.svelte:11` under the route's heading (`:10`).
- After a removal ends (removed, or already gone), `removeNow` reads the list again
  (`:120`, `const again = await list();`), awaits the render (`:121`, `await tick();`) and places
  the focus only when that read answers `ok` (`:122`,
  `if (again.kind === 'ok') land(again.value, index);`).
- A `reopen` or `unavailable` answer replaces the list with an alert paragraph (`:131-134`:
  `<p class="mt-6" role="alert">{m.reopen_from_telegram()}</p>` at `:132` and
  `<p class="mt-6" role="alert">{m.server_unavailable()}</p>` at `:134`), whose wording is
  `web/app/messages/en.json:11` and `:12`. Nothing focuses it: `... | grep -n -E
  'role="alert"|focus\('` prints the focus calls at lines 35, 79, 90, 91 and 118, none of them on
  an alert, and `... | grep -c -E 'bind:this=\{failure\}|failure!'` prints 0.
- So the focused button, the confirm step's Keep, leaves the page with the list and the focus falls
  to the document's body. The unit suite pins exactly that:
  `git show 36efa139:web/app/src/routes/sign-in-methods.test.ts | grep -n -F 'toBe(document.body)'`
  prints line 543, inside SPEC-463's mutation-coverage test at `:523`, "a removal whose list cannot
  be read again moves no focus and raises no error".
- The list itself, when no passkey is left, takes the focus from script only (`tabindex="-1"`,
  `:138`; SPEC-463 R4, ADR-480 D2).
- SPEC-463 names this focus as later work (its section 5, `:136`, #945), and ADR-480 D6 chose
  against moving it there (`:198`).

## 2. Requirements

- R1. When a removal ends (removed, or already gone) and the list is read again, and that read
  answers `reopen` or `unavailable`, the focus moves to the alert that replaces the list, once
  Svelte has rendered it (`removeNow`'s existing `await tick()`).
- R2. That alert, for each answer, takes the focus only from script (`tabindex="-1"`), keeps
  `role="alert"`, and shows the wording it shows today (`reopen_from_telegram`,
  `server_unavailable`).
- R3. When the screen's first read of the list answers `reopen` or `unavailable`, the alert shows as
  today and the focus does not move: an element focused before the screen renders keeps it.
- R4. Every other end of a removal keeps SPEC-463's focus (its R4 and R5), and no promise of the
  removal is left rejected.
- R5. No wording, catalog, style, route or other screen changes; nothing a sighted learner sees
  changes except a focus ring where the focus now lands.

## 3. Acceptance criteria of SPEC-642

| # | criterion | decided by |
|---|---|---|
| A1 | after a removal whose list cannot be read again, for each of `reopen` and `unavailable`, the focus is on the alert that says why, which takes the focus only from script, and no promise is left rejected (R1, R2, R4) | `sign-in-methods.test.ts`; red: the base leaves the focus on the document's body |
| A2 | a list the screen cannot read when it opens moves no focus: an element focused before the screen renders keeps it, for each of `reopen` and `unavailable` (R3) | `sign-in-methods.test.ts`; not red: it guards the first read, which the base already leaves alone, against the new focus move |

```acceptance
A1: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "a removal whose list cannot be read again moves the focus to the alert that says why"
A2: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "a list the screen cannot read when it opens moves no focus"
```

A1 enumerates its two answers and prints `examined 2 of 2 answers`, and A2 does the same. A1 finds
the alert by its role and asserts the focused element by identity, never only that the body does
not hold it. A1's red is its focus assertion, the first of its assertions the base fails: the
alert, its wording and the empty list already hold there.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-642-the-passkey-list-moves-the-focus-to-the-alert-that-takes-its-place-when-its-re-read-after-a-removal-fails.md` | document | new |
| `docs/decisions/ADR-665-a-failed-re-read-after-a-passkey-removal-moves-the-focus-to-the-alert-in-the-lists-place-and-a-failed-first-read-moves-none.md` | document | new |
| `docs/schematics/passkey-sign-in-on-the-web.md` | document | a new last section (§13) only; nothing above it changes |
| `docs/red-first/SPEC-642.md` | document | new |
| `changelog.d/passkey-reread-focus-642.md` | document | new |
| `web/app/src/lib/settings/SignInMethods.svelte` | web client | changed: the two alerts in the list's place are bound and focusable from script only, and `removeNow` focuses the one shown when the re-read fails |
| `web/app/src/routes/sign-in-methods.test.ts` | web client | changed: one block of two new tests, A1 and A2, appended; SPEC-463's mutation-coverage test that read the focus on the body after this failure removed, A1 keeping its fixture and its no-rejection check |

No `.rs`, `.swift`, workflow, catalog, edge or content-security file changes, and
`web/app/package.json` and the lockfile do not change.

## 5. What this does NOT cover

- A person's check of the focus and of what a screen reader says when the alert takes it, by keyboard and by a screen reader on a phone and on a computer: the pull request's test plan names it as a manual step, #945.
- A browser audit of the passkey list: #946.
- The settings screen that will embed the list: #57.
- The list's first read: its alert keeps the focus where it was, by R3, and nothing about that read changes, #945.

## 6. Risks

- Some screen reader and browser pairs may say the message twice, once as the alert announces
  itself and once as the focus lands on it. The person's check hears it, and ADR-665's "What would
  make this wrong" names the cure.
- The unit suite's document is not a browser: it moves the focus and computes roles by the same
  rules, but a focus ring and a screen reader's speech are met only in the person's check (#945).
- A page that embedded the list twice would hold two such alerts, each bound by its own component;
  the settings screen's embedding (#57) re-reads it.

## 7. Formal, rows and mutation

- **Formal: NOT APPLICABLE by surface.** The delivery is TypeScript and Svelte under `web/app`,
  which no `@phx covers` line names and which the formal registry cannot cover. The change adds
  one focus target to one page's existing removal path, which changes one event at a time, and a
  pending removal already holds every button.
- **Rows:** none. No rows table runs a vitest killer, so the band `S64200-S64299` is claimed and
  holds no file.
- **Mutation:** StrykerJS over `web/app/src/lib/settings/SignInMethods.svelte`, with the
  repository's own configuration at a break of 100, run before the push and by CI's
  `mutation-web`; each survivor is killed by a test.

## 8. Amendments

None.
