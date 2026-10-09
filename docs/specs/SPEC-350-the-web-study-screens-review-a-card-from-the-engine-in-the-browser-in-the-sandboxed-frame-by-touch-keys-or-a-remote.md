# SPEC-350: the web study screens review a card from the engine in the browser, in the sandboxed frame, by touch, keys or a remote

- **Wave:** the app campaign, Phase 1 (SPEC-334 row 1.4, and its R5, R15 and R18). **Issue:** #630 (the web
  study screens). **Context(s):** `miniapp` (`web/app/src`), `deck-streak-web-engine`
  (`crates/web-engine`), `deck-streak-engine-core` (`crates/engine-core`), CI.
- **Decided by:** ADR-361 (this SPEC's own: the shown-card rule, the card view, the frame's size
  and classes, focus and the key switch, media, voices and installability), ADR-336 (the engine in
  the browser, its size budget), ADR-338 (the next interval on the answer buttons), ADR-342 (the
  remote's web clause) and ADR-352 (the card frame).
- **Schematic:** `docs/schematics/web-study-screens.md` (the component diagram and the review loop;
  this delivery adds it).
- **Status:** two pull requests. Part 1 (this fence) builds the engine surface, the Worker's study
  operations, the deck list and the review screen with its answer buttons, undo, bury, flag, keys,
  gamepad and screen lock. Part 2 (section 7) builds media, audio, speech, installability and the
  mapping screen. **Mutation band:** S35000-S35099. **Changelog fragment:** `changelog.d/study-screens-350.md`.
- **Builds on:** #662 (the engine core's dispatcher, SPEC-345) and #663 (the remote's modules,
  SPEC-343), both on `dev` first.

## 1. The problem, measured

Refs: `D` is the read base `ac4fbdaeb02f682991a9b26db539dbc20c8ff559` (`dev`); `E` is #662's head
`af6da34688467f8866167c4affbb53a45cca9d8b`; `H` is #663's head
`d13f28f47c7088a22abf76ac7eeb5de86b1c92a7`; `PIN` is the engine fork's revision
`c538de55a23e695234e794029fce0dafff2d36a9` that `Cargo.toml:132-134` patches in. Every read is
`git show <ref>:<path>` with `grep -n`, or `git grep -n <pattern> <ref> -- <path>`.

### 1.1 What the tree holds

