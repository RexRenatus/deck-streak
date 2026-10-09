---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Firefox joins the browser matrix for the card frame only, as a third project of the card-sandbox job

## Context and Problem Statement

#652 asks whether Firefox joins the browser test matrix for the card frame and for the engine in
the browser, and which job would run it. SPEC-341 (the card frame on the web) runs its planted
suite in Chromium and WebKit and excludes Firefox, citing #652 from its exclusion section.

The card frame is a boundary the browser engine enforces: the sandbox with no token (W1), the
frame's own policy in its `srcdoc` document (W2), the strip (W3), the page's `frame-src 'none'`
(W4) and the inherited page policy (P). The planted suite is the only proof that each layer
holds, and it proves it once per engine. A learner on a desktop browser can open the app in
Firefox, where no test has read the card frame. This ADR decides which browser tests Firefox
joins, which job runs it, how long that job may take, and how the delivery is cut.

Every `file:line` below was read at DeckStreak `dev` `164ac206`.

## Decision Drivers

- A card frame that opens a channel fails silently: nothing a learner sees changes, and only the
  planted suite's listeners see the arrival, in the engine the suite ran.
- Each layer is enforced by the engine, and the tree already records readings that differ by
  engine: `meta-refresh` is held by W1 "in some engines", and `download` and the nested `srcdoc`
  child were measured in Chromium only (`docs/schematics/card-frame-channels.md:104`, `:109`,
  `:112`); ADR-352 D3 rejected the frame's `csp` attribute because one target engine does not
  enforce it.
- An existing check-run name, row anchor and held test constant stay unchanged wherever a new step
  can sit beside them.
- Hosted runners only, under the runner label every job of the workflow already uses.
- The engine configuration and its sync suite are open work in #748.

## The population, measured

