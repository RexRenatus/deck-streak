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

## Amendments after the first Firefox run, and what each was chosen against

The second push's `card-sandbox` run (job 114037978095) was the planted suite's first reading in
Firefox. Chromium and WebKit passed every planted test; Firefox passed all but nine. Seven of the
nine follow from four references that reached no listener in Firefox: the `preconnect`,
`shadow-link`, `ping` and `webrtc` pairs, the `W3 off` variants of the first two, and the
scripts-on peer connection. The other two are the `img` card's `W1 off` and `W4 off` variants, each
of which read `/img/2` and `/img/3` at the listener. D2 makes that a stop for a new design, never a
declaration. D6 to D12 are that design. They amend D2, D3 and D5 where they say so, and every
`file:line` in them was read at the second push's head, `04a5ab17`.

### D6. In Firefox, W2 holds five of the `img` card's seven forms alone, and its two image-set forms only beside W1 and W4

Each of the `W1 off` and `W4 off` variants read exactly `/img/2` and `/img/3`, the card's
`img srcset` and `picture source` forms (`web/app/tests-card/planted.ts:94-102`), and nothing else.
The `W2 off` variant opened the card, and the pair, its shipped frame included, and the `W3 off`
variant read as section 3's table says. With W1 off, the frame is the shipped document with no
sandbox (`web/app/tests-card/harness/main.ts:71`); with W4 off, the page is the shipped page with
no `frame-src` (`web/app/vite.card.config.ts:23-28`, `harness/main.ts:62-64`). In both, the one
layer that governs an image fetch is W2, the frame policy placed first in the frame's head
(`web/app/src/lib/card/policy.js:25-26`, `web/app/src/lib/card/frame-document.ts:20`): W1 and W4
stop no fetch (`docs/schematics/card-frame-channels.md:130-135`), W3 keeps every `img` and
`source` element (`frame-document.ts:17`), and the inherited page policy names no image source
(`web/app/svelte.config.js:24-31`). So W2 holds `/img/1` and `/img/4` to `/img/7` alone in
Firefox, as it does in Chromium and WebKit, and does not hold `/img/2` and `/img/3` alone.

The cause is the engine's speculative parse, read in its public source. Firefox's HTML parser
fetches images ahead of the tree builder. A meta policy it meets ahead of the tree builder is kept
only as the document's preload policy (`nsHtml5TreeOpExecutor::AddSpeculationCSP`), and the policy
check consults that preload policy only for the preload content types (`CSPService::ConsultCSP`,
`nsContentUtils::IsPreloadType`). A plain image is fetched ahead as an image preload, which W2's
policy covers. An image-set candidate, from `srcset` or a `picture` `source`, is fetched ahead
typed as an image set (`Document::PreLoadImage`), which is not a preload type, so only the
document's own policy is consulted, and the meta element is not yet part of it.

With every layer on, and with W3 off, the two forms stayed closed in the measured run. That closure
needs W1, W2 and W4 together, a conjunction no layer's design names, so the table cannot give it to
one layer. In Firefox, W2 holds the image-set forms only beside W1 and W4, never alone, and D7
gives them a layer built for them.

Chosen against:

- W2 holds no `img` form in Firefox: rejected because the five other forms never reached the
  listener with W1 off or with W4 off, and W2 is then the only layer on that governs an image fetch.
- A stray arrival from another test: rejected because every visit opens a fresh browser context
  (`web/app/tests-card/card.spec.ts:58-74`), both variants read the same two paths and no other,
  and those are exactly the two forms the engine fetches ahead as image sets.
- W1 or W4 holds image fetches by design: rejected because neither is built to stop a fetch
  (`card-frame-channels.md:130-135`), so a table giving them the image-set forms would claim a
  guarantee no layer is built to give.

### D7. A product change closes the image-set forms: the strip removes every `srcset`, in its own issue, and the last push here is read over it

