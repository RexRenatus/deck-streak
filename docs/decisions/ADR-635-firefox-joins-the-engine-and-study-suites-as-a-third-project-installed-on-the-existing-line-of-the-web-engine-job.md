---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Firefox joins the engine and study suites as a third project, installed on the existing line of the web-engine job

## Context and Problem Statement

#764 tracks adding Firefox to the engine suite (`web/app/tests-engine`, run by
`web/app/playwright.engine.config.ts`) and to the study suite (`web/app/tests-study`, run by
`web/app/playwright.study.config.ts`), each with its job's bound sized from a measured job time and
each engine's red read by name in CI. ADR-412 D2 put Firefox in the planted card suite only and
kept these two suites on Chromium and WebKit while the engine's sync suite was open work in #748,
which has since merged. SPEC-398 and ADR-412 cite #764 for the rest.

A learner on a desktop browser can open the app in Firefox, where the engine in its Worker over
OPFS, its Web Lock, its sync and the review loop have never run under test. This ADR decides how
the two configurations gain Firefox, how the `web-engine` job installs it, how Firefox's engine
project syncs, how long the job may take, and how the delivery is cut.

Every `file:line` below was read at DeckStreak `dev` `b065fce`.

## Decision Drivers

- The engine's guarantees are functional: a Firefox difference in OPFS, the Web Lock, the Worker or
  the sync client fails where a learner sees it, and only a test run in Firefox shows it first.
- An existing check-run name, row anchor and held test constant stay unchanged wherever the change
  can sit beside them.
- Each engine project syncs into a server collection of its own (SPEC-377 A1 to A3).
- Hosted runners only, under the runner label every job of the workflow already uses.

## The population, measured

| configuration | engines, where declared | job and steps that run it | its tests |
|---|---|---|---|
| engine, `web/app/playwright.engine.config.ts` | `chromium` and `webkit` (`:37-40`), one worker, not parallel (`:32-33`) | `web-engine`; install `ci.yml:1091-1092`, suite `:1093-1094` | `tests-engine/engine.spec.ts`, 8 tests, two of them run in one engine each (`:230`, `:250`); `tests-engine/sync.spec.ts`, 8 tests |
| study, `web/app/playwright.study.config.ts` | `chromium` and `webkit` (`:19-22`), one worker, not parallel (`:14-15`) | `web-engine`; the same install, suite `:1103-1104` | `tests-study/study.spec.ts`, 8 tests |

The engine configuration's sync server has one user per project, `SYNC_USER1` the run's base user
and `SYNC_USER2` `<base>-webkit` (`playwright.engine.config.ts:58-59`), and `account()` gives WebKit
`<base>-webkit` and any other project the base user (`tests-engine/sync.spec.ts:64-71`).

## Decisions, and the alternatives each was chosen against

### D1. Firefox is the third project of both configurations

Firefox is the third project of the engine configuration and of the study configuration, after
`webkit`, named `firefox`, spreading the runner's `'Desktop Firefox'` device. One worker and no
parallelism stay as they are, so the three engines run one after another. This amends ADR-412 D2
for these two suites only: the e2e configuration keeps its one engine, and the card configuration
is unchanged.

Chosen against:

- Firefox before WebKit, or first: rejected because every existing project keeps its place, so each
  engine's existing results, profiles and server user are unchanged and Firefox's are added after
  them.
- A configuration of its own for Firefox: rejected because it would repeat each configuration's
  servers, ports and environment, and the job would need a further suite step, while a third
  project needs neither.
- The e2e and accessibility suites too: rejected because they check layout, flow and
  accessibility in the runner's default engine, not the engine in the browser (#652).

### D2. The web-engine job installs Firefox on its existing install line

