# Red-first record: SPEC-613

The delivery takes two pushes. The first carries SPEC-613, ADR-635, the schematic, the changelog
fragment and the red tests alone, with no wiring: the engine and study configurations, the sync
suite's `account()` and the `web-engine` job still run two engines. The second carries the wiring:
each configuration's `firefox` project, the engine configuration's third sync user, the rule of
`account()`, the job's install line, comment and step names, the header comments of the engine and
study spec files, the mutation band and this record's fence.

Where each criterion's red is read:

- A1 is `test_ci_workflows.TheEngineAndStudySuitesRunInFirefox.test_the_engine_and_study_configurations_declare_three_engines`,
  a unittest case, with the Vitest test `the study suite covers the review loop in both engines` in
  `web/app/src/lib/study/study-coverage.test.ts` for the study half. Both are red at the commit of
  the red tests: the unittest case in CI's `hygiene` job, which runs that module, by its first
  assertion (the engine configuration declares two projects, not three), and the Vitest test
  locally and in CI by its device pin (two devices, not three).
- A2 is `test_ci_workflows.TheEngineAndStudySuitesRunInFirefox.test_the_web_engine_job_installs_every_engine_before_each_suite`,
  red at the commit of the red tests, in CI, by its first assertion (no install step before either
  suite installs `firefox`).
- A3 is `test_ci_workflows.TheEngineAndStudySuitesRunInFirefox.test_each_engine_project_syncs_as_a_user_of_its_own`,
  red at the commit of the red tests, in CI, by its first assertion (two sync users for three
  projects, and `account('firefox')` gives Chromium's user).
- A4 is not red: whether every engine, sync and study test passes in Firefox is a browser engine's
  behaviour, measured in CI at the second push, in its `web-engine` job.
- A5 is not red: the `web-engine` job's time in three engines is measured in CI at the second push.

A1's study half, the Vitest test `the study suite covers the review loop in both engines`, read
red at 98010cc locally and in CI run 41825749005 job 126559904937 web: `AssertionError: expected [ 'Desktop Chrome', 'Desktop Safari' ] to deeply equal [ 'Desktop Chrome', …(2) ]`.

Two commits between the reds of A1 to A3 and their greens edit test-shaped files. The first,
c7a9078, lists in the file-read census table of `scripts/tests/test_ci_workflows.py` the four
places the three Firefox checks hand a file to their reader, which the census test requires of
a helper that reads a path it is given; it adds table entries only and changes no line of the
three checks. The second, the wiring commit fa020ff, edits the account helper of
`web/app/tests-engine/sync.spec.ts` and its doc comment, which give each engine project but
Chromium a server user of its own (ADR-635 D3), and the header comments of
`web/app/tests-engine/engine.spec.ts` and `web/app/tests-study/study.spec.ts`, which name
Firefox. Neither changes an assertion of A1 to A3: the three checks are the same at the reds'
commit and at the greens', and so is `web/app/src/lib/study/study-coverage.test.ts`.

```red-first
A1: red at 98010cc: CI run 41825749005 job 126559905069 hygiene: AssertionError: Lists differ: ['the engine configuration declares chromium, webkit, not chromium, webkit, firefox'] != []
A1: green at fa020ff
A2: red at 98010cc: CI run 41825749005 job 126559905069 hygiene: AssertionError: Lists differ: ['no install step before test:engine installs firefox', 'no install step before test:study installs firefox'] != []
A2: green at fa020ff
A3: red at 98010cc: CI run 41825749005 job 126559905069 hygiene: AssertionError: Lists differ: ['the engine configuration makes 2 sync user(s) for 3 project(s)', "account('firefox') gives <base>, chromium's user"] != []
A3: green at fa020ff
A4: not red: whether every engine, sync and study test passes in Firefox is a browser engine's behaviour, measured in CI at the second push, in its web-engine job
A5: not red: the web-engine job's time in three engines is measured in CI at the second push
```
