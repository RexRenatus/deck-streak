# SPEC-689: each planted test holds a loaded page, so no view it builds waits on a GPU process relaunch

- **Issue:** #1474. **Context(s):** the card view's planted suite, `ios/CardProbeTests/`.
- **Decided by:** ADR-372 (D11, the keeper page), which this SPEC applies to every planted test.
- **Status:** delivered by the pull request that adds this file, with its test and `docs/red-first/SPEC-689.md`.
- **Mutation rows:** none, test code that no mutant sweep reads.

Follows SPEC-349, SPEC-361 (R16) and SPEC-609. The flow half of the same class is #1473.

## 1. The problem, measured

On #1473, the `apple / harness` job failed in the planted suite on the iPhone simulator
(job 131617974452): the assertion at `PlantedCardTests.swift:926` read `event-handler` among the
cards whose reference view reached nothing, because that card's reference view never finished
loading (`reached=false took=- loaded=false`, line 5775 of the job's log). A one-job rerun passed
it on both simulators (job 131660059794). The red job's log names the sequence (job 131617974452):

- Line 5718: `GPUProcessProxy::gpuProcessExited: reason=IdleExit`, once the previous card's two
  views were taken down, so no page held the GPU process.
- Lines 5722 to 5732: the event-handler card's reference view starts its own web content process,
  which answers the load wait's first ask.
- Lines 5735, 5746 and 5756: the next three GPU processes are each logged
  `GPUProcessProxy::didBecomeUnresponsive`, `terminating it`.
- Line 5759 (job 131617974452): `GPU Process has crashed more than 2 times in the last 30 seconds, terminating all WebProcesses`.
- Lines 5760 to 5768: `WebProcessPool::terminateAllWebContentProcesses`, then the reference view's
  page logs `processDidTerminate` and `dispatchProcessDidTerminate`,
  `reason=GPUProcessCrashedTooManyTimes`.
- Line 5775: the reading, `loaded=false`; line 6023: the assertion at `PlantedCardTests.swift:926`.

Those lines are the only ones of their kind in either job's log:
`grep -c -E 'GPUProcessProxy::didBecomeUnresponsive|terminateAllWebContentProcesses|WebPageProxy::processDidTerminate'`
reads 5 in job 131617974452, all in lines 5735 to 5767, and 0 in job 131660059794.

WebKit's GPU process exits when no page holds it. The planted test holds no page between its
views; the factory test that holds one keeper page across its 13 rows (ADR-372 D11) does. The
count of `GPUProcessProxy::gpuProcessExited: reason=IdleExit` lines between each test's start and
end lines, per simulator pass:

| pass | the planted test, no page held | the factory keeper test, one page held |
|---|---|---|
| job 131617974452, iPhone | 19 | 0 |
| job 131617974452, iPad | 13 | 0 |
| job 131660059794, iPhone | 20 | 0 |
| job 131660059794, iPad | 10 | 0 |

So each planted test relaunches the GPU process again and again, and the view built after each
teardown waits on the relaunch. In the red pass three relaunches in a row were declared
unresponsive, and WebKit's rule for a GPU process that keeps crashing then ended the reference
view's page before it loaded.

The flow half's cause (#1473), the app's main thread blocked in the device's voice list before any
web content process exists, does not explain this one. No source under `ios/CardProbeTests/`,
`ios/CardProbeHost/` or `ios/CardIsolation/Sources/` names `AVSpeech`, `speechVoices`,
`import AVFoundation`, `InstalledVoices` or `ReviewModel` (0 files). The host's main thread wrote
50 log lines from line 5694 to line 5775 with no gap above 2.67 s (job 131617974452), and the
reference view's own web content process existed and had answered.

## 2. Requirements

R1. Before `Probe.show` builds a view, it records whether a loaded page holds WebKit's shared
processes: it asks the held page, `Probe.held`, under the ask bound, whether it reports loaded. A
view built while it does is counted in `Probe.heldViews`; any other view adds its probe record to
`Probe.unheld`. Each test's `setUp` resets both through `Probe.resetRecord()`.

R2. Each planted test holds one loaded page, `Probe.keeper` (ADR-372 D11), from its `setUp`, after
the warm-up, to its `tearDown`. `Probe.hold()` loads it under the warm-up bound and keeps it as
`Probe.held`, and `setUp` asserts that it loaded; `Probe.release()` takes it down and clears
`Probe.held`. While it lives, no view the test builds is built after a teardown that left no page
to hold the GPU process.

R3. Nothing else changes. No retry, skip or quarantine is added, and no wait, timeout or tolerance
is raised or widened: `Probe.loadSeconds`, `Probe.warmUpSeconds`, `Probe.askSeconds`, the window
helper and `UNOBSERVABLE` keep their values, and no workflow, step or destination changes.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the planted test's own sequence over two planted cards, `img` and `event-handler`, each in the reference view and then the card view (four views, three of them built after another view's teardown), builds every view while a loaded page holds WebKit's shared processes: `Probe.unheld` is empty and `Probe.heldViews` is 4 (R1, R2) | `xcodebuild test` `CardProbeTests/PlantedCardTests/test_every_planted_view_is_built_while_a_kept_page_holds_the_shared_processes` |

