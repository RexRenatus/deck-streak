# Red-first record: SPEC-566

SPEC-566 and ADR-584, with the insert-only amendments of SPEC-352, ADR-363 and the lane
schematic, were committed first (7b768ab). The criteria's tests were then committed alone
(d76402d), before the code that makes them pass (6bad448). A1 to A4 and A8 run on the box
(`test_ios_lane.py` and `test_ios_icon.py`): each red and green below was read there at the
named commit. The app-project criterion's test sits in a workflow test module that CI alone
runs, so its red is quoted from the `hygiene` job's python stage log of run 43322422678 at the
first push's head d76402d, read from that run's `check-stage-logs-hygiene` artifact 13990909871. Its
green is read by name from the same job at the second push's head.

```red-first
A1: red at d76402d: AssertionError: Lists differ: ['arc[19 chars]'ios/Harness.xcodeproj', '-scheme', 'Harness',[207 chars]nal'] != ['arc[19 chars]'ios/Fixtureaf0b1e55fcd4.xcodeproj', '-scheme'[231 chars]nal']
A1: green at 6bad448
A2: red at d76402d: AssertionError: Lists differ: ['bui[17 chars]'ios/Harness.xcodeproj', '-scheme', 'Harness',[230 chars]nal'] != ['bui[17 chars]'ios/Fixture933f56dff720.xcodeproj', '-scheme'[254 chars]nal']
A2: green at 6bad448
A3: red at d76402d: AssertionError: Lists differ: ['-project', 'ios/Harness.xcodeproj', '-scheme', 'Harness'] != ['-project', 'ios/DeckStreak.xcodeproj', '-scheme', 'DeckStreak']
A3: green at 6bad448
A4: red at d76402d: AssertionError: Tuples differ: (0, '', [{'tool': 'xcodebuild', 'args': ['[454 chars][]}]) != (1, "the app's project spec ios/app.yml na[17 chars], [])
A4: green at 6bad448
A7: red at d76402d: CI run 43322422678 job hygiene 131546233010, artifact 13990909871: test_each_app_job_generates_the_apps_project_after_the_harness_steps: AssertionError: 0 != 1 : testflight-internal.yml
A7: green at 6bad448
A8: not red: the harness's control, SPEC-352 A23's test, unchanged
```

A1 and A2 amend SPEC-352 A9 and A13: their pinned argument lists now name the project and the
scheme the fixture's app spec names. Rows S56601 to S56605 were each proved on Linux by installing
the row's replacement, reading its killer fail and restoring the file by its sha256; S56611 and
S56612 are read by name in CI's mutation-rows job. The app-project criterion's green is CI's
reading at this pull request's second push. `git diff --quiet 6bad448 HEAD -- scripts/tests/test_testflight_workflows.py .github/workflows`
exits 0: no workflow test and no lane file changed after the green commit 6bad448.

The first internal lane run after this delivery landed, run 43594481191, read by job and step
name in each of its four attempts:

- Run 43594481191, attempt 1: job harness 132446938968 failed in its step 11 and job release-and-app 132446938921 was cancelled in its step 12 after 53.3 min, so job verdict 132470172080 failed and job app 132470202446 was skipped: no step of the app job ran.
- Run 43594481191, attempt 2: job harness 132472822665 passed; job release-and-app 132472822658 was cancelled in its step 18 after 59.9 min with its 89 finished cases passed, so job verdict 132500517234 failed and job app 132500545732 was skipped.
- Run 43594481191, attempt 3: job release-and-app 132501181468 failed in its step 18 after 69 min 10 s, `test_a12_the_review_survives_the_sidebar` failing on the iPad alone, so job verdict 132520487241 failed and job app 132520534785 was skipped.
- Run 43594481191, attempt 4: job release-and-app 132524127323 passed in 78 min 33 s and job verdict 132546475563 passed.
- Run 43594481191, attempt 4, job app 132546500010 (16 s): steps 1 to 9 passed, "the credential, by presence" among them finding every credential placed; "the app, unsigned" was skipped; "the archive, signed" failed at the provisioning profile's parse, before any certificate read; "the upload" was skipped; steps 13 and 14 passed. Nothing was signed or uploaded.
