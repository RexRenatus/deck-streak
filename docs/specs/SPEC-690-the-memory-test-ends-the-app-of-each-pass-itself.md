# SPEC-690: the memory test ends the app of each pass itself

- **Issue:** none filed for this record. **Context(s):** `ios` (`ios/HarnessUITests/`).
- **Decided by:** SPEC-339's measurement M2, in the form its measurement M1 already takes in the
  same file; no new ADR.
- **Status:** built when its pull request lands. The change is not proven to cure the failure it
  targets until A1's three later runs read green.

## 1. The problem, measured

`test_m2_memory` (SPEC-339's M2) measures the app's peak physical memory over one pass of open,
list, render and answer. The `apple / release-and-app` job runs it in Release on the iPhone 17 and
then the iPad (A16) simulator, in the step "the measurements, Release, on the iPhone and then the
iPad" (`.github/workflows/xcframework.yml` lines 758 to 771).

- At this SPEC's base, `test_m2_memory` (`ios/HarnessUITests/HarnessMeasureTests.swift` line 49)
  sets `[.manuallyStart]` alone (line 52). Its measure block launches the app at line 54, starts
  measuring at line 55 and never ends the app, so passes 2 to 6 each launch over the earlier
  pass's running instance, which `launch()` must end first. `test_m1_cold_start` in the same file
  sets `[.manuallyStart, .manuallyStop]` (line 32) and, after `stopMeasuring()` (line 40), ends
  the app itself (line 41), so its next pass finds no running instance to end. Measured by
  `grep -n -E 'invocationOptions|stopMeasuring|terminate' ios/HarnessUITests/HarnessMeasureTests.swift`.
- On the iPad the step has failed in one class: an XCTest request to the simulator (a launch, the
  end of an earlier instance, a background assertion or an accessibility snapshot) goes unanswered
  within XCTest's own fixed wait, while the app itself waits on nothing. What triggers it is not
  proven. The best-supported candidate is a launch at line 54 that must first end the earlier
  pass's instance, which is the form this SPEC removes.
- The UI-query form, job 128632751015 (run 42443317851): the iPad's `test_m2_memory` failed at
  line 65 with "Failed to get matching snapshots: Timed out while evaluating UI query.", and the
  job's report row reads iPad (A16) Failed, iPhone 17 Passed. Its result bundle places the failure
  in the second pass, the first whose launch ended a running instance. That end completed, the
  deck and study taps were answered, and then the card's snapshot request went unanswered through
  XCTest's retries. The failure carries one spindump text attachment (`public.plain-text`, 1836418
  bytes as stored) that XCTest's diagnostics requested, in which the app's main thread was idle.
  In the same test the first pass, a launch with no instance to end, also waited on an app-idle
  notification that did not arrive (a second spindump, 1825338 bytes as stored) and recovered.
- The background-assertion form, job 131737192964 (run 43379037977): `test_m1_cold_start` timed
  out acquiring a background assertion at its launch on line 38, on the iPhone, at a fresh launch
  with no instance to end. The same job read `test_m2_memory` Passed on both simulators.
- After that job, seven release-and-app jobs finished, four as `apple / release-and-app` and three as `release-and-app` in run 43384953899: jobs 131779618608, 131752580592,
  131864992201, 131842555491 and 131880144608 read `test_m2_memory` Passed on both simulators,
  and jobs 131791440929 and 131872983969 never finished the step and left no log to read.

## 2. Requirements

- **R1.** In `test_m2_memory`, the measure options take `[.manuallyStart, .manuallyStop]`, and the
  measure block ends with `stopMeasuring()` and then `app.terminate()` after the `new-count` wait,
  the form `test_m1_cold_start` takes.
- **R2.** The measured window keeps its bounds: it opens at `startMeasuring()` after the launch
  and closes right after the `new-count` wait, where the block's automatic stop fell; the end of
  the app falls outside it.
- **R3.** Nothing else changes: `iterationCount` stays 5 (XCTest runs one more pass and discards
  it), every wait keeps `engineWait`, `test_m1_cold_start` and `HarnessFlowTests.swift` keep every
  line, and the job, its bound, its step, its destinations and both Apple workflows keep every
  line. No test is removed, skipped or retried, and no wait or timeout is raised.
- **R4.** This SPEC states the change as not proven to cure the class, and names the two forms it
  leaves open: the background-assertion form at `test_m1_cold_start`'s line 38, and the UI-query
  form.

## 3. Acceptance criteria of SPEC-690

| id | criterion | decided by |
|---|---|---|
| A1 | `test_m2_memory` reads Passed on the iPad (A16) and the iPhone 17 in three later hosted `apple / release-and-app` runs, each cited by job id | `test_m2_memory`, by name in the step's report row on both destinations |

```acceptance
A1: xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -configuration Release -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessUITests/HarnessMeasureTests/test_m2_memory
```

A1 cannot be red first: the test asserts nothing before or after the change, as SPEC-339's record
discloses for M2 (not red: a measurement). One run at this pull request's head measures the change
on both destinations; A1's proof is the three later runs after it lands.

## 4. File manifest

| path | context | change |
|---|---|---|
| `ios/HarnessUITests/HarnessMeasureTests.swift` | `test_m2_memory`'s measure options and the end of its block | changed: line 52 takes `.manuallyStop`, and the block gains `stopMeasuring()` and `app.terminate()` |
| `docs/specs/SPEC-690-the-memory-test-ends-the-app-of-each-pass-itself.md` | this SPEC | added |
| `changelog.d/m2-memory-ends-each-pass-690.md` | the changelog fragment | added |
| `docs/red-first/SPEC-690.md` | A1's record | added in the record commit after the change |

## 5. What this does NOT cover

- The background-assertion form at `test_m1_cold_start`'s line 38: a fresh launch with no
  instance to end, which this change does not reach (SPEC-339).
- The UI-query form: the result bundle places it in the first pass whose launch ended a running
  instance, but does not show that the end caused it, so it stays open (SPEC-339).
- Where the measurements run: no runner, destination, step or job bound changes (SPEC-339).

## 6. Risks

- **An end that never finishes moves to the new line.** `app.terminate()` (line 72 after the
  change) can meet the same unanswered request a launch's end met. The report row and the job log
  name the line, so a red there reads as this class, not a new one.
- **The class has another trigger.** The first pass in job 128632751015 and the line-38 form show
  requests unanswered with no instance to end; A1's three runs decide whether this change is
  enough.
- **The memory reading shifts.** The metric now stops at `stopMeasuring()` instead of the block's
  end; both points follow the `new-count` wait with nothing between them, so the window holds the
  same work.

## 7. What only CI proves

A1 runs only on the Apple runner, in the step named above, and is read by test name and
destination in the job's report table (`| test | result |`).

## 8. Formal model

None: `config/formal.json` names no path under `ios/`.

## 9. References

- SPEC-339 (M1 and M2) and its red-first record, which discloses M2 as not red: a measurement.
- XCTest: `XCTMeasureOptions.invocationOptions` with `.manuallyStart` and `.manuallyStop`,
  `stopMeasuring()` and `XCUIApplication.terminate()`.
