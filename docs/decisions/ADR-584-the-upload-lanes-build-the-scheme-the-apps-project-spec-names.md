---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The upload lanes build the scheme the app's project spec names

## Context and Problem Statement

SPEC-566 asks that the internal and release upload lanes archive, export and upload the DeckStreak
app. Every fact below was read at `4e600dbb7565bd7cd6becb8f1c724b5e502c3d4a` by
`git show <sha>:<path> | grep -n`, and the app target's resources again at `36744c89`, after #1050.

- The lane script names one build, the harness's: `scripts/ios_lane.py:73-82` holds the tuple
  `-project ios/Harness.xcodeproj -scheme Harness -configuration Release -destination
  generic/platform=iOS`. The `sign` step archives it (`scripts/ios_lane.py:336-338`) into
  `app.xcarchive`, the `upload-to-testflight` step exports and uploads that archive
  (`scripts/ios_lane.py:341-379`), and `build-unsigned` builds it with signing off
  (`scripts/ios_lane.py:402-413`). Its tests pin both argument lists
  (`scripts/tests/test_ios_lane.py:711-730` and `:845-863`).
- Both lane files generate one project, the harness's: `.github/workflows/testflight-internal.yml:98-100`
  and `.github/workflows/testflight-release.yml:107-109`, the step "the project, generated", whose
  steps from the framework's download through it SPEC-352 A20 holds equal to the harness job's
  (`scripts/tests/test_testflight_workflows.py:382-422`). Each lane writes the harness's icon
  (`testflight-internal.yml:90-92`, `testflight-release.yml:99-101`).
- The app's project spec, `ios/app.yml`, names the project and the scheme `DeckStreak`
  (`ios/app.yml:5` and `:71-84`, whose `archive` takes `Release`); its target lists its sources
  and its privacy manifest (`ios/app.yml:30-33`); at `36744c89`, after #1050, it also lists the asset
  catalog `App/Assets.xcassets` (`:34`), and its settings name the app icon `AppIcon` (`:44`).
- The harness already carries what an upload needs: its spec lists its catalog and names its icon
  set (`ios/project.yml:32-34` and `:48`), its icon set lists one 1024-pixel image
  (`ios/Harness/Assets.xcassets/AppIcon.appiconset/Contents.json:1-14`), git ignores that image
  (`.gitignore:46`), and `scripts/ios_icon.py` writes it as an opaque square
  (`scripts/ios_icon.py:19-21` and `:30-40`).
- The app's settings already read the lane's signing: `ios/Config/App.xcconfig:22` includes the
  ignored `Signing.local.xcconfig` last, the file the `sign` step writes
  (`scripts/ios_lane.py:327-332`) with the settings `DS_APP_ID`, `DEVELOPMENT_TEAM`,
  `CODE_SIGN_STYLE`, `CODE_SIGN_IDENTITY` and `PROVISIONING_PROFILE_SPECIFIER`.
- `xcframework.yml`'s `harness` job, the one job body every app build runs, already generates and builds the app: its step "the app's
  tests, Debug, on the iPhone and then the iPad" opens with the app's generate line
  (`.github/workflows/xcframework.yml:481-486`), and its step "the app, archived unsigned for a
  device" archives the scheme `DeckStreak` for a device with signing off
  (`.github/workflows/xcframework.yml:538-548`).
- The internal lane's `on:` block starts a run on a push to `dev` that changes `ios/**`,
  `scripts/ios_lane.py`, `scripts/ios_icon.py` or its own file
  (`.github/workflows/testflight-internal.yml:10-25`), so this delivery's land starts one run.

What the tree cannot show is named, not assumed: whether the lane's credential is placed, which
app id its profile carries, what the store's records hold and what the store's processing accepts.

## Decision Drivers

- The upload ships the product, and the measurement harness keeps every job it serves today.
- One source names the scheme, and the lane script, both lane files and the tests read it.
- No binary enters the tree or its history.
- No signing setting, credential, trigger, dependency or tool changes.
- A step that holds a credential runs no more code than it does today.

## Decisions and what each was chosen against

### D1. The lane, measured

- Chosen: measure the lane from the tree at the base and name every step the tree cannot show as
  not measured, because the store's records and a run not yet made are not in the tree.
- Reading the lane's past runs or the store's records to fill the gaps: rejected, because those
  are the owner's and are not public, and a design resting on them could not be reviewed.

### D2. The scheme: one source, the app's project spec

- Chosen: the lane script reads `ios/app.yml`'s top-level `name` and builds the project
  `ios/<name>.xcodeproj` and the scheme `<name>` in both builds, and each lane's `app` job gains
  one step, "the app's project, generated", right after the copied harness steps, whose one line
  is `xcframework.yml`'s own app generate line, because then the spec the generator reads names the
  scheme once, and the lane script, both lane files and the tests all read that spec.
- A second literal tuple beside the harness's in the lane script: rejected, because a rename of
  the spec would leave the script archiving a scheme no generated project holds, and two spellings
  of one name can drift.
