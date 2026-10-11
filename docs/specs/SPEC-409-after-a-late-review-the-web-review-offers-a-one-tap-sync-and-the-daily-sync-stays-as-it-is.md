# SPEC-409: after a late review, the web review offers a one-tap sync, and the daily sync stays as it is

- **Issue:** `#807`. After a review of a card past its due day, the review offers one tap that
  syncs this device's reviews now; the scheduled daily sync does not change.
- **Context(s):** `miniapp` (`web/app/src/lib/study`, `web/app/src/routes/study/review`,
  `web/app/messages`).
- **Decided by:** ADR-423 (this SPEC's own). It works under ADR-037 (the server syncs once a study
  day, plus the owner's trigger), ADR-388 D12 (the session syncs once after the open and once
  before the close) and ADR-387 (the late line), and changes no rule of any of them.
- **Schematic:** `docs/schematics/web-study-screens.md`, amended by one section appended at its end
  (section 8, the late review to the offer and the one-tap sync). No existing line of it changes.
- **Status:** one delivery. **Mutation band:** `S40900-S40999`, claimed, holding no rows file
  (section 9). **Model:** none (section 8).

## 1. The problem, measured

Every fact below is at the base `6e256acb925d5f6b4da908974e698a696bc651d8` (`dev`).

### 1a. What a late review is, and where the web reads it

- The engine core decides "past its due day" in the engine's own day, and the web engine puts its
  answer on the card view as `late` (SPEC-376 section 2, R1 to R4; `web/app/src/lib/engine/protocol.ts:145`).
- The review screen draws the late line above the card when `late` is true
  (`web/app/src/lib/study/ReviewScreen.svelte:190-192`).
- The review machine rates the shown card with `client.rate(...)` and moves on when the engine
  answers (`web/app/src/lib/study/review.ts:353-354`, `'settled'` at `:366`). It holds the shown view
  in its own fields (`review.ts:151-168`), and nothing counts the late reviews answered.
- The review's client, `StudyClient` (`review.ts:122-135`), holds card, rate, bury, flag, undo,
  undo offer and faces, and no sync.

### 1b. What syncs today, and when

- **The web session.** It syncs once after the open and once before the close, neither awaited,
  through the Worker's one queue (`web/app/src/lib/study/engine.ts:5-6`; the one call at `:38`,
  `void client.sync().catch(() => undefined);`; the open at `:76-83`, the close at `:108-114`). The
  shipped test `a session syncs at its start and end` holds it (`web/app/src/lib/study/engine.test.ts:309`).
- **The Sync page.** Its "Sync now" runs the client's `sync()`, keeps the answer's status word and
  required kind, then reads the unsynced count (`web/app/src/lib/sync/SyncScreen.svelte:111-120`,
  the count at `:68-71`), and shows "Syncing…" while it runs and "Reviews waiting to sync: N" after
  (`:216-221`).
- **The sync's answer.** `Synced` is a status word and a required kind
  (`web/app/src/lib/engine/protocol.ts:82-85`; the words at `:90`); `Unsynced` holds the review count
  (`:340-344`). The engine answers, for example, `{ status: 'held', required: 'normal-sync' }` and
  `{ status: 'absent', required: null }` (`web/app/src/lib/engine/sync.test.ts:81-101`).
- **The server.** It syncs the collection once a study day, claimed per study day
  (ADR-037; `crates/coordination/src/jobs.rs:62-69`), held by the shipped tests
  `the_daily_sync_slot_keeps_off_the_predecessors_ticks_and_every_other_slot`
  (`crates/coordination/tests/job_table.rs:133`) and
  `the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day` (`:245`). The owner asks for
  one more through the bot's `/sync` (`crates/bot/src/commands.rs:244-254`), whose request
  directory only the bot's unit writes (`docs/schematics/sync-request-doorbell.md`). No Mini App
  route reaches it.