The image-set forms are closed by a layer built for them: W3 removes the `srcset` attribute from
every element that carries one, `img` and `source`, and its re-parse check refuses the card if one
came back (`frame-document.ts:46-51`). Then no image-set candidate reaches the frame document in
any engine, and W2 holds every image form left, alone. That is a change to a layer, which SPEC-398
excludes, so it is its own delivery, #771, with its own SPEC and its own ADR amending
ADR-352's layer table, and this delivery does not make it. This delivery waits for it: its third
push is made after the strip is on `dev`, and that push's `card-sandbox` run, which checks out the
pull request merged with `dev`, is the reading.

No assertion of the planted suite changes here, in any engine. By measurement against `dev`, the
single-layer assertion (`card.spec.ts:137`) is one line for every engine, and this delivery leaves
that file as it is. The `img` card's single-layer expectations stay `{}` in Firefox as in Chromium
and WebKit, whose expectations are byte-identical. Nothing is weakened.

Chosen against:

- Firefox's `img` cells declared per engine, its expectation equal to the measured set: rejected
  because it narrows the single-layer assertion at `card.spec.ts:137` with an engine exception, a
  weakening; Chromium's and WebKit's expectations would stay byte-identical, but Firefox's two
  variants would assert less than theirs, and D2 makes such a reading a stop, never a declaration.
- Firefox joins without the `img` card's two variants, each excluded citing an issue: rejected
  because it removes two checks the second push already runs, a weakening, and it leaves the
  image-set forms with no single-layer reading in Firefox until a later delivery restores them.
- The strip made in this delivery: rejected because SPEC-398 changes no layer, and a layer change
  owes its own design against ADR-352, its own red-first record and its own review.
- An image source list in the page policy: rejected because it constrains every image the app
  shows, and it would make the inherited page policy, which this work does not own, hold what the
  frame's own layers should.
- The frame's `csp` attribute: rejected because ADR-352 D3 already rejected it, since one target
  engine does not enforce it.
- A rewrite of each `srcset` that keeps only `data:` candidates: rejected because it must parse
  candidate lists as each engine does, commas inside `data:` URLs included, where removing the
  attribute leaves nothing to parse and the element's `src` still shows the image.

### D8. Firefox's four unobservable channels, the card configuration's named read, and SPEC-341's two other sentences

The third commit declares `preconnect`, `shadow-link`, `ping` and `webrtc` unobservable in Firefox,
each a new line of the UNOBSERVABLE table (`planted.ts:197-207`) with its measured reason, and
section 3 of the schematic notes each in its layer-alone cell. In the second push's run each card's
reference frame, every layer off, reached no listener in Firefox, and the `W3 off` variants of the
first two and the scripts-on peer connection followed. Each channel stays observed elsewhere:
`preconnect` and `shadow-link` in WebKit, and `ping` and `webrtc` in Chromium and WebKit. So the
suite still proves each one closed, and the coverage test still finds every layer holding a channel
some engine observes (`web/app/src/lib/card/planted-coverage.test.ts:179-183`) and `webrtc`
measured in at least one engine (`:201-204`). A declaration is checked both ways: in an engine that
declares a channel, the reference must reach nothing (`card.spec.ts:114-116`).

A2's test reads the card configuration for its project names (`card_projects`,
`scripts/tests/test_ci_workflows.py:2497`). The module's census refuses any file read that is not
the workflow loader or a named read, so the configuration is named in `NOT_WORKFLOW_READS`
(`:4788-4811`) with its reason. That is a named read, not a filter: one entry is added, no
assertion is removed, and the census still refuses every read it does not name.

SPEC-341's section 7 gains one paragraph naming its two other two-engine sentences, section 4's row
for the card configuration and section 6's risk on the loopback interface. The changelog fragment
`changelog.d/firefox-matrix-398.md` was carried by the first push. The third commit adds the band
`scripts/mutation-rows.d/S39800-S39899.json`, whose three rows' finds each occur once at
`04a5ab17`.

Chosen against:

- A longer settle window for Firefox: rejected because the four references reached nothing at all,
  not late, and a wider window cannot make an engine send what it does not.
