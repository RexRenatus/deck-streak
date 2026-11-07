# SPEC-581: the web app's tagline names no Anki in any of its seven locales

- **Issue:** named by the pull request. **Context(s):** the web app's catalogs (`web/app/messages/`)
  and its tests (`web/app/src/`).
- **Decided by:** ADR-600 (the web app's tagline drops Anki in all seven locales and keeps each
  locale's streak word); ADR-599 (the identity text).
- **Schematic:** `docs/schematics/tagline.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-581.md`, cut after SPEC-580 merges. **Mutation band:** none (ADR-600 D3: the
  rows table takes no Vitest killer). **Models:** none (ADR-600 D3).

## 1. The problem, measured

Read at dev before this delivery, by `git show <dev>:<path>` with `grep -n` and line ranges:

- Line 3 of each of the seven catalogs is its tagline, and each names Anki: `en.json` "Turn your
  daily Anki reviews into a streak worth keeping." (57 characters), `es.json` (77), `fr.json` (91),
  `ja.json` (24), `ko.json` (35), `zh-Hans.json` (22) and `zh-Hant.json` (22).
- `web/app/src/lib/smoke.test.ts` (24 lines) lines 19 to 21 require every locale's tagline to
  contain "Anki"; `web/app/src/routes/today.test.ts` (117 lines) line 82 requires the English text.
- `web/app/src/lib/study/late-line.test.ts` lines 24 to 32 and 194 to 197, and
  `web/app/src/lib/study/update-now.test.ts` lines 36 to 44 and 93 to 96, require each locale's
  tagline to hold its streak word.
- `web/app/tests/smoke.spec.ts` lines 5 to 8, 23 and 49 compare the rendered paragraph with the
  catalog's own tagline; `web/app/src/routes/+page.svelte` line 31 renders it.

## 2. Requirements

R1. Line 3 of each catalog is its new tagline: `en.json` "Turn your daily reviews into a streak worth
    keeping."; `es.json` "Convierte tus repasos diarios en una racha que vale la pena mantener.";
    `fr.json` "Transformez vos révisions quotidiennes en une série que vous aurez envie de
    prolonger."; `ja.json` "毎日の復習を、続けたくなる連続記録に。"; `ko.json` "매일의 복습을 이어 가고 싶은 연속
    기록으로 만드세요."; `zh-Hans.json` "把每天的复习变成值得坚持的连续记录。"; `zh-Hant.json`
    "把每天的複習變成值得堅持的連續紀錄。". Each is in Unicode NFC; no other line of a catalog changes.
R2. `smoke.test.ts` lines 19 and 21 require a tagline in every locale that is not empty and names no
    Anki and no Telegram; `today.test.ts` line 82 requires the new English tagline.
R3. Each locale's tagline holds its streak word, as `late-line.test.ts` and `update-now.test.ts`
    already require; neither test changes.
R4. No other web, Rust, Swift or document file changes, the shell's `describe` name included; no
    screen's layout changes.

## 3. Acceptance criteria of SPEC-581

| id | criterion | decided by |
|---|---|---|
| A1 | every locale's tagline is a non-empty line that names no Anki and no Telegram (R1, R2) | `smoke.test.ts` |
| A2 | the Today screen renders the new English tagline (R1, R2) | `today.test.ts` |
| A3 | every locale's tagline holds its streak word, read by the late line's test (R3) | `late-line.test.ts` |
| A4 | every locale's tagline holds its streak word, read by the update offer's test (R3) | `update-now.test.ts` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/smoke.test.ts -t "has a tagline in every locale that names no Anki and no Telegram"
A2: pnpm exec vitest run web/app/src/routes/today.test.ts -t "Today names the app and the screen"
A3: pnpm exec vitest run web/app/src/lib/study/late-line.test.ts -t "every locale holds the late line"
A4: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "every locale holds the offer words"
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/messages/en.json` | web | changed: line 3 (R1) |
| `web/app/messages/es.json` | web | changed: line 3 (R1) |
| `web/app/messages/fr.json` | web | changed: line 3 (R1) |
| `web/app/messages/ja.json` | web | changed: line 3 (R1) |
| `web/app/messages/ko.json` | web | changed: line 3 (R1) |
| `web/app/messages/zh-Hans.json` | web | changed: line 3 (R1) |
| `web/app/messages/zh-Hant.json` | web | changed: line 3 (R1) |
| `web/app/src/lib/smoke.test.ts` | tests | changed: lines 19 and 21, A1 |
| `web/app/src/routes/today.test.ts` | tests | changed: line 82, A2 |
| `docs/red-first/SPEC-581.md` | records | new |
| `docs/specs/SPEC-581-the-web-apps-tagline-names-no-anki-in-any-of-its-seven-locales.md` | docs | new: this file |
| `docs/decisions/ADR-600-the-web-apps-tagline-drops-anki-in-all-seven-locales-and-keeps-each-locales-streak-word.md` | docs | new |
| `docs/schematics/tagline.md` | docs | new |
| `changelog.d/ds-tagline-581.md` | changelog | new |

## 5. What this does NOT cover

- The shell's `describe` name and every identifier that still names the Mini App, which change with the retirement of the bot and the Mini App (#1046).
- How the tagline looks on a device and in the forge's render of the app, read after the pull request (#1047).

## 6. Risks

- **A translation that loses its streak word.** A3 and A4 read every catalog's tagline for it, at the
  red commit and the green one.
- **A pin outside the two named.** `web/app/tests/smoke.spec.ts` compares the rendered paragraph
  with the catalog's own tagline, so it follows the catalog; CI's end-to-end run reads it by name.
- **An open pull request that changes a catalog.** Three open pull requests change the seven
  catalogs elsewhere; the build re-reads line 3 of each at its cut and stops if any differs.
- **A green that selected nothing.** Each acceptance command names its test; the build quotes each
  run's test count and refuses a run of 0.
