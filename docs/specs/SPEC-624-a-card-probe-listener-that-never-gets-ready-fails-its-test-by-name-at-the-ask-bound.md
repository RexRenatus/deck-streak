# SPEC-624: a card probe listener that never gets ready fails its test by name at the ask bound

- **Issue:** #839. **Context(s):** the card view's probe suite, `ios/CardProbeTests/`.
- **Decided by:** ADR-646.
- **Status:** delivered by the pull request that adds this file, with its test and `docs/red-first/SPEC-624.md`.
- **Mutation rows:** none, test code that no mutant sweep reads.

## 1. The problem, measured

Read at the dev commit `7bd6923`, SPEC-609's ask bound reaches page asks only. Five waits the suite starts on its
own still wait with no bound of their own, each in `ios/CardProbeTests/PlantedCardTests.swift`:

- `show` awaits `try await listeners.start()` (line 664) before its first ask, and registers the listeners'
  cancel only after it (line 665).
- `test_a_lookup_a_card_asks_for_reaches_the_witness_only_from_the_reference` awaits `witness.start()` (line 1336)
  and registers the witness's cancel only after it (line 1337).
- `Probe.make` awaits its own rule list compile, `compileContentRuleList` (lines 300-301).
- The warm-up, `Probe.warmUp`, awaits `RuleList.compile()` (line 563).
- `test_a_scripted_card_reads_no_state_the_app_holds` awaits the cookie store's `setCookie` (line 1239).

The listeners' ready wait (`ios/CardProbeTests/Listeners.swift` lines 132-149) resumes only on a `.ready`,
`.failed` or `.cancelled` state; every other state falls to `default: break`. SPEC-609 section 5 names these waits,
with the compile inside `CardWebViewFactory.makeCardWebView`, as kept. So a listener pair that never gets ready
holds the test, the job and its runner to the job's own bound, and names no test.

## 2. Requirements

R1. **One start helper.** `Probe` gains `Probe.started(label:seconds:_:)`, generic over a `Sendable` value,
    documented as the one bound on a wait the suite starts on its own (a listener pair, the witness, a rule list
    compile, the planted cookie), which is not a page ask. It runs its operation in an unstructured main-actor task
    and returns the operation's value or rethrows its error, or throws `ProbeStall` (SPEC-609 R2's type and
    description, unchanged), labelled with its label and naming `Probe.probing`, when the operation has not
    finished within `seconds`, by default `Probe.askSeconds`, a value of 120. It resumes its continuation exactly
    once through SPEC-609's one-shot guard, `AskGuard`: whichever of the finish and the bound comes first is taken,
    a late finish after a stall is dropped and never resumed a second time, and on a finish first the bound's timer
    is cancelled. A start wait is no page ask: it never touches `Probe.answeredAsks` or `Probe.slowestAsk`, so
    SPEC-609 R5's `card probe asks:` lines keep their meaning. `Probe.bounded`, `Probe.ask`, `ProbeStall` and
    `AskGuard` are unchanged.

R2. **The five waits it bounds.** `show` starts its listeners through the start helper under the label
    `listeners`, and registers `listeners.cancel()` as its `defer` before the start, so a stalled start cancels
    both listeners, whose `.cancelled` state then ends the inner wait, its finish dropped by the guard. The witness
    test starts its witness the same way, under the label `witness`, with its `defer { witness.cancel() }` moved
    above the start. `Probe.make` compiles its rule list under the label `rule list`, with its `try?` kept inside
    the operation, so a compile error still reads as no list and only a stall throws. The warm-up compiles its rule
    list under the label `warm-up rule list`, and the cookie test sets its planted cookie under the label `cookie`.
    A stall in any of them ends the running test, and XCTest names the test with the stall's description, which
    reads `card probe stall: the <label> ask in <probe> had no answer within <seconds> s`.

R3. **The one wait it keeps.** The rule list compile inside `CardWebViewFactory.makeCardWebView`
    (`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` line 26), which the factory suite awaits
    at `ios/CardProbeTests/FactoryTests.swift` lines 127, 267, 300 and 318 and the planted suite at
    `ios/CardProbeTests/PlantedCardTests.swift` line 1040, keeps its own wait. It is the product's own build, the
    API those tests judge, and this test-only delivery does not wrap it (section 5).

R4. **Nothing is weakened.** Every planted card, view, wait, reading, bound and assertion of the suite stays as it
    is (`Probe.askSeconds` 120, `Probe.loadSeconds`, `Probe.warmUpSeconds`, every `deadline`, every `window`), with
    no retry, skip, expected-failure mark or test-level time allowance. `ios/CardProbeTests/Listeners.swift` and
    `ios/CardProbeTests/FactoryTests.swift` are unchanged: the listeners' and the witness's own state handling, a
    failed or cancelled listener included, still ends their waits with their own errors, which the start helper
    rethrows. Outside the start helper, the only body changes are R2's five waits and R5's seam and test. SPEC-609's
    A1 and A2 tests and SPEC-349's A10 to A13 tests are unchanged. The bound is `Probe.askSeconds` because a
    listener pair, a witness, a compile and a cookie each finish far inside one page ask's allowance on a running
    simulator, and one bound keeps one stall reading.