- The lane script running the project generator inside `sign` and `build-unsigned`: rejected,
  because the `sign` step's environment holds the certificate, its password and the profile, and
  it would then also run the generator.
- A second line in the step "the project, generated": rejected, because the lanes copy that step
  from the harness job, so it either reddens SPEC-352 A20 or changes the harness job (ADR-358 D10).
- Replacing the lanes' copy of the harness's steps with the app's: rejected, because it changes a
  kept check, A20, and the harness job's copied range, for a saving of minutes and no behaviour.
- A workflow input naming the scheme: rejected, because it adds an input surface to both lanes,
  and a trigger change is not this delivery's to make.

### D4. Production or test

- Chosen: production changes sit in `scripts/ios_lane.py` and the two lane files; test changes sit
  in `scripts/tests/test_ios_lane.py` and `scripts/tests/test_testflight_workflows.py`, because each
  rule then lives where it acts and is held where its readers already look. Byte-unchanged: every
  signing step and its names, every credential and its role words, the build numbering
  (`plan`), the export options and the upload, `clean`, `summary`, `ios/project.yml`,
  `ios/Harness/`, `ios/Config/Harness.xcconfig`, `ios/Config/App.xcconfig`,
  `ios/app.yml`, `ios/App/`, `scripts/ios_icon.py`, `.github/workflows/xcframework.yml` and the lanes' copied harness steps.
  No kept assertion is removed or loosened: the two pinned argument lists change what they expect,
  and the harness's placeholder assertion stays beside the app's new one.
- A test-side constant naming the app's scheme, with the script keeping its own literal:
  rejected, because the test would then hold the script to a copy, not to the spec.
- Editing the opening comment of `ios/Config/Harness.xcconfig`, whose lane sentence this change
  makes stale: rejected, because the harness's CI reads that file and the harness stays
  byte-unchanged; the SPEC-352 amendment supersedes the sentence instead.

### D5. Shape

- Chosen: one delivery for one builder, landing on `dev` after the release pull request and after
  the lane's legacy-retry change land, composing with the open runner-selection changes wherever they
  land first, and amending SPEC-352, ADR-363 and `docs/schematics/testflight-lane.md` insert-only,
  because the app the scheme selects already carries its catalog and icon set on `dev` (#1050).
- Waiting for a delivery of the app's icon first: rejected, because #1050 already put the app's
  catalog and icon set on `dev`, so this delivery's land uploads an app that carries its icon.
- A new schematic: rejected, because the lane's own schematic already draws the job graph this
  changes, and a second would draw it twice.

### D6. Tests, rows and pushes

- Chosen: one test per criterion, red at the base for its reason; A1 to A4 and A8 run on Linux over
  fixture checkouts and the tree, the app-project criterion sits in the workflow test module CI alone runs, so the
  delivery takes two pushes, the first holding every red; rows S56601 to S56605, S56611 and S56612
  hold the changed lines; formal methods do not apply, because no interleaving or pure algorithm changes.
- One push, with the app-project criterion recorded as mutation coverage: rejected, because its red would never be read,
  and a row's kill shows a test can fail, not that it failed before the change.
- A text check of the lane files from a box module: rejected, because it is a second reader of the
  lane files that reads less than the house reader the hardening tests use.

### D7. Not in this delivery

- Chosen: the store's records and settings, signing, the credentials, the release lane's other
  steps, the sync endpoint and user the app reads (SPEC-347 R14), the app's catalog and icon set (#1050), the
  app's screens, and every step only a later lane run can measure stay out, because each is
  another owner's act, another delivery's design, or a measurement after the land.
- Rendering the sync endpoint and user in the lane now: rejected, because it needs values the tree
  does not hold, under names that do not exist yet, and new names for the lane are the owner's.

## Consequences

- Good, because the upload lanes ship the app the review screens and the deck list live in.
- Good, because the harness's jobs, scheme, catalog, icon and settings are unchanged.
- Bad, because each lane still generates the harness's project it no longer builds, a few minutes
  per run, kept so that A20 holds unchanged.
- Bad, because an internal build's sync settings stay the committed placeholders until the lane
  renders them, so an internal build lists the decks on the device and does not sync.

## What would make this wrong

- The store needs an icon field beyond the icon set dev holds for the app (#1050): the first
  signed internal run's upload step reads it.
- A scheme named other than the spec's `name`: A3 reads the spec's scheme block and fails.
- The profile's app id belongs to a store record that refuses the app: the first signed internal
  run's upload step reads it, and the store's records are the owner's.

## More Information

- SPEC-566 states the requirements and the criteria; `docs/red-first/SPEC-566.md` records the reds.
- SPEC-352 and ADR-363 (the lane), ADR-358 D10 (where the app's project is generated), SPEC-347 R14
  (what the app reads), ADR-350 (the harness), ADR-355 (one job body for the app builds).
- #634 holds the lane.
