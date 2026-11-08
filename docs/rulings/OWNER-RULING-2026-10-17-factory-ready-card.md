The owner's decision, carried by the owner's signature on the commit that adds this file: one more test-only card view is admitted by name beside the keeper, a ready card that each card factory test loads in its own set-up, before anything it measures, under the existing warm-up bound and with its own load asserted; no wait, timeout or bound moves, and no row's or test's wait, reading or assertion changes.

# OWNER RULING 2026-10-17: a ready card before each card factory test

It amends `docs/rulings/OWNER-RULING-2026-10-07-factory-row-load-wait.md` in one place: its "What replaces it" bullet that begins "Admitted by name: one keeper card view, test-only." now has a second admitted view, the ready card below. Everything else that ruling says stands, its list of what is unchanged included, and its file keeps the text it was signed with. That ruling says "A cure that changes anything in the list above needs a new signed ruling"; this file is that ruling for the ready card alone. Cited by SPEC-444 and ADR-458. The build that carries the ready card is SPEC-444's (Closes #796, Refs #781); it lands on `dev` only after the signed commit of this file is there.

## What was held

| place | what it holds |
|---|---|
| the factory-row ruling, "What replaces it" | "Admitted by name: one keeper card view, test-only." and no other view |
| the same bullet list | "Unchanged: `loadSeconds` and every other wait, timeout and bound; no skip, retry or expected-failure mark; every row's assertions (ran, verdict, loaded); the warm-up; the planted rows, the layer map and the blind set." |
| `ios/CardProbeTests/FactoryTests.swift` | each test's set-up runs the once-per-process warm-up, then the test builds its own card view and waits for it under `loadSeconds` |
| #796 | a card factory test's view did not finish loading inside its own `loadSeconds` wait, and a rerun cleared it |

## What was measured

The last 100 completed runs of the `apple-on-change` workflow held 112 `apple / harness` jobs, 19 of them failed; the logs of all 19 failed jobs and of 21 passed jobs were read. Three failures were card factory tests whose view never loaded inside its wait:

| job | destination (xcodebuild order) | test | test duration | process-termination lines in the test's window | cleared by |
|---|---|---|---|---|---|
| 113616302490 | first | the observable-layer test | 27.765 s (job 113616302490) | 19 | job 113632280282 |
| 113804867998 | first | the default-build test | 30.723 s (job 113804867998) | 17 | no rerun |
| 114227330129 | second | the observable-layer test | 20.923 s (job 114227330129) | 14 | job 114242838275 |

Across the 138 windows of those two tests, counted by the `Failed to terminate process` lines that the previous test's view teardown leaves in the job log:

| termination lines in the window | windows | failed | slowest window |
|---|---|---|---|
| 0 | 124 | 0 | 5.037 s (job 114097915719) |
| 1 to 5 | 8 | 0 | 6.909 s (job 113616302490) |
| 6 or more | 6 | 3 | 30.723 s (job 113804867998); the fastest of the six 11.625 s (job 115279304148) |

The keeper's 13 rows read no load at or above their `loadSeconds` wait in 70 destination-runs; the slowest was 7.70 s (job 113804867998). The warm-up's 140 loads all loaded under the warm-up bound; the slowest was 17.83 s (job 116510533486).

## What it closes

Whether a card factory test's measured wait may begin while the simulator is still relaunching what the previous test's teardown ended. Every failure sat in a window with 6 or more termination lines, and none of the 132 windows with fewer failed. The warm-up covers the first test of each process, and the keeper covers the 13 rows of the one test that holds it; neither covers a test that starts after another test's teardown.

## What replaces it

- Admitted by name: one ready card view per card factory test, test-only. The factory builds it with scripts off. Each test of `FactoryTests` loads it in its set-up, after the warm-up and before anything the test measures, and removes it in its tear-down.
- Its wait is the existing warm-up bound, `warmUpSeconds`, unchanged; no new bound is set. That bound is at least 1.95 times the slowest failing window, 30.723 s (job 113804867998).
- Its own load is asserted in the set-up, so it only adds a way to fail. It prints its own reading. It is not a row and is not timed as one, and it changes no test's wait, reading or assertion.
- The keeper stays as the factory-row ruling admitted it. The planted suite (`PlantedCardTests`) and every UI test gain no ready card.
- Unchanged, as the factory-row ruling lists it: `loadSeconds` and every other wait, timeout and bound; no skip, retry or expected-failure mark; every row's assertions (ran, verdict, loaded); the warm-up; the planted rows, the layer map and the blind set.

## Why it is admitted

It adds a test-side wait under an existing bound and a second test-only view beside the one the factory-row ruling admitted by name, so it changes that ruling's admitted list. In effect it moves the relaunch that follows a teardown out of the measured wait and into a wait whose own load is asserted; every measured wait keeps its bound, its reading and its assertion, and a card that loads only after its wait still fails. The rejected alternatives:

- **Raising `loadSeconds`.** It moves the wait itself, so a slower card would pass; the factory-row ruling rejected it.
- **One keeper for the whole process.** Job 113804867998 failed with no idle exit of the helper processes before it, so holding a view open alone does not cure it.
- **A pause before each test.** It buys nothing certain, as ADR-372 D11 found.
- **Reordering the tests.** Test order is no contract.
- **A retry or an expected-failure mark.** It lets a red pass.
- **A ready card in the planted suite too.** Its load-wait class read 0 in the 112 harness jobs; its other readings are another class.
- **Editing the factory-row ruling in place.** A signed record keeps its signed text. This file adds one admission and leaves the signed file as it was signed.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
