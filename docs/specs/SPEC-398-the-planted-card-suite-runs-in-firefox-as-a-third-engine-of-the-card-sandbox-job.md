# SPEC-398: the planted card suite runs in Firefox as a third engine of the card-sandbox job

- **Wave:** the app campaign, the card frame on the web (SPEC-341). **Issue:** #652 (Firefox in
  the browser test matrix). **Context(s):** `miniapp` (`web/app`) and CI (`.github/workflows`).
- **Decided by:** ADR-412 (this SPEC's own: the population, Firefox for the card frame only, the
  job and its bound, FORMAL, the pushes) and ADR-352 (the card frame's layers and their planted
  proof).
- **Schematic:** `docs/schematics/card-frame-channels.md`: section 9, the browser test matrix,
  which this delivery adds, and section 3's notes for each channel Firefox cannot observe.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-398.md`. **Mutation
  band:** S39800-S39899.

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `164ac206` by `git show 164ac206:<path>` and the line
range named.

- The planted card suite runs in two engines: `web/app/playwright.card.config.ts:17-20` declares
  the projects `chromium` and `webkit`, and nothing else; the suite's engine type is
  `'chromium' | 'webkit'` (`web/app/tests-card/planted.ts:9`) and its coverage test's engine list is
  the same two (`web/app/src/lib/card/planted-coverage.test.ts:18`).
- The `card-sandbox` job installs those two engines and no other
  (`.github/workflows/ci.yml:285-286`), runs the suite once (`:287-288`) and is bounded at 20
  minutes (`:273`); the aggregate `ci` check needs it (`:876`).
- One engine runs 144 planted tests and the two-engine job runs 288: `docs/red-first/SPEC-341.md`
  lines 55 and 102 (`git show 164ac206:docs/red-first/SPEC-341.md | sed -n '55p;102p'`).
- SPEC-341 excludes Firefox and cites #652 for it (SPEC-341, lines 183-184).
- The card frame's readings already differ by engine: `meta-refresh` is held by W1 "in some
  engines", `download` and the nested `srcdoc` child were measured in Chromium only, and WebKit and
  Chromium each cannot observe a different resource hint
  (`docs/schematics/card-frame-channels.md:100-104`, `:109`, `:112`). A verdict in two engines does
  not say what a third does.

## 2. Requirements

- R1. The card suite's Playwright configuration declares three projects, in order `chromium`,
  `webkit` and `firefox`; the `firefox` project spreads the runner's `'Desktop Firefox'` device.
  The configuration keeps one worker and no parallelism, so the engines run one after another.
- R2. The suite's engine type and the coverage test's engine list name `firefox`, so every channel
  the schematic calls unobservable in every engine is declared for Firefox too, with its reason.
- R3. The `card-sandbox` job installs Firefox with its system libraries in a step of its own, once,
  before the suite runs; its existing Chromium and WebKit install step and its suite command are
  unchanged; every project the card configuration declares is installed by one of its install
  steps; the suite step's name names the three engines.
- R4. The `card-sandbox` job's `timeout-minutes` is a whole number from 30 to 45, and this delivery
  sets 30. The job stays on the hosted runner label it uses today.
- R5. In Firefox, every planted card reaches its listener from the reference frame and nothing from
  the card frame, each layer removed alone opens exactly the channels the schematic gives it, the
  scripts-on measurement and the render proof hold, and the declared UNOBSERVABLE table for Firefox
  equals the set Firefox was measured unable to observe from the reference frame. No declaration
  covers an arrival from the card frame.
- R6. The verdict is read by the check-run name `card-sandbox`, which does not change, and the
  aggregate `ci` check needs the job as it does today.
- R7. The schematic gains section 9, the browser test matrix: each engine, the job that runs it,
  the card-frame and engine tests in it, and where each verdict is read. Each channel Firefox
  cannot observe gets a section 3 note in the form "UNOBSERVABLE in Firefox, measured: ...".
- R8. SPEC-341 and ADR-352 gain insert-only amendments that point here and name each of their
  sentences that says the web suite runs in two engines.

## 3. Acceptance criteria of SPEC-398

| id | criterion | decided by |
|---|---|---|
| A1 | the card configuration runs the planted suite in Chromium, WebKit and Firefox, and its projects are the suite's engines | `web/app/src/lib/card/planted-coverage.test.ts` "the card config runs the planted suite in Chromium, WebKit and Firefox" |
| A2 | the card-sandbox job installs Firefox once, before the suite, and installs every engine the card configuration declares | `scripts/tests/test_ci_workflows.py` `the_card_sandbox_job_installs_firefox_and_runs_the_suite_in_every_engine` |
| A3 | the card-sandbox job's bound holds three engines | `scripts/tests/test_ci_workflows.py` `the_card_sandbox_timeout_holds_three_engines` |
| A4 | every channel the schematic calls unobservable is declared in every engine, Firefox included | `planted-coverage.test.ts` "every channel in the schematic has a planted card" |
| A5 | in Firefox, a planted card reaches its listener from the reference frame and nothing from the card frame, each layer alone opens its channels, and the scripts-on and render proofs hold | the Playwright `web/app/tests-card/card.spec.ts` in `test:card`'s `firefox` project, read in the `card-sandbox` job; its engine coverage by `planted-coverage.test.ts` "the card config runs the planted suite in Chromium, WebKit and Firefox" |