- The engine's own preferences switched on in the `firefox` project: rejected because the suite
  would then measure a configured engine rather than the engine as installed, which SPEC-398
  excludes (#764).
- The card configuration read through the workflow loader: rejected because the loader is the
  module's reader of workflow files, and the card configuration is not one.
- SPEC-341's two sentences left unnamed: rejected because SPEC-341's amendment says where to read
  three engines for two, and a sentence it does not name reads as current where it is history.

### D9. The card-sandbox bound stays 30 minutes, now by measurement

D3 sized the bound by scaling the two-engine bound; the second push measured it. The bound is the
per-engine cost at three engines, times 1.5, plus the job's setup and tail:

job 114037978095: Chromium 4m36s, WebKit 4m54s, Firefox 8m13s and the suite's start 3s, 17m46s; times 1.5, 26m39s; with setup 1m15s and tail 2s, 27m56s, inside the bound of 30 minutes.

Firefox's share includes seven references that waited out their poll before failing; the third
commit's declarations end those waits. A run over two-thirds of the bound still stops a build for a
re-size from its own measurement, as D3 says.

Chosen against:

- 45 minutes, the band's top: rejected because the measured need with its margin is inside 30, and
  a larger bound only lets a stuck run hold the runner longer.
- A bound sized from the job's total alone: rejected because a total hides which engine grew, and a
  slower Firefox is read from its own share.
- A bound under 30 minutes: rejected because A3's band starts at 30, and the measured need with its
  margin sits close below it, so a lower bound leaves no room for a slower runner.

### D10. Whichever of this delivery and #765 lands second renumbers its own schematic section

#765 also appends a `## 9.` to `docs/schematics/card-frame-channels.md`, after the same last line.
The pull request that lands second takes the next free number, `## 10.`, and appends its section
after the first lander's last line, insert-only: the file on `dev` is a byte prefix of the second
lander's file. The second lander renumbers only its own section and each reference to it in its own
SPEC, ADR and red-first record, and never edits the first lander's section. If #765 lands first,
this delivery's renumbering is its own amendment, made before its last push.

Chosen against:

- One section merging the two: rejected because each section is decided by its own SPEC and ADR,
  and a section with two owners has no single record to amend.
- Numbers fixed now, #765 as 9 and this delivery as 10: rejected because the order of landing is
  not known, and a section numbered past a missing one leaves a number no section holds.
- The second lander inserting its section before the first's: rejected because it moves the first
  lander's section, which an insert-only check against `dev` refuses.

### D11. FORMAL stays not applicable, by surface

D7 adds no product actor, state or step to this delivery. The third commit adds test data, notes
and a record; the strip is another delivery's. That delivery decides its own FORMAL by its own
surface: removing an attribute from a parsed card is a total function of the card's text, ground
for a proof or a property test there, not a model here.

Chosen against:

- A model of the engine's speculative fetch beside its tree builder: rejected because that
  interleaving is the engine's, not this delivery's code, and the planted suite reads its outcome
  in the engine itself.
- A proof here that the composed document holds no `srcset`: rejected because the strip is not in
  this delivery, so the proof would cover code this delivery does not change.

### D12. The third push is the last, and it is read over the strip

The delivery keeps D5's three pushes. The third carries the third commit alone, and it is made only
after the strip (#771) is on `dev`. No criterion the third commit touches reads red only in
CI: the second push read Firefox's `img` variants and the four references, and the third push's
`card-sandbox` run reads them over the strip, through the pull request's merge with `dev`. A5 stays
"not red", with the reason its record gives.

Chosen against:

- A fourth push carrying a red commit alone: rejected because the third commit adds no test whose
  red is unread; its declarations' red was the second push's reading.
- The third push now, before the strip: rejected because its run would read the same two variants
  open again and decide nothing new.
- A merge of `dev` into the branch before the third push: rejected because the run already checks
  out the pull request merged with `dev`, so a merge commit adds history without changing what CI
  reads; a conflict with `dev` is the one reason to make one, and that is its own amendment.

## What would make the amendments wrong

- With the strip on `dev`, a Firefox single-layer variant of an image form still opens: D6's cause
  is incomplete, and the strip's design is reopened before this delivery's last push.
- The strip changes which forms the `img` card plants or which layer the table gives them: this
  delivery's notes and declarations are re-read against the new table before its last push.
- Firefox later observes one of the four channels: its declaration then fails its own check, and
  the delivery that reads it removes the declaration and the note.
- The third push's run takes more than two-thirds of the bound: D9 is re-sized from that
  measurement.
