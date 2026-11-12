# SPEC-408: Each mastery percentage says in plain words what it measures, on every surface that shows it

- **Issue:** #799
- **Context(s):** web (the Mini App's law block and Road to C2) and bot (the progress reply). The curriculum context is read and tested, never changed.
- **Decided by:** ADR-422 (D1 to D7)
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-408.md`

## 1. The problem, measured

Every citation was read at commit `a68db18a`.

1. **Two different measures carry the word "mastery".**
   - Course mastery is `card_mastery` (`crates/curriculum/src/progress.rs:85-112`) averaged by `course_progress` (:141-236) over each band's counted cards and over the course, times 100 (SPEC-077 R2 to R4).
   - Law mastery is `mastery_pillar` (`crates/curriculum/src/law.rs:10-19`): 100 less 3 points per active law leech, the penalty capped at 30 (SPEC-077 R9).
   - They are different functions of different inputs: each counted card's memory state, against the count of active law leeches.
2. **The surfaces show the figure bare.**
   - **Law block.** `web/app/src/lib/law/LawBlock.svelte:30` renders `m.law_mastery({ pct })` ("Law mastery: {pct}%", `web/app/messages/en.json:130`) inside `<li data-line={key}>` (:48).
   - **Road to C2, summary.** `web/app/src/lib/progress/CourseLadder.svelte:17-18` renders `m.progress_summary` ("Band {band}, {mastery}% mastery.", `en.json:114`).
   - **Road to C2, band cells.** Each band cell renders `progress_band_mastery` (`en.json:118`) beside a meter labelled by `progress_band_meter` (`en.json:122`).
   - **Bot.** The bot's `course_line` formats `{}% mastery` (`crates/bot/src/progress_commands.rs:59-70`), and `progress_reply` (:21-28) joins the course lines under "Road to C2".
   - None of these says what the figure measures.
3. **Seven message files carry the keys.**
   - `en.json` has 269 lines.
   - `es`, `fr`, `ja`, `ko`, `zh-Hans` and `zh-Hant` have 169 lines each (`wc -l web/app/messages/*.json`), and they carry the same progress and law keys at lines 40, 44, 48, 56 and 60.
   - No test checks that a key is present in every locale. `web/app/src/lib/sync/sync-locales.test.ts` checks its own keys that way.
4. **Literals this delivery changes are pinned in five places.**
   - `web/app/src/lib/law/LawBlock.test.ts:75` expects `Law mastery: 94%`.
   - `crates/bot/tests/messages/progress.msg.json:21` holds the progress reply's text. `progress_shows_each_course_band_and_mastery` (`crates/bot/tests/progress_commands.rs:59`) and `every_golden_message_is_what_the_bot_sends` (`crates/bot/tests/commands.rs:556`) compare the reply with it.
   - `crates/daemon/tests/role_bot.rs:319` holds a progress reply in full.
   - `CourseLadder.test.ts`, `routes/progress.test.ts` and `routes/law.test.ts` pin no literal this delivery changes.
5. **The law figure is pending in production.** `crates/api/src/law_routes.rs:52` passes no leech count, because the leech port is not wired (#133). Coordination shows the mastery line only when the leech count is not 0 and the figure is above 0 (`crates/coordination/src/law/mod.rs:79-86`).
6. **The score screen's Mastery pillar is a third measure, not a percentage.** `crates/analytics/src/score.rs:167-176` computes it from graduations against a target of 3, less a leech penalty. `web/app/src/lib/score/ScoreBreakdown.svelte:32` shows it as a bare score from 0 to 100 among five pillars. ADR-422 D7 keeps it as it is.
7. **The native clients show no mastery figure.** `git grep -l -i mastery a68db18a -- ios` lists 0 files.
8. **No formal model and no mutation row covers a file this delivery edits.** No `@phx covers` line under `formal/` names an edited file: 0 of 239. No mutation row anchors on an edited file: `progress_commands`, `LawBlock` and `CourseLadder` appear in no file under `scripts/`.

## 2. Requirements

R1. The law block's mastery line shows the law sentence (`law_mastery_about`) as visible text inside the same list item, after the figure. The pending mastery line shows no sentence.

R2. Each course on Road to C2 shows the course sentence (`progress_mastery_about`) once, as a paragraph inside the course's section, directly after its summary paragraph.

R3. When the bot's progress reply lists at least one course, it ends with one line: the course sentence in italics, passed through `escape_html`. The no-course reply and the failed reply are unchanged.

R4. The two sentences are different texts. The law block never shows the course sentence, and Road to C2 and the bot never show the law sentence.

R5. Every shipped locale's message file carries both keys. Each value is non-empty. Outside English it is not the English text. It holds the same numbers as the English sentence.

R6. The English law sentence's numbers are `mastery_pillar`'s: the value at 0 leeches, the step from 0 to 1 leech, and the floor.

R7. The English course sentence calls the figure an estimate, names no memory-model term, and holds no number but 0 (ADR-422 D1).

R8. The bot's course sentence equals the English message file's `progress_mastery_about`.

R9. Neither web sentence depends on hover or on an extra act to be seen, and neither sets a colour of its own (ADR-422 D2).

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | The law block's shown mastery line holds the law sentence after its figure, inside the same list item. No other shown line holds a sentence. A block with the mastery pending holds none. The course sentence appears nowhere in the block (R1, R4). | ADR-422 D1, D2 |
| A2 | Each course's summary paragraph is followed directly by a paragraph holding the course sentence. The section holds exactly one sentence, and the law sentence appears nowhere in it (R2, R4). | ADR-422 D1, D2 |
| A3 | Every locale in the app's locale list has both keys in its message file. Each value is non-empty, not the English text outside English, and different from the other key's value, and it holds the same numbers as English (R5, R4). | ADR-422 D4 |
| A4 | The bot's progress reply for stored courses ends with the italic course sentence, equal to `en.json`'s `progress_mastery_about`. The golden reply and the bot role's reply carry that line (R3, R8). | ADR-422 D2, D4 |
| A5 | The English law sentence's numbers are, in order, `mastery_pillar(0)`, `mastery_pillar(0) - mastery_pillar(1)` and `mastery_pillar(1000)` (R6). | ADR-422 D1, D5 |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/law/LawBlock.test.ts -t "the mastery line says what law mastery measures and no other line does"
A2: pnpm exec vitest run web/app/src/lib/progress/CourseLadder.test.ts -t "each course says what its mastery measures right after its summary"
A3: pnpm exec vitest run web/app/src/lib/mastery-about-locales.test.ts -t "every locale carries both mastery descriptions with the English figures"
A4: cargo test -p deck-streak-bot --test progress_commands -- --exact the_progress_reply_ends_with_the_course_mastery_description
A4: cargo test -p deck-streak-bot --test progress_commands -- --exact progress_shows_each_course_band_and_mastery
A4: cargo test -p deck-streak-daemon --test role_bot -- --exact the_bot_role_answers_progress_from_the_courses_its_settings_name
A5: cargo test -p deck-streak-curriculum --test mastery_about -- --exact the_law_mastery_description_states_the_pillars_own_figures
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-408-each-mastery-percentage-says-in-plain-words-what-it-measures-on-every-surface-that-shows-it.md` | docs | this SPEC, new |
| `docs/decisions/ADR-422-each-mastery-measure-keeps-its-own-plain-sentence-shown-as-text-beside-its-figure.md` | docs | its decision, new |
| `docs/schematics/each-mastery-figure-from-its-computation-to-its-plain-description.md` | docs | each figure from its computation to its surfaces and sentences, new |
| `docs/red-first/SPEC-408.md` | docs | the red-first record, new |
| `changelog.d/mastery-description-408.md` | docs | the changelog fragment, new |
| `web/app/src/lib/law/LawBlock.svelte` | web | the mastery line carries the law sentence (R1) |
| `web/app/src/lib/law/LawBlock.test.ts` | web test | A1, and the `:75` literal gains the sentence |
| `web/app/src/lib/progress/CourseLadder.svelte` | web | the course sentence after each summary (R2) |
| `web/app/src/lib/progress/CourseLadder.test.ts` | web test | A2 |
| `web/app/src/lib/mastery-about-locales.test.ts` | web test | A3, new |
| `web/app/messages/en.json` | web messages | `progress_mastery_about` and `law_mastery_about` |
| `web/app/messages/es.json` | web messages | both keys, in Spanish |
| `web/app/messages/fr.json` | web messages | both keys, in French |
| `web/app/messages/ja.json` | web messages | both keys, in Japanese |
| `web/app/messages/ko.json` | web messages | both keys, in Korean |
| `web/app/messages/zh-Hans.json` | web messages | both keys, in simplified Chinese |
| `web/app/messages/zh-Hant.json` | web messages | both keys, in traditional Chinese |
| `crates/bot/src/progress_commands.rs` | bot | `MASTERY_ABOUT` and the reply's last line (R3, R8) |
| `crates/bot/tests/progress_commands.rs` | bot test | A4 |
| `crates/bot/tests/messages/progress.msg.json` | bot golden | the reply's text gains its last line |
| `crates/daemon/tests/role_bot.rs` | daemon test | the `:319` literal gains the last line |
| `crates/curriculum/tests/mastery_about.rs` | curriculum test | A5, new |
| `scripts/mutation-rows.d/S40800-S40899.json` | rows | S40801 to S40803, new |

## 5. What this does NOT cover

- The leech port, and so the law figure leaving pending in production: until it is wired, the law sentence is not seen in production (#133).
- The Road to C2 chart's mastery bars: they carry the course sentence when they are built (#152).
- The today snapshot and the bot's today reply, which will show each course's mastery and the law block: they carry these sentences when they are built (#69).
- The digest and the weekly report: they carry these sentences if they show a mastery figure when they are built (#129, #130).
- The agent read tool's law-track text, which its own golden pins, is unchanged (#157).

## 6. Risks

- **A translation reads wrong.** Japanese and Korean especially are written without a native reader. Detected by: the review in section 7. The locale census (A3) proves only presence, difference from English and the numbers.
- **The message files collide with open work.** Other open changes edit all seven files. Detected by: the builder re-measures each file at its cut. Each key is inserted beside its sibling, never at the file's end.
- **The course computation changes and the sentence goes false.** No test here detects it. ADR-422's "What would make this wrong" names the conditions, and a change to `card_mastery` revisits the sentence.
- **The law pillar changes.** Detected by: A5 fails.
- **The sentence crowds a small screen.** It wraps as body text. The rendered audit in CI (`web/app/tests/a11y.spec.ts`, whose fixtures already show both pages) judges contrast and reflow.
- **The bot reply grows past the message limit.** The sentence adds one line of about two hundred characters, and the transport chunks any longer text.

## 7. What only a person proves

- A reader of each of the six translated languages reads both sentences beside their figures and confirms that they say what the English says. The review records each locale's verdict on the pull request.
- A screen-reader walk of the law page (with a fixture showing the mastery line) and of Road to C2 confirms that each sentence is read after its figure.

## 8. Mutation testing

- Hand rows in `scripts/mutation-rows.d/S40800-S40899.json`, all on `crates/bot/src/progress_commands.rs`, each killed by `progress_commands::the_progress_reply_ends_with_the_course_mastery_description`:
  - `S40801-PROGRESS-MASTERY-ABOUT-LINE`: the reply's last line is dropped (`lines.push(format!("<i>{}</i>", escape_html(MASTERY_ABOUT)));` becomes `let _ = MASTERY_ABOUT;`).
  - `S40802-PROGRESS-MASTERY-ABOUT-TEXT`: the constant's sentence changes (`"Mastery is an estimate from your reviews` becomes `"Mastery is a measure from your reviews`).
  - `S40803-PROGRESS-MASTERY-ABOUT-ITALIC`: the italic markup becomes bold (`format!("<i>{}</i>", escape_html(MASTERY_ABOUT))` becomes `format!("<b>{}</b>", escape_html(MASTERY_ABOUT))`).
- StrykerJS judges `LawBlock.svelte` and `CourseLadder.svelte` whole, at break 100. A1's assertions that exactly one shown line holds a sentence, and that a pending block holds none, kill the mutants of the mastery-line condition.

## 9. Formal

NOT APPLICABLE by surface (ADR-422 D5). The delivery adds display text and a test-only read of a message file. It adds no actor, no shared state and no write path, and 0 of the 239 `@phx covers` lines name a file it edits.
