# SPEC-606: The score screen's pillars each say in plain words what they measure

- **Issue:** #800
- **Context(s):** web (the score screen). The analytics context is read and tested, never changed.
- **Decided by:** ADR-627 (D1 to D7)
- **Status:** judged by this delivery, with its tests and docs/red-first/SPEC-606.md

## 1. The problem, measured

Every citation was read at commit `b065fce`.

1. **The score screen shows five bare figures.**
   - `web/app/src/lib/score/ScoreBreakdown.svelte:10-16` maps each pillar to its label function, and :24-44 render each pillar as one list item.
   - Each item holds the pillar's name in `<span id="pillar-{pillar}">` (:28), its rounded value or `m.retention_absent()` (:29-33), and a `<meter>` named by `aria-labelledby` (:35-42).
   - Nothing in the item says what the figure measures.
2. **The five pillars are five computations.** `crates/analytics/src/score.rs` holds them, with their constants at `crates/analytics/src/constants.rs:13-45`.
   - Consistency (:109-117): 0.7 times 100 on a day with a review, plus 0.6 times a streak bonus of 2 a day, capped at 50.
   - Retention (:119-131): the true retention scaled from `RETENTION_FLOOR_PCT` 50 to `RETENTION_CEIL_PCT` 95 (constants :23, :25), and blended halfway toward `RETENTION_BLEND_TARGET` 70 below `RETENTION_MIN_SAMPLE` 5 answers (:27, :29).
   - Workload (:133-151): the share of due review cards cleared, less the backlog divided by `WORKLOAD_BACKLOG_DIVISOR` 5, the penalty capped at `WORKLOAD_BACKLOG_PENALTY_CAP` 40 (:43, :45).
   - Volume (:153-165): the reviews' and the minutes' ratios to the baseline, weighted 0.6 and 0.4, capped at `VOLUME_CAP_RATIO` 1.25 and scaled so the cap scores 100 (:31-35).
   - Mastery (:167-176): 100 times the graduations over `MASTERY_GRADUATION_TARGET` 3, less `MASTERY_LEECH_PENALTY` 3 per active leech, the penalty capped at `MASTERY_LEECH_PENALTY_CAP` 30 (:37-41).
3. **The Mastery pillar is not a mastery percentage.**
   - A graduation is a review answer whose interval crosses from under 21 days to 21 or more (`crates/analytics/src/metrics.rs:40-41`, :122-127, with `MATURE_IVL_DAYS` at `constants.rs:11`).
   - An active leech is a card that is not suspended and has lapsed at least the leech threshold's times, 8 by default (`crates/analytics/src/settings.rs:5-10`, `crates/analytics/src/snapshot.rs:69`).
   - Road to C2 and the law block show mastery percentages, which SPEC-408 describes. ADR-422 D7 (:64-66, :121-124) left the score pillar for this follow-up.
4. **The breakdown's texts exist only in English.**
   - The component reads `m.score_title`, `m.score_pillars`, the five `m.pillar_*` labels and `m.retention_absent`.
   - `web/app/messages/en.json:14-22` holds those keys. `git grep -c` over `web/app/messages` for them and `score_none` counts 9 lines in `en.json` and none in the six other locale files.
   - `en.json` has 372 lines and each of the six others 242 (`wc -l`).
5. **The score reaches the screen through one route.** `GET /api/score` (`crates/api/src/analytics_routes.rs:35`, :55) answers the pillars by `score_json` (:120-134), and `web/app/src/lib/score/score.ts:60-67` parses them for the breakdown.
6. **No native screen shows a pillar.** `git grep -l -i pillar b065fce` lists no file under `ios`.

## 2. Requirements

R1. Each pillar's list item on the score breakdown ends with its sentence: a `p` element with the id `pillar-{pillar}-about`, the item's last element child, after the bar, or after the gap line when the retention is absent. It is ordinary visible text: no `title`, `hidden`, `aria-hidden`, `details` or tooltip, and no hover or tap is needed to see it.

R2. Each pillar's meter names its sentence by `aria-describedby="pillar-{pillar}-about"`, beside its `aria-labelledby`.