The tdd probe resolves no Playwright command, so A5 names its Playwright spec in the table and its
fence line runs the Vitest test that proves the suite runs in Firefox, as SPEC-341's A9 to A12 do.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the card config runs the planted suite in Chromium, WebKit and Firefox"
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_card_sandbox_job_installs_firefox_and_runs_the_suite_in_every_engine
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_card_sandbox_timeout_holds_three_engines
A4: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every channel in the schematic has a planted card"
A5: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the card config runs the planted suite in Chromium, WebKit and Firefox"
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/playwright.card.config.ts` | `miniapp` | changed: the `firefox` project; the header comment names three engines |
| `web/app/tests-card/planted.ts` | `miniapp` | changed: `Engine` names `firefox`; the Firefox UNOBSERVABLE entries |
| `web/app/src/lib/card/planted-coverage.test.ts` | `miniapp` | changed: the engine list names `firefox` (A4); A1 added |
| `.github/workflows/ci.yml` | CI | changed: `card-sandbox`'s Firefox install step, its comment, its suite step's name and its bound |
| `scripts/tests/test_ci_workflows.py` | CI | changed: A2 and A3 added, with their constants |
| `scripts/mutation-rows.d/S39800-S39899.json` | CI | added: S39800 to S39802 |
| `docs/schematics/card-frame-channels.md` | docs | changed: section 9 added; section 3's Firefox notes |
| `docs/specs/SPEC-398-the-planted-card-suite-runs-in-firefox-as-a-third-engine-of-the-card-sandbox-job.md` | docs | added |
| `docs/decisions/ADR-412-firefox-joins-the-browser-matrix-for-the-card-frame-only-as-a-third-project-of-the-card-sandbox-job.md` | docs | added |
| `docs/specs/SPEC-341-a-card-face-renders-on-the-web-in-a-sandboxed-frame-that-reaches-neither-the-app-nor-the-network.md` | docs | changed, insert-only: an amendments section |
| `docs/decisions/ADR-352-a-card-face-runs-no-script-and-each-platform-closes-every-other-channel-with-a-named-layer-a-planted-card-proves.md` | docs | changed, insert-only: an amendments section |
| `docs/red-first/SPEC-398.md` | docs | added |
| `changelog.d/firefox-matrix-398.md` | docs | added |

## 5. What this does NOT cover

- It does not run the engine tests (`tests-engine/engine.spec.ts`, `tests-engine/sync.spec.ts`) in
  Firefox: the engine configuration and its sync suite are open work in #748, and the engine half
  of the matrix stays with #764.
- It does not run the study, e2e or accessibility suites in Firefox: they check layout, flow and
  accessibility rather than a boundary the engine enforces (#764).
- It does not make `card-sandbox` a required context of its own on the `dev` ruleset; the aggregate
  `ci` check already needs it (#764).
- It does not test a learner's installed Firefox with its own preferences, extensions or policies;
  the suite runs the engine the test runner installs (#764).
- It does not change any layer of the card frame: a Firefox reading that opens a channel from the
  card frame is a new design, never an UNOBSERVABLE declaration (#652).
- It does not touch the iPhone and iPad card view or its planted suite (#616).
- It does not write the campaign's threat model, whose card-frame citations this delivery leaves
  in place (#653).

## 6. Risks

- **Firefox cannot observe a channel from the reference frame** (a resource hint, a hyperlink
  audit, or a peer connection to the loopback listener). Detected by push 2's `card-sandbox` run:
  the pair's reference never reaches its listener. The channel is declared for Firefox with the
  measured reason, and a declaration that would leave a channel observable in no engine stops the
  build, because the suite would then prove nothing about it.
- **Firefox opens a channel from the card frame, or a single layer opens a different set.**
  Detected by the pair's card-frame assertion and by the single-layer variants. Either stops the
  build for a new design; neither is declared away.
- **Firefox renders no `srcdoc` frame under `frame-src 'none'`.** Detected by the render proof, a
  blank card frame failing it; ADR-352 D5 would then need another mechanism.
- **The bound is short for three engines.** Detected by the job's own timeout, and by push 2's
  measured time against two-thirds of the bound, which stops the build for a re-size.
- **A late Firefox arrival lands after the settle window.** The window is the reference's measured
  latency for that card times the suite's factor, with a floor, per engine, and a sentinel after
  it proves the listener still counted (SPEC-341's last risk), so a slow engine widens its own
  window.
- **Another delivery edits `scripts/tests/test_ci_workflows.py` or `.github/workflows/ci.yml`.**
  Detected by the builder's re-measure at the cut, and by the anchor census over every row on each
  touched file before each commit.

## 7. Mutation rows

The band is `scripts/mutation-rows.d/S39800-S39899.json`, in its `SCRIPT_MUTATIONS` table.

| row | file | mutant | killer |
|---|---|---|---|
| `S39800-THE-CARD-JOB-INSTALLS-FIREFOX` | `.github/workflows/ci.yml` | the Firefox install step runs nothing | A2 |
| `S39801-THE-CARD-CONFIG-RUNS-FIREFOX` | `web/app/playwright.card.config.ts` | the `firefox` project is removed | A2 |
| `S39802-THE-CARD-JOB-IS-BOUNDED-FOR-THREE-ENGINES` | `.github/workflows/ci.yml` | the bound falls back to 20 minutes | A3 |
