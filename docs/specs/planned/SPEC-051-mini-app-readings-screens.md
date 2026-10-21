# SPEC-051: the Mini App shows one card per topic on Today, a focused reader with the read tap and a live studied chip, and the history by ISO week

- **Wave:** W1. **Issue:** #37 (epic #2). **Context(s):** the Mini App (`web/app`); `deck-streak-api` (the read-only routes); `deck-streak-coordination` (the views).
- **Decided by:** ADR-005 (a SvelteKit SPA in Telegram's webview), ADR-006 (every request owner-only),
  ADR-012 (Vitest for the Mini App), ADR-019 (the Mini App is the primary reading surface), ADR-037
  (a sync once per study day plus the owner's triggers, and never on opening the app), ADR-054 (with
  no AI route, the Mini App says readings are not enabled), and ADR-051 (a closed renderer, the day's
  offline cache, and the audit in Vitest's browser mode).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-051.md` (ADR-016).

## 1. The problem, measured

- **The only surface was an Obsidian note.** The predecessor's readings, had one ever been written,
  would have been read only in the vault; the owner's own action was invisible to code, and no
  screen browsed the archive (the second-brain inventory, private). The owner decided that the Mini
  App is the primary surface, with the vault an archive copy (ADR-019).
- **What the owner needs from it.** One card per topic with an honest state (no new cards, could not
  tell, paused and failed never blur, and nothing is a placeholder); a focused reader that renders the
  gated text and nothing else; the read tap; a studied chip that moves with the owner's study; the
  history by ISO week; and Regenerate, tap to pick.
- **Why the reader cannot trust the body.** A reading is AI text grounded in card text the owner
  wrote; it is rendered on an origin that holds the owner's session. The body's markup is data.
- **Prerequisites.** SPEC-028 (the Mini App shell, the Telegram wrapper, the startapp token map and
  the API handshake), SPEC-025 (the API shell), SPEC-046, SPEC-047 and SPEC-048 (readings, the read
  route and the regenerate route). SPEC-049 and SPEC-050 add to these screens afterwards.

## 2. Requirements

R1. Today shows one card per topic of the server's study day, with the topic's display name from the
    taxonomy, its state, its new-card and note counts, and, for a ready reading, its reading minutes,
    its read state and its studied chip. Each state has its own words: ready; "No new cards today",
    with no reading and no placeholder; could not tell, with words for `rail_broken` and for
    `config_fault` that never merge; paused ("Paused after two days without study"); and failed, with
    its closed reason. No two states render the same.
R2. The reader renders the gated text only, through a parser (`web/app/src/lib/readings/render.ts`)
    that emits text, headings, paragraphs, lists, blockquotes, emphasis and ruby annotations built
    from their validated form. It never uses `{@html}`, renders a link as its text and never as an
    anchor, and loads no remote resource. A scroll indicator shows the progress through the text. A
    language reading's container carries its `lang` (the persona output's), so CJK text carries its
    language.
R3. The reader's "I read it" button calls the read route (SPEC-047) and then shows the reading as
    read; a vault tick recorded `pending` shows as pending, with a tap that retries it.
R4. The studied chip reads "Studied n/N" from the reading view, which is fetched again when the reader
    regains focus and after an owner-triggered sync: returning from the bot's `/sync` (SPEC-026)
    regains focus, and a Mini App action that triggers a sync, once one exists (ADR-037), fetches the
    view again when it answers. The reader sets no refresh timer, because the view changes only with a
    sync, and opening the Mini App never triggers one (ADR-037).
R5. An opened reading is kept in the webview's local storage under a key carrying the server's study
    day and served from there when the network is unavailable; every key of an earlier study day is
    removed when the server's study day changes. `privacy.json` declares this device cache.
R6. The history lists past readings by ISO week (Monday to Sunday) with each reading's topic, read
    state and studied verdict, and opens any of them read-only in the reader.
R7. Each topic card has a Regenerate button that sends the topic's pick token (SPEC-048): `started`
    shows the topic generating and refreshes the card when the generation ends; `busy` says a
    generation is already running; a failure shows its closed reason beside the reading that stands.
R8. `GET /api/readings/today`, `GET /api/readings/{id}` and `GET /api/readings/history?week=<ISO week>`
    answer the authenticated owner only, from coordination's views.
R9. The readings routes pass the axe-core audit with the tags `wcag2a`, `wcag2aa`, `wcag21a`,
    `wcag21aa` and `wcag22aa` in both Telegram colour schemes, run in a real browser through Vitest's
    browser mode (ADR-051), and the cjk-typography pack's rows are green over the readings screens.
R10. The startapp token `r_<reading id>` opens that reading in the reader; an unknown reading id opens
    Today (SPEC-028's token map learns the prefix).
R11. With the AI route absent (ADR-054), the today view says so, and Today shows "Readings are not
    enabled" in place of the topic cards: no topic card, no Regenerate button, and no failed or
    could-not-tell state, because nothing failed. The words are their own, never a failure's and never
    a placeholder reading.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | Today shows one card per topic with its honest state, every state in its own words | `shows one card per topic with its honest state` |
| A2 | a topic with no new cards shows "No new cards today" and no reading or placeholder | `a topic with no new cards shows no placeholder reading` |
| A3 | the reader renders no HTML element and no anchor from the body, even when the body holds markup and links | `renders no html and no link from the body` |
| A4 | an opened reading is served offline for its study day and removed when the study day changes | `serves the reading of the day offline and clears it at the rollover` |
| A5 | with a mocked API, the studied chip refreshes when the reader regains focus after a sync that brought reviews, and no timer fetches it | `refreshes on focus after a sync that brings reviews and sets no timer` |
| A6 | the readings routes pass the axe-core audit in both Telegram colour schemes in a real browser | accessibility runtime audit; `the readings routes pass axe in both colour schemes` |
| A7 | a language reading's container carries its `lang`, and furigana renders as ruby | cjk-typography `cjk-lang`; `a language reading carries its lang and ruby` |
| A8 | the cjk-typography rows are green over the readings screens with non-zero examined counts | cjk-typography, every row; `test_the_cjk_typography_rows_are_green_over_the_readings_screens` |
| A9 | the history groups past readings by ISO week and opens one read-only | `groups past readings by ISO week` |
| A10 | Regenerate shows generating after `started` and a running generation after `busy` | `regenerate shows generating or busy` |
| A11 | an `r_` startapp token opens the reader, and an unknown id opens Today | `an r_ token opens the reader and an unknown id opens Today` |
| A12 | the three reading routes answer the owner only | `the_reading_routes_answer_only_the_owner` |
| A13 | the today view holds exactly one entry per topic of the study day, with its state, class and reason | `the_today_view_holds_one_entry_per_topic` |
| A14 | with the route absent, Today says "Readings are not enabled" and shows no topic card, no Regenerate and no failure | `says readings are not enabled when the route is absent` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/readings/TodayReadings.test.ts -t "shows one card per topic with its honest state"
A2: pnpm exec vitest run web/app/src/lib/readings/TodayReadings.test.ts -t "a topic with no new cards shows no placeholder reading"
A3: pnpm exec vitest run web/app/src/lib/readings/render.test.ts -t "renders no html and no link from the body"
A4: pnpm exec vitest run web/app/src/lib/readings/offline.test.ts -t "serves the reading of the day offline and clears it at the rollover"
A5: pnpm exec vitest run web/app/src/lib/readings/StudiedChip.test.ts -t "refreshes on focus after a sync that brings reviews and sets no timer"
A6: pnpm exec vitest run web/app/src/routes/readings/readings.a11y.browser.test.ts -t "the readings routes pass axe in both colour schemes"
A7: pnpm exec vitest run web/app/src/lib/readings/render.test.ts -t "a language reading carries its lang and ruby"
A8: python3 -m unittest discover -s scripts/tests -p test_readings_screens_rows.py -k test_the_cjk_typography_rows_are_green_over_the_readings_screens
A9: pnpm exec vitest run web/app/src/routes/readings/history/history.test.ts -t "groups past readings by ISO week"
A10: pnpm exec vitest run web/app/src/lib/readings/TodayReadings.test.ts -t "regenerate shows generating or busy"
A11: pnpm exec vitest run web/app/src/lib/startapp.test.ts -t "an r_ token opens the reader and an unknown id opens Today"
A12: cargo test -p deck-streak-api --test readings_views -- --exact the_reading_routes_answer_only_the_owner
A13: cargo test -p deck-streak-coordination --test readings_views -- --exact the_today_view_holds_one_entry_per_topic
A14: pnpm exec vitest run web/app/src/lib/readings/TodayReadings.test.ts -t "says readings are not enabled when the route is absent"
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/lib/readings/api.ts` | miniapp | added: the readings client |
| `web/app/src/lib/readings/types.ts` | miniapp | added |
| `web/app/src/lib/readings/TodayReadings.svelte` | miniapp | added |
| `web/app/src/lib/readings/TodayReadings.test.ts` | miniapp | added |
| `web/app/src/lib/readings/render.ts` | miniapp | added: the closed renderer |
| `web/app/src/lib/readings/Body.svelte` | miniapp | added |
| `web/app/src/lib/readings/render.test.ts` | miniapp | added |
| `web/app/src/lib/readings/offline.ts` | miniapp | added: the day's cache |
| `web/app/src/lib/readings/offline.test.ts` | miniapp | added |
| `web/app/src/lib/readings/StudiedChip.svelte` | miniapp | added |
| `web/app/src/lib/readings/StudiedChip.test.ts` | miniapp | added |
| `web/app/src/routes/+page.svelte` | miniapp | changed: Today gains the readings cards |
| `web/app/src/routes/readings/[id]/+page.svelte` | miniapp | added: the reader |
| `web/app/src/routes/readings/history/+page.svelte` | miniapp | added |
| `web/app/src/routes/readings/history/history.test.ts` | miniapp | added |
| `web/app/src/routes/readings/readings.a11y.browser.test.ts` | miniapp | added |
| `web/app/src/lib/startapp.ts` | miniapp | changed: the `r_` prefix |
| `web/app/src/lib/startapp.test.ts` | miniapp | changed |
| `web/app/messages/en.json` | miniapp | changed: the readings strings |
| `vitest.config.ts` | repo | changed: the root's Vitest projects gain a browser project (headless Chromium) for `web/app/src/**/*.browser.test.ts`, which the Node project excludes; the acceptance lines run from here |
| `web/app/vite.config.ts` | miniapp | changed: the app's own Vitest run (`pnpm -r test`, the gate's web stage) keeps `*.browser.test.ts` out of Node and runs it in the same browser project |
| `web/app/package.json` | miniapp | changed: dev dependencies `@vitest/browser-playwright` and `axe-core` (ADR-051) |
| `pnpm-lock.yaml` | repo | changed |
| `crates/coordination/src/readings/views.rs` | `deck-streak-coordination` | added: today, reading and history views; the today view carries whether an AI route is configured |
| `crates/coordination/tests/readings_views.rs` | `deck-streak-coordination` | added |
| `crates/api/src/readings_routes.rs` | `deck-streak-api` | changed: the three read-only routes |
| `crates/api/tests/readings_views.rs` | `deck-streak-api` | added |
| `scripts/tests/test_readings_screens_rows.py` | repo | added |
| `privacy.json` | repo | changed: the device cache |
| `docs/specs/SPEC-051-mini-app-readings-screens.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-051-closed-renderer-day-cache-and-browser-audit.md` | docs | added |
| `docs/red-first/SPEC-051.md` | docs | added |

## 5. What this does NOT do

- It shows no carried-nights badge and no comeback reading; the comeback delivery adds both
  (#35).
- It has no status panel (#36).
- It has no settings for the readings' switches (#57).
- It draws no chart of the reading history (#152).
- It sends nothing through the bot (#38).

## 6. Risks

- **The renderer meets markup it does not know.** It renders it as text; the render tests hold the
  golden readings, ruby included.
- **Telegram clears the webview's storage.** The cache is a convenience; the reader fetches again when
  online, and nothing depends on the cache.
- **The browser-mode audit is slow or flaky in CI.** It runs headless Chromium, the same browser the
  shell's Playwright smoke test installs; a failure names the rule and the route.
- **Reading text sits on the owner's device.** It is the owner's own text on the owner's device,
  declared in the privacy policy and removed at the next study day.