```acceptance
A1: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_every_planted_view_is_built_while_a_kept_page_holds_the_shared_processes
```

A1's test is committed with R1's record and no page held, so it fails at its first assertion in
the pull request's own run, by name, on both simulators. R2's hold comes in a later commit with the
test unchanged, and `docs/red-first/SPEC-689.md` quotes the red and names the green.

## 4. File manifest

| path | part | change |
|---|---|---|
| `ios/CardProbeTests/PlantedCardTests.swift` | probe and planted suite | changed: the held page, its record in `Probe.show` and `Probe.resetRecord` (R1), and the A1 test after the planted test's own; then `Probe.hold()` and `Probe.release()` after `Probe.keeper`, called from `setUp` and `tearDown` (R2) |
| `docs/specs/SPEC-689-each-planted-test-holds-a-loaded-page-so-no-view-waits-on-a-gpu-process-relaunch.md` | docs | added: this delivery's SPEC |
| `docs/red-first/SPEC-689.md` | docs | added: the red-first record, A1's red and green lines |
| `changelog.d/planted-reference-load-689.md` | docs | added: the fragment |

## 5. What this does NOT do

- It does not change the factory suite's other tests, whose views are not a planted card's
  reference view (#796).
- It does not show that the reds of #843 and #841 share this cause: neither job's log was kept.
  The pull request's own run and the next iOS pull requests are compared by name on both
  simulators for that (#843, #841).
- No public WebKit interface reports the GPU process's lifetime, so A1 asks the held page at each
  view's build, not the GPU process; the count of
  `GPUProcessProxy::gpuProcessExited: reason=IdleExit` lines in the planted test's part of the
  harness job's log is a reading only (#1474).
- The keeper's own load still starts the GPU process once per test. Three unresponsive GPU
  processes during it fail that test in `setUp` by name, never a reading (#1474).
- It touches none of the flow half's paths (#1473).
- It changes no workflow, destination, bound or the declared `UNOBSERVABLE` set (#1474).

## 6. Risks

- A keeper whose web content process ends during a test leaves the test's later views unheld:
  R1's record lists them, and in A1 they fail it by name.
- Each planted test now loads one more page in `setUp`, under the warm-up bound, so the harness
  job's time is compared by name with its `timeout-minutes` bound on each run.
- The hold check is one more ask per view, counted in each test's asks line and held by the ask
  bound.

## 7. What CI shows, by name

- The planted suite on the iPhone simulator and then the iPad simulator, in step 11 of
  `apple / harness`, at the pull request's head, every planted test by name.
- The `apple / harness` job's time against its `timeout-minutes` bound, a reading only.

## 8. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani: `config/formal.json` names no `ios/` or Swift path, and the
change is Swift test code in the planted suite.

## 9. References

SPEC-349, SPEC-361 (R16), SPEC-609, ADR-372 (D11).
