# SPEC-566: the upload lanes archive and upload the DeckStreak app

- **Issue:** #634 (the lane). **Context(s):** `scripts/` and the two upload lane files under CI.
- **Decided by:** ADR-584 (the lane measured, the scheme's one source,
  production or test, the shape, the tests and rows, what is left out), resting on ADR-363 (the
  lane), ADR-358 D10 (where the app's project is generated), ADR-350 (the harness) and ADR-355 (one
  job body for the app builds).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-566.md`. **Mutation band:** `S56600-S56699`.
- **Base:** designed at `dev` `4e600dbb7565bd7cd6becb8f1c724b5e502c3d4a`; built from `dev` after the
  lane's legacy-retry change lands.

## 1. The problem, measured

Each fact was read at the base by `git show 4e600dbb:<path> | grep -n` and line ranges; the app target's resources were read again the same way at `36744c89`, after #1050.

| fact | where | reading |
|---|---|---|
| the lane's one build | `scripts/ios_lane.py:73-82` | `-project ios/Harness.xcodeproj -scheme Harness -configuration Release -destination generic/platform=iOS` |
| the signed archive | `scripts/ios_lane.py:336-338` | `archive` over that tuple into `app.xcarchive` |
| the export and upload | `scripts/ios_lane.py:341-379` | export options for the store, then the export of `app.xcarchive` with the upload key |
| the unsigned build | `scripts/ios_lane.py:402-413` | `build` over that tuple with signing off |
| the tests' pins | `scripts/tests/test_ios_lane.py:711-730`, `:845-863` | both argument lists name `ios/Harness.xcodeproj` and `Harness` |
| the lanes' one project | `.github/workflows/testflight-internal.yml:98-100`, `.github/workflows/testflight-release.yml:107-109` | the step "the project, generated" generates `ios/project.yml` alone |
| the lanes' icon | `.github/workflows/testflight-internal.yml:90-92`, `.github/workflows/testflight-release.yml:99-101` | the harness's icon is written; the app's is not |
| the app's project and scheme | `ios/app.yml:5`, `:71-84` | `name: DeckStreak`; the scheme `DeckStreak` archives `Release` |
| the app target's resources | `ios/app.yml:30-34`, `:40-46`, at `36744c89` | its sources, its privacy manifest and its asset catalog `App/Assets.xcassets` (`:34`); its settings name the app icon `AppIcon` (`:44`), both from #1050 |
| the harness's icon pattern | `ios/project.yml:32-34`, `:48`; `.gitignore:46`; `scripts/ios_icon.py:19-40` | a committed catalog, an ignored image, an icon name, an opaque square written by a script |
| the app's signing seam | `ios/Config/App.xcconfig:22`; `scripts/ios_lane.py:327-332` | the app's settings include the lane's ignored signing include last |
| the app built elsewhere | `.github/workflows/xcframework.yml:481-486`, `:538-548` | `xcframework.yml`'s `harness` job generates the app's project and archives the scheme `DeckStreak` unsigned |
| the land starts a run | `.github/workflows/testflight-internal.yml:10-25` | a push to `dev` changing `ios/**` or `scripts/ios_lane.py` starts the internal lane |

So the upload lanes archive and upload the measurement harness, not the app.

## 2. Requirements

R1. **The lane builds the app.** The `sign` step archives, and the `build-unsigned` step builds, the
    project `ios/<name>.xcodeproj` and the scheme `<name>`, for `Release` and a generic iOS device,
    where `<name>` is the top-level `name` of `ios/app.yml`, the app's project spec. The export and
    the upload take that archive as they take it today.
R2. **One source, refused when it names nothing.** A spec that is absent, or holds no top-level
    `name:` line naming one word, is refused by both steps with the one line
    `the app's project spec ios/app.yml names no project`, before any tool runs and before the
    lane's directory is made.
R3. **The lanes generate the app's project.** Each lane's `app` job holds one step,
    "the app's project, generated", right after "the project, generated" and right before
    "the credential, by presence"; its one line is `xcframework.yml`'s own app generate line, and
    no other step of the lane names the app's spec. The steps the lanes copy from the harness job
    are unchanged.
R6. **The harness keeps every job.** `ios/project.yml`, `ios/Harness/`,
    `ios/Config/Harness.xcconfig`, `scripts/ios_icon.py`, `.github/workflows/xcframework.yml` and
    the lanes' copied harness steps are byte-unchanged, and SPEC-352 A20, A23 and A24 pass
    unchanged.
R7. **Signing is unchanged.** The include file, its five settings, the export options, the
    credential's six role words, the build numbering, the upload, `clean` and `summary` are
    byte-unchanged; no secret name, trigger, dependency or tool is added.

## 3. Acceptance criteria of SPEC-566

| id | criterion | decided by |
|---|---|---|
| A1 | with a fixture checkout whose app spec names a project drawn at run time, the signed archive's argument list names that project under `ios/` and that scheme, `Release`, a generic device, the archive path and the plan's three settings, and no signing setting | `test_ios_lane.py`, `test_signing_imports_the_certificate_non_extractable_into_a_job_keychain` (SPEC-352 A9, amended) |
| A2 | with that fixture, the unsigned build's argument list names the same project and scheme with signing off, the app's committed app id is a placeholder, and the summary says it stopped before the upload | `test_ios_lane.py`, `test_the_unsigned_build_stops_before_the_upload_and_says_so` (SPEC-352 A13, amended) |
| A3 | with the tree's own `ios/app.yml` in the fixture, both builds name `ios/DeckStreak.xcodeproj` and `DeckStreak`, and that spec's scheme of its name archives `Release` | `test_ios_lane.py`, `test_the_lane_builds_the_deckstreak_app_from_the_trees_project_spec` |
| A4 | an absent spec, a spec whose only `name:` is indented and a spec whose name is quoted are each refused by both steps with the one line, with no tool run, no lane directory and no include file | `test_ios_lane.py`, `test_a_project_spec_that_names_no_project_is_refused_before_any_tool_runs` |
| A7 | each lane's `app` job holds one step "the app's project, generated", after "the project, generated" and before the credential's step, whose run is `xcframework.yml`'s app generate line, and no other step names the app's spec | `test_testflight_workflows.py`, `test_each_app_job_generates_the_apps_project_after_the_harness_steps` |
| A8 | the harness still declares what an upload needs, unchanged: the control | `test_ios_icon.py`, `test_the_app_declares_what_an_upload_requires` (SPEC-352 A23) |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_signing_imports_the_certificate_non_extractable_into_a_job_keychain
A2: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_unsigned_build_stops_before_the_upload_and_says_so
A3: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_lane_builds_the_deckstreak_app_from_the_trees_project_spec
A4: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_a_project_spec_that_names_no_project_is_refused_before_any_tool_runs
A7: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py -k test_each_app_job_generates_the_apps_project_after_the_harness_steps
A8: python3 -m unittest discover -s scripts/tests -p test_ios_icon.py -k test_the_app_declares_what_an_upload_requires
```

A1 to A4 and A8 run on Linux over fixture checkouts and the tree, with recording shims for the
build and signing tools and no credential. The app-project criterion sits in a workflow test module that CI alone runs,
and is read there by name. Section 7 holds what only a lane run can measure.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/ios_lane.py` | scripts | changed: `SPEC`, `NAME` and `app()` replace the harness's tuple; `sign` and `build-unsigned` build `app()` (R1, R2) |
| `.github/workflows/testflight-internal.yml` | CI | changed: one step, "the app's project, generated" (R3) |
| `.github/workflows/testflight-release.yml` | CI | changed: the same step (R3) |
| `scripts/tests/test_ios_lane.py` | tests | changed: the fixture app spec; A1 and A2 amended; A3 and A4 added |
| `scripts/tests/test_testflight_workflows.py` | tests | changed: the app-project criterion's test added |
| `scripts/mutation-rows.d/S56600-S56699.json` | mutation | added: rows S56601 to S56605, S56611 and S56612 |
| `docs/specs/SPEC-566-the-upload-lanes-archive-and-upload-the-deckstreak-app.md` | docs | added |
| `docs/decisions/ADR-584-the-upload-lanes-build-the-scheme-the-apps-project-spec-names.md` | docs | added |
| `docs/red-first/SPEC-566.md` | docs | added |
| `docs/specs/SPEC-352-testflight-lane.md` | docs | changed: an appended amendment, insert-only |
| `docs/decisions/ADR-363-testflight-lane.md` | docs | changed: an appended amendment, insert-only |
| `docs/schematics/testflight-lane.md` | docs | changed: an appended section, insert-only |
| `changelog.d/app-archive-scheme-566.md` | docs | added |

## 5. What this does NOT cover

- It changes no signing setting, credential, role word, profile, build number or store step, and
  creates no store record: those are the owner's and the lane's as they stand (#634).
- It renders no sync endpoint or sync user into a lane build: an internal build carries the
  committed placeholders and lists the decks on the device without syncing until a later change
  renders SPEC-347 R14's two settings (#634).
- It changes no other step of the release lane and uploads no release build: the first one waits
  for the real-data gates (#636).
- It changes no icon, mark or colour: the app's asset catalog and its brand icon set stand as dev
  holds them from #1050 (#1042).
- It changes no screen and no Swift source of the app: the lane ships the app as `ios/App/` holds
  it at the cut (#634).
- It stops no lane from generating the harness's project, which the lanes still copy from the
  harness job and no longer build (#634).
- It measures nothing a lane run alone can measure: section 7's steps are read after the land
  (#634).

## 6. Risks

- **The store's processing.** An upload missing an icon field the store needs is refused at the
  upload step of the first signed internal run, which names the cause.
- **A sibling's lines.** The lane's legacy-retry change and a lint pin edit `scripts/ios_lane.py`
  and its tests near R1's lines; the runner-selection changes edit both lane files within the `app`
  jobs. The build re-measures every find at its cut and composes with each.
- **A census-read test module.** The workflow hardening module reads every test module's dynamic
  sites: the new tests reuse the modules' existing helpers and add no site.

## 7. Measured after the delivery lands

The land changes `scripts/ios_lane.py` and the internal lane's own file, so it starts one
internal lane run. Read by step name: "the app's project, generated", "the credential, by presence",
then either "the app, unsigned" (no credential placed: the scheme selection is measured, the upload
is not) or "the archive, signed" and "the upload" (the app's archive and its upload), then "the
summary". Each reading is recorded in `docs/red-first/SPEC-566.md` with its
run and job ids.
