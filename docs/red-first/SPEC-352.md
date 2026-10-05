# Red-first record: SPEC-352

The SPEC, its schematic and ADR-363 were committed first (4f4b7c7a), then the lanes' shape and the
lane script's verbs (ba7fac99). Each criterion's test was then committed alone, before the code
that makes it pass. A1 to A13, A23 and A24 run on the box (`test_ios_lane.py` and
`test_ios_icon.py`): each red and green below was read by running that criterion's one test in a
`git archive` of the named commit. A14 to A22 sit in workflow test modules that CI alone runs, so
their reds are quoted from the `hygiene` job's python stage log of run 37298620987 at the first
push's head 01dbb0a8, where `scripts/tests` read `Ran 978 tests` and `FAILED (failures=13)`. Their
greens are read by name from the same job at the completing push's head.

```red-first
A1: red at 12092762: AssertionError: 'the internal lane runs on workflow_dispatch only, not on push' not found in []
A1: green at 9b8f8d31
A2: red at 12092762: AssertionError: 'the checkout is shallow; the build number needs the full history' not found in []
A2: green at 9b8f8d31
A3: red at 12092762: AssertionError: None != '4'
A3: green at 9b8f8d31
A4: red at 12092762: AssertionError: None != '3.14.15'
A4: green at 9b8f8d31
A5: red at 12092762: AssertionError: 'the tag v0.0.1 is a lightweight tag; a release is an annotated tag' not found in []
A5: green at 9b8f8d31
A6: red at 12092762: AssertionError: None != '2'
A6: green at 9b8f8d31
A7: red at 84fd5d05: AssertionError: "the credential is half placed; absent: the key id, the issuer id, the certificate, the certificate's password, the profile" not found in []
A7: green at 192c1b65
A8: red at 84fd5d05: AssertionError: Items in the first set but not the second: (the fixture profile's four identifiers, none masked)
A8: green at 192c1b65
A9: red at 84fd5d05: AssertionError: 0 != 1 : security create-keychain ran 0 times
A9: green at 192c1b65
A10: red at 84fd5d05: AssertionError: 0 != 1 : xcodebuild -exportArchive ran 0 times
A10: green at 192c1b65
A11: red at 84fd5d05: AssertionError: Lists differ: [] != [['PASSWORD'], []]
A11: green at 192c1b65
A12: red at 84fd5d05: AssertionError: Lists differ: [] != [['list-keychains', '-d', 'user', '-s', '/[98 chars]db']]
A12: green at 192c1b65
A13: red at 84fd5d05: AssertionError: 0 != 1 : xcodebuild build ran 0 times
A13: green at 192c1b65
A14: not red: the shape commit ba7fac99 already gave each lane its one trigger, so the test passed at the first push; it is mutation coverage, held by row S35201
A15: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_job_graph_is_plan_then_framework_then_app: AssertionError: Lists differ: ['plan'] != ['plan', 'framework', 'app']
A15: green at 54954977
A16: red at 01dbb0a8: CI run 37298620987 job hygiene: test_only_the_app_job_names_an_environment_and_reads_a_credential: AssertionError: {} != {'app': 'testflight-internal'}
A16: green at 54954977
A17: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_lanes_are_hardened_pinned_uncached_and_queued: AssertionError: {'clean': None, 'summary': None} != {'clean': '${{ always() }}', 'summary': '${{ always() }}'}
A17: green at 54954977
A18: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_two_app_jobs_differ_only_in_environment_and_lane: AssertionError: 'app' not found in ['plan']
A18: green at 54954977
A19: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_framework_job_calls_the_apple_job_body_with_nothing_passed: AssertionError: None != {'needs': 'plan', 'uses': '$/.github/workflows/xcframework.yml'} : testflight-internal.yml
A19: green at 54954977
A20: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_app_job_places_the_framework_as_the_harness_does: AssertionError: Lists differ: [] != ['plan', 'framework']
A20: green at 54954977
A21: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_lanes_credential_reads_are_admitted_and_no_other: AssertionError: {} != {'preflight': {'ISSUER', 'PROFILE', 'KEYID[118 chars]EY'}}
A21: green at 54954977
A22: red at 01dbb0a8: CI run 37298620987 job hygiene: test_the_macos_runner_is_admitted_to_the_lane_app_jobs_only: AssertionError: None != (the admitted macOS image) : testflight-internal.yml
A22: green at 54954977
A23: red at 83b2734a: AssertionError: None != 'AppIcon'
A23: green at edf2fa57
A24: red at 83b2734a: AssertionError: False is not true : the script wrote no icon
A24: green at edf2fa57
```

- Three red lines are shortened where the failure printed more than a criterion needs: A8's set
  difference lists the fixture profile's four synthetic identifiers, A22's expected value is the
  admitted runner image's label, and A21's sets carry role words, never a secret's name.
- A23 is red on the project's half (no app icon named); its property-list half, the launch screen
  and the four iPad orientations, is mutation coverage, held by row S35207.
- Commits that edited a test after a criterion's red, each named here: 192c1b65 adds one call that
  cleans the step's material between A8's refused profiles (a fixture cure, no assertion changed);
  f488e3a4 adds the kill tests after A7 to A13 were green; 01dbb0a8, the first push's head,
  strengthens assertions in `test_ios_lane.py` after A7 to A13 were green and in the R3 tests
  before any of them was read, each equal to or stronger than the one it replaces; 54954977 adds the three admissions to
  `test_ci_workflows.py` that A21 and A22 name (the exact call list's two `framework` calls, the
  per-file and per-job runner admission, and the secret admission table); 236dd7d9 rebuilds the
  lane-plan fixture as a static method so the read census can place it; 83b2734a, A23's and A24's
  red commit, adds one `DYNAMIC_IMPORTS` entry for A24's `zlib` call.
- The first push also read two reds that are no criterion's: the file-read census in
  `WorkflowFilesAreReadAsBytes` (its census test and three planted-site controls), cured in 236dd7d9
  by the fixture change above and never by a change to the census; and the planned-SPEC check,
  cured in ebb03f0a, where the SPEC's status reads delivered.
- A14 to A22 are never run on the box. Their greens are the CI reading by name at the completing
  push's head; the green lines name 54954977, the commit that wrote the lanes in full.
