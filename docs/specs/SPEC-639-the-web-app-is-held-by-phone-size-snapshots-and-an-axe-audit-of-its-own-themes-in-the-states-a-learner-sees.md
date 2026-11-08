# SPEC-639: the web app is held by phone-size snapshots and an axe audit of its own themes in the states a learner sees

- **Issue:** #1284. **Context(s):** the Mini App's tests (`web/app/tests`), its Playwright configuration, `scripts/tests` and `docs/`.
- **Decided by:** ADR-662; ADR-616 (the colour mode); ADR-028 (the token file).
- **Schematic:** `docs/schematics/web-verify.md`, new.
- **Status:** delivered by one pull request into dev, with `docs/red-first/SPEC-639.md`. **Mutation band:** S63900-S63999. **Models:** none.

## 1. The problem, measured

At dev `36efa139` axe runs only under a stand-in for Telegram's script, in Telegram's two palettes, at the runner's default desktop size and in one state per route (`web/app/tests/a11y.spec.ts:21`, `:100-105`, `:338-361`; `web/app/playwright.config.ts:3-10`). The app's own Tide theme (`web/app/src/lib/design/tokens.json:92-95`) is never audited, no audit runs at a phone size, and the tree holds 0 screenshot assertions.

## 2. Requirements

R1. `web/app/playwright.config.ts` keeps its default project as `chromium`, ignoring `tests/verify/`, and adds six phone projects: `phone-light`, `phone-dark`, `phone-light-text200`, `phone-dark-text200` (390 by 844, touch) and `phone-small-light`, `phone-small-dark` (360 by 740, touch), each in Chromium.
R2. `web/app/tests/verify/states.ts` holds, for every route of `routes.ts`, its synthetic states: `first` (the screen with its data), `empty` (no data) and `error` (its call answered with a server error), and for the review screen `question` and `answer`. Every value is synthetic; no real user data enters a fixture or a baseline.
R3. `web/app/tests/verify/a11y-themes.spec.ts` runs axe over every route and state in all six phone projects with the tags `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa` and `wcag22aa` and `target-size` on by name, asserts that `target-size` ran, and asserts before axe that the root paints the Tide background of its scheme and that no `--tg-theme-*` variable is set.
R4. `web/app/tests/verify/snapshots.spec.ts` compares every route and state in the four 390-wide projects with `toHaveScreenshot`, CSS scale, the clock fixed, `threshold: 0` and `maxDiffPixels: 0` with no `maxDiffPixelRatio`, so one changed pixel fails; baselines are committed under `web/app/tests/verify/__screenshots__/` with the platform in each name, and CI never updates one. The first baselines, and a later route's first one, come from a render run: one CI push sets the render mode of ADR-662 D9, the config then writes each missing baseline under the run's stage logs by its committed name with `updateSnapshots: 'default'`, every test that writes one fails, and a commit of their own copies the images by name; every other run compares with `updateSnapshots: 'none'`, and no run uses `'missing'`, which passes a test that writes its baseline.
R5. `web/app/tests/verify/exceptions.ts` is the only place a rule is disabled; each entry names its rule, route, reason and issue. It starts empty.
R6. `web/app/tests/verify/planted.spec.ts` holds the planted controls: a low-contrast pair and a small target, each refused by axe at a phone size; a host palette refused as not the app's own theme; a colour change of one 8 by 8 CSS pixel element (64 pixels, under the 329 that a ratio of 0.001 admits on a 390 by 844 frame) refused by the snapshot gate; and a clean planted page that passes axe and matches its baseline.
R7. `scripts/tests/test_web_verify.py` holds the tree to R1, R2, R3, R4 and R5: the project matrix, a state for every route, a baseline for every route, state and snapshot project, the five tags, the `target-size` option and its ran-assertion, the zero screenshot margin, no disable outside `exceptions.ts`, and synthetic-only fixtures.
R8. `web/app/tests/a11y.spec.ts`, `web/app/tests/voice-picker-a11y.spec.ts`, `web/app/tests/telegram-palettes.ts` and `web/app/src/lib/a11y-coverage.test.ts` stay byte-unchanged, and no screen, palette, bot or Mini App file changes.

## 3. Acceptance criteria of SPEC-639

