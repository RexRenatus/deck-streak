---
status: "proposed"
---

# ADR-600: the web app's tagline drops Anki in all seven locales and keeps each locale's streak word

Decides SPEC-581. It is cut after SPEC-580 merges, whose README opening states the identity this
tagline follows.

## Context and Problem Statement

The web app's Today screen shows one tagline under its heading (`web/app/src/routes/+page.svelte`
line 31, `m.tagline()`). Read at dev before this delivery, line 3 of each of the seven catalogs in
`web/app/messages/` names Anki: "Turn your daily Anki reviews into a streak worth keeping." in
English (57 characters), and the same sentence in Spanish, French, Japanese, Korean, simplified and
traditional Chinese. `web/app/src/lib/smoke.test.ts` lines 19 to 21 require every locale's tagline
to name Anki, and `web/app/src/routes/today.test.ts` line 82 requires the English one. DeckStreak's
identity is a spaced-repetition study game whose own memory model, Rex, becomes its only scheduler
before 1.0 (ADR-599), and its README's opening names no Anki.

## Decision Drivers

- The tagline names no Anki and no Telegram, in every locale.
- Each locale keeps its own word for the streak, which two existing tests read from the tagline.
- The translations stay ones a reader of each language already had, where they can.
- The change is seen red first, by the tests that pin it.

## D1, the surfaces, measured: what it was chosen against

Line 3 of `en.json`, `es.json`, `fr.json`, `ja.json`, `ko.json`, `zh-Hans.json` and `zh-Hant.json`
is each locale's tagline. Its pins: `smoke.test.ts` lines 19 to 21 (every locale names Anki),
`today.test.ts` line 82 (the English text), `web/app/src/lib/study/late-line.test.ts` lines 24 to 32
and 194 to 197 and `web/app/src/lib/study/update-now.test.ts` lines 36 to 44 and 93 to 96 (each
locale's tagline holds its streak word: streak, racha, série, 連続記録, 연속 기록, 连续记录, 連續紀錄), and
`web/app/tests/smoke.spec.ts` lines 5 to 8, 23 and 49 (the rendered paragraph is the catalog's
tagline, read from the catalog itself). No snapshot holds the tagline.

- Chosen: every file that names the tagline or reads the catalogs' tagline, found by one fenced search, because a pin missed is a red found in CI
- The two pins the issue names alone: rejected because two more tests read every tagline for its streak word

## D2, the tagline: what it was chosen against

The English tagline becomes "Turn your daily reviews into a streak worth keeping." (52 characters
against 57), and each translation drops its one mention of Anki and nothing else: Spanish "Convierte
tus repasos diarios en una racha que vale la pena mantener." (69 against 77), French "Transformez vos
révisions quotidiennes en une série que vous aurez envie de prolonger." (86 against 91), Japanese
「毎日の復習を、続けたくなる連続記録に。」 (19 against 24), Korean "매일의 복습을 이어 가고 싶은 연속 기록으로
만드세요." (30 against 35), simplified Chinese 「把每天的复习变成值得坚持的连续记录。」 (18 against 22) and
traditional Chinese 「把每天的複習變成值得堅持的連續紀錄。」 (18 against 22). Each keeps its streak word and is in
Unicode NFC. With the Latin word gone, the Japanese and Chinese lines hold no mixed-script run.

- Chosen: the current sentence without Anki, because it is true before and after Rex schedules, needs no new claim, and keeps every translation a reader had
- "A spaced-repetition study game, with a streak worth keeping." (60 characters): rejected because it repeats the heading's context in a longer line and needs six new translations
- A tagline that names Rex: rejected because Rex does not yet schedule every review, and a tagline is no place for the condition

## D3, the shape, the tests and the rows: what it was chosen against

One delivery, cut after SPEC-580 merges. The red commit changes `smoke.test.ts` lines 19 and 21 (a
tagline in every locale that names no Anki and no Telegram, matched as a non-empty line) and
`today.test.ts` line 82 (the new English line); both read red on dev's catalogs, every locale and
every run. The green commit changes line 3 of the seven catalogs. The streak-word tests in
`late-line.test.ts` and `update-now.test.ts` stay unchanged and green at both commits: they are the
control that each translation kept its streak word. Every red runs locally under Vitest. No rows:
the rows table takes no Vitest killer (ADR-598 D7), and the changed lines are catalog data and test
literals. One push. FORMAL: not applicable to TLA+ (no interleaving) and not applicable to Lean (no
function changes; the catalogs are data).

- Chosen: Vitest reds on the two pins, with the streak-word tests as the control, because the pins already render the tagline the way a reader sees it
- A new census module over the catalogs: rejected because the existing tests already read every catalog's tagline
- One delivery with SPEC-580: rejected because the docs delivery changes no web file, and a web red belongs to the web suite

## D4, not this round: what it was chosen against

The `describe` name `the Mini App shell` in `smoke.test.ts` and every identifier that still names the
Mini App, the bot and the Mini App themselves, every screen's layout and copy other than line 3, and
how the tagline looks on a device.

- Chosen: line 3 of the seven catalogs and its two pins only, because each excluded item is a rename, a retirement or a device measurement of its own
- Rename the shell's `describe` now: rejected because the rename of the Mini App's identifiers is the retirement's

## Decision Outcome

D2 is built by SPEC-581 in one pull request, cut after SPEC-580 merges (D3); D1 measures it; D4
bounds it.

### Consequences

- Good, because the app's first line says what it does without naming another product.
- Good, because each translation changes by one word, so no locale gains new wording to review.
- Bad, because the tagline no longer tells a reader that their reviews come from Anki; the README's Built on section says it instead.

### Confirmation

SPEC-581's acceptance criteria A1 to A4, its red-first record, and CI's web suite and e2e smoke run at
the pull request's head.

## What would make this wrong

- A locale's reader finds the shortened line wrong or unnatural.
- A later screen shows the tagline where the streak word does not fit.

## More Information

Built by SPEC-581; drawn in `docs/schematics/tagline.md`. ADR-599 (the identity text), SPEC-376 and
ADR-387 (the streak words).
