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