- **The native app.** Its engine session names no sync call (`ios/App/Sources/EngineSession.swift:8-15`),
  and it has no late line (`#738`); its session sync is SPEC-358 part c (`#633`).

### 1c. What the review screen has to hold an offer

- Its properties are `client` and `player` (`ReviewScreen.svelte:36`); it builds the machine with an
  `onChange` that re-reads the machine's face (`:43-48`, `read()` at `:50-61`).
- Its status region is `role="status"` above the card (`:186-188`); the undo dialog replaces the
  control bar while it asks (`:203-246`); the nav ends the screen (`:318-321`).
- A key typed while the focus is on a button belongs to the button (`web/app/src/lib/remote/keys.ts:21-30`).
- The strings the offer reuses are `sync_now` "Sync now", `sync_working` "Syncing…", the status
  lines, `sync_unsynced` "Reviews waiting to sync: {count}" and `sync_full_required`
  (`web/app/messages/en.json:236-245`), all held in every locale by
  `web/app/src/lib/sync/sync-locales.test.ts:11-40`.

### 1d. What pins what this delivery touches

- No shipped test, message fixture or mutation row pins a literal this delivery changes: it
  changes no existing string, and 0 of 49 `*.msg.json` files hold any string it reuses.
- Under `web/app/src/lib/study` and `web/app/src/routes/study`, the one shipped `.sync(` call is
  `engine.ts:38`.
- 0 of 227 `@phx covers` lines under `formal/` name a `web/` file. The one model whose state a sync
  moves fires its sync action at every interleaving (`formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla:116-119`, `:199`).
- Every `mermaid` block under `docs/` is parsed by `web/app/src/lib/docs-mermaid.test.ts`, so the
  schematic's new diagram is judged by the shipped suite.

## 2. Requirements

R1. The review machine counts its settled late reviews: when `client.rate(...)` answers for a shown
card whose view has `late: true`, its `lateReviews` count rises by one. A rate of an on-time card, a
bury, a flag, an undo and a refused rate leave it as it was.

R2. The review screen takes an optional `update` property, `() => Promise<UpdateClient>`, where
`UpdateClient` holds `sync()` and `unsynced()` as `SyncClient` types them
(`web/app/src/lib/sync/status.ts`). A screen given none shows no offer and makes no sync call. The
review route gives `update={() => studyEngine().client()}`.

R3. A screen with an `update` shows the offer once `lateReviews` is above zero, until the learner
presses "Not now". It sits after the control bar and before the nav, outside every phase block, so
it stays while the undo dialog asks and when the deck is done. It holds the line
`study_update_prompt` and two buttons, "Sync now" (`sync_now`) and "Not now" (`study_update_dismiss`).

R4. Raising the offer and dismissing it make no call to the update client.

R5. One press of "Sync now" makes exactly one `sync()` call and, once it answers, one `unsynced()`
call. While the sync runs, the line reads `sync_working` and "Sync now" is disabled.

R6. After the sync answers, the line names the outcome, and `sync_unsynced` with the count read
follows it: (a) status `held` with a required kind `choosing` does not name: `study_update_done`;
(b) a required kind `choosing` names: `sync_full_required` and a link to the Sync page reading
`study_update_open_sync`, and no full-sync call; (c) any other status word: the Sync page's own line
for it, and for a word `signsIn` names, the same link. (d) A `sync()` or `unsynced()` that throws:
`study_refused_engine`, and no count. After every outcome, "Sync now" is enabled again.

R7. "Not now" hides the offer for the rest of the screen's life: a later late review raises nothing.
Pressed while a sync runs, it hides the offer, and the running sync ends without a line.

R8. The offer never blocks the review: it takes no focus when it appears or when a result shows;
the card, the answer buttons and their keys keep working while a sync is pending.

R9. Reach: both buttons are `<button type="button">` with the screen's `min-h-11` class, each
named by its visible text; the link is built as the screen's nav builds its links
(`ReviewScreen.svelte:318-321`); the line is a `role="status"` region present from the screen's
mount and empty until the offer shows.

