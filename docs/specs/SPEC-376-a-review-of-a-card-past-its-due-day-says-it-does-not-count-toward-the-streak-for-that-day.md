# SPEC-376: a review of a card past its due day says it does not count toward the streak for that day

- **Issue:** `#713`. When the shown card is past its due day, the review shows one line saying that
  this review does not count toward the streak, in every locale. Rejoining the streak is not part
  of the issue.
- **Context(s):** `engine-core` (`crates/engine-core`), `web-engine` (`crates/web-engine`),
  `miniapp` (`web/app/src/lib/engine`, `web/app/src/lib/study`, `web/app/messages`).
- **Decided by:** ADR-387 (this SPEC's own). It works under ADR-356 D4 (the engine core depends on
  the engine and on no crate of this workspace), which it keeps, and it changes no rule of SPEC-076
  or SPEC-071: the streak counts what it counted before.
- **Schematic:** `docs/schematics/web-study-screens.md`, amended by one section appended at its end
  (section 6, the due day from the engine to the review's line). No existing line of it changes.
- **Status:** one delivery. **Mutation band:** `S37600-S37699` (section 9). **Model:** none
  (section 8).

## 1. The problem, measured

Every fact below is at the base `5fe4a48a09bc72700f434e7747ad64916af76902` (`dev`).

### 1a. What the streak counts, and what a late review is

- The language row advances once for every study day the fold settles, "by `update_on_study` when
  the day has a study review" (SPEC-076 R2, `:76-79`). A study review counts toward the study day it
  was made on (`crates/analytics/src/metrics.rs:111`, `crates/coordination/src/lapse.rs:40`), so a
  review of an overdue card, made on a day the fold has not settled, counts toward that day.
- SPEC-076 section 6 (`:387-390`) names the review that does not rejoin the streak: "a review that
  syncs after its day has settled". ADR-076 (`:66-67`) and SPEC-071 (`:397`) say the same. That
  review's own day has passed; a review screen cannot know, when it shows a card, whether the
  review will reach the server before its day settles.
- So the one claim a review screen can make truthfully about a card past its due day is about the
  day it was due: a review now cannot count toward the streak for that day, because the review is
  made on a later study day. The line says exactly that (R5, ADR-387 D3).

### 1b. Two clocks name the day

- The kernel's study day is `floor((t + m*60000 - h*3600000) / 86400000)` for the instant `t`, a
  fixed offset `m` and a rollover hour `h`, with no zone database
  (`crates/kernel/src/study_day.rs:103-113`; default `h = 4`, `:25`, and `m = 0`, `:120`). The
  streak is kept in this frame.
- The engine's day is its own scheduler's: whole days since the collection's creation, ending at
  its next rollover in the device's zone (`crates/ingest/src/engine.rs:194-201`). A card's due day
  counts in this frame.
- The two agree exactly when both days end at the same instant
  (`crates/ingest/src/skip_write.rs:929-939`). The server's own skip write refuses to act when they
  may not: the zone not pinned, a zone that observes daylight saving, a configured offset that
  differs from the process's zone, or an engine day that is not the study day (`:897-927`).
- Where they differ, worked from the two formulas with the kernel's defaults and an engine whose
  rollover is also hour 4 in a zone 3 hours east of the rule's offset: at 22:00 of the rule's day
  `N`, the kernel names study day `N` (22 - 4 = 18 hours into it), while the engine's day `N + 1`
  began at 01:00 of the rule's clock (04:00 local). For those 3 hours of every day, a card due on
  the engine's day `N` reads past its due day while the kernel still counts study day `N`.

### 1c. What reaches the review today

- The web review's card view carries an id, an ordinal, a flag, both sides, the CSS, the four
  interval labels and the undo word; no due day and no field from which one could be computed
  (`web/app/src/lib/engine/protocol.ts:101-114`).
- The web engine builds that view in `current_card` (`crates/web-engine/src/wasm.rs:560`): the
  queue's head through `GetQueuedCards` (13,3), whose card carries its queue, its due and its home
  deck's due, and its states through `DescribeNextStates` (13,24) (`:600`). It reads no engine day.
- The engine core is the one client-side holder of the engine; both clients reach it, and it
  depends on no crate of this workspace (`crates/engine-core/src/lib.rs:1-34`). It already makes one
  call of its own, behind no adapter pair: the undo status (`crates/engine-core/src/dispatch.rs:238`).
- The review screen draws the counts, a status line and the card frame
  (`web/app/src/lib/study/ReviewScreen.svelte:174-197`).
- Strings today: 0 late-review strings in the 7 message files, in the web sources, in the engine
  crates or in the native sources. The 7 locales are `en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `fr`
  and `es` (`web/app/project.inlang/settings.json`); `en` holds 226 keys and each other locale 126.
- The native review exists (`ios/App/Sources/ReviewView.swift`), and the native app holds no
  localized string file of any kind (0 files under `ios/` named as a string catalog, a strings file
  or a language folder).

## 2. Requirements

R1. The engine core holds one rule, `past_due_day`, that answers whether a card is past its due
    day, from the card's queue, due, home deck due and home deck, and the engine's day. It reads no
    clock and holds no engine type beyond the card's own message.
R2. The rule judges in the engine's day (ADR-387 D1):
    - a card in the review queue or the day-learning queue is past its due day when its due day is
      earlier than the engine's day; its due day is its home deck due when it sits in a filtered
      deck and that due is set, and its own due otherwise;
    - a card in the intraday learning queue is past its due day when its due instant is earlier
      than the instant the engine's day began, the engine's next rollover less one day;
    - every other card, a new card and a preview card among them, is never past its due day.
R3. The engine core reads the engine's day itself, through one call it holds and no adapter makes:
    the scheduler's timing of today, decoded to the day count and the next rollover. No adapter
    pair is added to the core's table or to the web engine's study table.
R4. The web engine's card view carries `late`, the core's answer for the card it shows, computed
    when the view is built; the page's `CardView` declares `late` as a required boolean.
R5. When the shown card's view carries `late` true, the review shows one line, above the card and
    below the status line, on both sides of the card: in English, `This card was due on an earlier
    day, so this review does not count toward the streak for that day.` It carries no number, no
    date and no count.
R6. When the shown card's view carries `late` false, the review shows no such line.
R7. Every locale holds the key `study_late_review`, non-empty, carrying the locale's own word for
    the streak as its tagline spells it (`streak`, `racha`, `série`, `連続記録`, `연속 기록`,
    `连续记录`, `連續紀錄`), and no locale but `en` holds the English text.
R8. Nothing else changes: no streak rule, no write, no table pair, no store and no state.

## 3. Acceptance criteria of the late-review line

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | A review card due on the engine's yesterday is past its due day | the stub answers false for every card | `crates/engine-core/tests/late.rs` `a_review_card_due_yesterday_is_past_its_due_day` (added) |
| A2 | A review card due on the engine's today is not past its due day | not red: the stub answers false; it guards the boundary | `late.rs` `a_review_card_due_today_is_not_past_its_due_day` (added) |
| A3 | A day-learning card due yesterday is past its due day, and one due today is not | the stub answers false for the card due yesterday | `late.rs` `a_day_learning_card_is_judged_by_its_due_day` (added) |
| A4 | A card in a filtered deck is judged by its home deck due, never by its filtered position | the stub answers false for a home due of yesterday | `late.rs` `a_filtered_card_is_judged_by_its_home_due` (added) |
| A5 | An intraday learning card due before the engine's day began is past its due day, and one due at the day's start is not | the stub answers false for the card due before the start | `late.rs` `an_intraday_learning_card_is_judged_by_the_days_start` (added) |
| A6 | A new card and a preview card are never past their due day, whatever their due | not red: the stub answers false; it guards the arms | `late.rs` `a_new_or_preview_card_is_never_past_its_due_day` (added) |
| A7 | The core's engine day equals the engine's own timing of today on the same collection | the stub answers a day of 0 ending at instant 0 | `late.rs` `the_core_reads_the_engines_own_day` (added) |
| A8 | The web engine's card view carries `late` from the core's rule, in one place; a planted view without it is refused by name | the census finds no `late` key in `current_card` | `crates/web-engine/tests/late_view.rs` `the_shown_card_carries_the_cores_late_answer` (added) |
| A9 | A card the engine marks late shows the English line above the card, on the question and on the answer side | no line is drawn | `web/app/src/lib/study/late-line.test.ts` "a late card shows the line above the card" (added) |
| A10 | A card on time shows its question and no line, and the line goes when an on-time card follows a late one | not red: the base draws no line; it guards the absence beside a drawn card | `late-line.test.ts` "a card on time shows no line" (added) |
| A11 | Every locale holds `study_late_review` with its own streak word, and no locale but `en` holds the English text; each plant is refused by name | all 7 locales lack the key | `late-line.test.ts` "every locale holds the late line" (added) |

```acceptance
A1: cargo test -p deck-streak-engine-core --test late -- --exact a_review_card_due_yesterday_is_past_its_due_day
A2: cargo test -p deck-streak-engine-core --test late -- --exact a_review_card_due_today_is_not_past_its_due_day
A3: cargo test -p deck-streak-engine-core --test late -- --exact a_day_learning_card_is_judged_by_its_due_day
A4: cargo test -p deck-streak-engine-core --test late -- --exact a_filtered_card_is_judged_by_its_home_due
A5: cargo test -p deck-streak-engine-core --test late -- --exact an_intraday_learning_card_is_judged_by_the_days_start
A6: cargo test -p deck-streak-engine-core --test late -- --exact a_new_or_preview_card_is_never_past_its_due_day
A7: cargo test -p deck-streak-engine-core --test late -- --exact the_core_reads_the_engines_own_day
A8: cargo test -p deck-streak-web-engine --test late_view -- --exact the_shown_card_carries_the_cores_late_answer
A9: pnpm exec vitest run web/app/src/lib/study/late-line.test.ts -t "a late card shows the line above the card"
A10: pnpm exec vitest run web/app/src/lib/study/late-line.test.ts -t "a card on time shows no line"
A11: pnpm exec vitest run web/app/src/lib/study/late-line.test.ts -t "every locale holds the late line"
```

Each card fixture of A1 to A6 is built field by field in the test, with the engine's day given as
literal numbers; no expected value is computed by `past_due_day`. A7's oracle is the engine's own
timing of today, read through the engine's own interface on the same collection file, never through
the core. The census of A8 and A11 reports what it examined and refuses zero, and each carries
planted refusals.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-376-a-review-of-a-card-past-its-due-day-says-it-does-not-count-toward-the-streak-for-that-day.md` | docs | added |
| `docs/decisions/ADR-387-the-engine-core-decides-past-its-due-day-in-the-engines-own-day-and-the-web-review-shows-one-line.md` | docs | added |
| `docs/schematics/web-study-screens.md` | docs | one section appended at its end; no existing line changes |
| `docs/red-first/SPEC-376.md` | docs | added |
| `changelog.d/late-review-line-376.md` | docs | added |
| `scripts/mutation-rows.d/S37600-S37699.json` | docs | added (section 9) |
| `crates/engine-core/src/late.rs` | `engine-core` | added: the rule (R1, R2) |
| `crates/engine-core/src/lib.rs` | `engine-core` | `pub mod late;` and its line in the module list |
| `crates/engine-core/src/dispatch.rs` | `engine-core` | the core's own read of the engine's day (R3) |
| `crates/engine-core/tests/late.rs` | `engine-core` | added: A1 to A7 |
| `crates/web-engine/src/wasm.rs` | `web-engine` | `current_card`'s view carries `late` (R4) |
| `crates/web-engine/tests/late_view.rs` | `web-engine` | added: A8 |
| `web/app/src/lib/engine/protocol.ts` | `miniapp` | `CardView.late` (R4) |
| `web/app/src/lib/study/ReviewScreen.svelte` | `miniapp` | the line (R5, R6) |
| `web/app/messages/en.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/messages/es.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/messages/fr.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/messages/ja.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/messages/ko.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/messages/zh-Hans.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/messages/zh-Hant.json` | `miniapp` | `study_late_review` (R7) |
| `web/app/src/lib/study/late-line.test.ts` | `miniapp` | added: A9 to A11 |
| `web/app/src/lib/engine/session.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/answer-buttons.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/audio.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review-screen.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review-templates.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/voice.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |
| `web/app/src/routes/study.test.ts` | `miniapp` | `late: false` in its card view fixtures; no assertion changes |

The eight fixture files are those whose card view literals the type check refuses once `late` is
required; one the type check does not refuse is left unchanged and named in the pull request.

## 5. What this does NOT cover

- It does not let a late review rejoin the streak, and it changes no rule of SPEC-076 or SPEC-071;
  whether a review can rejoin the streak for a day that has passed is its own decision (`#739`).
- It adds no line to the native review: the native app holds no localized strings, so a line in
  every locale there needs its own string catalog first; the core's rule is the one the native
  review will read (`#738`).
- It does not tell a review that syncs after its own day has settled (SPEC-076 section 6): a review
  screen cannot know when the review will reach the server (`#739`).

## 6. Risks

- **The two clocks differ.** Where the kernel's rule and the engine's day do not end at the same
  instant (section 1b), the line can name a due day the kernel's current study day still overlaps.
  Detected by the server's own skip refusals, which name the differing zone or day
  (`crates/ingest/src/skip_write.rs:897-927`); bounded to the hours by which the two rollovers
  differ.
- **The day turns while a card is shown.** The view is built when the card is shown, so a card due
  today that is answered after the rollover shows no line. The line is then absent, never false:
  a card past its due day when shown is still past it when answered.
- **A device clock set backwards.** The engine's day moves back with it, and a line shown before
  the change can describe a day the clock no longer reaches. Not detected; the engine's own
  scheduling is equally moved.
- **A translation reads wrong.** Detected by A11 for the key, the streak word and an English copy,
  and by the owner's acceptance session for the sense.
- **A sibling change to the shared files.** Two open pull requests touch `wasm.rs`, `lib.rs`,
  `protocol.ts` and `session.test.ts`; the build re-measures each at its cut.

## 7. What only CI or a device proves

| # | what | where |
|---|---|---|
| C1 | The type check passes with `late` required on every card view | the pull request's `web` job (svelte-check) |
| C2 | The shipped study suite's seeded cards, all new, show no line | `web/app/tests-study/study.spec.ts`, shipped, in CI |
| C3 | Every changed production file under `web/app` has its mutants killed or recorded equivalent | `mutation-web`, on the pull request |
| C4 | The band's rows are killed by their named killers | the pull request's mutation job, by stem |
| V1 | On a device, a card past its due day shows the line in the app's locale, and an on-time card does not | the owner, in an acceptance session |

## 8. Formal model

None, by surface (ADR-387 D5). No actor, store or write path is added: the core reads the engine's
day and the shown card inside one synchronous export of the one Worker, and the only other mover is
the clock, which only advances, so a line true when shown stays true when answered (section 6).
The rule is a total function over the card's queue, tested on both sides of every boundary. Of the
220 `@phx covers` lines at the base, 0 name a web path, 1 names `crates/engine-core/src/dispatch.rs`
(`run_undo`) and 3 name `crates/web-engine/src/wasm.rs` (`undo`, `undo_offer`, `rate`); this
delivery leaves those four items byte for byte as they are.

## 9. Mutation rows

The band `S37600-S37699`, in `scripts/mutation-rows.d/S37600-S37699.json`, table `MUTATIONS`, ten rows
from `S37600`: one row per arm and boundary of `past_due_day` (the strict comparison of a day due, the
home deck due, the day-learning arm, the intraday start and its one-day constant, and the arm that
answers false), killed in `crates/engine-core/tests/late.rs`; one row on the core's call that reads
the engine's day, killed by A7; and one row on the view's `late` key, killed by A8's census, the one
native killer for code only the `wasm32` target compiles. Web code is proved by StrykerJS on the
pull request (C3), never by hand rows.
