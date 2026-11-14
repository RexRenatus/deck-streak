# Red-first record: SPEC-690

SPEC-690's one criterion, A1, is `test_m2_memory`, read by name in the
`apple / release-and-app` job's report row on the iPad (A16) and the iPhone 17. It runs only
on the Apple runner, and it asserts nothing before or after the change, as SPEC-339's record
discloses for it (not red, a measurement). So A1 has no red commit: its green line names
the commit that carries the change, and its proof is three later hosted `apple / release-and-app`
runs that read `test_m2_memory` Passed on both simulators, each cited by job id (SPEC-690,
section 3).

```red-first
A1: not red: a measurement (the test asserts nothing before or after the change; it measures peak physical memory)
A1: green at 36218f5
```

The green commit `36218f5` carries the change, and `git diff --quiet 36218f5 HEAD -- ios/HarnessUITests/HarnessMeasureTests.swift` reads rc 0.