| id | what | measured | command |
|---|---|---|---|
| M1 | The base against `dev`'s tip | `dev` later moved 16 commits ahead of `D`, all one merge (#659, the Apple build): no path under `web/`, `crates/web-engine/` or `crates/engine-core/`, and none of the documents this SPEC cites | `gh api repos/{owner}/{repo}/compare/<D>...<dev tip> --jq '.files[].filename'` |
| M2 | The web client | SvelteKit, client-rendered (`ssr = false`), twelve routes in `ROUTES` (`web/app/src/lib/routes.ts:10`), held equal to `src/routes` by `a11y-coverage.test.ts` and each audited with axe at WCAG 2.2 AA in both palettes (`tests/a11y.spec.ts`). Runners: Vitest (`test`), Playwright (`test:e2e`, `test:card`, `test:engine`; `package.json:13-16`). StrykerJS mutates every shipped file under `src`, break 100 (`stryker.config.json:7-18`) | `git show D:web/app/package.json`, `git show D:web/app/stryker.config.json`, `git show D:web/app/src/lib/routes.ts` |
| M3 | Whether any screen starts the engine or shows a card | none: the only `new Worker` is the engine harness's (`web/app/engine-harness/main.ts:40`), outside `src`; no route imports `EngineClient` or `CardFrame` | `git grep -n -e 'new Worker' -e EngineClient -e CardFrame D -- web/app/src web/app/engine-harness` |
| M4 | The page policy | `svelte.config.js:24-33`, hash mode: `script-src 'self'` Telegram `'wasm-unsafe-eval'`, `object-src 'none'`, `base-uri 'self'`, `connect-src 'self'`, `frame-src 'none'`; `csp.test.ts:37-43` holds the exact map and `:72` holds the additions to two. No `default-src`, so `manifest-src` and `media-src` are unrestricted and `worker-src` falls back to `script-src 'self'`: a same-origin Worker, a same-origin manifest and a page audio element need no change | `git show D:web/app/svelte.config.js`, `git show D:web/app/src/lib/csp.test.ts` |
| M5 | The card frame | `CardFrame.svelte:8-12` (props `html`, `css`, `title`; `sandbox={FRAME_SANDBOX}`; `srcdoc`); `policy.js:11` `PAGE_FRAME_SRC`, `:18` `FRAME_SANDBOX = ''`, `:25` `FRAME_POLICY` (`default-src 'none'`, `img-src data:`, `media-src data:`, `font-src data:`, `style-src 'unsafe-inline'`, `form-action 'none'`, `base-uri 'none'`); `frame-document.ts:26-34` parses inert, strips `link, meta, base, template`, composes, re-parses and refuses `escaped`; the composed body element carries no class. `card-sinks.test.ts:68`: the one sink in `src` is that `srcdoc`, and no window message listener exists | `git show D:web/app/src/lib/card/<file>` for each |
| M6 | The Worker's surface | `protocol.ts:6-24`: eight `OPS` (`open`, `seed`, `next`, `answer`, `undo`, `snapshot`, `memory`, `close`), five error codes, `Rating` 1 to 4; `parseRequest` refuses an unknown operation and an extra or malformed argument; `:59-68` `admitsOrigin`. `session.ts:7` the lock `deck-streak-collection`, `:13-25` `EngineModule`, `:151-171` the operation switch; `client.ts:22` `EngineClient`; `worker.ts:11` `ENGINE_BASE = '/engine/'`, `:96` `start` | `git show D:web/app/src/lib/engine/<file>` for each |
| M7 | The engine's exports at `D` | `wasm.rs:104-277`: `init` with `preferred_langs` `["en"]` (`:120`), `open` with an empty media folder (`:140`), `next_card` (`:221`), `answer` (`:229-231`) which reads the queue's head again when it answers, `undo` (`:252`), `run_method` (`:277`) | `git grep -n -e 'pub fn ' -e first_queued -e media_folder_path -e preferred_langs D -- crates/web-engine/src/wasm.rs` |
| M8 | The study rule at `D` | `study.rs:74-83`: eight pairs, (3,0) (3,1) (3,8) (13,3) (13,4) (23,8) (25,0) (25,2); `admit` (`:89-95`); only `run_method` calls it (SPEC-345 M4) | `git grep -n -e STUDY_CALLS -e 'pub fn admit' D -- crates/web-engine/src/study.rs` |
| M9 | The engine core at `E` | `table.rs:113` `ORDINARY`, ten rows, among them (27,6) `RenderExistingCard` and (7,13) `GetDeckNames`, both native only; `:190` `EXEMPT`, six owner-gesture writes; `:239` `decide`. The web `call()` crosses `Dispatcher::run` on `Transport::Web` (`wasm.rs:60`, `:123`). SPEC-345 A5 (`:130`) holds `STUDY_CALLS` equal to the web column; its section 5 (`:216`) adds no Worker route for `run_exempt` or `run_method` (#630, #631) | `git show E:crates/engine-core/src/table.rs`, `git grep -n -e Dispatcher -e 'fn call' E -- crates/web-engine/src/wasm.rs` |
| M10 | The pairs a review needs that the web column lacks | (7,4) `DecksService.DeckTree`, (7,22) `DecksService.SetCurrentDeck`, (27,6) `CardRenderingService.RenderExistingCard`, (27,9) `CardRenderingService.StripAvTags`, (13,24) `SchedulerService.DescribeNextStates`, (3,7) `CollectionService.GetUndoStatus`, (13,14) `SchedulerService.BuryOrSuspendCards`, (5,4) `CardsService.SetFlag`; part 2 adds (27,3) `CardRenderingService.ExtractAvTags` and (41,8) `MediaService.ExtractMediaFiles`. None is an owner-gesture write. Numbering as SPEC-345 M7 (`:31`) states it: an odd index is a backend service, its own methods first | the method order inside each service of `git show PIN:proto/anki/<decks,scheduler,card_rendering,media,collection,cards>.proto`, read with `awk '/^service /{s=$2;i=0} /^}/{s=""} s!="" && /rpc /{print s, i++, $2}'` |
| M11 | What the engine already guards | `AnswerCard` compares the card's current state with the request's and refuses `card was modified` (`rslib/src/scheduler/answering/mod.rs:337`), and caps the time taken itself (`:330`); `DeckTree` with `now` unburies on a day rollover (`rslib/src/decks/tree.rs:267`); a normal sync discards the undo queue (SPEC-345 M15) | `git show PIN:<path> \| grep -n -e 'card was modified' -e cap_answer -e unbury_if_day_rolled_over` |
| M12 | The size budget | 8000000 bytes `gzip -9` for module plus bindings (ADR-336 `:71`); measured 7534582, 465418 under (SPEC-338 M3, `:273`); the gate runs in the `web-engine` job (`ci.yml:823-824`) | `git grep -n -e 8000000 -e 7534582 D -- .github/workflows/ci.yml docs/` |
| M13 | Whether a release serves the engine | no: the release builds the app and stages `web/app/build` alone (`release.yml:91`, `:101`); the module is built only in CI's `web-engine` job (`ci.yml:761-830`) | `git show D:.github/workflows/release.yml \| sed -n '84,104p'` |
| M14 | The plan's row and the interval rule | SPEC-334 `:129` (row 1.4) names the deck list, review in a sandboxed frame, **undo**, templates, media, speech, Anki's keys and the Gamepad API, and the Home Screen; R15 (`:87-91`) names show answer, four grades, undo, bury, flag and replay; R18 (`:103-105`) shows the next interval on the answer buttons. STAT-01 is answered by the owner's app-surfaces ruling (`docs/rulings/`, its row at line 14, its text at `:34-36`) and ADR-338 `:56`: the intervals are shown, so the plan and R18 agree | `git grep -n -e STAT-01 -e 'row 1.4' D -- docs/`, then the ranges |
| M15 | The remote's modules at `H` | `actions.ts:32-45` (`resolve`, `sideAfter`), `mapping.ts:8-47` (buttons, stick, keys, Control or Command with 1 for the flag), `keys.ts:21,45-54` (a key in a control, a button included, fires nothing), `gamepad.ts:46-98`, `wake-lock.ts:99-103` (the condition: review, gamepad, visible). SPEC-343 `:289-300` and `:320` give #630 the review's use of them, the mapping screen, the persisted mapping and the keyboard-mode screen lock | `git show H:web/app/src/lib/remote/<file>`, `git show H:docs/specs/SPEC-343-…md \| sed -n '286,321p'` |
| M16 | The shell outside Telegram | the root layout asks for the wallet on mount (`+layout.svelte:25-36`); in a plain tab those requests fail closed (SPEC-343 `:324-325`, at `H`). `static/` holds `robots.txt` alone and `app.html` links no manifest | `git show D:web/app/src/routes/+layout.svelte`, `git ls-tree --name-only D web/app/static/` |

### 1.2 What the platform requires

| id | rule | source |
|---|---|---|
| P1 | A key event targets the focused element, so a key pressed while the card frame holds focus is dispatched in the frame's document, which runs no script; the page receives nothing | UI Events, section 3.5.5, `w3c.github.io/uievents/#events-keyboard-event-order` |
| P2 | Activation comes from `keydown` (not Escape or a browser shortcut), `mousedown`, `pointerdown` from a mouse, `pointerup` from another pointer, and `touchend`; gamepad input is not among them, and sticky activation does not expire | HTML, `html.spec.whatwg.org/multipage/interaction.html#activation-triggering-input-event` |
| P3 | A sandbox without `allow-scripts` sets the automatic-features flag, so nothing plays itself inside the card frame | HTML, `html.spec.whatwg.org/multipage/browsers.html#sandboxed-automatic-features-browsing-context-flag` |
| P4 | `getGamepads()` exposes no gamepad until a gamepad gesture; `gamepadconnected` fires on it; the standard mapping puts the right cluster at buttons 0 to 3 and the left at 12 to 15; the `gamepad` policy feature's default allowlist is `*` | Gamepad, `w3.org/TR/gamepad/` |
| P5 | A screen wake lock needs a visible document and is released when it is hidden | Screen Wake Lock API, `w3.org/TR/screen-wake-lock/` |
| P6 | `getVoices()` may be empty until `voiceschanged`; an utterance's `lang` is BCP 47; a null voice means the default for `lang`; a refused utterance reports `not-allowed` | Web Speech API, `webaudio.github.io/web-speech-api/` |
| P7 | Sound may play after the user has interacted with the site; on iPhone and iPad, `play()` must be called inside the handler of a user gesture | `developer.chrome.com/blog/autoplay`, `webkit.org/blog/6784/new-video-policies-for-ios/` |
| P8 | Chrome installs a site served over HTTPS whose manifest names `name` or `short_name`, 192 and 512 pixel icons, `start_url` and `display` `standalone` (or `fullscreen`, `minimal-ui`, `window-controls-overlay`), without `prefer_related_applications`; no service worker is required | `web.dev/articles/install-criteria`, `developer.chrome.com/blog/update-install-criteria` |
| P9 | On iPhone and iPad, a site added to the Home Screen with a manifest `display` of `standalone` opens as a web app, and a Home Screen web app keeps its own website data apart from the browser's | `webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/`, `webkit.org/blog/17333/`, `webkit.org/tracking-prevention/` |

### 1.3 What CI can decide, and what only a device or a person can

CI decides: the engine's tables and the shown-card rule (cargo tests), the Worker protocol and the
screens' logic (Vitest, StrykerJS at break 100), the card frame's census and policy (unchanged
SPEC-341 tests), the size gate, the routes' axe audits, the answer buttons' names and keyboard
reach, and the review loop end to end in Chromium and WebKit over the real module.

Only a device or a person decides: the 8BitDo remote in both modes on Safari on iPhone and iPad and
on desktop browsers, gamepad input after a tap on the card, keys a browser takes first, the screen
lock while dimmed, and a screen-reader walk of the review. The owner reads these in the first
device session (#629) and the acceptance session (#637); part 2 adds voices, autoplay after a
gesture and the Home Screen install to the acceptance session.

## 2. Requirements

R1. **The engine surface.** `STUDY_CALLS` gains the eight pairs of M10's first list and holds
    sixteen. The core's `ORDINARY` gains seven rows, web on and native off, and its (27,6) row turns
    web on. SPEC-345 A5's parity holds. No pair joins `EXEMPT`, and no Worker operation reaches
    `run_method` or `run_exempt`.

R2. **The shown-card rule.** The engine side keeps the card it last showed, with the scheduling
    states it read to show it and the card's flag. `rate`, `bury` and `flag` act only on that card
    and refuse any other id with `not-shown`. `rate` answers with the kept states and the next state
    the rating picks. Showing replaces the kept card; rating, burying and undo clear it. DEV's
    `next` and `answer` stay for the engine harness, and no study screen calls them.

R3. **The card view.** One Worker operation returns the queue's head as a view: its id, ordinal and
    flag; its question and answer rendered by the engine through the note type's templates (a full
    render, never a partial one), with sound and speech tags stripped by the engine; the note type's
    CSS; the four interval labels the engine describes for the kept states; the queue's new,
    learning and review counts; and the engine's undo label, empty when nothing can be undone. It
    returns no view, with the counts, when the deck is done.

R4. **The Worker protocol.** `OPS` gains `decks`, `study`, `card`, `rate`, `bury` and `flag`;
    `parseRequest` refuses an unknown operation and an extra or malformed argument for each, as
    it does today; the error codes gain `not-shown`. `open` takes an optional language list, which
    the engine's `init` receives, so its labels speak the app's locale; with none, `["en"]` holds.

R5. **One Worker.** `src/lib/study/engine.ts` starts the engine's Worker on the first study screen
    and closes it on `pagehide`; it is the only `new Worker` under `src`. Each refusal
    (`collection-busy`, `storage-refused`, `engine-failed`, `not-open`, `not-shown`) shows a
    translated message.

R6. **The deck list** (`/study`) shows the engine's deck tree with each deck's new, learning and
    review counts. Choosing a deck makes it current and opens the review. A collection with no
    deck shows a state whose one action is the web sync screens' entry.

R7. **The review screen** (`/study/review`) shows the question in `CardFrame`, whose sandbox and
    policy do not change. Show answer reveals the answer. The four answer buttons each show the
    engine's interval label (SPEC-334 R18). A rating sends `rate` with the milliseconds since the question
    showed, and the next card follows; a done deck returns to the deck list. Undo is enabled only
    while the engine names an undoable action; bury and flag act on the shown card, and the flag's
    state shows. The frame fills the card area and scrolls itself. Its body carries the card's
    classes, `card card<ordinal + 1>`, and the night-mode classes in a dark palette. A card the
    frame refuses (`escaped`) shows a translated message, and its answer controls stay usable.

R8. **Input.** The review takes #663's modules unchanged: `readKey` on `keydown`, the
    `GamepadReader` each animation frame while a gamepad is connected and the page is visible, and
    `resolve` and `sideAfter` for the side. A key, a gamepad button, the stick and a click reach one
    handler. A switch, stored per device, turns the single-character keys off (WCAG 2.1.4). Focus
    returns to the review region after every action, and when a pointer moves it into the card
    frame (P1).

R9. **The screen lock.** #663's `WakeLockHolder` holds it while a review is shown, a gamepad is
    connected and the page is visible (ADR-342's web clause).

R10. **Accessibility.** The two routes join `ROUTES` and the axe audit in both palettes. The answer
    buttons are native buttons whose accessible names hold their grade and their interval; the
    frame has a title naming its side; a refusal, a refused card and a done deck are announced
    through a status region.

R11. **Messages.** Every string the screens show is a message in all seven locales.

R12. **The guards hold unchanged.** `card-sinks.test.ts`'s census, `csp.test.ts`, `FRAME_SANDBOX`,
    `FRAME_POLICY` and `PAGE_FRAME_SRC` do not change, and the size gate passes over the module with
    the new exports.

R13. **The study suite.** A Playwright suite runs the built app over the real module, staged at
    `/engine/`, in Chromium and WebKit with persistent profiles, in CI's `web-engine` job. It
    seeds a collection through the shipped Worker's own `seed` operation, then reviews: deck list,
    show answer, intervals on the buttons, a rating by key and one by button, undo returns the
    rated card, bury and flag act on the shown card, a key after a tap on the card still rates,
    and axe passes on both screens.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | The study calls are the review's sixteen pairs | the eight-pair table | `crates/web-engine/tests/study.rs` `the_study_calls_are_the_reviews_pairs` |
| A2 | The core names each review pair ordinary on the web and the web column equals the study calls | `decide(Web, 7, 4)` reads `NotAllowed` | `crates/engine-core/tests/table.rs` `the_review_pairs_are_ordinary_on_the_web`; SPEC-345's `crates/engine-core/tests/parity.rs` `each_adapter_table_equals_its_transport_column` |
| A3 | Only the shown card is rated, buried or flagged | a guard that admits any id | `crates/web-engine/tests/study.rs` `only_the_shown_card_is_rated_buried_or_flagged` |
| A4 | The flag action toggles red, and a card with another flag turns red | a toggle that returns its input | `crates/engine-core/tests/review.rs` `the_flag_toggles_red` |
| A5 | Bury is the user's bury of the one shown card | the scheduler's bury mode | `crates/engine-core/tests/review.rs` `bury_is_the_users_bury_of_the_shown_card` |
| A6 | Each study operation parses its arguments and refuses any other | `OPS` without them | `web/app/src/lib/engine/protocol.test.ts` "each study operation parses its arguments and refuses any other" |
| A7 | Each study operation reaches its engine call, and a stale card is refused as `not-shown` | the session's default refusal | `web/app/src/lib/engine/session.test.ts` "each study operation reaches its engine call" |
| A8 | The client posts each study operation and settles its answer | no such method | `web/app/src/lib/engine/client.test.ts` "the client posts each study operation and settles its answer" |
| A9 | The engine's languages follow the app's locale | every locale maps to `en` | `web/app/src/lib/study/locale.test.ts` "the engine's languages follow the app's locale" |
| A10 | The app starts the engine's Worker in one place | the census finds none | `web/app/src/lib/study/engine.test.ts` "the app starts the engine's Worker in one place" |
| A11 | The review shows, reveals, rates and moves on, and a gesture during a request fires nothing | a machine that rates twice on a double press | `web/app/src/lib/study/review.test.ts` "the review shows, reveals, rates and moves on" and "a gesture during a request fires nothing" |
| A12 | A refused card keeps its answer controls and announces the refusal | the controls hidden | `review.test.ts` "a refused card keeps its answer controls" |
| A13 | Each answer button names its grade and its interval and is reached by Tab | buttons named by grade alone | `web/app/src/lib/study/answer-buttons.test.ts` "each answer button names its grade and its interval" |
| A14 | A key, a gamepad button, the stick and a click reach one action, and the key switch silences single-character keys only | the switch ignored | `web/app/src/lib/study/input.test.ts` "every source reaches one action" and "the key switch silences single-character keys" |
| A15 | Focus returns to the review after an action and after a pointer moves it into the card frame | focus left in the frame | `input.test.ts` "focus returns to the review" |
| A16 | The screen lock is wanted while a gamepad is connected during a visible review | the condition without the gamepad | `web/app/src/lib/study/review-screen.test.ts` "the screen lock follows the review, the gamepad and the page" |
| A17 | The study screens call only the study operations | a planted `answer(` call | `web/app/src/lib/study/study-calls.test.ts` "the study screens call only the study operations" |
| A18 | The frame body carries the card's classes and nothing else, and a class outside the card's set is refused | a body with no class | `web/app/src/lib/card/frame-document.test.ts` "the frame body carries the card's classes and nothing else" |
| A19 | Every route has an audit, the study routes included | `/study` in `src/routes` and not in `ROUTES` | `web/app/src/lib/a11y-coverage.test.ts` (unchanged) |
| A20 | The study suite reviews, undoes, buries, flags and rates after a tap, in both engines, and audits both screens | no study suite | the Playwright `web/app/tests-study/study.spec.ts` in `test:study`; its structure by `web/app/src/lib/study/study-coverage.test.ts` "the study suite covers the review loop in both engines" |
| A21 | The `web-engine` job stages the module and runs the study suite | the job without the step | `scripts/tests/test_ci_workflows.py` `the_web_engine_job_runs_the_study_suite` |
| A22 | The stage step copies the module and its bindings, and refuses when either is missing | a step that copies one file | `scripts/tests/test_web_engine_stage.py` `the_stage_refuses_a_missing_module` |
| A23 | The guards hold: the census, the page policy and the frame policy are unchanged, and the module stays within 8000000 bytes `gzip -9` | each test's planted controls (SPEC-341 A1, A7; SPEC-338 A3) | the unchanged `card-sinks.test.ts`, `csp.test.ts`, `policy.test.ts`; the `web-engine` job's size gate |

The tdd probe resolves no Playwright command, so A20 names its spec in the table and its fence line
runs the Vitest test that proves the spec's coverage, as SPEC-341 A9 to A12 do.

```acceptance
A1: cargo test -p deck-streak-web-engine --test study -- --exact the_study_calls_are_the_reviews_pairs
A2: cargo test -p deck-streak-engine-core --test table -- --exact the_review_pairs_are_ordinary_on_the_web
A2: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column
A3: cargo test -p deck-streak-web-engine --test study -- --exact only_the_shown_card_is_rated_buried_or_flagged
A4: cargo test -p deck-streak-engine-core --test review -- --exact the_flag_toggles_red
A5: cargo test -p deck-streak-engine-core --test review -- --exact bury_is_the_users_bury_of_the_shown_card
A6: pnpm exec vitest run web/app/src/lib/engine/protocol.test.ts -t "each study operation parses its arguments and refuses any other"
A7: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "each study operation reaches its engine call"
A8: pnpm exec vitest run web/app/src/lib/engine/client.test.ts -t "the client posts each study operation and settles its answer"
A9: pnpm exec vitest run web/app/src/lib/study/locale.test.ts -t "the engine's languages follow the app's locale"
A10: pnpm exec vitest run web/app/src/lib/study/engine.test.ts -t "the app starts the engine's Worker in one place"
A11: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "the review shows, reveals, rates and moves on"
A11: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "a gesture during a request fires nothing"
A12: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "a refused card keeps its answer controls"
A13: pnpm exec vitest run web/app/src/lib/study/answer-buttons.test.ts -t "each answer button names its grade and its interval"
A14: pnpm exec vitest run web/app/src/lib/study/input.test.ts -t "every source reaches one action"
A14: pnpm exec vitest run web/app/src/lib/study/input.test.ts -t "the key switch silences single-character keys"
A15: pnpm exec vitest run web/app/src/lib/study/input.test.ts -t "focus returns to the review"
A16: pnpm exec vitest run web/app/src/lib/study/review-screen.test.ts -t "the screen lock follows the review, the gamepad and the page"
A17: pnpm exec vitest run web/app/src/lib/study/study-calls.test.ts -t "the study screens call only the study operations"
A18: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "the frame body carries the card's classes and nothing else"
A19: pnpm exec vitest run web/app/src/lib/a11y-coverage.test.ts
A20: pnpm exec vitest run web/app/src/lib/study/study-coverage.test.ts -t "the study suite covers the review loop in both engines"
A21: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_web_engine_job_runs_the_study_suite
A22: python3 -m unittest discover -s scripts/tests -p test_web_engine_stage.py -k the_stage_refuses_a_missing_module
A23: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "card HTML reaches the page only through the card frame"
A23: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits WebAssembly compilation and nothing else new"
```

What only a device or a person proves, who proves it and when, is section 9.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/web-engine/src/study.rs` | `deck-streak-web-engine` | changed: `STUDY_CALLS` sixteen pairs, with the services `CARDS`, `DECKS` and `CARD_RENDERING`; `Shown<S>`, `shown_for`, `RED`, `toggled_red`, `BURY_USER`, `BuryOf`, `bury_of`, `StudyError::NotShown`; `engine_languages`, the language rule `init` applies (R4) |
| `crates/web-engine/tests/study.rs` | `deck-streak-web-engine` | changed (A1, A3, A4, A5, and the language rule); the study list of `run_method_admits_only_the_study_calls` grows to sixteen, insert-only (R1) |
| `crates/web-engine/src/wasm.rs` | `deck-streak-web-engine` | changed: the exports `deck_tree`, `set_current_deck`, `current_card`, `rate`, `bury`, `flag`; `init` takes the language list |
| `crates/web-engine/tests/boundary.rs` | `deck-streak-web-engine` | changed: the census reads each new export, insert-only |
| `crates/engine-core/src/table.rs` | `deck-streak-engine-core` | changed: seven rows, and (27,6) web on |
| `crates/engine-core/tests/table.rs` | `deck-streak-engine-core` | changed (A2); SPEC-345's `WEB` list grows to sixteen, insert-only (R1) |
| `web/app/src/lib/engine/protocol.ts` | `miniapp` | changed: six operations, `not-shown`, `open`'s languages |
| `web/app/src/lib/engine/protocol.test.ts` | `miniapp` | added (A6) |
| `web/app/src/lib/engine/session.ts` | `miniapp` | changed: `EngineModule` and the operation switch |
| `web/app/src/lib/engine/session.test.ts` | `miniapp` | changed (A7) |
| `web/app/src/lib/engine/client.ts` | `miniapp` | changed: the study methods |
| `web/app/src/lib/engine/client.test.ts` | `miniapp` | changed (A8) |
| `web/app/src/lib/card/frame-document.ts` | `miniapp` | changed: the body's card classes |
| `web/app/src/lib/card/frame-document.test.ts` | `miniapp` | changed (A18) |
| `web/app/src/lib/card/CardFrame.svelte` | `miniapp` | changed: the `classes` property; the one `srcdoc` stays |
| `web/app/src/lib/card/card-frame.test.ts` | `miniapp` | changed: the classes reach the frame, the sandbox unchanged |
| `web/app/src/lib/study/engine.ts` | `miniapp` | added: the app's one Worker |
| `web/app/src/lib/study/engine.test.ts` | `miniapp` | added (A10) |
| `web/app/src/lib/study/locale.ts` | `miniapp` | added: the app's locale to the engine's languages |
| `web/app/src/lib/study/locale.test.ts` | `miniapp` | added (A9) |
| `web/app/src/lib/study/review.ts` | `miniapp` | added: the review's machine |
| `web/app/src/lib/study/review.test.ts` | `miniapp` | added (A11, A12) |
| `web/app/src/lib/study/input.ts` | `miniapp` | added: keys, gamepad, the key switch, focus |
| `web/app/src/lib/study/input.test.ts` | `miniapp` | added (A14, A15) |
| `web/app/src/lib/study/refusal.ts` | `miniapp` | added: each refusal's message |
| `web/app/src/lib/study/refusal.test.ts` | `miniapp` | added |
| `web/app/src/lib/study/DeckList.svelte` | `miniapp` | added |
| `web/app/src/lib/study/deck-list.test.ts` | `miniapp` | added |
| `web/app/src/lib/study/ReviewScreen.svelte` | `miniapp` | added |
| `web/app/src/lib/study/review-screen.test.ts` | `miniapp` | added (A16) |
| `web/app/src/lib/study/AnswerButtons.svelte` | `miniapp` | added |
| `web/app/src/lib/study/answer-buttons.test.ts` | `miniapp` | added (A13) |
| `web/app/src/lib/study/study-calls.test.ts` | `miniapp` | added (A17) |
| `web/app/src/lib/study/study-coverage.test.ts` | `miniapp` | added (A20) |
| `web/app/src/routes/study/+page.svelte` | `miniapp` | added: the deck list |
| `web/app/src/routes/study/review/+page.svelte` | `miniapp` | added: the review |
| `web/app/src/lib/routes.ts` | `miniapp` | changed: the two routes |
| `web/app/src/lib/startapp.test.ts` | `miniapp` | changed: the two study routes are opened by path, insert-only |
| `web/app/messages/en.json` | `miniapp` | changed: the screens' messages |
| `web/app/messages/es.json` | `miniapp` | changed: the screens' messages |
| `web/app/messages/fr.json` | `miniapp` | changed: the screens' messages |
| `web/app/messages/ja.json` | `miniapp` | changed: the screens' messages |
| `web/app/messages/ko.json` | `miniapp` | changed: the screens' messages |
| `web/app/messages/zh-Hans.json` | `miniapp` | changed: the screens' messages |
| `web/app/messages/zh-Hant.json` | `miniapp` | changed: the screens' messages |
| `web/app/tests-study/study.spec.ts` | `miniapp` | added (A20) |
| `web/app/tests-study/seed.ts` | `miniapp` | added: seeds through the shipped Worker's `seed` operation |
| `web/app/playwright.study.config.ts` | `miniapp` | added: Chromium and WebKit, persistent profiles |
| `web/app/vite.study.config.ts` | `miniapp` | added: `vite preview` of the staged build alone, with the single-page fallback, on the study port |
| `web/app/package.json` | `miniapp` | changed: `"test:study"` |
| `web/app/svelte.config.js` | `miniapp` | changed: the TypeScript project reads `tests-study` and the study suite's two configurations, as it reads the card harness's; the page policy's directives unchanged |
| `scripts/web-engine-stage.sh` | CI | added: copies the module and its bindings into `web/app/build/engine/` |
| `scripts/tests/test_web_engine_stage.py` | CI | added (A22) |
| `.github/workflows/ci.yml` | CI | changed: the `web-engine` job builds the app, stages the module and runs `test:study` |
| `scripts/tests/test_ci_workflows.py` | CI | changed (A21) |
| `scripts/mutation-rows.d/S35000-S35099.json` | mutation | added |
| `scripts/mutation-equivalent.d/deck-streak-web-engine.json` | CI | changed (record 21 removed: its undo mutant is now caught, rulings 344 and 357) |
| `docs/schematics/web-study-screens.md` | docs | added |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md` | docs | added |
| `docs/red-first/SPEC-350.md` | docs | added |
| `changelog.d/study-screens-350.md` | docs | added |

## 5. What this does NOT do

- It builds no sync screen, no full-sync choice, no backup or loss count, no sync at session start
  or end and no unsynced-reviews warning; the deck list's empty state offers their entry as its one
  action, and their transport fills the media directory part 2 reads (#631).
- It does not decide where the browser holds the sync credential (#654).
- It adds no sign-in outside Telegram: the shell's wallet requests still fail closed in a plain
  tab or a Home Screen app, and the study screens depend on none of them (#627).
- It builds no undo across a sync: the engine's undo reaches the session's own actions, a normal
  sync discards its queue, and undo after a sync is measured and decided elsewhere (#620, #631).
- It runs no card script: MathJax, hint toggles and other scripted faces render without their
  script (#651).
- It builds no typed-answer box: a `[[type:]]` field shows nothing to type into (#611).
- It edits no note type's templates: SPEC-334 row 1.4's "templates" is read as the cards rendered
  through their note type's templates, which R3's card view shows. Editing a template is a later
  parity phase (#611).
- It buries one card, the user's bury of the card on screen (R2, A5). Burying the card's whole
  note, the desktop's `=` key, is a later parity phase (#611).
- It builds no later parity screen: browse and search, adding and editing notes, editing a note
  type's templates, Forget, set due date, a card's due date, the Due column, the forecast, deck
  options, suspend, the flags other than red, burying a whole note, and a switch that hides the
  intervals are the later parity phases (#611).
- It shows no video picture; part 2 plays a sound tag's audio track alone (#611).
- It adds no service worker, so the installed app needs the network to start; the browser's
  service worker arrives with web push (#640).
- It holds no screen lock for the remote's keyboard mode, which a page cannot tell from a keyboard;
  that choice follows the first device session's reading (#629).
- It awards no XP per review on the web (#639) and runs no FSRS-7 (#641).
- It builds nothing for iPhone and iPad (#632, #633).
- It does not test Firefox (#652), write the campaign's threat model (#653) or re-check the
  shipped headers (#638).
- It does not stage the engine's module at `/engine/` in the release, or hold the release to the
  module's size bound, and so has no A29 row; both are a release workflow change (#685).
- It does not speak with the voice a speech tag names: the core's speech clip carries no tag voice
  (#666).

## 6. Risks

- **A tap moves focus into the card frame,** where keys reach no listener (P1). Detected by A15
  and the study suite's tap-then-key test; the device reading of a remote after a tap is V1.
- **A browser delivers no gamepad input while the card frame holds focus.** The Gamepad standard
  names no focus rule. Detected by V1 (#629); the focus return of R8 narrows the window.
- **The iPhone and iPad review adds the same pairs to the core's table,** native on, while this
  delivery adds them web on, and a pair gets two rows. Whichever lands second sets its column on the
  row the first added; the core's table tests and SPEC-345 A5 detect a miss, and the train's union
  run reads both.
- **The new exports push the module past its budget.** Detected by the size gate (A23); the
  exports reach backend methods the module already links, so the growth is their glue.
- **WebKit refuses OPFS in an ephemeral context** (ADR-336 `:66`). The study suite runs in
  persistent profiles, as the engine suite does.
- **Opening the deck list writes:** `DeckTree` with `now` unburies on a day rollover (M11). It is
  the engine's own rule, as on the desktop, and a read-only deck list would show yesterday's
  buried cards.
- **The engine's labels stay English** if the language list never reaches `init`. Detected by A9
  and A7.
- **Telegram's in-app browser refuses OPFS, Web Locks or a module Worker.** The refusal shows its
  message (R5); the owner reads it in the first device session (#629).

## 7. Delivered by the next pull request

Part 2 builds on part 1. Its requirements:

R14. **Media.** The Worker asks the engine for each side's media names (41,8), reads each named file
     from the OPFS media directory the sync screens fill, and returns `data:` URLs for an image and
     audio type allow-list, within a per-file and a per-card bound. `frameDocument` replaces a
     `src` that names a returned file, in its inert parse. `FRAME_POLICY`, `FRAME_SANDBOX` and
     the census do not change.

R15. **Audio.** The engine extracts each side's sound tags (27,3). They play in one page-owned
     audio element: on show and on reveal once the page has user activation, and on the replay
     control and the remote's replay at any time. A blocked play leaves the replay control as the
     way to hear it.

R16. **Speech.** A speech tag is spoken with `speechSynthesis`, its language converted to BCP 47.
     The voice is the device's stored choice for that language, else a voice the tag names that the
     device has, else the language's default. A voice picker lists the device's voices for the
     card's languages and stores the choice per device and per language.

R17. **The Home Screen.** A manifest (`name`, `short_name`, 192 and 512 pixel icons, `start_url`
     `/study`, `display` `standalone`) and a touch icon, linked from `app.html`; no service worker;
     the page policy unchanged. The release stages the module at `/engine/` beside the app and holds
     it to the size gate.

R18. **The mapping screen.** The remote's mapping is stored per device and edited on a screen, with
     a default per mode; the key switch moves there.

| id | criterion | delivered by |
|---|---|---|
| A24 | A named media file reaches the frame as a `data:` URL, and a name outside the type allow-list or over a bound does not | the next pull request's `media.test.ts` and `frame-document.test.ts` |
| A25 | The frame policy, the sandbox and the census are unchanged with media | the unchanged `policy.test.ts`, `card-sinks.test.ts`, `card-frame.test.ts` |
| A26 | Sound tags play in order on show and reveal, and replay replays them | the next pull request's `audio.test.ts` |
| A27 | A speech tag's language and voice follow the stored choice, the named voice and the default, in that order | the next pull request's `voice.test.ts` |
| A28 | The manifest meets the install criteria, and the page policy is unchanged | the next pull request's `manifest.test.ts`; the unchanged `csp.test.ts` |
| A29 | A release carries the module at `web/engine/` within the budget | the next pull request's `test_release_workflow.py` test |
| A30 | A stored mapping drives the review, and each mode keeps its default | the next pull request's `mapping-store.test.ts` |
| V4 | Voices on Safari on iPhone and iPad and on desktop; sound after the first tap; the Home Screen install and its first sync into its own storage | the owner, in the acceptance session (#637) |

## 8. Formal models

None. The review loop's one safety property, that a rating, bury or flag acts only on the card on
screen, is a guard over one value behind one ordered channel: the Worker handles messages one at
a time, the Web Lock keeps a second tab out, and the engine refuses an answer whose states moved.
A3's table and the review machine's tests enumerate it. SPEC-334 section 9's two candidates (the
full-sync ordering and XP reconciliation) belong to #631 and #639.

## 9. What only a device or a person proves

These are read by the owner, not by CI, so they sit outside section 3's table and its fence.

| id | criterion | who, and when |
|---|---|---|
| V1 | The 8BitDo remote, in its gamepad mode and its keyboard mode, shows the answer, grades, undoes, buries and flags on Safari on iPhone and iPad and on a desktop browser, and the gamepad still drives the review after a tap on the card | the owner, in the first device session, read from the screens' own behaviour beside the remote harness's log (#629) |
| V2 | The screen stays lit while a gamepad is connected during a review, and the browser keeps no mapped key (Control-1, Command-1) for itself | the owner, in the first device session (#629) |
| V3 | A review with a synced collection, read with VoiceOver, names every control and announces each refusal | the owner, in the acceptance session, after the web sync screens land (#637, #631) |

## 10. Amendments

- **R6: a collection with no deck shows a message only.** The deck list of a collection with no
  deck says "There is no deck to study yet." and offers no action beside it. The entry to the web
  sync screens, which R6 named as that state's one action, is not part of this screen. Held by
  `web/app/src/lib/study/deck-list.test.ts` "a collection with no deck says so".
- **R7: a done deck ends where it is.** A done deck shows a designed end: the status region's "This
  deck is done for today." and a link back to the deck list, `/study`. The screen never navigates
  away on its own, so the announcement R10 requires is heard; where R7 says a done deck returns to
  the deck list, the user returns by that link. Held by
  `web/app/src/lib/study/review-screen.test.ts` "a refusal, a card the frame refuses and a done deck
  are announced".
- **R14, as part 2 amends it: media through the core's face.** The Worker answers a `faces`
  operation for the card on screen only, through the check `rate` makes. The engine builds both
  sides through the core's one face call, which holds the one cap pair and the one closed type
  table both clients read (ADR-359 D1), and writes every `data:` URL itself. The core reads media
  synchronously and the media directory answers asynchronously, so the Worker asks twice: first
  with no files, which answers each name the core asked for with its limit, then with each named
  file's first `limit` bytes. The media directory is `deck-streak-media` at the origin's root,
  flat, each file under the name the engine stores; a file or a directory that is absent is
  absent, never an error. The second answer is the reply. `frameDocument`, `FRAME_POLICY`,
  `FRAME_SANDBOX` and the census do not change, and no pair joins a table. Held by A24 and A25 in
  section 11 (ADR-361 D12).
- **R15, as part 2 amends it: sound.** The face's clips play in one page-owned audio element, in
  order: its `autoplay` clips on show and on reveal, and its `replay` clips on the Replay control
  and on the remote's replay, on either side. A sound plays from a page URL made from its bytes and
  the type the core gives it, revoked after use; nothing reaches the frame. A blocked play leaves
  the Replay control. A speech clip in the sequence goes to R16's speaker. Held by A26 (ADR-361
  D13).
- **R16, as part 2 amends it: speech.** A speech clip is spoken with `speechSynthesis` in the
  language the core gives it, at the core's rate divided by the native default of 0.5 (SPEC-348
  P3). The voice is the device's stored choice for that language, else the language's default
  voice; the voice a tag names is #666's. A voice picker lists the device's voices for the card's
  languages and stores the choice per language under one local-storage key. Held by A27 (ADR-361
  D13).
- **R17, as part 2 amends it: the Home Screen.** A manifest (`name`, `short_name`, 192 and 512
  pixel icons, `start_url` `/study`, `display` `standalone`) and a 180 pixel touch icon, linked
  from `app.html`; no service worker; the page policy unchanged. The release's staging of the
  module at `/engine/` and its size gate are #685's, with A29 (section 5). Held by A28 (ADR-361
  D14, D16).
- **R18, as part 2 amends it: the mapping screen.** The remote's mapping is stored per device
  under one local-storage key and edited on `/study/mapping`, with a default per mode, gamepad and
  keyboard. #663's `readKey` and `GamepadReader` each take it as one trailing parameter that
  defaults to today's map. The mapping screen also binds the key switch, and the review keeps it.
  Held by A30 (ADR-361 D15).
- **Part 2 touches these paths:** `docs/specs/SPEC-350-the-web-study-screens-review-a-card-from-the-engine-in-the-browser-in-the-sandboxed-frame-by-touch-keys-or-a-remote.md`,
  `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md`,
  `docs/schematics/web-study-screens.md`, `docs/red-first/SPEC-350.md`,
  `changelog.d/study-media-350.md`, `scripts/mutation-rows.d/S35000-S35099.json`,
  `crates/web-engine/src/study.rs`, `crates/web-engine/src/wasm.rs`,
  `crates/web-engine/tests/study.rs`, `crates/web-engine/tests/boundary.rs`,
  `web/app/src/lib/engine/protocol.ts`, `web/app/src/lib/engine/protocol.test.ts`,
  `web/app/src/lib/engine/session.ts`, `web/app/src/lib/engine/session.test.ts`,
  `web/app/src/lib/engine/client.ts`, `web/app/src/lib/engine/client.test.ts`,
  `web/app/src/lib/engine/worker.ts`, `web/app/src/lib/engine/worker.test.ts`,
  `web/app/src/lib/engine/media.ts`, `web/app/src/lib/engine/media.test.ts`,
  `web/app/src/lib/card/frame-document.test.ts`, `web/app/src/lib/study/review.ts`,
  `web/app/src/lib/study/review.test.ts`, `web/app/src/lib/study/input.ts`,
  `web/app/src/lib/study/input.test.ts`, `web/app/src/lib/study/audio.ts`,
  `web/app/src/lib/study/audio.test.ts`, `web/app/src/lib/study/speech.ts`,
  `web/app/src/lib/study/voice.ts`, `web/app/src/lib/study/voice.test.ts`,
  `web/app/src/lib/study/VoicePicker.svelte`, `web/app/src/lib/study/ReviewScreen.svelte`,
  `web/app/src/lib/study/mapping-store.ts`, `web/app/src/lib/study/mapping-store.test.ts`,
  `web/app/src/lib/study/MappingScreen.svelte`, `web/app/src/routes/study/mapping/+page.svelte`,
  `web/app/src/lib/remote/keys.ts`, `web/app/src/lib/remote/gamepad.ts`,
  `web/app/src/lib/routes.ts`, `web/app/src/lib/startapp.test.ts`,
  `web/app/src/lib/manifest.test.ts`, `web/app/src/app.html`,
  `web/app/static/manifest.webmanifest`, `web/app/static/icon-192.png`,
  `web/app/static/icon-512.png`, `web/app/static/apple-touch-icon.png`, `web/app/messages/en.json`,
  `web/app/messages/es.json`, `web/app/messages/fr.json`, `web/app/messages/ja.json`,
  `web/app/messages/ko.json`, `web/app/messages/zh-Hans.json` and `web/app/messages/zh-Hant.json`.
- **R15's citation, corrected:** A26 holds R15 under ADR-361 D7 and D12 (sound), not D13 (voice).
- **Part 2 also touches** `web/app/src/lib/study/review-screen.test.ts`, which the bullet above omits.
- **Part 2's icons, corrected:** part 2 touches `web/app/src/lib/icon.ts`, `web/app/src/routes/icon-192.png/+server.ts`, `web/app/src/routes/icon-512.png/+server.ts` and `web/app/src/routes/apple-touch-icon.png/+server.ts` in place of `web/app/static/icon-192.png`, `web/app/static/icon-512.png` and `web/app/static/apple-touch-icon.png`, which no commit touches; the build prerenders each icon at its path (ADR-361 D16).

## 11. Acceptance criteria of part 2

Section 7's rows as the amendments above decide them. A29 has no row here: it is #685's.

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A24 | The core names each media file a face needs once, with its limit; the Worker reads no file past that limit and reads an absent file as absent; the faces it answers are the core's, `data:` URLs included, and reach the frame unchanged; no cap or media type is copied outside the core | a reader that keeps every ask and returns a whole file; `OWED` naming a `faces` export that does not exist; a session with no `faces` operation; no reader of the media directory; no census | `crates/web-engine/tests/study.rs` `each_name_the_core_asks_for_is_wanted_once`, `a_file_is_read_no_further_than_its_limit`; `crates/web-engine/tests/boundary.rs`; `session.test.ts`, `media.test.ts`, `review.test.ts`; `frame-document.test.ts` |
| A25 | The frame policy, the sandbox and the census are unchanged with media | none: they stand on dev, unchanged | the unchanged `policy.test.ts`, `card-sinks.test.ts`, `card-frame.test.ts` |
| A26 | The face's clips play in order on show and on reveal, replay replays them on either side, and a blocked play leaves the Replay control | no player | `audio.test.ts`; `input.test.ts` |
| A27 | A speech clip is spoken in its language at the web's rate, with the stored voice for that language, else the language's default; the picker stores the choice per language | no speaker, no voice choice, no picker | `voice.test.ts` |
| A28 | The manifest meets the install criteria, and the page policy is unchanged | no manifest | `manifest.test.ts`; the unchanged `csp.test.ts` |
| A30 | A stored mapping drives the review, and each mode keeps its default | no mapping store | `mapping-store.test.ts` |

```acceptance
A24: cargo test -p deck-streak-web-engine --test study -- --exact each_name_the_core_asks_for_is_wanted_once
A24: cargo test -p deck-streak-web-engine --test study -- --exact a_file_is_read_no_further_than_its_limit
A24: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
A24: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "faces asks the engine twice, the second time with the files it named"
A24: pnpm exec vitest run web/app/src/lib/engine/media.test.ts -t "the worker reads each name the engine asks for, no further than its limit"
A24: pnpm exec vitest run web/app/src/lib/engine/media.test.ts -t "the media rules have one copy"
A24: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "the frame shows the faces the engine completed"
A24: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "a data: media source reaches the frame unchanged"
A25: pnpm exec vitest run web/app/src/lib/card/policy.test.ts -t "the card frame's policy fetches only data and runs no script"
A25: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "card HTML reaches the page only through the card frame"
A25: pnpm exec vitest run web/app/src/lib/card/card-frame.test.ts -t "the card frame is a sandboxed srcdoc frame with no token"
A26: pnpm exec vitest run web/app/src/lib/study/audio.test.ts -t "the face's clips play in order on show and reveal, and replay replays them"
A26: pnpm exec vitest run web/app/src/lib/study/audio.test.ts -t "a blocked play leaves the replay control"
A26: pnpm exec vitest run web/app/src/lib/study/input.test.ts -t "the remote's replay fires on either side"
A27: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "a speech clip's voice is the stored choice, else the language's default"
A27: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "a speech clip is spoken in its language at the web's rate"
A27: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "the picker stores the choice per language"
A28: pnpm exec vitest run web/app/src/lib/manifest.test.ts -t "the manifest meets the install criteria"
A28: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits WebAssembly compilation and nothing else new"
A30: pnpm exec vitest run web/app/src/lib/study/mapping-store.test.ts -t "a stored mapping drives the review, and each mode keeps its default"
```

## 12. Amendments for #685: the release builds, gates and stages the web engine's module

Issue #685 asks that a release serve the engine's module at `/engine/` beside the app, with A29's
test red before the change and green after it. This section amends R17's last sentence (section 7)
and A29; ADR-361 D17 to D22 decide it. M13 (`:43`) and section 5's staging bullet (`:306-307`)
describe the release before this amendment.

- **R17's last sentence, as #685 amends it: the release builds, gates and stages the module.** The
  release job builds the module and its bindings with the steps CI's `web-engine` job runs, copied
  byte for byte: the pinned toolchain and its wasm32 target, the bindings generator and the
  optimiser with each download's digest checked, the C compiler and archiver SQLite's source needs,
  and `scripts/web-engine-build.sh` with CI's compiler, archiver and output directory (ADR-361
  D17). It then runs CI's size gate, `scripts/web-engine-size.py`, over the module and the bindings
  it built, before the app's dependencies, the stage, the tarball and the draft release; a module
  over ADR-336's budget, or a gate that cannot measure, fails the job, so no release is made (D19).
  After the app's build it runs `scripts/web-engine-stage.sh`, which puts both files in the build
  under `engine/`. The tarball step is unchanged: it copies the build into `web/`, so the release
  carries both files at `web/engine/` and its manifest holds their digests, and an origin that
  serves `web/` at its root answers the Worker's `/engine/` (D18). Held by A29 (section 14).
- **A29, as #685 amends it.** Section 7's row stands as the criterion. Three tests in
  `scripts/tests/test_release_workflow.py` decide it, each with its own fence line in section 14
  (ADR-361 D20).
- **The release job's bound.** Its `timeout-minutes` rises from 90 to 150, so the module's cold
  wasm32 build fits beside the builds the job already runs (ADR-361 D22).
- **Risks of the #685 amendment.** The release job holds the release's write token while it
  downloads and runs the module's tools: each download is checked against its digest, and the C
  step installs a package only when the image lacks the archiver, as CI's job does on every pull
  request. A build slower than the job's bound fails the tag's run; D22 sizes the bound, and the
  run's log shows each step's time.
- **The #685 amendment touches these paths:** `.github/workflows/release.yml`,
  `scripts/tests/test_release_workflow.py`, `scripts/tests/test_ci_workflows.py` (one census
  entry), `scripts/mutation-rows.d/S35000-S35099.json` (rows S35044 to S35055),
  `scripts/mutation-rows.d/S19000-S19099.json` (S19011 and S19016 re-anchored on the new bound),
  `docs/specs/SPEC-350-the-web-study-screens-review-a-card-from-the-engine-in-the-browser-in-the-sandboxed-frame-by-touch-keys-or-a-remote.md`,
  `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md`,
  `docs/schematics/release-from-a-tag-push-or-a-dispatch-at-the-tags-ref.md`,
  `docs/red-first/SPEC-350.md` and `changelog.d/release-stages-web-engine-685.md`.

## 13. What this does NOT do, in the #685 amendment

- It does not run the browser tests over the release's own module: CI's `web-engine` job runs them
  over a module built by the same steps at the same commit (#685).
- It changes no CI job: `ci.yml`'s `web-engine` job is the source the release copies, and stays as
  it is (#685).
- It does not change how the host serves the release's `web/` folder or answers for the module's
  file; the deploy files decide that, and the acceptance session reads the published app (#637).
- It does not check the shipped headers again (#638).
- It keeps no module between releases: each tag builds its own, and no step restores or saves a
  cache (#685).

## 14. Acceptance criteria of the #685 amendment

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A29 | A release carries the module and its bindings at `web/engine/`, each in its manifest, built by CI's `web-engine` steps and held to ADR-336's budget before the draft release exists | the release's own steps from the app's build to the draft pack nothing under `web/engine/`; no release step adds the wasm32 target; no release step runs the size gate | `scripts/tests/test_release_workflow.py` `test_the_release_carries_the_module_at_web_engine`, `test_the_release_builds_and_gates_the_module_as_ci_does`, `test_an_over_budget_module_stops_the_release_before_the_draft` (ADR-361 D17 to D20) |

```acceptance
A29: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k the_release_carries_the_module_at_web_engine
A29: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k the_release_builds_and_gates_the_module_as_ci_does
A29: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k an_over_budget_module_stops_the_release_before_the_draft
```

## Amendment: A4 and A5 are decided in the core (SPEC-358)

- SPEC-358 R2 moves the flag and bury rules into the shared core, and A4's and A5's tests move
  with them, names kept: `crates/engine-core/tests/review.rs` `the_flag_toggles_red` and
  `bury_is_the_users_bury_of_the_shown_card` (#633).
