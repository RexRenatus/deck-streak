# SPEC-644: a scripted form card tries its form five times, so one failed try does not blind its reference

- **Issue:** #843. **Context(s):** the card view's probe suite, `ios/CardProbeTests/`.
- **Decided by:** ADR-667.
- **Status:** delivered by the pull request that adds this file, with its test and `docs/red-first/SPEC-644.md`.
- **Mutation rows:** none, test code that no mutant sweep reads.

## 1. The problem, measured

SPEC-355's scripted-view test, `test_a_scripted_card_reaches_nothing_from_the_scripted_view`, read one blind reference that
its pinned set does not hold, once, on the iPad simulator (job 115407471652, run 38427570442; #843).

- **The red** (job 115407471652). The planted suite's `xcodebuild` named the iPhone simulator, then the iPad
  simulator, with `-disable-concurrent-destination-testing`. Its first `PlantedCardTests` session read 14 of 14
  test cases passed, and its second read 1 of 14 failed (job 115407471652): that test read
  `blind scripted references: 3 ["capture", "script-form", "script-open"], pinned 2 ["capture", "script-open"]`,
  with this reading of the form's reference (job 115407471652):
  `scripted reference script-form without L12,L3,L5: reached=false took=- loaded=true paths=[] connections=0 datagrams=0 allowed=1 windows=0 messages=0 marker=true`.
  The reference loaded, its script set its marker, and its one form navigation was allowed (`allowed=1`).
- **Its cause** (job 115407471652). Between `script-nav`'s scripted reading and `script-form`'s reference reading
  of the second session, WebKit logged `didFailProvisionalLoadForFrame` with
  `isMainFrame=1, domain=NSURLErrorDomain, code=-1009`: the form's one navigation failed its provisional load as
  offline before anything reached the listener. It is that job's only NSURLErrorDomain provisional failure: of its
  53 `didFailProvisionalLoadForFrame` lines, 30 read WebKitErrorDomain 102, 22 read WebKitErrorDomain 104 and one
  reads NSURLErrorDomain -1009 (job 115407471652).
- **The card never tried again.** `script-form` submits its form once (`ios/CardProbeTests/Planted.swift` lines
  360-365 at the dev commit `bd953ba`), while its sibling `script-nav` assigns its location up to five times on a
  short loop (lines 353-359).
- **Across jobs** (run 38355861156 to run 42416540106). The 209 `apple / harness` jobs, every attempt, of the
  `apple-on-change` runs in that range read 107 success, 73 failure, 19 cancelled, 7 skipped and 3 in progress at
  the dev commit `bd953ba` (jobs 128512699711, 128535158730 and 128542879622). In each success the test passed on
  both simulators (sampled: jobs 115162512195 and 128504947552, each with two
  `scripted reference script-form ... reached=true` lines and two passed test cases). Of the 92 that failed or were
  cancelled, 55 reached the test (51 in both sessions, 4 in one), 106 sessions in all, and `script-form`'s
  reference reached in 105 of them; the one miss is job 115407471652. Per simulator: the iPhone simulator 0
  misses, the iPad simulator 1.
- **Since the per-simulator report** (#1147). In the 46 jobs from its merge on, every `script-form` reference
  reached (10 sessions read by log, 31 jobs succeeded); the one failed row of the scripted test, job 128398808159
  on the iPhone simulator, is `scripted cards whose script did not run in the scripted view` for `script-open`
  (`PlantedCardTests.swift:1185`), not a blind reference.
- **Other one-session blinds of the same test, not this card.** `script-nav` in job 126495221484
  (`loaded=true ... allowed=0 ... marker=true`: its script ran and no navigation was asked) and `script-websocket`
  in job 126751045905 (`loaded=false ... marker=false`: its page never loaded). Neither job holds an
  NSURLErrorDomain provisional failure; neither is `script-form`'s cause, and this SPEC cures neither (section 5).

The method: every `apple / harness` job of `apple-on-change` from run 38355861156 to run 42416540106 was read by
its conclusion, and every one that failed or was cancelled was read by its log's `scripted reference script-form`
and `blind scripted references` lines. A green rerun is a measurement, never an admitted pass: every count is of
one session's own reading.

The offline verdict came from the simulator's network, which the tree does not control, so the red can recur, and
no record can say it cannot. The cure is the card's own next try.

## 2. Requirements

R1. **The card's tries.** `script-form` submits its form up to five times, on the loop `script-nav` uses
    (`let tries = 0; const go = () => { tries += 1; ...; if (tries < 5) { setTimeout(go, 200); } }; setTimeout(go, 0);`),
    every try the same POST to `/script-form/1`. A try that fails before it is sent, an offline verdict or a
    refused policy decision, leaves the page where it is, so the next try comes while the reference's poll
    (`deadline: 5`) still reads; a try that reaches ends the poll at its first arrival. The card's id, held set
    (`L3`, `L5` and `L12`), `alone`, observable (`.path`), method and action stay as they are.

R2. **The plant.** `Recorder` in `ios/CardProbeTests/PlantedCardTests.swift` gains `static var refusesFirstTry:
    String?` and `static var refusedTries = 0`. In `webView(_:decidePolicyFor:decisionHandler:)`, before it counts
    an allowed navigation, a recorder refuses (`.cancel`) the first navigation whose request URL's `absoluteString`
    holds `refusesFirstTry`, counts it in `refusedTries` and clears `refusesFirstTry`. With the default, `nil`,
    nothing changes for any test. Only A1 sets it, and its `defer` clears it.

R3. **Nothing is weakened.** Every planted card but `script-form`'s script, every view, every wait
    (`Probe.loadSeconds`, `Probe.warmUpSeconds`, `Probe.askSeconds`, every `deadline` and `window`), every reading,
    every pinned set (`BLIND_SCRIPTED` stays `capture` and `script-open`, and `ABSENT_ALLOWED` is unchanged), every
    assertion and every test stays as it is; no reading or test is retried, skipped or marked as an expected
    failure. The scripted view, every control on, and each view with one control off still reach nothing from any
    of the five tries: SPEC-355's scripted-view test `test_a_scripted_card_reaches_nothing_from_the_scripted_view` and
    `test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels` stand unchanged, and five
    tries are a stronger attack than one, never a weaker one. The tries fall inside the scripted view's window
    (`window(_:)`, at least `1.5`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with `Recorder.refusesFirstTry` set to `/script-form/`, `script-form` shown in its reference view (`.scriptedReference(scripted.held)`, `window: nil`) has exactly one try refused (`refusedTries` reads 1, so the plant fired) and still reaches its listener (R1, R2) | `xcodebuild test` `CardProbeTests/PlantedCardTests/test_a_scripted_form_reaches_its_reference_when_its_first_try_fails` |

```acceptance
A1: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_form_reaches_its_reference_when_its_first_try_fails
```

A1's test is committed with the scripted form card unchanged, one try and no second, so the plant refuses its only
try and the reference reaches nothing: it reads red by its `XCTAssertTrue` line
`A1: the scripted form reached nothing from its reference after its first try was refused`, by name, on both
simulators, in the pull request's own run, before the five tries come in; `docs/red-first/SPEC-644.md` quotes the
red and names the green.

## 4. File manifest

| path | part | change |
|---|---|---|
| `ios/CardProbeTests/Planted.swift` | planted cards | changed only in the green commit: `script-form`'s comment and script, base lines 360-364, replaced by a comment and a script that submit its form up to five times (R1); its id, held set, `alone`, observable, method and action kept as at the base, and no other line of the file |
| `ios/CardProbeTests/PlantedCardTests.swift` | probe and planted suite | changed only in the red-first commit (and its one compile fix when made): two static properties after base line 29 and a one-shot refusal before base line 35 in `Recorder` (R2), and the criterion test after base line 1191, before the `@MainActor` of `test_no_peer_connection_leaves_the_scripted_view_from_any_frame` (A1); no other line |
| `docs/specs/SPEC-644-a-scripted-form-card-tries-its-form-five-times-so-one-failed-try-does-not-blind-its-reference.md` | docs | added in the first commit: this delivery's SPEC |
| `docs/decisions/ADR-667-the-scripted-form-card-tries-its-form-five-times-as-its-navigation-sibling-does-and-no-reading-is-retried.md` | docs | added in the first commit: the decision |
| `docs/schematics/scripted-form-tries.md` | docs | added in the first commit: the sequence of the reference's tries, the first refused or failed, a later one reaching |
| `docs/red-first/SPEC-644.md` | docs | added in the red-first commit: the red-first record; its red and green lines appended in the last commit |
| `changelog.d/scripted-form-tries-644.md` | docs | added in the first commit: the changelog fragment |

## 5. What this does NOT do

- It changes no reading, wait or deadline, and it retries no reading and no test (#843).
- It does not change the simulator's network, so an offline verdict that outlasts all five tries still reads the
  form's reference blind, a red by name (#843).
- It does not cure the two other one-session blinds of section 1, `script-nav` with no navigation asked and
  `script-websocket` with its page not loaded, each a different signature from this one (#843).
- It changes no other card, the web card suite or `docs/schematics/card-frame-channels.md` (#843).

## 6. Risks

- **An offline verdict longer than the five tries.** A simulator network that fails every try of the loop, five
  tries `setTimeout(go, 200)` apart, reads the reference blind as before: the scripted-view test fails by name against
  `BLIND_SCRIPTED`, and no reading is admitted as a pass (#843).
- **The plant matches text.** It refuses only a navigation whose request URL holds `/script-form/`. The form's
  reference view loads its page from a string with no base URL, so the plant never meets the page's own load, and
  A1's `refusedTries` reading of 1 shows it fired on a try of the form.

## 7. Mutation rows

None. The change is test code in `ios/CardProbeTests/`, which no mutant sweep reads; A1, red with one try and
green with five, is its proof.

## 8. What this keeps, supersedes and amends

| clause | reading |
|---|---|
| SPEC-355 R9 and its scripted-view criterion | kept: no reference is blind but the pinned residual, and that criterion's test and its pinned `BLIND_SCRIPTED` stand unchanged |
| SPEC-349 R11 | kept: the load wait and its bound stand as written; the form's reference read `loaded=true` |
| SPEC-609 | kept: every page ask races `Probe.askSeconds`, and A1's asks go through the same helpers |

It keeps SPEC-355 R9 and its scripted-view criterion, SPEC-349 R11 and SPEC-609 and every bound they name, supersedes nothing, and amends no
file of any of them.
