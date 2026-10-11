# SPEC-352: iPhone and iPad builds reach internal TestFlight from a dispatch on dev and from a SemVer tag on main, and stop before the upload until the credential is placed

- **Issue:** #634. **Context(s):** the CI workflows and `scripts/`, and the app target under `ios/`
  (the Swift harness's, #656), whose project and asset catalog gain what an upload needs.
- **Decided by:** ADR-363 (where the credential is readable, where the lane lives, how the app is
  signed, how it is uploaded, how the lane knows the credential is placed, how a public log stays
  private, how the release lane holds real data back, what an upload needs that a simulator build
  does not, and how the build states its lane), resting on ADR-344 (the dispatch on `dev` and the
  first-parent build number), ADR-335 (the client, and an upload as artifact publication), ADR-350
  (the Swift harness's seam this lane inherits), ADR-355 (one Apple job body and its callers) and
  ADR-292 (queued runs), under the owner ruling on the lane's secrets (#668).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-352.md`. **Mutation band:** `S35200-S35299`.
- **Base:** cut from `dev` at `4da5cfcadcfd407df49c1fb2967664e2f48e87dd`, which holds the Swift
  harness (#656) and the owner ruling on the lane's secrets (#668).

## 1. The problem, measured

Every count below was read at `dev` `4da5cfcadcfd407df49c1fb2967664e2f48e87dd` (DEV) with `git
ls-tree`, `git show DEV:<path>` and `grep -n`.

- Ten workflows exist (`git ls-tree -r --name-only DEV | grep -c '^.github/workflows/'` prints
  10). None uploads to TestFlight, and none reads a secret: `scripts/tests/test_ci_workflows.py`
  (`test_no_workflow_reads_a_secret_or_checks_out_another_repository`) refuses every secret read
  other than the repository's own token, and its runner rule admits only versioned Ubuntu images
  outside the one macOS file it admits by name, `xcframework.yml` (`ADMITTED_RUNNERS`, line 43).
- ADR-344 decides the internal lane (a dispatch on `dev` only, the dev app id, the owner the only
  tester, `CFBundleVersion` from `git rev-list --count --first-parent HEAD` on full history, a
  shallow clone refused, the credential the owner's to place, and a build that stops before the
  upload until it is placed). It leaves the release lane to "its own ADR" (ADR-344, Decision
  Outcome, "Release builds"), and its "repository secrets" cannot be kept to one workflow; the
  owner ruling on the lane's secrets (#668) replaces them with one GitHub environment per lane.
- ADR-335 records that an upload to TestFlight is artifact publication and not a deploy, so CI may
  perform it from `dev` as from a tag on `main`; SPEC-334 R17 says the internal lane runs
  throughout the build, and R20 that the team id, the app ids and the upload key never enter this
  repository.
- `.github/workflows/release.yml` builds host releases from a SemVer tag with no secret, in two
  jobs, the sync server's audit and the release (`scripts/tests/test_release_workflow.py`,
  `steps_of`), and its release job's step after the checkout refuses a tag that is not SemVer, a
  lightweight tag and a commit that is not on `main` (lines 44 to 60).
- `xcframework.yml`, the one Apple job body, already takes `workflow_call` (line 19) and
  `workflow_dispatch` (line 20); `apple-on-change.yml` calls it on a pull request that changes an
  Apple or FFI path, and `apple-on-tag.yml` on every release tag, which builds and publishes
  nothing and passes no secret (SPEC-344).
- The Swift harness (#656) hands this lane a seam and nothing more: a shared scheme `Harness` whose
  archive builds Release (`ios/project.yml` lines 69 to 82), the bundle id from one build setting
  whose committed value is a reserved name written reversed, `invalid.deckstreak.harness`
  (`ios/Config/Harness.xcconfig` line 8), the build number and version from
  `CURRENT_PROJECT_VERSION` and `MARKETING_VERSION`, no team in any committed file, an optional
  include of an ignored local settings file, and signing turned off on the CI command line. Its
  property list already declares `UILaunchScreen` and the iPhone and iPad orientations
  (`ios/Harness/Info.plist` lines 32 to 46); the tree holds no asset catalog and no app icon
  (`git ls-tree -r --name-only DEV ios | grep -c -i -E 'xcassets|\.png$'` prints 0), which App
  Store Connect requires of an upload and a simulator build does not.
- The settings xcconfig's opening comment (lines 1 to 3) says the lane passes the real app id and
  the team on its own command line, which the include-file design of ADR-350 does not do.
- A workflow log of this repository is public, so whatever a signing tool prints there is
  published: the signing identity's name, the team and the app id included.

## 2. Requirements

R1. Two workflows carry the lane. `testflight-internal.yml` runs on `workflow_dispatch` and on
    nothing else. `testflight-release.yml` runs on a push of a tag matching
    `v[0-9]+.[0-9]+.[0-9]+` and on nothing else. Neither declares `pull_request`,
    `pull_request_target`, `workflow_run`, `schedule`, or a push to a branch.
    Amended by R22 (ruling 461: a push on `dev`, a path filter, and push runs that supersede one
    another): `testflight-internal.yml` also runs on a push to `dev` that changes a path its filter
    names, and on nothing else; the release lane's trigger is unchanged.
R2. Each lane runs three jobs in order: `plan` on a versioned Ubuntu image, with no environment
    and no secret; `framework`, the reusable call of `xcframework.yml` at the same commit, passing
    no secret; and `app` on the macOS image the hardening test admits, naming the lane's
    environment. A refusal in `plan` starts no macOS job.
R3. The internal `plan` refuses, naming the reason: an event other than `workflow_dispatch`; a ref
    other than `refs/heads/dev`; a shallow checkout (`git rev-parse --is-shallow-repository`
    prints `true`). It outputs the build number, `git rev-list --count --first-parent HEAD`, and
    the marketing version.
    Amended by R22 (ruling 461): the internal `plan` admits a push as well as a dispatch, each on
    `refs/heads/dev` only, and refuses any other event by name.
R4. The release `plan` refuses, naming the reason: an event other than a tag push; a tag that is
    not SemVer; a lightweight tag; a commit that is not on `main`'s first-parent chain; a tag
    whose version differs from the workspace version; a shallow checkout. It outputs the build
    number, the first-parent count of the tag's commit (ADR-344, "numbered by `main`'s own
    first-parent count"), and the marketing version. Its job also runs the release workflow's own
    ancestry step, `git merge-base --is-ancestor "$GITHUB_SHA" origin/main`, on a full-history
    checkout.
R5. The marketing version is the workspace version from the root `Cargo.toml`, and the plan refuses
    one that is not three dot-separated integers, the form `CFBundleShortVersionString` takes.
R6. Each lane's `framework` job calls `$/.github/workflows/xcframework.yml`, the one Apple job
    body, which already takes `workflow_call` with no input and no secret (SPEC-344), and passes
    it no `with` and no `secrets`. No trigger, path filter or concurrency of `xcframework.yml`
    changes: its header comment's caller list names the two lanes, and its `harness` job gains the
    icon step of R19.
R7. The `app` job's first credential step reads each of the six credential parts (the upload key,
    its key id, its issuer id, the distribution certificate, the certificate's password, and the
    lane's provisioning profile) only as a presence boolean. All six present: it signs and
    uploads. None present: it builds Release for a generic iOS device with signing off and the
    committed placeholder app id (a reserved `.invalid` name written reversed, so its first label
    is `invalid`), writes "stopped before the upload: the credential is not placed" to the run
    summary, and succeeds. Some present: it fails, naming each absent part by its role.
R8. The team and the app id are read from the placed profile, never placed separately. Before any
    tool runs, the lane checks the profile: an App Store distribution profile (no device list, no
    `get-task-allow`), unexpired, issued for the imported certificate (a SHA-1 of one of its
    developer certificates equals the identity's), and an app id whose first label is not
    `invalid`, the placeholder's. A failed check names it and stops the job before the archive.
R9. Signing: the certificate's password reaches `openssl pkcs12` on its standard input only, and
    openssl re-wraps the certificate under a passphrase generated for the step; the re-wrapped
    bundle is imported non-extractable (`security import -x -T /usr/bin/codesign`, with that
    run-time passphrase) into a keychain the step creates under the runner's temporary directory
    with a password generated at run time; every certificate file is removed before any build
    tool runs; the profile is installed for the step; and the archive is signed manually by the
    distribution identity with the lane's profile. The team, the app id, the identity and the
    profile reach the build through the ignored include file the harness's settings already name,
    written for the job and removed by R11: the include is project-level, so it reaches every
    native target and never a package target, and the archive builds the scheme `Harness`'s
    `Harness` target alone. No tracked file changes, and `ios/Config/Harness.xcconfig`'s opening
    comment says so.
R10. Upload: only the upload step writes the key, as a mode `0600` file in a fresh mode `0700`
    directory under the runner's temporary directory, and removes that directory when the step
    ends, pass or fail. The step exports and uploads with `xcodebuild -exportArchive` and export
    options it writes under the runner's temporary directory: method `app-store-connect`, manual
    signing, destination `upload`, the build number left as the plan set it, and internal testing
    only. Its `run` names `upload-to-testflight`.
R11. An `always()` step removes the key directory, the installed profile, the include file and the
    job keychain wherever any remains, restores the keychain search list, and succeeds when there
    is nothing to remove.
R12. No secret value (the key, the certificate, its password, the profile) reaches a child's
    argument list or environment. The certificate's password reaches `openssl pkcs12` on its
    standard input, and `security import` is given only the passphrase generated for the step. The
    key reaches a child only as a file path.
R13. The log publishes no private value. The identity's name, the profile's name and UUID, the team,
    the app id and the keychain password are masked with `::add-mask::` before the first tool
    runs, and so is the passphrase that re-wraps the certificate; every build and signing tool
    writes its full output to a file under the runner's temporary directory that is never
    uploaded; a step prints only that output's error and warning lines, with each private value
    replaced. No step traces its commands, and the lane uploads no artifact.
R14. Only the `app` job of the two lane files reads a secret, and only in its preflight, signing and
    upload steps, each reading exactly the parts it uses, in that step's own `env` under a neutral
    role-word variable name, never at the workflow's or the job's level and never in a `run` text.
    The hardening test's one admission table admits those reads by file, job and step, spelling
    each as `secrets.<NAME>`, and refuses every other secret read, `secrets: inherit`, and a
    secret passed to the reusable call, each shape planted at run time and refused by name.
R15. The macOS image is admitted, by file and job, to the two lane files' `app` jobs and to no
    other job they hold; their `plan` jobs run on a versioned Ubuntu image. `xcframework.yml`'s
    admission by file name, and its tests, are unchanged.
R16. Each lane file defaults its token to `contents: read`, checks out with
    `persist-credentials: false`, pins every action by its full commit SHA, restores and saves no
    cache, expands no `${{ }}` expression inside a `run` script, and queues its runs in one group
    per lane and ref that never cancels a run in progress and never replaces a waiting one, up to
    `queue: max`'s hundred waiting runs (ADR-292):
    `testflight-internal-${{ github.ref }}` and `testflight-release-${{ github.ref }}`, with
    `cancel-in-progress: false` and `queue: max`, and no job-level block.
    Amended by R22 (ruling 461): the internal lane's group is
    `testflight-internal-${{ github.event_name }}-${{ github.ref }}`, with
    `cancel-in-progress: false` and `queue: single`, so a dispatch and a push wait in groups of
    their own, no run in progress is cancelled (SPEC-190 R9), and a newer run replaces the waiting
    run of its own event: push runs supersede one another while they wait. The release lane's
    block is unchanged.
R17. The two `app` jobs are identical except for the environment's name and the lane value, and the
    two `plan` jobs differ only in the lane argument and the release lane's ancestry step.
R18. The lane passes `DS_LANE`, `internal` or `release`, as a build setting on the `xcodebuild`
    command line only, and the summary names the lane. No app or harness source reads it, so
    SPEC-347 R14's "The app reads no other configuration" holds; the app's sync settings are
    SPEC-347 R14's, rendered when the lane moves to the app's scheme (#625), and a dev or internal
    build carries no staging sync credential: it is entered at login and kept in the Keychain.
R19. The app target declares what an upload requires and a simulator build does not. Its property
    list already holds a launch screen (`UILaunchScreen`) and the four iPad orientations, and
    keeps them. Its project names an asset catalog among the target's sources, with
    `ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon`; the catalog's icon set lists one 1024-pixel
    image, which a committed standard-library script, `scripts/ios_icon.py write <path>`, writes
    at build time as an opaque square with no text. The image is never committed: `.gitignore`
    holds its path, and the scrubber refuses a committed binary. The script runs as one step in
    the harness job, between placing the framework and fetching XcodeGen, and as the same step in
    each lane's `app` job.
R20. Every lane run's summary names the lane, the commit, the marketing version, the build number,
    whether the credential was placed, and the upload's outcome, and no private value.
R21. `RELEASING.md` says how to dispatch an internal build, what a release tag starts, where the
    build number comes from, and what "stopped before the upload" means.
R22. Ruling 461: the internal lane also starts on a push to `dev`, filtered to the app's inputs,
    and push runs supersede one another. `testflight-internal.yml` runs on `workflow_dispatch` and
    on a `push` to `dev` whose `paths` filter names exactly: every crate the XCFramework links,
    read from the packages `xcframework.yml`'s cargo commands build and the workspace manifests'
    path dependencies (`crates/ffi/**` and `crates/engine-core/**`); `ios/**`; `Cargo.lock`,
    `Cargo.toml` and `rust-toolchain.toml`; the two workflow files the lane runs,
    `.github/workflows/xcframework.yml` and `.github/workflows/testflight-internal.yml`; and the two
    scripts its steps call, `scripts/ios_lane.py` and `scripts/ios_icon.py`. It declares no other
    trigger, so no pull request, schedule, tag or other branch starts it. Its `plan` admits that
    push (R3 as amended), and its queue is R16's as amended. The token stays read-only, every
    action stays pinned by its full commit SHA, the jobs stay on hosted runners, a run with no
    credential placed builds and stops before the upload (ADR-363 D5), and ADR-363 D7's rule that
    no real data reaches a device before the real-data gates clear is unchanged.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the internal plan refuses a push, a pull request and a dispatch on any ref but `dev`, each by name, and admits a dispatch on `dev` | `test_ios_lane.py`, `test_the_internal_plan_refuses_any_event_or_ref_but_a_dispatch_on_dev` |
| A2 | each plan refuses a shallow clone of a fixture repository by name | `test_ios_lane.py`, `test_the_plan_refuses_a_shallow_checkout` |
| A3 | on a fixture `dev` with a merged side branch, the internal number is the first-parent count, not the commit count | `test_ios_lane.py`, `test_the_internal_build_number_is_devs_first_parent_count` |
| A4 | the marketing version is the workspace version; `1.2` and `1.2.3-rc.1` are refused | `test_ios_lane.py`, `test_the_marketing_version_is_the_workspace_version_in_three_integers` |
| A5 | the release plan admits an annotated SemVer tag on `main`'s first-parent chain whose version is the workspace's, and refuses a lightweight tag, a non-SemVer tag, a tag on a `dev` commit that `main` reaches only through a merge's second parent, and a version mismatch, each by name | `test_ios_lane.py`, `test_the_release_plan_admits_only_an_annotated_semver_tag_on_mains_first_parent_chain` |
| A6 | the release number is `main`'s first-parent count at the tag's commit | `test_ios_lane.py`, `test_the_release_build_number_is_mains_first_parent_count_at_the_tag` |
| A7 | the preflight answers `all` for six present parts, `none` for none, and fails naming the absent roles for each partial set | `test_ios_lane.py`, `test_the_preflight_uploads_on_all_stops_on_none_and_fails_on_some` |
| A8 | a fixture profile's team, app id, name and UUID are read, checked and masked before the first tool runs; a development profile, an expired one, one for another certificate and one whose app id's first label is `invalid` are each refused by name | `test_ios_lane.py`, `test_the_profile_is_read_checked_and_masked_before_any_tool_runs` |
| A9 | with recording shims on `PATH`, signing creates its keychain under the temporary directory, imports with `-x`, removes the certificate file before `xcodebuild` runs, and writes the include file the app target reads | `test_ios_lane.py`, `test_signing_imports_the_certificate_non_extractable_into_a_job_keychain` |
| A10 | the key exists only during the upload step, as a `0600` file in a `0700` directory under the temporary directory, reaches `xcodebuild` as `-authenticationKeyPath`, and is gone after the step, also when the shim fails | `test_ios_lane.py`, `test_the_upload_key_lives_in_one_private_file_for_the_upload_step_only` |
| A11 | sentinel secret values appear in no shim's argument list or environment and in no printed line; the certificate's password reaches `openssl` on its standard input; sentinel identifiers appear only in `::add-mask::` lines printed before the first tool | `test_ios_lane.py`, `test_no_secret_value_reaches_a_childs_argv_or_environment_or_the_log` |
| A12 | the clean step removes the key directory, the profile, the include file and the keychain, restores the search list, and exits 0 when none exists | `test_ios_lane.py`, `test_the_clean_step_removes_every_piece_of_signing_material` |
| A13 | with no part placed, the build runs unsigned with the committed placeholder app id (first label `invalid`), the plan's number, version and lane, and the summary says it stopped before the upload | `test_ios_lane.py`, `test_the_unsigned_build_stops_before_the_upload_and_says_so` |
| A14 | each lane runs on its one trigger, and neither declares a trigger a pull request starts | `test_testflight_workflows.py`, `test_each_lane_runs_on_its_one_trigger_and_nothing_a_pull_request_starts` |
| A15 | each lane's jobs are `plan`, `framework` and `app`, in that order of `needs`, and `framework` passes no secret | `test_testflight_workflows.py`, `test_the_job_graph_is_plan_then_framework_then_app` |
| A16 | only the `app` job names an environment, and it reads secrets only in its preflight, signing and upload steps | `test_testflight_workflows.py`, `test_only_the_app_job_names_an_environment_and_reads_a_credential` |
| A17 | both lanes are read-only by default, check out full history in `plan` and persist no token in any job, are pinned and uncached, expand no expression in a script, run `clean` and `summary` under `always()`, and queue without cancelling | `test_testflight_workflows.py`, `test_the_lanes_are_hardened_pinned_uncached_and_queued` |
| A18 | the two `app` jobs differ only in the environment and the lane; the two `plan` jobs only in the lane argument and the ancestry step; and that step is `release.yml`'s, byte for byte | `test_testflight_workflows.py`, `test_the_two_app_jobs_differ_only_in_environment_and_lane` |
| A19 | each lane's `framework` job calls `$/.github/workflows/xcframework.yml` with no input and no secret, and that workflow's `on` is exactly `workflow_call` and `workflow_dispatch`, with no input, no secret and no concurrency block | `test_testflight_workflows.py`, `test_the_framework_job_calls_the_apple_job_body_with_nothing_passed` |
| A20 | the `app` job needs `plan` and `framework`, downloads exactly one artifact, `xcframework`, with no run id, token or repository, and places the framework, writes the icon and generates the project with the harness job's own steps, from the download through "the project, generated"; its env defines the harness's report, results, developer directory and generator constants, the plan's outputs, and no secret | `test_testflight_workflows.py`, `test_the_app_job_places_the_framework_as_the_harness_does` |
| A21 | the lanes' reads are admitted by file, job and step, and each shape planted at run time from the live lane text is refused by name: a read in `plan`, in a third file, in a lane file that gains a pull request trigger, in an admitted step reading a part it does not use, in the `app` job's job-level env and in a `run` text, and `secrets: inherit` or a named secret passed to the reusable call | `test_ci_workflows.py`, `test_the_lanes_credential_reads_are_admitted_and_no_other` |
| A22 | the macOS image is admitted by file and job to the two `app` jobs, and refused on a lane's `plan` job and on a job named `app` in another file; every `runs-on` is judged | `test_ci_workflows.py`, `test_the_macos_runner_is_admitted_to_the_lane_app_jobs_only` |
| A23 | the app's property list declares `UILaunchScreen` and the four iPad orientations, and the project names the asset catalog among the target's sources and its app icon | `test_ios_icon.py`, `test_the_app_declares_what_an_upload_requires` |
| A24 | the icon `scripts/ios_icon.py` writes decodes to a 1024 by 1024 RGB image with no alpha channel and no text chunk, every unfiltered pixel one opaque colour | `test_ios_icon.py`, `test_the_generated_icon_is_an_opaque_square_with_no_text` |
| A25 | the internal lane's push filter watches exactly the crates the XCFramework links, read from the packages its cargo commands build and the workspace's manifests; a filter missing one linked crate, and a filter naming a crate the XCFramework does not link, are each refused by name | `test_testflight_workflows.py`, `test_the_internal_push_filter_watches_every_crate_the_xcframework_links` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_internal_plan_refuses_any_event_or_ref_but_a_dispatch_on_dev
A2: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_plan_refuses_a_shallow_checkout
A3: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_internal_build_number_is_devs_first_parent_count
A4: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_marketing_version_is_the_workspace_version_in_three_integers
A5: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_release_plan_admits_only_an_annotated_semver_tag_on_mains_first_parent_chain
A6: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_release_build_number_is_mains_first_parent_count_at_the_tag
A7: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_preflight_uploads_on_all_stops_on_none_and_fails_on_some
A8: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_profile_is_read_checked_and_masked_before_any_tool_runs
A9: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_signing_imports_the_certificate_non_extractable_into_a_job_keychain
A10: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_upload_key_lives_in_one_private_file_for_the_upload_step_only
A11: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_no_secret_value_reaches_a_childs_argv_or_environment_or_the_log
A12: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_clean_step_removes_every_piece_of_signing_material
A13: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_unsigned_build_stops_before_the_upload_and_says_so
A14: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_each_lane_runs_on_its_one_trigger_and_nothing_a_pull_request_starts
A15: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_job_graph_is_plan_then_framework_then_app
A16: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_only_the_app_job_names_an_environment_and_reads_a_credential
A17: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_lanes_are_hardened_pinned_uncached_and_queued
A18: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_two_app_jobs_differ_only_in_environment_and_lane
A19: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_framework_job_calls_the_apple_job_body_with_nothing_passed
A20: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_app_job_places_the_framework_as_the_harness_does
A21: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_lanes_credential_reads_are_admitted_and_no_other
A22: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_macos_runner_is_admitted_to_the_lane_app_jobs_only
A23: python3 -m unittest discover -s scripts/tests -p test_ios_icon.py -k test_the_app_declares_what_an_upload_requires
A24: python3 -m unittest discover -s scripts/tests -p test_ios_icon.py -k test_the_generated_icon_is_an_opaque_square_with_no_text
A25: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_the_internal_push_filter_watches_every_crate_the_xcframework_links
```

Every criterion runs on Linux in the hygiene job's `python` stage, with no credential, no macOS
runner and no Apple tool: the lane's Apple tools are recording shims on `PATH`, and its git
history is a fixture repository built in a temporary directory. Two things no test here can read
are recorded in `docs/red-first/SPEC-352.md` after the delivery lands, as section 7 says.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/testflight-internal.yml` | CI | added |
| `.github/workflows/testflight-release.yml` | CI | added |
| `.github/workflows/xcframework.yml` | CI | changed: the header comment's caller list names the two lanes, and the `harness` job writes the icon (R19) |
| `scripts/ios_lane.py` | scripts | added: `plan`, `preflight`, `build-unsigned`, `sign`, `upload-to-testflight`, `clean`, `summary` |
| `scripts/ios_icon.py` | scripts | added: the icon generator |
| `scripts/mutation-python.json` | scripts | changed: the two scripts mapped to their tests |
| `scripts/tests/test_ios_lane.py` | tests | added |
| `scripts/tests/test_ios_icon.py` | tests | added |
| `scripts/tests/test_testflight_workflows.py` | tests | added |
| `scripts/tests/test_ci_workflows.py` | tests | changed: the lanes' secret, runner and call admissions, their plants built at run time, and the census's entry for the icon test's decoder |
| `scripts/mutation-rows.d/S35200-S35299.json` | mutation | added |
| `ios/project.yml` | `ios-harness` | changed: the asset catalog and its app icon name |
| `ios/Harness/Assets.xcassets/Contents.json` | `ios-harness` | added |
| `ios/Harness/Assets.xcassets/AppIcon.appiconset/Contents.json` | `ios-harness` | added |
| `ios/Config/Harness.xcconfig` | `ios-harness` | changed: its opening comment says how the lane signs |
| `.gitignore` | `ios-harness` | changed: the generated icon's path |
| `RELEASING.md` | docs | changed: a TestFlight section |
| `changelog.d/testflight-lane-352.md` | docs | added |
| `docs/specs/SPEC-352-testflight-lane.md`, `docs/decisions/ADR-363-testflight-lane.md`, `docs/schematics/testflight-lane.md`, `docs/red-first/SPEC-352.md` | docs | added |

## 5. What this does NOT cover

- It places no credential and creates no app record, environment or tester group: those are the
  owner's acts, and until they are done every run stops before the upload (#634).
- It uploads nothing until the owner places the credential; the first upload, and its build in
  internal TestFlight under the dev app id, is recorded after the delivery lands (#634).
- It uploads no release build: the first one is the milestone build from a SemVer tag, after the
  real-data gates clear (#636).
- It builds no minimum-client handshake and no re-release check before a TestFlight build expires
  (#671).
- It points no build at a sync user and places no sync credential: the harness has no sync, the
  app's sync settings are the ones SPEC-347 R14 names, rendered when the lane moves to the app's
  scheme (#625), and a dev or internal build's staging credential is entered at login and kept in
  the Keychain, never built in (#634).
- It archives the harness's scheme `Harness`, not the app's: the move to the scheme `DeckStreak`
  and SPEC-347 R14's settings is #625's.
- It answers no export-compliance question: `ITSAppUsesNonExemptEncryption` stays false, and the
  answer is the owner's before the first upload (#634).
- It opens no external testing group, no public link and no App Store submission (#634, beside the
  ruling #612).
- It changes no trigger, path filter or concurrency of `xcframework.yml`: its callers on a change
  and on a release tag are SPEC-344's (#622), and a release tag runs that job body once for each
  caller, this lane's included.
- It adds no device to a profile and installs nothing on a device; the owner's first device session
  is #629.
- It replaces no product icon: the generated square stands until the app shell's own (#625).
- It signs no push or associated-domains entitlement; a later delivery that adds one re-issues the
  lane's profiles through the owner (#640).

## 6. Risks

- **A credential reaches a pull request's code.** The lanes declare no trigger a pull request
  starts, the environments admit only `dev` and SemVer tags, and A14, A16 and A21 refuse every
  other shape by name.
- **A private value reaches the public log.** Masks precede the first tool, tool output stays in
  files that are never uploaded, and A11 plants sentinels in every input a shim can echo.
- **The key outlives its step.** The upload step removes its directory on any exit, an `always()`
  step removes it again, and A10 and A12 read both, the failing shim included.
- **A shallow or rewritten history numbers a build wrongly.** The plan refuses a shallow clone
  (A2), and `dev` refuses a force-push (ADR-344's "What would make this wrong").
- **The first upload is refused for a reason only App Store Connect states.** Section 7 records
  it; A23 and A24 hold the requirements a simulator build never meets.
- **The release lane uploads real data before the gates clear.** Its environment is the owner's to
  create after them, with the owner as its required reviewer (ADR-363 D7); before that, a tag's
  run stops before the upload.

## 7. Measured after the delivery lands

Neither figure is a criterion: no pull request can read them, because the lanes run only on `dev`
and on tags.

- **The first dispatch, before the credential is placed.** `gh workflow run testflight-internal.yml
  --ref dev`, then `gh run view <id> --json jobs`: `plan`, `framework` and `app` succeed, and the
  summary names the commit, the version, the number, which equals `git rev-list --count
  --first-parent <sha>` read on a full-history clone, and "stopped before the upload".
- **The first upload, after it is placed.** The same dispatch: the summary reads "uploaded", and the
  build appears in internal TestFlight under the dev app id with that number (ADR-344,
  Confirmation).

## 8. Amendments

### Ruling 461: the internal lane also starts on a push to dev

The internal lane could not run: GitHub starts a `workflow_dispatch` workflow only from a file on
the default branch, the default branch stays `main`, and `main` does not hold the lane's file.
Ruling 461 adds a push trigger on `dev`, filtered to the app's inputs, with push runs that
supersede one another (R22). R1, R3 and R16 carry amendment lines, ADR-363 D2 records the reversal
of its "one file with both triggers" rejection, and ADR-344 carries an amendment line on its "no
push starts it" clause.

- A1 (R3) now admits a push on `dev` beside a dispatch on `dev`, and refuses a pull request, a
  schedule, and a push or a dispatch on any ref but `dev`, each by name.
- A14 (R1) now holds the internal lane's `on` to exactly the dispatch and the filtered push; the
  release lane's assertion is unchanged.
- A17 (R16) now holds the internal lane's queue to exactly R16's amended block; the release lane's
  is unchanged.
- A25 (R22) is new.

| file | context | change |
|---|---|---|
| `.github/workflows/testflight-internal.yml` | CI | changed: the push trigger, its filter and the queue (R22) |
| `scripts/ios_lane.py` | scripts | changed: the internal plan admits a push on `dev` (R3 as amended) |
| `scripts/tests/test_ios_lane.py` | tests | changed: A1 |
| `scripts/tests/test_testflight_workflows.py` | tests | changed: A14 and A17, and A25 added |
| `scripts/tests/test_ci_workflows.py` | tests | changed: the read census's entry for A25's manifest read |
| `scripts/mutation-rows.d/S35200-S35299.json` | mutation | changed: rows for the new arms, appended |
| `changelog.d/testflight-internal-push-352.md` | docs | added |
| `docs/specs/SPEC-352-testflight-lane.md`, `docs/decisions/ADR-363-testflight-lane.md`, `docs/decisions/ADR-344-internal-testflight-builds-come-from-a-dispatch-on-dev-numbered-by-its-first-parent-count.md`, `docs/schematics/testflight-lane.md`, `docs/red-first/SPEC-352.md` | docs | changed: insert-only amendments |

The amendment changes no trigger, filter or queue of the release lane or of `xcframework.yml`, and
no repository or environment setting (#634).

### SPEC-405: the release lane also takes a dispatch at the tag's own ref

#733 gives the release lane an input-free manual dispatch at the tag's own ref, through the same
guard step, plan and group (SPEC-405, ADR-419). R1's "and on nothing else" for
`testflight-release.yml` now reads: on a push of a tag matching its filter, and on a manual
dispatch with no input. The plan refuses a run that a workflow's own token started.

- A14 (R1) now holds the release lane's `on` to exactly the tag push and the input-free dispatch;
  the internal lane's assertion is unchanged.
- A5's refused dispatch moves from the tag's own ref to a branch named like it, `refs/heads/v0.2.0`,
  with the new message (ADR-419 D6); no assertion is removed.
- The second path's criteria are SPEC-405 A1 to A7, and its rows are S40500 to S40515.
