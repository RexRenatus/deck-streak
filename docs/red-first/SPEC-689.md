# Red-first record: SPEC-689

The SPEC and the fragment were committed first. A1's test was then committed alone at
26194aa, with the record of SPEC-689 R1 and no page held, before the hold of SPEC-689 R2 that
makes it pass. A1 runs only in CI's `apple / harness` job, in the planted suite step, on the
iPhone and then the iPad simulator. Its red is quoted from job 131842555527 at the first
push's head 26194aa, where it failed by name on both simulators at the same first assertion,
and its green is read by name from the same step at the completing push's head.

```red-first
A1: red at 26194aa: ios/CardProbeTests/PlantedCardTests.swift:969: error: -[CardProbeTests.PlantedCardTests test_every_planted_view_is_built_while_a_kept_page_holds_the_shared_processes] : XCTAssertEqual failed: ("["img in reference (probe 1)", "img in shipped (probe 2)", "event-handler in reference (probe 3)", "event-handler in shipped (probe 4)"]") is not equal to ("[]") - views built while no loaded page held WebKit's shared processes, so each could wait on a GPU process relaunch
A1: green at db16428
```

The green commit db16428 changes the file that holds A1's test: it adds `Probe.hold()` and
`Probe.release()` after `Probe.keeper` and calls them from `setUp` and `tearDown`. A1's own
method and its assertions are unchanged, and so is every bound.
The green commit `db16428` carries the cure, and `git diff --quiet db16428 HEAD` over ios/CardProbeTests/PlantedCardTests.swift reads rc 0.