The install step's line becomes `pnpm --dir web/app exec playwright install --with-deps
--only-shell chromium webkit firefox`, and its name names the three browsers. `--only-shell`
applies to the Chromium alias alone: Playwright's registry resolves `chromium` to its headless
shell under that flag and passes `webkit` and `firefox` through unchanged (`resolveBrowsers` in the
runner's registry). The two suite steps keep their commands and name the three engines.

Chosen against:

- A separate `install --with-deps firefox` step, as the card job has: rejected because that run
  line would then occur twice in `ci.yml`, and row S39800 anchors on it, while the rows runner needs
  each anchor exactly once in its target (`scripts/mutation_rows.py` line 950). No row and no held
  test constant anchors on the `web-engine` install line; row S34102's anchor ends on the card job's
  next step.
- A job of its own for Firefox: rejected as in ADR-412 D3: a new check-run name, a second build of
  the module, and the aggregate's needs list changed.
- A matrix over the three engines: rejected as in ADR-412 D3: each leg reports a check-run under a
  new name, and each leg builds the module again.

### D3. Each engine project syncs as a server user of its own

`SYNC_USER3` names `<base>-firefox`, and `account()` gives `chromium` the base user and any other
project `<base>-<project>`, so `webkit` keeps `<base>-webkit` and no project's uploads reach
another's server collection (SPEC-377 A1 to A3). The engine's sync server reads `SYNC_USER1`,
`SYNC_USER2` and on until the first missing index (the pinned engine's
`rslib/src/sync/http_server/mod.rs`, `new_from_env`, lines 94-98), so the entries are numbered from
1 without a gap.

Chosen against:

- A third branch for Firefox in `account()`: rejected because the next project would need another,
  while one rule gives every project but Chromium its own name.
- One shared user for all projects: rejected because one project's upload would then reach the
  collection another project downloads, which SPEC-377 refuses.
- A user built from the project's index: rejected because the names would then depend on the
  projects' order, and WebKit's existing user would move.

### D4. The bound stays as it is, sized from measured job times

The bound stays `timeout-minutes: 45` (`ci.yml:1013`). The five latest successful `dev` runs of the
`web-engine` job, in two engines, read from the jobs API (durations in seconds):

| run | job | job s | engine suite step s | study suite step s |
|---|---|---|---|---|
| 41796046856 | 126460294844 | 563 | 203 | 31 |
| 41772806414 | 126386179876 | 448 | 140 | 26 |
| 41745076887 | 126295917993 | 434 | 141 | 27 |
| 41649243317 | 125962723847 | 488 | 146 | 30 |
| 41616970396 | 125852731159 | 647 | 222 | 37 |

A third engine scales the two suite steps by about three halves and adds Firefox's download to the
install step. The second push's `web-engine` job time is measured and recorded with its job id; a
time over two-thirds of the bound stops the build for a re-size from that measurement. The existing
band test (`WEB_ENGINE_TIMEOUT_MINUTES`, `scripts/tests/test_ci_workflows.py` line 3116) keeps
holding the bound.

Chosen against:

- Raising the bound now: rejected because the measured jobs leave the bound most of its room, and
  a bound is re-sized from a measurement, not ahead of one.
- A larger runner: rejected because the bound fits a hosted runner and the design admits no other.

### D5. FORMAL is not applicable, by surface

The projects run in turn in one worker, each against its own persistent profile and its own server
user. No product actor, state or step is added, and nothing here is a total function, a bound over
values or a transition table.

Chosen against:

- A TLA+ model of the three projects: rejected because they run in turn in one worker and no second
  actor touches another's profile or server collection, so there is no interleaving to check.
- A Lean proof of the bound: rejected because the bound is a measured time budget, which the band
  test and the second push's measured run decide, not an invariant over values.

### D6. Two pushes

The first push carries the documents and the red tests, and its CI reads the reds by name. The
second carries the wiring, the rows and the red-first record, and its `web-engine` run is the first
measurement of both suites in Firefox.

Chosen against:

- One push: rejected because the three unittest reds run only in CI, so their red must be read
  there before the wiring lands.
- Three pushes, the record after the wiring's run: rejected because no Firefox reading here is a
  declaration the tree must carry; a Firefox failure stops the build instead.

## Decision Outcome

Firefox becomes the third engine of the engine suite and of the study suite, installed on the
existing install line of the `web-engine` job, each of its engine projects syncing as a server user
of its own, under the same bound, with its verdict read as `web-engine`. The delivery is two
pushes.

### Consequences

- Good: the engine in its Worker over OPFS, its sync and the review loop are proved in the three
  engines a desktop learner is likely to use.
- Good: no check-run name, aggregate need, existing row anchor or held test constant changes.
- Bad: the two suite steps take about half as long again.
- Neutral: the two engine tests that already run in one engine each keep their conditions, so in
  Firefox both are skipped by their existing conditions.

### Confirmation

SPEC-613's A1 to A5, and the `web-engine` check-run on every pull request.

## What would make this wrong

- A Firefox test fails in the engine, sync or study suite: the engine or the app does not hold in
  Firefox, and that failure is a design question, never a skip.
- The second push's measured run takes more than two-thirds of the bound: D4's bound is re-sized
  from that measurement.
- Firefox needs a profile, lock or storage setting no other engine needs: D1's single project per
  engine is revisited.

## More Information

- SPEC-613 (this ADR's SPEC); SPEC-398 and ADR-412 D2 and D3; SPEC-377 A1 to A3; SPEC-338 R9;
  SPEC-350 R13.
- The schematic `docs/schematics/firefox-engine-study-613.md`.