R5. **The criterion and its seam.** `StalledListeners`, a main-actor class, stands for a listener pair whose state
    never reaches ready: its `start()` awaits one checked continuation that it keeps and never resumes, and its
    `cancel()` resumes every kept continuation once by throwing `CancellationError()`, as a cancelled listener's
    `.cancelled` state does. The criterion's test registers the seam's cancel at once, sets the probe record to
    `a planted card in a stalled view (probe 1)`, starts the call under test, the start helper with the label
    `listeners` and `seconds: 2`, in an unstructured main-actor task that stores its outcome, waits for that
    outcome by `Probe.poll` under its own watchdog, `seconds: 10`, and fails by `XCTFail` naming the watchdog when
    the call has not returned, so a start wait that never returns reads red by assertion, never as a hang.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a start wait over a listener pair that never gets ready, with the probe record `a planted card in a stalled view (probe 1)`, the label `listeners` and `seconds: 2`, ends inside its watchdog, `seconds: 10`, with a `ProbeStall` whose description holds the start label, the record and the bound in whole seconds, and not before its own bound (R1, R5) | `xcodebuild test` `CardProbeTests/PlantedCardTests/test_a_listener_that_never_gets_ready_fails_at_the_ask_bound_and_names_its_probe` |

```acceptance
A1: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_listener_that_never_gets_ready_fails_at_the_ask_bound_and_names_its_probe
```

A1's test is committed with the start helper as an inert seam that awaits the operation with no bound, so the call
under test never returns, and it reads red by its own watchdog's `XCTFail` in the pull request's own run, by name,
on both simulators, before the bound comes in; `docs/red-first/SPEC-624.md` quotes the red and names the green.

## 4. File manifest

| path | part | change |
|---|---|---|
| `ios/CardProbeTests/PlantedCardTests.swift` | probe and planted suite | changed: the start helper `Probe.started` (R1), the five start waits through it and the two moved `defer` lines (R2), and the criterion test A1 with its seam `StalledListeners` (R5) (the test goes after the last test method, `test_a_page_that_answers_then_falls_silent_fails_the_load_wait_at_the_ask_bound`, lines 1539-1573 at `7bd6923`, before the class's closing brace, line 1574; the seam goes after `FallsSilentPage`, lines 818-833) |
| `docs/specs/SPEC-624-a-card-probe-listener-that-never-gets-ready-fails-its-test-by-name-at-the-ask-bound.md` | docs | added: this delivery's SPEC |
| `docs/decisions/ADR-646-every-card-probe-start-wait-races-the-ask-bound-and-the-factory-build-keeps-its-own-wait.md` | docs | added: the start helper, its bound, the cancel on a stall and the kept factory compile |
| `docs/schematics/card-probe-start-bound.md` | docs | added: the sequence of one start wait, finished and stalled |
| `docs/red-first/SPEC-624.md` | docs | added: the red-first record (RECORD at C2, its red and green lines at C4) |
| `changelog.d/card-probe-start-bound-624.md` | docs | added: FRAGMENT, byte for byte |

## 5. What this does NOT do

- It adds no test-level time allowance and changes no workflow, so a main thread that WebKit itself blocks, where
  no timer can fire, still ends only at the job's own bound (#839).
- The rule list compile inside the card view factory's own build, `CardWebViewFactory.makeCardWebView`, keeps its
  own wait at the five call sites R3 names, so a stall inside the factory's own build still holds the job to its
  bound (#839).
- `ios/CardProbeTests/Listeners.swift` and `ios/CardProbeTests/FactoryTests.swift` are unchanged (#839).
- Beyond R2's five start waits, it changes no bound, wait, card or assertion, and it retries nothing (#839).

## 6. Risks

- **A stalled start leaves its operation going.** A stalled start's operation goes on until its owner's `defer`
  cancels the listeners or the witness, and a compile or cookie operation may finish after its test has failed;
  that late finish is dropped by the guard. The test has already failed by name with the stall's description.
- **A cold start past the bound.** A cold simulator whose first start wait needs more than `Probe.askSeconds` reads
  as a stall. A later change of the value is its own delivery, from measured lines.

## 7. Mutation rows

None. The change is test code in `ios/CardProbeTests/`, which no mutant sweep reads; A1, red at the inert seam and
green at the bound, is its proof.

## 8. What this keeps, supersedes and amends

| clause | reading |
|---|---|
| SPEC-609 R1 to R6 | kept: the bound, `Probe.askSeconds` 120, the stall, `ProbeStall`, and the one-shot guard, `AskGuard`, are reused unchanged |
| SPEC-609 section 5, its second bullet | superseded for the five waits R2 names; kept for the rule list compile inside the factory's own build (R3) |
| SPEC-355 R10 and SPEC-349 R5 | kept: the witness and the listeners are unchanged |
| SPEC-361 R16 | kept: `Probe.loadSeconds`, `Probe.warmUpSeconds` and the once-per-process warm-up stand as written |

It amends no file of any of them.
