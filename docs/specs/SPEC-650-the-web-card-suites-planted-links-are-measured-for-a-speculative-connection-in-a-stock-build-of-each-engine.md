# SPEC-650: the web card suite's planted links are measured for a speculative connection in a stock build of each engine

- **Context(s):** the web card suite's measurement (`web/app/tests-card/speculative.measure.ts`,
  `web/app/playwright.speculative.config.ts`, `web/app/tsconfig.json`), its workflow
  (`.github/workflows/card-speculative.yml`), and `docs` (ADR-412, with one appended amendment).
- **Decided by:** ADR-672, and ADR-412's amendment for #941
- **Status:** delivered with its measurement workflow (#941).

## 1. The problem, measured

The card suite runs Chromium as its headless shell, and WebKit and Firefox as the builds Playwright
installs (`web/app/playwright.card.config.ts:17-21`, `.github/workflows/ci.yml:313-316`). Under it,
Chromium and Firefox open no `preconnect` connection (`web/app/tests-card/planted.ts:245-250`), so a
link predictor that only a stock build runs is not exercised (#941; SPEC-461 and SPEC-641 section
5; ADR-478 D1, D6). Playwright's full Chromium build, its `chromium` channel, is the real browser in
headless mode; Playwright drives no Firefox or WebKit build but the ones it installs.

## 2. Requirements

- R1. `web/app/playwright.speculative.config.ts` runs `web/app/tests-card/speculative.measure.ts` alone, on one worker, in four projects: Chromium's full build (`chromium-full`), the suite's headless shell (`chromium-shell`), WebKit and Firefox.
- R2. The measurement opens each planted card a link predictor could act on, the two preconnect hints and every card whose link points at the TCP listener, nine cards by name, in the reference frame and in the card frame, then hovers the frame's centre, then clicks it when the card acts on a click, and after each step reads the TCP listener's accepted connections and request paths.
- R3. Each reading is printed in the log and written to the test's output, and no reading is judged.
- R4. Its controls are judged: the population is the nine cards, by name; a bare connection the file opens reads `tcp` in the same run; the sentinel arrives after each settle window; and `nav-self`'s followed link in the reference frame reads its connection in every project.
- R5. `.github/workflows/card-speculative.yml` runs it on a pull request into `dev` that changes the measurement, its configuration, the planted cards, the listeners or the workflow, and uploads `web/app/test-results/` as `card-speculative-results`. It is no need of `ci`.
- R6. The card suite, its configuration, the `card-sandbox` job, the aggregate's needs and the coverage tests are unchanged.

## 3. Acceptance criteria of SPEC-650

| id | criterion | decided by |
|---|---|---|
| A1 | the population is the nine cards, and every control holds in every project | `card-speculative`'s `measure` job on the delivery's pull request |
| A2 | each project's reading is recorded for each card, frame and step | the same run's log lines and its `card-speculative-results` artifact |

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/card-speculative.yml` | ci | added |
| `web/app/playwright.speculative.config.ts` | tests | added |
| `web/app/tests-card/speculative.measure.ts` | tests | added |
| `web/app/tsconfig.json` | config | changed: one include line, for the measurement's configuration |
| `docs/specs/SPEC-650-the-web-card-suites-planted-links-are-measured-for-a-speculative-connection-in-a-stock-build-of-each-engine.md` | docs | added |
| `docs/decisions/ADR-672-a-measurement-workflow-reads-the-planted-links-in-a-stock-build-of-each-engine-beside-the-card-suite-and-outside-its-job.md` | docs | added |
| `docs/decisions/ADR-412-firefox-joins-the-browser-matrix-for-the-card-frame-only-as-a-third-project-of-the-card-sandbox-job.md` | docs | changed: one amendment appended |
| `changelog.d/card-suite-speculative-941.md` | docs | added |

## 5. What this does NOT cover

- Branded Google Chrome or Microsoft Edge, which Playwright drives only through their own channels, and branded Firefox or Safari, which it does not drive.
- Firefox's own preferences: Playwright's Firefox runs under Playwright's profile.
- Any reading turned into a judged assertion or an UNOBSERVABLE change: a stock build that opens a connection from the card frame is a new design, never a declaration.

## 6. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani. The change is a measurement test, its configuration, a
workflow of its own and documents; no product actor, state, step or engine path gains or changes.
