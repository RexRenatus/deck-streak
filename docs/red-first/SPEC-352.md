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
A8: red at 84fd5d05: AssertionError: Items in the first set but not the second: (the fixture profile's five identifiers, none masked)
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
A25: red at 7124d3ce: AssertionError: Lists differ: ["crates/engine-core/**: the XCFramework l[157 chars] it"] != []
A25: green at 85c644d7
A26: red at 6cd59182: AssertionError: 1 != 2 : the certificate was read 1 time(s), not twice
A26: green at 5071e917
A27: red at 6cd59182: AssertionError: Lists differ: [] != ['pkcs12 read failed: class wrong-password; reading once more with -legacy']
A27: green at 5071e917
A28: red at 6cd59182: AssertionError: 'openssl pkcs12 read failed: first class wrong-password, -legacy class option-refused' not found in ['openssl pkcs12 failed with exit 1']
A28: green at 5071e917
A29: red at 3adb425: AssertionError: Lists differ: [] != ['fixture openssl version line']
A29: green at 409e44c
A30: red at d2c61361: AssertionError: 0 != 1 : security find-certificate ran 0 times
A30: green at 71427932
A31: red at d2c61361: AssertionError: Lists differ: [('scripts/ios_lane.py', 312, 'sha1')] != []
A31: green at 71427932
A32: red at 06dbdb2: AssertionError: 1 != 0 : openssl pkcs12 failed with exit 1
A32: green at ce15791
A33: red at 88bad39: AssertionError: 0 != 1 : the archive's command line names 0 identities
A33: green at f89e6c0
A34: red at 7f2ba31: AssertionError: 'testFlightInternalTestingOnly' unexpectedly found in ['destination', 'manageAppVersionAndBuildNumber', 'method', 'provisioningProfiles', 'signingCertificate', 'signingStyle', 'teamID', 'testFlightInternalTestingOnly']
A34: green at 5457904
```

- Three red lines are shortened where the failure printed more than a criterion needs: A8's set
  difference lists the fixture profile's five synthetic identifiers, A22's expected value is the
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
- Ruling 461's amendment (SPEC-352 R22). Its tests were committed alone at 7124d3ce, before the
  workflow and the plan changed at 85c644d7. Each red and green was read locally by running the two
  workflow modules, `test_testflight_workflows.py` and `test_ci_workflows.py`, at each commit
  (`Ran 62 tests`: three failures at 7124d3ce, `OK` at 85c644d7), and A1's one test at each. A1,
  A14 and A17 are amended criteria, so their new reds are recorded here and not in the fence. A1
  was red at 7124d3ce on a push to `dev`, `AssertionError: None != 'internal'`, and green at
  85c644d7. A14 was red at 7124d3ce, the internal `on` reading `{'workflow_dispatch': None}`
  against the dispatch and the filtered push, and green at 85c644d7. A17 was red at 7124d3ce, the
  internal queue reading `testflight-internal-${{ github.ref }}` with `queue: max` against R16's
  amended block, and green at 85c644d7. A25's planted control, the live workflow with
  `crates/engine-core/**` dropped, was red by name: `crates/engine-core/**: the XCFramework links
  it, and the internal lane's push filter does not watch it`.
- The first certificate read's amendment (SPEC-352 section 8, R9 and R13 as amended). Its three
  criteria, A26 to A28, are new, so each takes one red and one green line in the fence above. Their
  tests and the shim's refused-read setting were committed alone, before the lane script changed;
  each red and green is read locally by running `test_ios_lane.py` at the named commit, where the
  module's other tests pass. A26 is red when no second read is recorded, A27 when a failed read
  prints no class line, and A28 when the refusal names no class.
- A27's control is an exact mutation of the fixed script: only the class line's print changed, to
  print openssl's standard error after the class. A27 run alone failed by name in each of its four
  cases (`FAILED (failures=4)`), the shim's planted marker text appearing in the step's output;
  the script was then restored by copy, its sha256 equal before and after.
- The certificate read's killers. d246c585 adds the four survivors' killers, two tests after
  A28's, committed after the greens at 5071e917 and changing no test before them.
  `test_the_re_wrap_is_given_exactly_what_the_certificate_read_wrote` kills the two mutants that
  make the read hand nothing to the re-wrap, on the first read's path and on the `-legacy` path;
  `test_each_certificate_read_line_is_written_out_before_the_next_tool_runs` kills the two that
  leave the class line or the `-legacy` ok line unflushed when the next tool runs. Each kill was
  proved by an exact mutation of the script, its killer run alone failing by name and the script
  restored by copy, its sha256 equal before and after. The script was unchanged from the green to bc368d99:
  `git -C <wt> diff --quiet 5071e917 bc368d99 -- scripts/ios_lane.py` reads rc 0.
- The openssl version line's amendment (SPEC-352 section 8, R9 and R13 as amended). Its one
  criterion, A29, is new, so it takes one red and one green line in the fence above. Its test was
  committed alone at 3adb425, before the lane script changed, with one test edit of that red
  commit disclosed here: the shims gain an answer to `openssl version`, kept apart from the reads
  (a record of its own, the planted version line or nothing, and a planted decoy made executable),
  so no existing assertion sees the call. A29 is red at 3adb425 when the step prints no version
  line (`AssertionError: Lists differ: [] != ['fixture openssl version line']`), and green at
  409e44c, where the module's other tests pass. Each kill was proved by an exact mutation of the
  fixed script, A29 run alone failing by name and the script restored by copy, its sha256 equal
  before and after: the version line's print removed, its flush made false or its end changed;
  the read, its call site or the re-wrap naming the bare `openssl`; the version call given the
  step's whole environment or another argument; `tool()` naming the path in its refusal; and the
  missing-openssl check negated, its refusal text changed, its raise removed or its search given
  another name. No mutation coverage commit follows the green.
  The green commit `409e44c` carries the cure, and `git diff --quiet 409e44c HEAD` over scripts/ios_lane.py and scripts/tests/test_ios_lane.py reads rc 0.
- The identity-match amendment (SPEC-352 R8 as amended). Its tests were committed alone at d2c61361, before the lane changed at 71427932, and each red and green was read by running `test_ios_lane.py` in a `git archive` of each commit. With the fixture's identity now the keychain's, every test that signs is red at d2c61361 on the refusal "the profile was not issued for the imported certificate", because the base computes a SHA-1 the fake tool no longer prints; A30 and A31 are new, and each red quoted above is its own assertion. The census's planted control, `test_the_digest_census_refuses_each_planted_spelling_of_a_weak_digest`, is green at both commits by design: it judges the census, not the lane.
- The re-wrap's amendment (SPEC-352 R9 as amended). Its one criterion, A32, is new, so it takes one
  red and one green line in the fence above. Its three tests were committed at 06dbdb2, before the
  lane script changed at ce15791, and each red and green was read locally by running
  `test_ios_lane.py` at each commit: `Ran 32 tests` with six failures at 06dbdb2, and `OK` at
  ce15791. The red quoted above is
  `test_a_legacy_certificate_re_wraps_with_both_bags_under_3des_and_a_sha1_mac`'s: the real openssl
  reads the legacy bundle under `-legacy`, and the re-wrap then refuses.
  `test_the_re_wrap_reads_its_input_from_a_private_file_removed_once_it_returns` is red at 06dbdb2 on
  the same refusal, and
  `test_a_failed_re_wrap_refuses_as_before_and_leaves_no_certificate_in_the_lane` is red at 06dbdb2
  when the re-wrap names no `-in` file,
  `AssertionError: Lists differ: [None] != [{'in the lane': True, 'name': 'certificat[68 chars]y']}]`,
  its refusal line and its check that no `security` call ran already holding there.
  The two tests that pinned the re-wrap's standard input, which decide no criterion of the SPEC,
  are amended in the same red commit, each with no assertion removed.
  `test_each_signing_tool_is_given_exactly_its_arguments` is red at 06dbdb2, the export's recorded
  arguments giving `-out` where its exact list now gives `-in`, and
  `test_the_re_wrap_is_given_exactly_what_the_certificate_read_wrote` is red at 06dbdb2 on both read
  paths, the re-wrap's recorded input `[None]` against what the read wrote. Both are green at
  ce15791, which changes no test.
  The green commit `ce15791` carries the signing step's change, and `git diff --quiet ce15791 HEAD` over `scripts/ios_lane.py` and `scripts/tests/test_ios_lane.py` reads rc 0.
- The archive identity's amendment (SPEC-352 R9 as amended). Its one criterion, A33, is new, so it
  takes one red and one green line in the fence above. Its test and the two amended pins were
  committed at 88bad39, before the lane script changed at f89e6c0, and each red and green was read
  locally by running `test_ios_lane.py` at each commit: `Ran 33 tests` with two failures at
  88bad39, and `OK` at f89e6c0. The red quoted above is
  `test_the_archive_names_the_paired_identity_and_never_the_other_one`'s: on A30's fixture the
  archive's command line names no identity.
  The two pins in A9's test,
  `test_signing_imports_the_certificate_non_extractable_into_a_job_keychain`, that the change
  meets by construction are amended in the same red commit, each with no assertion removed. That
  test is red at 88bad39 on its exact argument list, which now ends with the identity's element,
  `Second list contains 1 additional elements.`, and green at f89e6c0. Its guard over the six
  signing settings comes after that list's assertion, so it was read alone in a scratch copy of
  the test with the list's assertion set aside: red at 88bad39 on `CODE_SIGN_IDENTITY`, where the
  archive's command line names no identity and the guard now expects the include's, and green at
  f89e6c0, which changes no test.
  The green commit `f89e6c0` carries the archive's change, and `git diff --quiet f89e6c0 HEAD` over `scripts/ios_lane.py` and `scripts/tests/test_ios_lane.py` reads rc 0.
- The external export's amendment (SPEC-352 R10 as amended). Its one criterion, A34, is new, so
  it takes one red and one green line in the fence above. Its test and the amended pin were
  committed at 7f2ba31, before the lane script changed at 5457904, and each red and green was read
  locally by running `test_ios_lane.py` at each commit: `Ran 34 tests` with two failures at
  7f2ba31, and `OK` at 5457904. The red quoted above is
  `test_the_export_options_carry_no_internal_testing_only_key`'s: the export options still hold
  the internal-only key.
  The export-options pin in A10's test,
  `test_the_upload_key_lives_in_one_private_file_for_the_upload_step_only`, that the change meets
  by construction is amended in the same red commit, with no assertion removed: the internal-only
  key leaves its expected options. That test is red at 7f2ba31 on that pin, the options the export
  reads still ending `'testFlightInternalTestingOnly': True}`, and green at 5457904, which
  changes no test.
  The green commit `5457904` carries the export options' change, and `git diff --quiet 5457904 HEAD` over `scripts/ios_lane.py` and `scripts/tests/test_ios_lane.py` reads rc 0.