| id | criterion | decided by |
|---|---|---|
| A1 | the config holds the kept default project and the six phone projects, and a planted missing project is refused (R1) | `test_web_verify.py` |
| A2 | every route has its `first`, `empty` and `error` states, the review screen its `question` and `answer`, and a planted route with no state is refused (R2) | `test_web_verify.py` |
| A3 | every fixture value is synthetic, and a planted email-shaped or numeric-account value is refused (R2) | `test_web_verify.py` |
| A4 | every route and state has no WCAG 2.2 AA violation axe can find on Tide light and dark at both phone sizes and at the accessibility text size (R3) | `a11y-themes.spec.ts` |
| A5 | the audit runs the five tags, turns `target-size` on and asserts it ran, and a planted audit missing a tag, the option or the assertion is refused (R3) | `test_web_verify.py` |
| A6 | a planted low-contrast pair and a planted small target are each refused by axe at a phone size (R6) | `planted.spec.ts` |
| A7 | a planted host palette is refused as not the app's own theme (R3, R6) | `planted.spec.ts` |
| A8 | every route and state matches its committed baseline in the four 390-wide projects, compared with `updateSnapshots: 'none'` (R4) | `snapshots.spec.ts` |
| A9 | a planted colour change of one 8 by 8 CSS pixel element is refused by the snapshot gate, and the clean planted page passes axe and matches its committed baseline (R4, R6) | `planted.spec.ts` |
| A10 | every route, state and snapshot project has a committed Linux baseline, so no comparing run meets a missing one, and a planted missing baseline is refused (R4) | `test_web_verify.py` |
| A11 | no rule is disabled outside `exceptions.ts`, every entry there names a rule, route, reason and issue, and a planted bare disable is refused (R5) | `test_web_verify.py` |
| A12 | the kept stand-in audit and its coverage test are unchanged and pass (R8) | the SPEC-028 A4 tests, unchanged |
| A13 | the screenshot expectation sets `threshold: 0` and `maxDiffPixels: 0` and no `maxDiffPixelRatio`, and a planted config with any margin is refused (R4) | `test_web_verify.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_the_config_holds_the_kept_project_and_the_six_phone_projects
A1: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_missing_phone_project_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_every_route_has_its_learner_states
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_route_with_no_state_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_real_looking_fixture_value_is_refused
A4: pnpm --dir web/app exec playwright test tests/verify/a11y-themes.spec.ts
A5: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_audit_without_target_size_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_audit_missing_a_wcag_tag_is_refused
A6: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted low-contrast pair is refused by axe at a phone size"
A6: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted small target is refused by axe at a phone size"
A7: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted host palette is refused as not the app's own theme"
A8: pnpm --dir web/app exec playwright test tests/verify/snapshots.spec.ts
A9: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted colour change is refused by the snapshot gate"
A9: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "the clean planted page passes axe and matches its baseline"
A10: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_missing_baseline_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_bare_disable_is_refused
A12: pnpm --dir web/app exec vitest run src/lib/a11y-coverage.test.ts
A13: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_the_screenshot_expectation_allows_no_margin
A13: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_screenshot_margin_is_refused
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/playwright.config.ts` | web | changed: the kept default project and six phone projects, the screenshot path template and its render mode, `updateSnapshots: 'none'` outside it |
| `web/app/tests/verify/states.ts` | web | added: the synthetic state table |
| `web/app/tests/verify/fixtures.ts` | web | added: the launch with an empty theme, the text-size style, the fixed clock, the Tide check |
| `web/app/tests/verify/exceptions.ts` | web | added: the recorded-exception table, empty |
| `web/app/tests/verify/a11y-themes.spec.ts` | web | added: R3 |
| `web/app/tests/verify/snapshots.spec.ts` | web | added: R4 |
| `web/app/tests/verify/planted.spec.ts` | web | added: R6 |
| `web/app/tests/verify/__screenshots__/` | web | added: the CI-rendered baselines |
| `scripts/tests/test_web_verify.py` | scripts | added: R7 |
| `scripts/mutation-rows.d/S63900-S63999.json` | scripts | added: the rows |
| `docs/specs/SPEC-639-the-web-app-is-held-by-phone-size-snapshots-and-an-axe-audit-of-its-own-themes-in-the-states-a-learner-sees.md` | docs | added |
| `docs/decisions/ADR-662-the-web-apps-verification-uses-screenshot-baselines-rendered-in-ci-and-axe-over-its-own-themes-at-phone-sizes.md` | docs | added |
| `docs/schematics/web-verify.md` | docs | added |
| `docs/red-first/SPEC-639.md` | docs | added |
| `changelog.d/web-verify-639.md` | docs | added |

## 5. What this does NOT cover

- The iPhone and iPad app's audits and snapshots: its own delivery, under the issue the app pull request names (#1285).
- A vision or rubric critique, Core Web Vitals and token conformance: advisory or separate gates (#1286, #1287, #1288).
- New screens, the palette and any look: unchanged here (#1043).
- The messages in other locales in the snapshots (#1289).
- How a phone's text setting reaches the web view and how a device reads a screen: no test here measures it (#1290).
- The loading and offline states of each route in the snapshot and axe matrix: a later item adds them (#1292).

## 6. Risks

- The first run on Tide at phone sizes may find violations the desktop stand-in run never saw; the pull request lands only green, and any entry in `exceptions.ts` names its issue.
- A runner image change moves every baseline at once; the fix is a re-render commit, never a looser tolerance.
- A pixel that differs between two renders of one settled state fails the snapshot gate; its cause is found and fixed in the state table, never allowed by a margin.
