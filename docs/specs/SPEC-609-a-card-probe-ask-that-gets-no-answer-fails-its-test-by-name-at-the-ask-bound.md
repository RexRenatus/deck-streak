# SPEC-609: a card probe ask that gets no answer fails its test by name at the ask bound

- **Issue:** #839. **Context(s):** the card view's probe suite, `ios/CardProbeTests/`.
- **Decided by:** ADR-631.
- **Status:** delivered by the pull request that adds this file, with its tests and `docs/red-first/SPEC-609.md`.
- **Mutation rows:** none, test code that no mutant sweep reads.

## 1. The problem, measured

On a pull request that changed no iOS path, the `apple / harness` job hung in the card view's planted suite and
was cancelled at its own bound (job 115216068432).

- `test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels` passed on the iPhone simulator
  (job 115216068432).
- On the iPad simulator the same test stopped at probe 152 of 286, and the log stayed silent from then until the
  cancel (job 115216068432).
- A one-job rerun passed it on both simulators (job 115279304148). A green rerun is a measurement, never an
  admitted pass.

Read at the dev commit `b065fce`, every page ask of the suite waits with no bound of its own:

- `Probe.evaluate` (`ios/CardProbeTests/PlantedCardTests.swift` lines 280-286) awaits one
  `withCheckedContinuation` that only WebKit's `evaluateJavaScript` completion resumes.
- The load wait's ready ask, `Probe.reply` (lines 401-405), awaits one that only the page's answer resumes.
- `Probe.snapshot` (lines 471-479) awaits one that only the view's `takeSnapshot` completion resumes.
- `Probe.poll` (lines 315-322) bounds only the time between its checks, never one check.

So a page that stops answering holds the test, the job and its runner until the job's own bound, and names no
test.

## 2. Requirements

R1. **One bound.** `Probe` gains `Probe.askSeconds`, a value of 120, documented as the most one page ask may wait
    for its answer. Every page ask of the suite, each wait on a page's `evaluateJavaScript` answer or a view's
    `takeSnapshot` completion, goes through one bounded helper, `Probe.bounded(label:seconds:_:)`, whose string
    form is `Probe.ask(asking:_:label:seconds:)` over the suite's `Asking` type: `Probe.evaluate`, the load wait's
    ready ask (`Probe.reply`) and `Probe.snapshot`. The answer phase's `askOnce` already waits under
    `answerSeconds` without awaiting an ask, and is unchanged. `Listeners.start`, `Witness.start`, the three content
    rule list compiles and the cookie store's `setCookie` are no page asks and are unchanged (section 5).