R10. Every locale under `web/app/messages` holds the four new keys `study_update_prompt`,
`study_update_dismiss`, `study_update_done` and `study_update_open_sync`. English reads exactly
"Your reviews stay on this device until they sync. You can sync them now.", "Not now", "Sync
finished." and "Open the sync page". Each other locale's value is not empty, is not the English
text, holds no word of the locale's own for the streak (as `late-line.test.ts` names them), and
holds no exclamation mark (`!` or `！`). Every reused key keeps its text.

R11. The daily sync and the session's syncs are unchanged: the delivery changes no file under
`crates/`, `deploy/` or `ios/`, no line of `web/app/src/lib/study/engine.ts` and no file under
`web/app/src/lib/sync/`. Under `web/app/src/lib/study` and `web/app/src/routes/study`, the
shipped `.sync(` calls are `engine.ts` (one) and `update-now.ts` (one).

R12. The offer's state lives in one new module, `web/app/src/lib/study/update-now.ts`: its class
`UpdateNow` holds hidden, shown, working, an outcome and dismissed, makes the calls of R5, and
chooses the line of R6; the screen draws it.

## 3. Acceptance criteria of the one-tap sync offer

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | A late review raises the offer: after a late card is rated and the rate answers, the line and both buttons show after the control bar, and focus stays on the screen's own region, never on the offer | the base shows no offer | `web/app/src/lib/study/update-now.test.ts` `a late review raises the offer below the controls` (added) |
| A2 | An on-time review and a late card shown but not yet rated raise no offer; the same screen raises it at its first late answer | the base never raises it, so the late half fails | `update-now.test.ts` `an on-time review and an unanswered late card raise no offer` (added) |
| A3 | "Not now" hides the offer for the screen's life, a later late review raises nothing, neither raising nor dismissing calls the update client, and "Not now" pressed while a sync runs hides the offer with no line after the sync answers | the base has no "Not now" | `update-now.test.ts` `not now hides the offer for the screen and syncs nothing` (added) |
| A4 | One press runs one sync then one count read; "Syncing…" shows with "Sync now" disabled while the sync runs; then "Sync finished." and "Reviews waiting to sync: 0", with the focus where it was | the base has no "Sync now" on the review | `update-now.test.ts` `one tap syncs once and shows the result` (added) |
| A5 | An offline answer shows the Sync page's offline line and the count, "Sync now" is enabled again, and a second press makes a second sync | the base has no offer | `update-now.test.ts` `an offline answer says so and lets the learner retry` (added) |
| A6 | An answer whose status word needs a sign-in (absent, needs-sign-in) shows that word's line and a link to the Sync page | the base has no offer | `update-now.test.ts` `a sync with no sign-in links to the sync page` (added) |
| A7 | A full sync required shows `sync_full_required` and the link, and the review makes no full-sync call | the base has no offer | `update-now.test.ts` `a full sync required sends the learner to the sync page` (added) |
| A8 | A sync that throws shows the engine-refusal line and no count, and "Sync now" is enabled again | the base has no offer | `update-now.test.ts` `a refused sync says the engine stopped and lets the learner retry` (added) |
| A9 | While a sync is pending and never answers, the next card shows and its rate reaches the client | the base has no "Sync now" to press | `update-now.test.ts` `the review goes on while a sync is pending` (added) |
| A10 | Both buttons are native buttons named by their visible text with `min-h-11`, and the line's status region is in the page, empty, before the offer shows | the base has no region and no buttons | `update-now.test.ts` `the offer is reachable by touch keyboard and screen reader` (added) |
| A11 | The review route gives the screen the engine's client as `update` | the base route passes no `update` | `update-now.test.ts` `the review route gives the screen the engine sync` (added) |
| A12 | The machine counts only settled late answers: a late rate counts one; an on-time rate, a bury of a late card and a refused late rate count none | the base machine has no count | `update-now.test.ts` `the review counts only settled late answers` (added) |
| A13 | Every locale holds the four keys under R10's rules, and each plant is refused by name | the base locales lack the keys | `update-now.test.ts` `every locale holds the offer words` (added) |
| A14 | The study code's shipped `.sync(` calls are `engine.ts` (one) and `update-now.ts` (one), and each plant is refused by name | the base holds `engine.ts` alone | `update-now.test.ts` `the tap is the one new sync call in the study code` (added) |
| A15 | The session still syncs once after the open and once before the close, and never between | not red: the base holds it, and this delivery keeps it | `web/app/src/lib/study/engine.test.ts` `a session syncs at its start and end` (shipped, kept) |
| A16 | The server's daily sync still runs once a study day, claimed per study day | not red: the base holds it, and this delivery changes no server file | `crates/coordination/tests/job_table.rs` `the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day` and `the_daily_sync_slot_keeps_off_the_predecessors_ticks_and_every_other_slot` (shipped, kept) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "a late review raises the offer below the controls"
A2: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "an on-time review and an unanswered late card raise no offer"
A3: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "not now hides the offer for the screen and syncs nothing"
A4: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "one tap syncs once and shows the result"
A5: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "an offline answer says so and lets the learner retry"
A6: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "a sync with no sign-in links to the sync page"
A7: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "a full sync required sends the learner to the sync page"
A8: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "a refused sync says the engine stopped and lets the learner retry"
A9: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "the review goes on while a sync is pending"
A10: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "the offer is reachable by touch keyboard and screen reader"
A11: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "the review route gives the screen the engine sync"
A12: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "the review counts only settled late answers"
A13: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "every locale holds the offer words"
A14: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "the tap is the one new sync call in the study code"
A15: pnpm exec vitest run web/app/src/lib/study/engine.test.ts -t "a session syncs at its start and end"
A16: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day
A16: cargo test -p deck-streak-coordination --test job_table -- --exact the_daily_sync_slot_keeps_off_the_predecessors_ticks_and_every_other_slot
```

Every expected string in A1 to A13 is spelt as a literal in the test, in English, as R6 and R10
word it; no expected value is read through the message functions the screen calls. The fake update
client counts each call and answers from fixed values written in the test. The censuses of A13 and
A14 report what they examined and refuse zero, and each carries planted refusals: for A13 a locale
lacking a key, a locale holding the English text, a locale holding its streak word and a locale
holding an exclamation mark; for A14 a third `.sync(` call in a study module and a `.sync(` call
in `review.ts`. A2, A3 and A9 each assert a positive artifact (the offer raised, the fake's rate
count) beside their absence, so a screen that crashed would fail them.
The criteria read, and leave unchanged, `web/app/src/lib/study/engine.ts`,
`web/app/src/lib/study/engine.test.ts`, `web/app/src/lib/sync/status.ts`,
`web/app/src/lib/sync/sync-locales.test.ts`, `web/app/src/lib/study/late-line.test.ts` and
`crates/coordination/tests/job_table.rs`; `web/app/src/lib/docs-mermaid.test.ts` parses the
appended section's diagrams in the whole suite.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-409-after-a-late-review-the-web-review-offers-a-one-tap-sync-and-the-daily-sync-stays-as-it-is.md` | docs | added |
| `docs/decisions/ADR-423-one-tap-runs-the-clients-own-sync-once-a-late-review-settles-and-the-daily-sync-stays-as-it-is.md` | docs | added |
| `docs/schematics/web-study-screens.md` | docs | section 8 appended; no existing line changes |
| `docs/red-first/SPEC-409.md` | docs | added: the red-first record |
| `changelog.d/update-now-prompt-409.md` | docs | added: the changelog fragment |
| `web/app/src/lib/study/update-now.ts` | miniapp | added: `UpdateNow` and `UpdateClient` (R2, R4 to R7, R12) |
| `web/app/src/lib/study/update-now.test.ts` | miniapp | added: A1 to A14 |
| `web/app/src/lib/study/update-now-machine.test.ts` | miniapp | added: the offer's state machine driven without a screen, one test per guard (R2, R4 to R7, R12) |
| `web/app/src/lib/study/review.ts` | miniapp | changed: the settled late count, `lateReviews` (R1) |
| `web/app/src/lib/study/ReviewScreen.svelte` | miniapp | changed: the `update` property, the offer and its status region, and the header comment's SPEC-409 sentence (R2, R3, R8, R9) |
| `web/app/src/routes/study/review/+page.svelte` | miniapp | changed: gives `update` (R2) |
| `web/app/messages/en.json` | miniapp | changed: four keys added (R10) |
| `web/app/messages/es.json` | miniapp | changed: four keys added (R10) |
| `web/app/messages/fr.json` | miniapp | changed: four keys added (R10) |
| `web/app/messages/ja.json` | miniapp | changed: four keys added (R10) |
| `web/app/messages/ko.json` | miniapp | changed: four keys added (R10) |
| `web/app/messages/zh-Hans.json` | miniapp | changed: four keys added (R10) |
| `web/app/messages/zh-Hant.json` | miniapp | changed: four keys added (R10) |

No other file changes.

## 5. What this does NOT cover

- It adds no offer to the native app: the native app has no late line yet (`#738`) and no sync at
  all (`#633`); the native offer follows both, under ADR-423's words.
- It does not let a late review rejoin the streak, and it changes no rule of SPEC-076 or SPEC-071;
  that decision is its own (`#739`).
- It does not ask the server to sync, and it does not tell a learner when a review reaches the
  server's study day: the server keeps its daily run and the owner's trigger, and a review
  screen cannot know when the server will read it (`#739`).

## 6. Risks

- **A sync during a review slows the next card.** The Worker orders every request in one queue, so
  a rate sent while a sync runs waits for it. A9 holds the review moving while a sync is pending in
  the test; the owner's device check (V1) reads the real wait.
- **The offer reads as pressure.** R10 pins the English words and refuses streak words and
  exclamation marks in every locale; ADR-423 D2 shows it once per screen and never again after
  "Not now".
- **An answer synced by the tap can no longer be undone.** That is true after any sync, and the
  undo offer already says so; the offer adds no new state to it.
- **Drift with open work on the same files.** Open pull requests touch `review.ts`,
  `ReviewScreen.svelte`, the seven locale files and the schematic; the build re-measures each at
  its cut, takes the schematic's next free section number, and places the new keys where no open
  hunk sits.
- **A translation that misreads.** The six non-English values are written by the build in each
  locale's own register; the census holds their shape, and the owner's device check reads them.

## 7. What only CI or a device proves

| # | what | where |
|---|---|---|
| C1 | The type check passes with the new property and module | the pull request's `web` job (svelte-check) |
| C2 | The shipped browser study suite still passes; its seeded cards are new, so it shows no offer | the shipped end-to-end study suite, in CI |
| C3 | Every changed production file under `web/app` has its mutants killed or recorded equivalent | `mutation-web`, on the pull request |
| V1 | On a device, a late review raises the offer in the app's locale, one tap syncs and shows the result, "Not now" hides it, and an on-time review raises none | the owner, in an acceptance session |

## 8. Formal model

None. The tap is one more caller of the client's own sync, and every request passes the Worker's
one queue in order. The one model whose state a sync moves, `formal/tla/UndoOwnAnswer`, already
fires its sync action at every interleaving (`UndoOwnAnswer.tla:116-119`, `:199`), so the tap adds
no step it lacks. No file a model covers is edited: 0 of 227 `@phx covers` lines name a `web/` file
at the base. ADR-423 D5 records the model it was chosen against.

## 9. Mutation rows

None. The band `S40900-S40999` is claimed and holds no rows file: the rows table holds Rust, and
web code is proved by StrykerJS on the pull request at break 100 (C3), never by hand rows. A change
outside `web/` would be outside section 4 and is not part of this delivery.
