# SPEC-366: the web review offers two grades, Again and Good, on its buttons, keys, remote and stick

- **Issue:** `#712`. **Decision:** the owner's grading decision, which settles the question
  issue #175 raised: grading is two buttons. It settles three things. The two-button rule: every
  surface that grades a card offers exactly two grades. The owner-press rule: only the owner's own
  tap or press grades a card, held by an engine-core token. The late-review rule: a late review is
  told plainly that it does not count toward the streak. This SPEC builds the web's half of the
  two-button rule: the review's buttons, keys, remote and stick, the page's protocol and its
  messages. The engine's half and the owner-press rule are SPEC-365's; the native half and the
  late-review copy are section 7's.
- **Context(s):** `miniapp` (`web/app/src/lib/study`, `web/app/src/lib/remote`,
  `web/app/src/lib/engine`, `web/app/messages`, `web/app/tests-study`).
- **Decided by:** ADR-377 (this SPEC's own). It amends ADR-342's mapping (the grades the web maps)
  by an insert-only amendment section appended at its end that names ADR-377; no existing line of
  ADR-342 changes. It works under ADR-361 D2 (a rating reaches only the card the review showed)
  and D15 (the mapping is stored per device), which it keeps.
- **Schematic:** `docs/schematics/web-study-screens.md` (the study screens' components and the
  review's machine). Two of its lines change, `:26` and `:120`, to name two grades. No schematic is
  added: no component, state, edge or store is added or removed (ADR-377 D11).
- **Status:** the web half of delivery D2, which lands first (section 7). **Mutation band:** none
  (section 9). **Model:** none (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `dee337bc` by `git grep -n` and `git show`. Nothing was
run: every figure is a read.

### 1a. Where the web offers four grades

| surface | path | at `dev` |
|---|---|---|
| the answer buttons | `web/app/src/lib/study/AnswerButtons.svelte:7`, `:12-17`, `:20`, `:28` | four buttons in `grid-cols-4`, button `index` showing the engine's `labels[index]` |
| the review's machine | `web/app/src/lib/study/review.ts:56`, `:79`, `:176`, `:280` | four grade cells; `RATING = { again: 1, hard: 2, good: 3, easy: 4 }`; four answer-side grade controls; `client.rate(card, RATING[event as Action] as Rating, …)` |
| the remote's actions | `web/app/src/lib/remote/actions.ts:11-20`, `:29`, `:36` | the `Intent` union names `hard` (`:14`) and `easy` (`:16`); `GRADES` holds four; confirm is Good on the answer side |
| the default map | `web/app/src/lib/remote/mapping.ts:8-19`, `:26-31`, `:34-44` | buttons 14 Again, 13 Hard, 15 Good, 12 Easy; the stick's axis 0 Again and Good, axis 1 Hard and Easy; keys `1` to `4` |
| the stored mapping | `web/app/src/lib/study/mapping-store.ts:19-29`, `:56-58`, `:101-127` | `INTENTS` names `hard` (`:22`) and `easy` (`:24`); `pairs()` keeps a stored pair only when `INTENTS` names its intent; a write happens only on a bind, a restore or the stick switch |
| the mapping screen | `web/app/src/lib/study/MappingScreen.svelte:19-29` | `NAMES` holds a row for `hard` (`:22`) and `easy` (`:24`) |
| the page's protocol | `web/app/src/lib/engine/protocol.ts:39-40`, `:179`, `:182` | `Rating = 1 \| 2 \| 3 \| 4`; `answer`'s and `rate`'s rating parse by `whole(value, 1, 4)` |
| the messages | `web/app/messages/<locale>.json`, 7 locales | `study_hard` and `study_easy` at `:187` and `:189` in `en`, `:87` and `:89` in the other six: 14 strings |

`git grep -c -i -w -E 'hard|easy|study_hard|study_easy' dev -- web/app`, excluding the generated
message sources (`web/app/src/lib/paraglide`), `web/app/messages` and the docs fence script, counts
57 lines in 15 files: 7 shipped sources (`protocol.ts` 1, `actions.ts` 3, `mapping.ts` 6,
`AnswerButtons.svelte` 3, `MappingScreen.svelte` 2, `mapping-store.ts` 2, `review.ts` 3) and 8 test
files (`gamepad.test.ts` 4, `keys.test.ts` 4, `answer-buttons.test.ts` 2, `input.test.ts` 4,
`mapping-store.test.ts` 8, `review-screen.test.ts` 3, `review.test.ts` 11,
`tests-study/study.spec.ts` 1). No native source names `study_hard` or `study_easy`.

### 1b. Who sends a rating

Under `web/app/src`, two shipped sources call `.rate(`: `study/review.ts:280` (the review's grade
cell, through the client) and `engine/session.ts:265` (the Worker passing the request to the
engine). The request `op: 'rate'` is built only at `engine/client.ts:110`. The cast at
`review.ts:280` lets any `RATING` value compile, so the type alone holds nothing. Three test lines
send a wire rating of 2 or 4: `engine/protocol.test.ts:16` (`rate`, 4), `engine/session.test.ts:231`
(`answer`, 4) and `:405` (`answer`, 2).

The engine at `dev` accepts every wire rating from 1 to 4, so a page that sends only 1 and 3 is
sound against it; SPEC-365 then refuses 2 and 4 by name. The engine's card view still carries four
interval labels, one per next state, and this delivery keeps that view.

### 1c. The state the change meets

No state is added. The stored mapping is read when the review and the mapping screen load
(`mapping-store.ts:56-58`) and written only by a bind, a restore or the stick switch (`:101-127`);
this delivery adds no write. Of the 208 `@phx covers` lines under `formal/`, 0 name a path under
`web/`.

## 2. Requirements

R1. **Two answer buttons.** The answer side shows two buttons, Again then Good, in two columns.
    Again shows the engine's first interval label (`labels[0]`) and Good its third (`labels[2]`).
    No button names Hard or Easy.

R2. **Two grade cells.** The review's answer side has grade cells for `again` and `good` alone.
    `RATING` is `{ again: 1, good: 3 }`, and the answer-side controls are `again`, `good`, `bury`
    and `flag`. A grade reaches `client.rate` with 1 or 3 and with no other number.

R3. **Two grade actions.** `hard` and `easy` leave the remote's `Intent` union and `GRADES`.
    Confirm on the answer side stays Good (`actions.ts:36`).

R4. **The default map.** Key `1` is Again and key `3` is Good; keys `2` and `4` fire nothing.
    Gamepad button 14 (d-pad left) is Again and 15 (d-pad right) is Good; buttons 12 and 13 fire
    nothing. The stick's axis 0 is Again to the left and Good to the right; axis 1 fires nothing.
    Every other binding is unchanged.

R5. **A stored mapping.** `INTENTS` drops `hard` and `easy`, so a stored mapping that names either
    loads with those pairs dropped and every other pair kept. Loading writes nothing: the stored
    value stays until the learner next binds, restores or switches the stick.

R6. **The mapping screen.** Its rows name Again and Good and no other grade. The stick switch
    stays.

R7. **The page's protocol.** `Rating` is `1 | 3`. `rate`'s and `answer`'s rating parse only as 1
    or 3, and any other value is refused as today, `rate's rating is malformed` and
    `answer's rating is malformed` (`protocol.ts:207`), code `bad-request` from the Worker.

R8. **The messages.** `study_hard` and `study_easy` leave all seven locales (14 strings), and
    `study_again` and `study_good` stay in each.

R9. **The census.** A new test file, `web/app/src/lib/study/two-grades.test.ts`, holds three
    things over the shipped sources under `web/app/src` and the locale files under
    `web/app/messages`. `.rate(` is called at exactly two sites, `study/review.ts` once and
    `engine/session.ts` once. `review.ts` declares `RATING` as exactly `{ again: 1, good: 3 }`.
    Every locale names `study_again` and `study_good`, and none names `study_hard` or
    `study_easy`. Each population prints its examined count and refuses zero, and planted texts
    are refused by name.

R10. **The records.** `docs/schematics/web-study-screens.md:26` reads "AnswerButtons: two grades,
    Again and Good, each with its interval", and `:120` reads "a grade (Again or Good: a key, the
    d-pad, the stick, a tap)". ADR-342 gains the insert-only amendment section
    `amendments-366.final.md` holds, byte for byte.

## 3. Acceptance criteria

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | Two answer buttons, named `Again <1m` and `Good <10m`, answer `again` and `good` | four buttons: the names read `['Again <1m', 'Hard <6m', 'Good <10m', 'Easy 4d']` | `web/app/src/lib/study/answer-buttons.test.ts` "each answer button names its grade and its interval" (name kept; SPEC-350 A13) |
| A2 | The review's table holds no Hard or Easy cell | the table holds `['answer', 'hard', 'busy', 'rate']` and `['answer', 'easy', 'busy', 'rate']` | `web/app/src/lib/study/review.test.ts` "the table has these cells and no others" (name kept) |
| A3 | The answer-side controls are `again`, `good`, `bury` and `flag`, and Again and Good send 1 and 3 | the controls read `['again', 'hard', 'good', 'easy', 'bury', 'flag']` | `review.test.ts` "the review shows, reveals, rates and moves on" (name kept; SPEC-350 A11) |
| A4 | Keys `1` and `3` fire Again and Good; keys `2` and `4` fire nothing | key `2` fires `hard` | `web/app/src/lib/remote/keys.test.ts` "desktop keys fire their actions" (name kept; SPEC-343 A26) |
| A5 | Buttons 14 and 15 fire Again and Good on a rising edge; 12 and 13 fire nothing | button 13 fires `hard` | `web/app/src/lib/remote/gamepad.test.ts` "each mapped button fires its action on its rising edge only" (name kept; SPEC-343 A23) |
| A6 | The stick's axis 0 fires Again and Good; axis 1 fires nothing | axis 1 down fires `hard` | `gamepad.test.ts` "the left stick fires past its threshold and re-arms below the lower one" (name kept; SPEC-343 A25) |
| A7 | Every source reaches one action, and no source reaches Hard or Easy | key `2` reaches `hard` | `web/app/src/lib/study/input.test.ts` "every source reaches one action" (name kept) |
| A8 | A stored mapping that names Hard or Easy loads without them, keeps every other pair, and is not written back | the stored `hard` and `easy` pairs load, because `INTENTS` names them | `web/app/src/lib/study/mapping-store.test.ts` "a stored mapping that names Hard or Easy loads without them and is not written back" (added) |
| A9 | The mapping screen's rows name Again and Good and no other grade | the rows include `Hard` and `Easy` | `mapping-store.test.ts` "the mapping screen changes a key and a button by pressing them, and restores each mode" (name kept) |
| A10 | `rate` and `answer` refuse a rating of 2 or 4 as malformed | `whole(value, 1, 4)` parses 2 and 4 | `web/app/src/lib/engine/protocol.test.ts` "each study operation parses its arguments and refuses any other" (name kept; SPEC-350 A6) |
| A11 | Two sites call `.rate(`, and `RATING` reads `{ again: 1, good: 3 }`; each plant is refused by name | `RATING` reads `{ again: 1, hard: 2, good: 3, easy: 4 }` | `web/app/src/lib/study/two-grades.test.ts` "the page rates with two grades and from two sites" (added) |
| A12 | Every locale names `study_again` and `study_good`, and none names `study_hard` or `study_easy` | all 7 locales name `study_hard` | `two-grades.test.ts` "every locale names the two grades and no other" (added) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/study/answer-buttons.test.ts -t "each answer button names its grade and its interval"
A2: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "the table has these cells and no others"
A3: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "the review shows, reveals, rates and moves on"
A4: pnpm exec vitest run web/app/src/lib/remote/keys.test.ts -t "desktop keys fire their actions"
A5: pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "each mapped button fires its action on its rising edge only"
A6: pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "the left stick fires past its threshold and re-arms below the lower one"
A7: pnpm exec vitest run web/app/src/lib/study/input.test.ts -t "every source reaches one action"
A8: pnpm exec vitest run web/app/src/lib/study/mapping-store.test.ts -t "a stored mapping that names Hard or Easy loads without them and is not written back"
A9: pnpm exec vitest run web/app/src/lib/study/mapping-store.test.ts -t "the mapping screen changes a key and a button by pressing them, and restores each mode"
A10: pnpm exec vitest run web/app/src/lib/engine/protocol.test.ts -t "each study operation parses its arguments and refuses any other"
A11: pnpm exec vitest run web/app/src/lib/study/two-grades.test.ts -t "the page rates with two grades and from two sites"
A12: pnpm exec vitest run web/app/src/lib/study/two-grades.test.ts -t "every locale names the two grades and no other"
```

Shipped tests whose names this delivery keeps and whose bodies change, because a Hard or Easy
action, button, key or wire rating they use is gone:
- `review.test.ts`: "a gesture during a request fires nothing" (`:178`), "a refused card keeps
  its answer controls" (`:239`), "the frame keeps its card while a request is in flight" (`:266`)
  and "the frame shows the faces the engine completed" (`:420`).
- `review-screen.test.ts`: "the screen lock follows the review, the gamepad and the page"
  (`:255`, `:259`: `Easy 4d` becomes `Good <10m`) and "a refusal, a card the frame refuses and a
  done deck are announced" (`:413`: `Hard <6m` becomes `Again <1m`).
- `input.test.ts`: "the key switch silences single-character keys" (`:89`) and "focus returns to
  the review" (`:131`).
- `keys.test.ts`: "the side moves by the action that fired" (`:98`).
- `mapping-store.test.ts`: "a change maps one input to one action, and each mode returns to its
  default apart" (`:109`) and "a key a capture takes does nothing else on the page, and Cancel
  shows only while a capture waits" (`:381`), whose Hard rows become Again rows.
- `protocol.test.ts` `:16` (`rate`, 4 becomes 3); `session.test.ts` `:231` (4 becomes 3) and `:405`
  (2 becomes 1), so each request stays well formed.
- `web/app/tests-study/study.spec.ts` "show answer reveals the answer and the buttons show the
  intervals" (`:102`): its loop over the grades (`:114`) reads two.

Shipped requirements this delivery changes, by their own SPECs (their text is not edited; this
SPEC and ADR-377 record the change, and their tests keep their names):

| shipped requirement | what changes |
|---|---|
| SPEC-343 section 7's default map (`:340` Hard, `:342` Easy) | Hard and Easy leave the default map |
| SPEC-350 R7 (`:110`, "The four answer buttons") and A13 (`:162`, `:192`) | two answer buttons |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-366-the-web-review-offers-two-grades-again-and-good-on-its-buttons-keys-remote-and-stick.md` | docs | added |
| `docs/decisions/ADR-377-the-web-review-grades-with-again-and-good-and-a-stored-mapping-drops-the-others-when-read.md` | docs | added |
| `docs/decisions/ADR-342-one-universal-iphone-and-ipad-app-driven-by-touch-keys-or-an-8bitdo-remote.md` | docs | an insert-only amendment section appended (R10) |
| `docs/schematics/web-study-screens.md` | docs | `:26` and `:120` name two grades (R10) |
| `docs/red-first/SPEC-366.md` | docs | added |
| `changelog.d/two-button-web-366.md` | docs | added |
| `web/app/src/lib/study/AnswerButtons.svelte` | miniapp | two buttons, `labels[0]` and `labels[2]`, two columns (R1) |
| `web/app/src/lib/study/review.ts` | miniapp | the answer cells, `RATING` and the controls (R2) |
| `web/app/src/lib/remote/actions.ts` | miniapp | `hard` and `easy` leave `Intent` and `GRADES` (R3) |
| `web/app/src/lib/remote/mapping.ts` | miniapp | `BUTTONS`, `STICK` and `KEYS` (R4) |
| `web/app/src/lib/study/mapping-store.ts` | miniapp | `INTENTS`; the stick switch's comment (`:110`) names two grades (R5) |
| `web/app/src/lib/study/MappingScreen.svelte` | miniapp | `NAMES` (R6) |
| `web/app/src/lib/engine/protocol.ts` | miniapp | `Rating` and the two validators (R7) |
| `web/app/messages/en.json`, `es.json`, `fr.json`, `ja.json`, `ko.json`, `zh-Hans.json`, `zh-Hant.json` | miniapp | `study_hard` and `study_easy` removed (R8) |
| `web/app/src/lib/study/two-grades.test.ts` | miniapp | added (A11, A12; R9) |
| `web/app/src/lib/study/answer-buttons.test.ts` | miniapp | A1 |
| `web/app/src/lib/study/review.test.ts` | miniapp | A2, A3, and the bodies section 3 names |
| `web/app/src/lib/study/review-screen.test.ts` | miniapp | the bodies section 3 names |
| `web/app/src/lib/remote/keys.test.ts` | miniapp | A4, and `:98`'s body |
| `web/app/src/lib/remote/gamepad.test.ts` | miniapp | A5, A6 |
| `web/app/src/lib/study/input.test.ts` | miniapp | A7, and the bodies section 3 names |
| `web/app/src/lib/study/mapping-store.test.ts` | miniapp | A8 (added), A9, and the bodies section 3 names |
| `web/app/src/lib/engine/protocol.test.ts` | miniapp | A10, and `:16` |
| `web/app/src/lib/engine/session.test.ts` | miniapp | `:231` and `:405` |
| `web/app/tests-study/study.spec.ts` | miniapp | `:114`'s loop reads two grades |

## 5. What this does NOT do

- It leaves the engine's four wire ratings and its four interval labels as they are: the engine
  refuses ratings 2 and 4 in delivery D1, SPEC-365 (`#711`).
- It leaves the native client's grades as they are: delivery D1, SPEC-365, carries the native
  half (`#711`).
- It adds no late-review copy: that is delivery D3's (`#713`).
- It leaves Undo, bury, flag and replay, and their keys and buttons, unchanged (`#714`).
- It leaves every sync path unchanged (`#715`).

## 6. Risks

- **A learner's habit.** Keys `2` and `4`, buttons 12 and 13 and the stick's axis 1 now fire
  nothing, so a press from habit does nothing and says nothing. A4 to A7 hold that nothing fires;
  the mapping screen shows two grade rows, and the owner's session reads the habit (V1).
- **Stale stored pairs.** A stored Hard or Easy pair stays in the device's storage until the
  learner next changes a mapping. Every read drops it (A8), and nothing else reads storage.
- **StrykerJS over each changed file whole.** CI's `mutation-web` job mutates every changed
  production file in full at break 100, so a survivor anywhere in such a file fails the run, not
  only one on a changed line. The build runs the same mutation over its changed files before its
  push; a survivor is killed by a test or, when shown equivalent, recorded with its reason.
- **The cast at `review.ts:280`.** A wrong `RATING` would compile; A11 holds its text.
- **A script in the page's Worker** can still send 2 or 4 through the engine's own doors until
  SPEC-365 lands. That is the owner-press rule's subject, and SPEC-365's.

## 7. Delivered by the other pull requests

| delivery | what | when |
|---|---|---|
| D1, SPEC-365 | the engine-core token; every engine door records Again or Good alone; every native caller of the answer moves to the token door | after this delivery and the open native review pull request are on `dev` |
| D1's native half, SPEC-365 | the native client's two grades | in D1 |
| D3 | the late-review copy | independent |

## 8. Formal model

None. No state, actor or write path is added: the stored mapping's read drops pairs in memory and
writes nothing (section 1c), and 0 of the 208 `@phx covers` lines under `formal/` name a web path.

## 9. Mutation rows

None. The rows table's killers are cargo tests or Python tests (`scripts/mutation_rows.py:79-93`),
and this delivery changes no Rust and no Python. Its mutants are StrykerJS's: the pull request's
`mutation-web` job (`.github/workflows/ci.yml:702-760`) mutates each changed production file under
`web/app` whole, and its verdict reads StrykerJS's report (`:753`) against
`web/app/stryker.config.json`'s break of 100 (`:18`). A survivor is killed by a test; one shown
equivalent is recorded in `scripts/mutation-equivalent.d/miniapp.json`, bound to the one survivor
it excuses. The band `S36600-S36699` stays claimed and holds no rows file, as SPEC-330 declared
none.

## 10. What only CI or a device proves

| # | what | where |
|---|---|---|
| C1 | The study suite shows the answer with two buttons, each with its interval, and a rating by key and one by button each show the next card | `web/app/tests-study/study.spec.ts` "show answer reveals the answer and the buttons show the intervals" and "a rating by key and one by button each show the next card" (shipped), in CI on the pull request |
| C2 | Every changed production file's mutants are killed or recorded equivalent | `mutation-web`, on the pull request |
| V1 | On the device, the review offers Again and Good by tap, key, remote and stick, and nothing else grades | the owner, in the acceptance session (`#718`) |

## Amendment: corrected citations (SPEC-365's delivery)

- R3 cites `actions.ts:36`; the line is `actions.ts:34`.
- R7 cites `protocol.ts:207`; the line is `protocol.ts:209`.
- Section 9 says each changed production file is mutated whole; CI's web mutation plan mutates five of the seven: `mapping-store.ts` (one added comment line) and `MappingScreen.svelte` (deletions only) are outside it.