| configuration | engines, where declared | job that runs it | its tests |
|---|---|---|---|
| e2e, `web/app/playwright.config.ts` | no `projects` key (`:3-10`), so the runner's default engine, Chromium; the job installs Chromium's headless shell only (`.github/workflows/ci.yml:241-243`) | `web` (`ci.yml:201-253`, its suites through the gate's web stage, `:250-253`) | `tests/a11y.spec.ts:296`, `tests/smoke.spec.ts:19`, `:27`, `:33`, `tests/streak-calendar.spec.ts:85`, `tests/card-policy.spec.ts:11` |
| card, `web/app/playwright.card.config.ts` | `chromium` and `webkit` (`:17-20`), one worker, not parallel (`:12-13`) | `card-sandbox` (`ci.yml:268-295`; install `:285-286`, suite `:287-288`) | `tests-card/card.spec.ts`: the census `:101`, 28 pairs `:109`, 112 single-layer variants `:133`, scripts on `:143` and `:158`, the render proof `:185`; 144 tests per engine |
| engine, `web/app/playwright.engine.config.ts` | `chromium` and `webkit` (`:36-39`) | `web-engine` (`ci.yml:767-838`; install `:829-830`, suite `:831-832`) | `tests-engine/engine.spec.ts:55`, `:104`, `:132`, `:169`, `:189`, `:214`, `:292`; `tests-engine/sync.spec.ts:99`, `:126`, `:153`, `:177` |
| study, `web/app/playwright.study.config.ts` | `chromium` and `webkit` (`:19-22`) | `web-engine` (`ci.yml:837-838`) | `tests-study/study.spec.ts:87`, `:102`, `:122`, `:138`, `:159`, `:182`, `:197` |

The card-frame tests are `card.spec.ts` whole and `card-policy.spec.ts:11`, which reads the built
page's policy text and so does not depend on the engine it runs in. The engine tests are
`engine.spec.ts` and `sync.spec.ts`; two of them are already skipped in one engine each
(`engine.spec.ts:170`, `:190`).

## The card frame's guarantees, and the planted tests that hold each per engine

| id | guarantee | held by, in each engine the suite runs | what could differ in Firefox |
|---|---|---|---|
| G1 | W1, the sandbox with no token: no top navigation, popup, form or download | the `nav-top`, `nav-blank`, `form`, `download` and `external-scheme` pairs (`card.spec.ts:109`), the `W1 off` variants (`:133`) | the engine's sandbox flags, and whether it treats `meta-refresh` as a navigation W1 stops |
| G2 | W2, the frame policy as a meta element in a `srcdoc` document | the `img`, `css-url`, `css-import`, `font`, `media` and `nested-frame` pairs, the `W2 off` variants | how a meta policy applies to a `srcdoc` document and its nested child |
| G3 | W3, the strip and `x-dns-prefetch-control` off | the `stylesheet`, `preload`, `prefetch`, `preconnect`, `dns-prefetch`, `shadow-link`, `meta-refresh` and `base` pairs, the `W3 off` variants | which resource hints the engine acts on; two engines already read differently (`card-frame-channels.md:100-103`) |
| G4 | W4, `frame-src 'none'` on the frame's own navigation (SEC01-F13) | the `nav-self` and `download` pairs, the `W4 off` variants | whether a `download` link to another origin is followed as a navigation, measured in Chromium only (`:109`) |
| G5 | P, the inherited page policy | the `ping`, `object`, `script-inline`, `script-src`, `event-handler` and `javascript-url` pairs | how the page policy is inherited into a `srcdoc` document |
| G6 | scripts off holds the peer connection (SEC01-F14) | the `webrtc` pair and the scripts-on measurement (`:143`) | whether the engine's peer connection reaches the loopback listener at all (SPEC-341's first risk) |
| G7 | an opaque origin: the parent message arrives from origin `null` (SEC01-F15) | the `bridge` pair and its scripts-on test (`:158`) | the origin the engine reports for a sandboxed `srcdoc` document |
| G8 | a `srcdoc` frame renders under `frame-src 'none'` | the render proof (`:185`) | whether the engine applies `frame-src` to a document it does not fetch (ADR-352's first "What would make this wrong") |

## Decisions, and the alternatives each was chosen against

### D1. The population is the four browser configurations' suites, counted by configuration and engine

The browser tests are the suites of the four Playwright configurations in the table above. The
card-frame tests among them are `card.spec.ts` and `card-policy.spec.ts`; the engine tests are
`engine.spec.ts` and `sync.spec.ts`.

Chosen against:

- Counting the Vitest suites as browser tests: rejected because they run with no browser engine
  (`test` is `vitest run`, `web/app/package.json:13`), so no engine difference can show in them.
- Counting by CI job: rejected because one job runs two configurations (`web-engine` runs both the
  engine and the study suites, `ci.yml:831-838`), and the e2e configuration runs inside the web
  job's gate stage, so a job count hides which suite meets which engine.

### D2. Firefox joins for the card-frame tests only

Firefox becomes the third engine of the planted card suite, and holds G1 to G8 the way Chromium
and WebKit do. The engine, study and e2e suites keep the engines they run in today.
`card-policy.spec.ts` stays where it is: its verdict does not depend on the engine.

A Firefox reading in which a planted card reaches a listener from the card frame, a single layer
opens a channel the schematic does not give it alone, or the render proof shows a blank frame, is
a stop for a new design, never an UNOBSERVABLE declaration. Only a channel whose reference frame
cannot reach its listener in Firefox is declared, with its measured reason, and the declared set
equals the measured set, as it does for the other two engines.

Chosen against:

- The engine tests too: rejected because the engine's guarantees are functional, so a Firefox
  difference fails in use where a learner sees it, while a card-frame failure is a silent channel
  that only the planted suite sees; and the engine configuration and sync suite are open in #748,
  so joining now would wait on it.
- The whole browser suite: rejected because the e2e, accessibility and study suites check layout,
  flow and accessibility, not a boundary the engine enforces, and a third engine across them adds
  load to two jobs for no guarantee the planted suite does not already prove.
- Not at all: rejected because every card-frame layer is enforced by the engine and the tree
  already records readings that differ by engine (G1, G3, G4), so a verdict in Chromium and WebKit
  does not carry to Firefox.

### D3. The card-sandbox job runs Firefox as a third project, from its own install step, bounded at 30 minutes

The card configuration gains a third project after `webkit`, named `firefox`, spreading the
runner's `'Desktop Firefox'` device. One worker and no parallelism stay as they are
(`playwright.card.config.ts:12-13`), so the three engines run one after another.

The `card-sandbox` job gains one step before its existing install step, which installs Firefox with
its system libraries: `pnpm --dir web/app exec playwright install --with-deps firefox`. The
existing Chromium and WebKit install step keeps its line, which SPEC-341's row S34102 anchors on
and A13's test holds exactly. The suite step keeps its command and its name's prefix, and names
the three engines.

The job stays on the hosted runner label it uses today (`ci.yml:272`). Its bound moves from 20 to
30 minutes, inside a tested band of 30 to 45. The existing bound holds two engines run in turn
(`ci.yml:273`), so three engines scale it by three halves. The suite's settle windows alone take
at least about four minutes per engine, by its own constants (`card.spec.ts:13-19`). The measured
time of push 2's run is recorded, and a run that takes more than two-thirds of the bound stops the
build for a re-size from that measurement.

The verdict is read by the check-run name `card-sandbox`, which does not change. The aggregate `ci`
check already needs the job (`ci.yml:876`). The job's log shows the third project ran with the
census line `examined 28 planted cards, 28 pairs, 112 variants in firefox` (`card.spec.ts:103`).
Making `card-sandbox` a required context of its own on the `dev` ruleset is not this decision's
(SPEC-398 excludes it).

Chosen against:

- A job of its own for Firefox: rejected because `card-sandbox` would then need a project filter,
  which moves the suite line S34101 anchors on and A13's held command, the new job repeats the
  checkout, setup and dependency install, and the aggregate's exact needs list and its row S34103
  change.
- A matrix over the three engines: rejected because a matrixed job reports one check-run per leg
  under a new name, so `card-sandbox` is no longer the name read, and A13's test and both of
  SPEC-341's install and suite anchors are rewritten.
- A leg of `web-engine` or `web`: rejected because `web-engine` already builds the engine for the
  browser and runs two suites under its 45-minute bound (`ci.yml:769`), and `web` runs the gate's
  web stages over a Chromium-only browser cache (`ci.yml:235-249`).
- One install line naming all three browsers: rejected because it rewrites the line S34102 anchors
  on and A13's test holds exactly, and `--only-shell` applies to Chromium alone, so the line gains
  nothing a separate step does not give.
- A larger runner: rejected because the bound fits a hosted runner and the design admits no other.

### D4. FORMAL is not applicable, by surface

A test matrix adds no product actor, state or step. The three projects run one after another in
one worker (`playwright.card.config.ts:12-13`), the listeners are that worker's module state
(`:2-3`), the harness server only serves the planted pages, and the one job writes one results
upload whose name does not change, so no two actors read or write shared state. Nothing here is a
total function, a bound over values or a transition table either.

Chosen against:

- A TLA+ model of the three projects: rejected because they run in turn in one worker and no
  second actor touches their listeners, so there is no interleaving to check.
- A Lean proof of the bound: rejected because the bound is a measured time budget, which the band
  test and push 2's measured run decide, not an invariant over values.

### D5. No wait on open work, and three pushes

No file in this delivery's manifest is changed by #748 (measured from its changed paths), so the
build does not wait on it. #752 changes `scripts/tests/test_ci_workflows.py`, which this build
also changes; the builder re-measures that file at the cut. The threat model's card-frame rows cite
`policy.js`, `svelte.config.js`, `csp.test.ts` and `card-frame.test.ts`, none of which this build
changes, so their citations do not move (#653). SPEC-057's campaign row for `miniapp` does not move
either: the build changes no production file under `web/app/src`, the only files the web mutation
tool mutates.

The delivery takes THREE pushes. Push 1 carries the red commit alone, because two of its criteria
run only in CI and their red is read there. Push 2 carries the wiring; its `card-sandbox` run is the
first measurement of the suite in Firefox. Push 3 carries the Firefox declarations that measurement
names, the red-first record, the rows and the changelog fragment.

Chosen against:

- Two pushes, the declarations with the wiring: rejected because the declared set must equal the
  measured set, and nothing measures Firefox before push 2's run: the delivery installs no browser
  system packages outside CI.
- Cutting after #748 lands: rejected because the two share no file, so a wait buys nothing.

## Decision Outcome

Firefox becomes the third engine of the planted card suite, run by the existing `card-sandbox` job
from its own install step, under a 30-minute bound, with its verdict read as `card-sandbox`. The
engine, study and e2e suites keep their engines. The delivery is three pushes.

### Consequences

- Good: every card-frame guarantee is proved in the three engines a desktop learner is likely to
  use, by the same planted cards and listeners.
- Good: no check-run name, aggregate need, existing row anchor or held test constant changes.
- Bad: `card-sandbox` takes about half as long again, and its bound grows to match.
- Bad: a channel Firefox cannot observe joins the declared UNOBSERVABLE table, so that channel is
  proved in the other engines only.
- Neutral: the engine in the browser is not tested in Firefox; that half of #652 is a follow-up.

### Confirmation

SPEC-398's A1 to A5, and the `card-sandbox` check-run on every pull request.

## What would make this wrong

- A planted card reaches a listener from the card frame in Firefox: the layer table is wrong for
  Firefox, and the frame does not ship there until it is right (ADR-352's own condition).
- Firefox renders no `srcdoc` frame under `frame-src 'none'`: the render proof shows it, and
  ADR-352 D5 needs another mechanism.
- Push 2's measured run takes more than two-thirds of the bound: D3's bound is re-sized from that
  measurement.
- A Firefox-only defect in the engine in the browser reaches learners: D2's choice to leave the
  engine tests out is revisited in the follow-up to #764.

## More Information

- SPEC-398 (this ADR's SPEC); SPEC-341 R8, R12, A9 to A13; ADR-352 D3, D5 and D7.
- The schematic `docs/schematics/card-frame-channels.md`: section 3 (the web channels), section 5
  (the planted suite) and section 9 (the browser test matrix, which SPEC-398 adds).