R3. Every shipped locale's message file carries the 13 texts the breakdown reads: `score_title`, `score_pillars`, the five `pillar_<pillar>` labels, `retention_absent` and the five `pillar_<pillar>_about` sentences. Each is not empty. Outside English, each sentence is not the English text and holds the English sentence's figures, and a locale's five sentences differ from one another.

R4. Every figure an English sentence states is the figure the shipped pillar function computes, from the fixed inputs A4 names.

R5. The English sentences are, byte for byte:

| key | English text |
|---|---|
| `pillar_consistency_about` | Whether you studied today and how long your streak is: studying today gives 70, and your streak adds up to 30 more as it grows to 25 days. |
| `pillar_retention_about` | The share of review cards you recalled on their first answer today: 50% or less scores 0, 95% or more scores 100, and with fewer than 5 answers it moves halfway toward 70. |
| `pillar_workload_about` | The share of the review cards due today that you cleared, less 1 point for every 5 overdue cards, at most 40. |
| `pillar_volume_about` | How much you studied today, in reviews and minutes, against your usual day: a usual day scores 80, and a quarter more scores 100. |
| `pillar_mastery_about` | Not a mastery percentage: 3 cards reaching a review interval of 21 days or more today score 100, and each active leech, a card you keep forgetting, takes off 3 points, at most 30. |