R2. **A stall is a named failure.** An ask whose answer has not come within its bound throws `ProbeStall`, an
    `Error` whose description reads `card probe stall: the <label> ask in <probe> had no answer within <seconds> s`:
    the label of the ask, R3's probe record and the bound in whole seconds. The helper resumes its continuation
    exactly once: a one-shot guard on the main actor takes whichever of the answer and the bound comes first, a
    late answer after a stall is dropped, never resumed a second time, and on an answer first the bound's timer is
    cancelled. Each helper that calls a bounded ask throws (`evaluate`, `links`, `reportsLoaded`, both `loadTime`,
    `reply`, `loaded`, `warmUp`, `warmUpLoad`, `keeper`, `snapshot`, `settle` and `show`'s reading, `read`),
    `poll` takes a throwing check and rethrows, and every caller adds `try`. A stall thus ends the running test,
    and XCTest names the test with the stall's description. Outside the two criteria's own tests, which hold the
    outcome to assert it, no `try?` and no `catch` swallows a stall.

R3. **The probe it stopped on.** `Probe` keeps a probe record on the main actor: `Probe.probing`, a string, and a
    per-test probe count. `show` counts one more probe and sets `Probe.probing` to
    `<card id> in <variant> (probe <n>)` before its first ask, the variant written by `String(describing:)`; each
    test's `setUp` sets `Probe.probing` to `no probe` and the count to zero. A stall's description carries the
    record, so a stall in the probe that hung names its ask, its card, its variant and its probe number.

R4. **Nothing is weakened.** Every planted card, view, wait, reading, bound and assertion of the suite stays as it
    is (`Probe.loadSeconds`, `Probe.warmUpSeconds`, every `deadline`, every `window`), with no retry, skip,
    expected-failure mark or test-level time allowance. Outside the bounded helper, the only changes to a body are
    `try`, `throws`, R3's record in `show` and R5's lines in `setUp` and `tearDown`. The two load-wait tests A10 and
    A11 of SPEC-349 keep every line but the added `try`. An existing caller that passes no ask bound gets
    `Probe.askSeconds`. The value is 120 because one ask may legitimately wait for WebKit's first process start,
    which the warm-up allows `Probe.warmUpSeconds`, a value of 60 (SPEC-361 R16, ADR-372 D11): twice that keeps a
    cold start from reading as a stall, and a stall now ends its test at the ask bound instead of holding the job
    to the job's own bound.

R5. **The measurement.** The bounded helper records, per test, the asks it answered and the slowest answered ask in
    seconds. Each of the two test classes' `setUp` resets them with R3's record, and its `tearDown` prints exactly
    one line, `card probe asks: <test name> asks=<n> slowest=<s.ss> bound=120`, the test name by `self.name`.
    Each push of this delivery gives one pass of every test on both simulators, so the lines read per simulator
    and across two pushes.

R6. **The criteria and their seam pages.** `SilentPage` keeps every answer it is handed and never calls one; it
    stands for a page whose process stopped answering. `FallsSilentPage` answers `Probe.answerScript` with
    `answered` at once and keeps, never calls, the answer to any other script. Each criterion's test starts the
    call under test in an unstructured main-actor task that stores its outcome (the value, or the thrown error),
    waits for that outcome by `Probe.poll` under its own watchdog, and fails by `XCTFail` naming the watchdog when
    the call has not returned, so a call that never returns reads red by assertion, never as a hang.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | an ask of a page that never answers, with the probe record `a silent card in a silent view (probe 1)`, the label `silent-state` and `seconds: 2`, ends inside its watchdog, `seconds: 10`, with a `ProbeStall` whose description holds the label, the record and the bound in whole seconds, and not before its own bound (R1, R2, R3, R6) | `xcodebuild test` `CardProbeTests/PlantedCardTests/test_an_ask_a_page_never_answers_fails_at_the_ask_bound_and_names_its_probe` |
| A2 | the load wait over a page that answers and then falls silent, with `answerSeconds: 3`, `loadSeconds: 1` and `askSeconds: 2`, ends inside its watchdog, `seconds: 12`, with a `ProbeStall` whose description holds the ready ask's label `ready` and the bound in whole seconds (R1, R2, R6) | `.../test_a_page_that_answers_then_falls_silent_fails_the_load_wait_at_the_ask_bound` |

```acceptance
A1: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_an_ask_a_page_never_answers_fails_at_the_ask_bound_and_names_its_probe
A2: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_page_that_answers_then_falls_silent_fails_the_load_wait_at_the_ask_bound
```

A1's and A2's tests are committed with the bounded helper as an inert seam that awaits the answer with no bound,
so the call under test never returns, and each reads red by its own watchdog's `XCTFail` in the pull request's
own run, by name, on both simulators, before the bound comes in; `docs/red-first/SPEC-609.md` quotes each red and
names the green.

## 4. File manifest

| path | part | change |
|---|---|---|
| `ios/CardProbeTests/PlantedCardTests.swift` | probe and planted suite | changed: the bounded ask and its error (R1 to R4), the probe record (R3), the measurement (R5), the `try` each new throwing call needs, and the two criteria tests A1 and A2 with their two seam pages (the two tests go after the last existing test method, `test_a_page_that_never_answers_reads_not_loaded_after_the_answer_bound`, base lines 1290-1304, before the class's closing brace, base line 1305; the seam pages go after `AnswerState`, base lines 652-658) |
| `ios/CardProbeTests/FactoryTests.swift` | factory suite | changed: `try` at each call of a Probe helper that now throws, `Probe.loadTime(_:handed:)` (base line 56) made `async throws` and its asking wrapper passed through, and the same per-test reset and measurement line as the planted suite (R5); no assertion, wait or reading changes |
| `docs/specs/SPEC-609-a-card-probe-ask-that-gets-no-answer-fails-its-test-by-name-at-the-ask-bound.md` | docs | added: this delivery's SPEC |
| `docs/decisions/ADR-631-every-card-probe-ask-races-one-bound-and-a-stall-is-a-named-failure.md` | docs | added: the bound, its value, the one-shot resume and the stall's reading |
| `docs/schematics/card-probe-ask-bound.md` | docs | added: the sequence of one bounded ask, answered and stalled |
| `docs/red-first/SPEC-609.md` | docs | added: the red-first record (RECORD at C2, its red and green lines at C4) |
| `changelog.d/card-probe-ask-bound-609.md` | docs | added: FRAGMENT, byte for byte |

## 5. What this does NOT do

- It adds no test-level time allowance and changes no workflow, so a main thread that WebKit itself blocks, where
  no timer can fire, still ends only at the job's own bound (#839).
- `Listeners.start`, `Witness.start`, the three content rule list compiles (`Probe.make`, the warm-up's
  `RuleList.compile()` and the one inside `CardWebViewFactory.makeCardWebView`) and the cookie store's `setCookie`
  keep their own waits, none of them a page ask, so a probe whose listeners never become ready still holds the job
  to its bound (#839).
- It changes no bound, wait, card or assertion, and it retries nothing (#839).

## 6. Risks

- **A cold start past the bound.** A cold simulator whose first WebKit start needs more than `Probe.askSeconds`
  reads as a stall. R5's lines measure the slowest ask on each simulator against the bound; a later change of the
  value is its own delivery, from measured lines.
- **A stalled test can leave its view mounted.** A test that mounts a view and takes it down after its asks does
  not reach the take-down when an ask throws; the test has already failed by name, and the next test mounts its
  own views beside it.

## 7. Mutation rows

None. The change is test code in `ios/CardProbeTests/`, which no mutant sweep reads; A1 and A2, red at the inert
seam and green at the bound, are its proof.

## 8. What this keeps, supersedes and amends

| clause | reading |
|---|---|
| SPEC-349 R11, A10 and A11 | kept: the load wait starts at the page's first answer; its two tests gain only `try` |
| SPEC-349 R12, A12 and A13 | kept: the handed wait asks through the same load wait; its two tests gain only `try` |
| SPEC-361 R16 | kept: `Probe.loadSeconds`, `Probe.warmUpSeconds` and the once-per-process warm-up stand as written |

It keeps every bound SPEC-349 and SPEC-361 name, and it amends no file of either.