R6. The sentence carries spacing and size classes only: no colour class and no inline style.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | Over a day with every pillar, each of the five items' last element child is the `p` holding its English sentence, not hidden and untitled, and its meter's `aria-describedby` names that `p` (R1, R2, R5). | ADR-627 D1, D2, D5 |
| A2 | With the retention absent, the retention item shows the gap line, no meter, and the retention sentence last (R1). | ADR-627 D1 |
| A3 | For every locale in the app's locale list, each of the 13 texts the breakdown's own source reads is present and not empty, and each sentence holds the English figures, is not the English text outside English, and differs from the locale's four other sentences (R3). | ADR-627 D4 |
| A4 | The digit runs of the five English sentences are the figures `deck_streak_analytics::score` computes from fixed inputs, 19 in all (R4). | ADR-627 D3 |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/score/ScoreBreakdown.test.ts -t "each pillar says in plain words what it measures beside its figure"
A2: pnpm exec vitest run web/app/src/lib/score/ScoreBreakdown.test.ts -t "the retention gap still says what retention measures"
A3: pnpm exec vitest run web/app/src/lib/score/pillar-about-locales.test.ts -t "every locale carries every text the score breakdown shows with the English figures"
A4: cargo test -p deck-streak-analytics --test pillar_about -- --exact each_pillar_sentence_states_its_pillars_own_figures
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-606-the-score-screens-pillars-each-say-in-plain-words-what-they-measure.md` | docs | this SPEC, new |
| `docs/decisions/ADR-627-each-score-pillar-keeps-a-plain-sentence-shown-as-text-beside-its-figure.md` | docs | its decision, new |
| `docs/schematics/score-pillar-descriptions.md` | docs | each pillar from its function to its sentence, new |
| `changelog.d/score-pillar-descriptions-606.md` | docs | the changelog fragment, new |
| `docs/red-first/SPEC-606.md` | docs | the red-first record, new |
| `web/app/src/lib/score/ScoreBreakdown.test.ts` | web test | A1 and A2, new tests; the two existing tests' row literals grow by the sentence |
| `web/app/src/lib/score/pillar-about-locales.test.ts` | web test | A3, new |
| `crates/analytics/tests/pillar_about.rs` | analytics test | A4, new |
| `web/app/src/lib/score/ScoreBreakdown.svelte` | web | each pillar row ends with its sentence; the meter names it (R1, R2) |
| `web/app/messages/en.json` | web messages | the five `pillar_<pillar>_about` keys |
| `web/app/messages/es.json` | web messages | the 13 breakdown texts, in Spanish |
| `web/app/messages/fr.json` | web messages | the 13 breakdown texts, in French |
| `web/app/messages/ja.json` | web messages | the 13 breakdown texts, in Japanese |
| `web/app/messages/ko.json` | web messages | the 13 breakdown texts, in Korean |
| `web/app/messages/zh-Hans.json` | web messages | the 13 breakdown texts, in simplified Chinese |
| `web/app/messages/zh-Hant.json` | web messages | the 13 breakdown texts, in traditional Chinese |
| `scripts/mutation-rows.d/S60600-S60699.json` | rows | S60601 to S60603, new |

## 5. What this does NOT cover

- The bot's `/score` reply shows no pillar: it answers the total, the grade, the reviews and the true retention (`crates/bot/src/score_commands.rs:18-37`), so it has no pillar to describe, and #800 names the score screen.
- The score page's own texts outside the breakdown, such as `score_none`, and the app-wide keys the page reads (`loading`, `back_to_today` and the error lines) stay as they are in each locale: every web message in every locale is #1051.
- The grade label the breakdown shows under the total (`score.grade.label`, such as `SOLID`) stays as it is: it is an English string the server sends from its grade bands (`GRADE_BANDS` in `crates/analytics/src/constants.rs`), not one of the 13 texts R3 names, so no locale file holds it and this delivery does not translate it.
- Any change to a pillar's computation: each sentence states its pillar as it is computed at `b065fce`. Judging the retention pillar against each card's desired retention is #727.
- The iPhone and iPad app, which has no score screen at `b065fce`: a native view of each web screen is #1057, and it carries these sentences when it is built.

## 6. Risks

- **A pillar function changes and its sentence goes false.** Detected by: A4 fails, because it computes each stated figure from the shipped functions.
- **A locale loses a text, or a translation drops a figure.** Detected by: A3 fails for that locale and key.
- **A translation reads wrong.** The six translations are written without a native reader. Detected by: the review in section 7. A3 proves only presence, difference from English, distinctness and the figures.
- **The message files collide with open work.** Other open changes edit all seven files. Detected by: the builder re-measures each file at its cut. The keys go beside their siblings: in `en.json` after `retention_absent`, in the six others after `tagline`.
- **The sentences crowd a small screen.** Each wraps as body text under its bar. The rendered audit in CI (`web/app/tests/a11y.spec.ts`, whose fixture shows the score page) judges contrast and reflow.

## 7. What only a person proves

- A reader of each of the six translated languages reads the 13 texts beside their figures and confirms that they say what the English says. The review records each locale's verdict on the pull request, and a fluent review of every message is #1053.
- A screen-reader walk of the score page confirms that each pillar's sentence is read after its name and figure, and that its meter announces the sentence as its description.

## 8. Mutation testing

- Hand rows in `scripts/mutation-rows.d/S60600-S60699.json`, all on `crates/analytics/src/constants.rs`, each killed by `pillar_about::each_pillar_sentence_states_its_pillars_own_figures`:
  - `S60601-PILLAR-ABOUT-LEECH-STEP`: each active leech takes 4 points, not 3 (`pub const MASTERY_LEECH_PENALTY: f64 = 3.0;` becomes `pub const MASTERY_LEECH_PENALTY: f64 = 4.0;`).
  - `S60602-PILLAR-ABOUT-VOLUME-CAP`: the volume cap moves to one and a half times the usual day (`pub const VOLUME_CAP_RATIO: f64 = 1.25;` becomes `pub const VOLUME_CAP_RATIO: f64 = 1.5;`).
  - `S60603-PILLAR-ABOUT-RETENTION-FLOOR`: the retention floor moves to 55% (`pub const RETENTION_FLOOR_PCT: f64 = 50.0;` becomes `pub const RETENTION_FLOOR_PCT: f64 = 55.0;`).
- StrykerJS judges `ScoreBreakdown.svelte` whole, at break 100 (`web/app/stryker.config.json`). A1's exact sentence, id and `aria-describedby` assertions kill the mutants of the new sentence map and markup.

## 9. Formal

Not required. The delivery adds five static sentences, their translations and an `aria-describedby`: no actor, no shared state, no protocol and no write path.
