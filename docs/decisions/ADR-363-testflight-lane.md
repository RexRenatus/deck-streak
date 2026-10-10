---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The TestFlight lane: two environments, two trigger files over one framework call, manual signing, a key that lives for one step, and a log that publishes nothing private

## Context and Problem Statement

ADR-344 settles the internal lane's trigger, app id, tester and build number: a manual dispatch on
`dev`, the dev app id, the owner the only tester, and `dev`'s first-parent count on a full-history
checkout, with a shallow clone refused. It names the release lane (a SemVer tag on `main`, the
release app id, `main`'s own first-parent count) and leaves that lane's shape to its own ADR. It
says the upload credential and the signing material are the owner's to place as repository
secrets, "available to this workflow only", and that until they are placed the workflow builds and
stops before the upload. The owner ruling on the lane's secrets (#668) supersedes ADR-344's
"repository secrets" for this lane: every workflow on every branch reads a repository secret, so
"this workflow only" cannot hold, and the credential lives instead as environment secrets in one
GitHub environment per lane. ADR-344 keeps its text; this ADR records the supersession. ADR-335
records that an upload to TestFlight is artifact publication, not a deploy. ADR-350, the Swift
harness's, hands this lane a seam: a shared scheme, the bundle id from one build setting whose
committed value is a reserved name written reversed (its first label is `invalid`), the number and
version from `CURRENT_PROJECT_VERSION` and `MARKETING_VERSION`, no team in the tree, an optional
include of an ignored local settings file, and signing off on the CI command line. ADR-355 runs
that one Apple job body from two callers, one on a change and one on every release tag.

Neither ADR settles nine things SPEC-352 needs: how a repository secret is kept to one workflow
and one ref; where the lane's jobs live and how they reach the framework their commit built; how
the app is signed; how the build is uploaded and where the key sits meanwhile; how the lane knows
the credential is placed; how a public run log stays free of private values; how the release lane
holds real data back until its gates clear; what an upload needs that a simulator build never
does; and how a build states which lane built it.

## Decision Drivers

- The credential is never exposed to code that has not reached `dev` (ADR-344), so no job a pull
  request can start, from a fork or from a branch of this repository, may read it.
- The key lives in the runner's temporary space for one step and is removed after it; it never
  enters the tree or the log.
- This repository is public, and so is every workflow log, every artifact and every run summary.
- Least privilege: the credential CI holds can upload a build and do nothing else.
- What uploads is what the commit built and what its own tests passed.
- Real data reaches a device only from a SemVer tag on `main`, after the real-data gates clear
  (SPEC-334 R17, ADR-344).
- Every refusal is provable on Linux before a credential exists; what only App Store Connect can
  answer is named as such.

## Considered Options (the alternatives it was chosen against)

### D1. Where the credential is readable

- Two GitHub environments, one per lane, admitted by file, job and step — chosen, because an
  environment secret reaches only a job that names the environment, on a ref its rule admits, so
  neither a pull request's run nor a branch's edited workflow can read it, and the test keeps
  every other workflow at zero reads. Each environment holds the lane's six parts as environment
  secrets: the internal environment admits the branch `dev` only, and the release environment
  admits tags matching `v*` only; the hardening test admits a secret read only in the lanes' `app`
  jobs, by file, job and step. This is the owner ruling on the lane's secrets (#668), which
  supersedes ADR-344's "repository secrets" for this lane; ADR-344 keeps its text.
- Plain repository secrets — rejected, because every workflow on every branch of this repository
  reads them, so a pull request from a branch here that edits a workflow could print them; ADR-344's
  "available to this workflow only" cannot hold.
- One environment for both lanes — rejected, because a dispatch on `dev` could then read the release
  lane's profile and app id, and the release lane's reviewer would gate every internal build.
- A required reviewer on the internal environment as well — rejected, because ADR-344 lets the
  orchestrator dispatch after each delivery, and a click per build would leave the device behind;
  the dispatch is already the ask.

### D2. Where the lane lives, and how it reaches the framework its commit built

- Two trigger files over `plan`, a reusable framework call and `app` — chosen, because a wrong ref
  is refused in seconds before a macOS minute is spent, the framework and the harness's simulator
  tests run at the same commit before anything is signed, and each file's triggers are exactly the
  ones ADR-344 and the release lane allow. The files are `testflight-internal.yml` (dispatch) and
  `testflight-release.yml` (tag push), each holding three jobs: `plan` on Linux (the ref, history
  and number checks), `framework` (the call `$/.github/workflows/xcframework.yml`, which already
  takes `workflow_call`, SPEC-344), and `app` on macOS, which downloads the framework artifact of
  its own run. `testflight-release.yml` sits beside `apple-on-tag.yml`, so a release tag runs the
  job body once for each caller.
- Extending `apple-on-tag.yml` with the lane's jobs — rejected, because SPEC-344 pins that file's
  shape: `test_ci_workflows.py`'s `test_a_call_job_is_a_local_call_or_pinned` holds the live call
  jobs to an exact list, `TheAppleBuildRunsFromOneBody` and `test_release_workflow.py`'s
  `test_the_tag_caller_runs_the_build_on_every_release_tag` each hold its jobs to the one `apple`
  call, and its header states that it builds and publishes nothing and passes no secret; a job
  that reads the credential there breaks all three, and a second run of the job body per tag is
  the cheaper cost.
- One file with both triggers — rejected, because ADR-344 says no push starts the internal
  workflow.
- A reusable lane workflow holding the `app` job, called by two thin files — rejected, because its
  trigger would be `workflow_call`, and the release-ops pack reads any job naming an environment as
  a deploy and refuses a deploy whose workflow runs on anything but a tag push, a release or a
  dispatch (`ro.deploy-from-tags`).
- Jobs added to `xcframework.yml` — rejected, because that workflow runs on pull requests through
  its change caller, so the job that reads the credential would sit in a workflow a pull request
  starts, kept out only by an `if`.
- A job added to `release.yml` — rejected, because that workflow's contract is two jobs, the audit
  and the release, and no secret, and a TestFlight failure would then fail a host release.
- Rebuilding the framework inside the `app` job — rejected, because a second copy of the recipe
  drifts, and the build would upload without the harness's tests at its commit.
- Downloading another run's framework — rejected, because its commit can differ from the one being
  signed.

**Amendment (ruling 461): the internal lane's file carries both triggers.** The internal lane could
not run: GitHub starts a `workflow_dispatch` workflow only from a file on the default branch, the
default branch stays `main`, and `main` does not hold the lane's file. The reason the bullet above
rejected one file with both triggers, ADR-344's "no push starts it", is reversed for the internal
lane by ADR-344's own amendment line, and the release lane keeps its one trigger (SPEC-352 R22).

- A push to `dev` filtered to the app's inputs, beside the dispatch, in the one internal file — chosen, because it runs while `dev` is the only branch that moves, needs nothing from `main`, builds only reviewed code that has reached `dev`, and starts only when a merge changes a path the app is built from: every crate the XCFramework links, `ios/**`, the workspace manifest, the lockfile, the toolchain pin, and the two workflow files and the two scripts the lane runs.
- Flipping the default branch to `dev` — rejected, because `main` stays the protected, visible default branch.
- A dispatch-only lane — rejected, because it cannot run while `main` lacks the file.
- A `schedule` — rejected, because a scheduled workflow also runs only from the default branch.
- Cancelling a push run in progress when a newer push arrives — rejected, because SPEC-190 R9 holds that no workflow cancels a push run in progress, and a cancelled run can stop inside the upload; instead each event waits in a group of its own, `testflight-internal-${{ github.event_name }}-${{ github.ref }}`, with `cancel-in-progress: false` and `queue: single`, so a newer push replaces the push run still waiting and never stops one that is running.
- Keeping `queue: max` for the dispatch group alone — rejected, because GitHub's documentation gives `queue` no expression form, so one value covers both groups, and a push group that never replaced a waiting run would build every merge in turn rather than the newest; a newer dispatch now replaces a waiting dispatch too, which narrows ADR-292's "never replaces" for this lane's dispatches.

**The mid-upload case.** No run in progress is cancelled by a newer one, so a push run that has
begun its upload finishes it. A run cancelled by hand, or stopped by its job's timeout, while
`xcodebuild` uploads still runs its `clean` and `summary` steps, which run under `always()`: the
key and the job keychain are removed, and App Store Connect either holds the build, so a later run
of the same commit is refused as already uploaded (ADR-344), or holds none, so the next push's run
uploads a higher number. A waiting run that a newer one replaces never started, so it read no
credential and uploaded nothing. This is acceptable because nothing a cancel leaves behind is
unsafe, and nothing it loses cannot be rebuilt from `dev`. Cancelling push runs in progress would
add a loss this shape avoids: merges in quick succession would each cancel the build in flight, so
a steady run of merges could finish no build at all.

**The environment.** The `testflight-internal` environment admits deployments from the branch `dev`
only and names no required reviewer, so a push run on `dev` neither waits for a review nor is
refused, and a dispatch on any other ref is refused at the `app` job as before. The release-ops
pack's `ro.deploy-from-tags` row now reads the internal `app` job as a deploy started by a branch
push, as its `ro.deploy-tag-on-main` row already did (Consequences, below); an upload is artifact
publication, not a deploy (ADR-335).

### D3. How the app is signed

- Manual signing with a non-extractable job keychain — chosen, because it needs no credential that
  can mint certificates or profiles, the signature carries the target's entitlements into the
  archive, and the certificate's password never reaches an argument list. An Apple Distribution
  certificate the owner creates once is placed as a password-protected bundle; its password
  reaches `openssl pkcs12` on standard input, which re-wraps the bundle under a passphrase
  generated for the step, and the re-wrapped bundle is imported non-extractable
  (`security import -x -T /usr/bin/codesign`) into a keychain the job creates under its temporary
  directory. With an App Store profile per lane, the archive is signed at build time by that
  identity, the signing settings reaching the build through the ignored include file the
  harness's settings already name. The include is project-level, so it reaches every native
  target and never a package target, which refuses a manually named profile; the archive builds
  the scheme `Harness`'s `Harness` target alone.
- The certificate's password on `security import`'s argument list — rejected, because the owner
  ruling keeps every secret out of argv, and a process listing on the runner reads an argument
  list.
- An unencrypted `0600` PEM key imported without a passphrase — rejected, because the private
  key would then sit unencrypted on the runner's disk for the length of the step, where the re-wrap
  keeps it encrypted under a passphrase that lives only in the step.
- Cloud-managed signing through `-allowProvisioningUpdates` and the upload key — rejected, because
  App Store Connect lets an API key use a cloud-managed distribution certificate only with the Admin
  role, the team's most powerful credential, which would then sit in CI.
- Automatic signing with a lower-role key — rejected, because on an ephemeral runner automatic
  signing creates a development certificate per run until the team reaches its certificate limit.
- Archiving unsigned and signing only at export — rejected, because export keeps the entitlements of
  the archive's own signature, so an unsigned archive loses push and associated domains when a later
  delivery adds them.
- A certificate store in a separate encrypted repository, through a third-party tool — rejected,
  because it adds a second store, a second toolchain and a second supply chain for one certificate.

### D4. How the build is uploaded, and where the key sits

- `xcodebuild -exportArchive` with a Developer-role key that lives for one step — chosen, because
  it is the platform's own upload path, the Developer role can upload a build and nothing beyond
  it, and the key exists for one step that runs no project build phase. The export options are
  written under the runner's temporary directory (method `app-store-connect`, manual signing,
  destination `upload`, the build number untouched, internal testing only), authenticated by a
  Team key with the Developer role; the upload step alone writes the key, as a `0600` file in a
  fresh `0700` directory, and removes it when the step ends, pass or fail.
- The standalone upload tool — rejected, because its upload command is deprecated, and its
  rewritten form on the current toolchain has been reported failing on API keys and choosing the
  wrong app on accounts with several.
- A third-party release tool's TestFlight action — rejected, because it brings a Ruby toolchain into
  the job that holds the key, and it manages testers, which needs the App Manager role.
- The key in a job-level environment variable — rejected, because every step, the build's own
  included, could read it.

### D5. How the lane knows the credential is placed

- A preflight step that reads each part only as a presence boolean — chosen, because ADR-344's
  "builds and stops before the upload" becomes a green run with a plain reason, and a half-placed
  credential fails at once by name rather than deep inside signing. It answers `all` (sign and
  upload), `none` (build unsigned, say it stopped, succeed) or fails naming the absent roles.
- Attempting the upload and letting it fail — rejected, because every dispatch would read red until
  the owner acts, and a partial placement would surface as an opaque signing error.
- A repository variable the owner flips — rejected, because it is a second fact that can disagree
  with the secrets it describes.

### D6. How a public run log stays private

- Masks before the first tool, tool output in files, and no artifact — chosen, because the
  platform masks only what it is told, the signing identity's name carries a person's name, and a
  profile is the one source of the team and the app id that cannot disagree with itself. The team
  and the app id are read from the placed profile, never placed apart from it; the identity's
  name, the profile's name and UUID, the team, the app id, the keychain password and the re-wrap
  passphrase are masked with `::add-mask::` before the first tool runs; every Apple tool writes
  its full output to a file under the temporary directory that is never uploaded, and the step
  prints only its error and warning lines with each private value replaced; no step traces its
  commands, and the lane uploads no artifact.
- The team and the app ids as repository variables — rejected, because variables are not masked.
- Uploading the tool logs, the archive or the signed package as artifacts — rejected, because an
  artifact of a public repository is downloadable by anyone signed in, and each holds the team and
  the app id.
- Printing the tools' output as it comes — rejected, because it publishes the identity's name and
  the profile.

### D7. How the release lane holds real data back

- A release environment the owner creates after the real-data gates — chosen, because no tag
  pushed by anyone uploads a build that syncs real data without the owner's click on that run,
  and the gate is a repository setting no pull request can edit. The owner is its required
  reviewer, it admits tags matching `v*` only with no administrator bypass, and its secrets are
  placed only after the real-data gates clear (both owner rulings on `dev`, and the sync
  cutover); until then a tag's run builds and stops before the upload.
- Trusting the tag alone — rejected, because a tag pushed by an agent would put a real-data build
  on a device unasked.
- A flag in the tree — rejected, because a pull request could flip it.

### D8. What an upload needs that a simulator build does not

- A launch screen, the iPad orientations and an icon generated at build time — chosen, because App
  Store Connect refuses an upload without them, a reviewer reads the generator rather than
  trusting an image, and the repository's scrubber refuses a committed binary. The app target's
  property list already declares the launch screen and the four iPad orientations; its asset
  catalog's icon set lists one 1024-pixel opaque icon that a committed standard-library generator
  writes in CI, before the project is generated, into a path git ignores; a test decodes what the
  generator writes and compares its pixels.
- An icon file committed with no generator — rejected, because a reviewer cannot read what it is.
- The generated icon committed beside its generator — rejected, because the scrubber refuses any
  binary in the tree and in its history, and an admission for one image would weaken that rule.
- Leaving them to the app shell — rejected, because the lane's first upload is the harness, and it
  would be refused.

### D9. How a build states which lane built it

- A build setting, `DS_LANE`, passed on the command line only — chosen, because a build's lane is
  then a fact the lane states and its summary names, while no source reads it, so SPEC-347 R14's
  "The app reads no other configuration" holds and the setting is inert for the harness. The
  value is `internal` or `release`. The app's own sync settings are SPEC-347 R14's, rendered when
  the lane moves to the app's scheme (#625), and a dev or internal build carries no staging sync
  credential: it is entered at login and kept in the Keychain (ADR-335, ADR-344).
- Inferring the lane from the bundle id inside the app — rejected, because the app would then hold
  a private value to compare against.
- A staging sync credential built into dev and internal builds — rejected, because a build is an
  artifact anyone holding it can read, and the credential's home is the Keychain after a login.

## Decision Outcome

Proposed options: D1 two environments with branch and tag rules and a test-held admission; D2 two
trigger files over `plan`, a reusable framework call and `app`; D3 manual signing with a
non-extractable job keychain and app-target settings; D4 the platform's export-and-upload with a
Developer-role key that lives for one step; D5 a presence preflight; D6 masks, files and no
artifacts, with the team and app id read from the profile; D7 a release environment the owner
creates after the real-data gates, with the owner as reviewer; D8 a launch screen, iPad orientations
and a generated icon; D9 a lane build setting no source reads.

### Consequences

- Good, because nothing a pull request starts can read the credential, and the test refuses every
  shape that would let it.
- Good, because the key exists in one step that runs no project code, and is removed by that step
  and again by an `always()` step.
- Good, because the owner places six parts, not eight: the team and the app id come from the
  profile.
- Good, because a dispatch before the owner acts is green and says plainly why it stopped.
- Bad, because a dispatch reruns the framework and the harness's simulator tests on macOS before it
  archives, and a release tag runs that job body twice, once for `apple-on-tag.yml` and once for
  this lane.
- Bad, because the certificate and the profiles expire, and the owner renews and re-places them.
- Bad, because a capability a later delivery adds re-issues both lanes' profiles through the owner.
- Bad, because a failed signing step prints only its error lines; the full log stays on the
  runner, which is discarded.
- Bad, because the release-ops pack reads the internal `app` job as a deploy by its environment,
  and its `ro.deploy-tag-on-main` row refuses it, since a build from `dev` cannot be on `main`.

### Confirmation

SPEC-352 A1 to A6 hold the plans; A7 to A13 hold the credential's path, its masks and its
removal with recording shims; A14 to A22 hold the workflows and the hardening admissions, with a
planted shape refused by name for each refusal; A23 and A24 hold D8. After the delivery lands, the
first dispatch records the stop, and the first upload records the build in internal TestFlight with
the number ADR-344 names.

## What would make this wrong

- App Store Connect refuses an upload by a Developer-role key through `xcodebuild`'s upload
  destination; the key would then be re-made with the App Manager role, a role change and no
  design change.
- The package targets accept a manually named profile; the include file would then be needless,
  not wrong.
- A runner image stops honouring a keychain made by the job for `codesign`; the import would then
  move to the default keychain's search list with the same removal.
- The runner's `openssl` cannot read the placed bundle's encryption; the owner would then re-export
  the bundle in a form it reads, with no design change.
- An environment's branch rule stops covering a reusable call's jobs; D2's `framework` job reads no
  secret, so D1 would hold.

## More Information

- SPEC-352; SPEC-334 (row 1.5; R17, R20).
- ADR-344 (internal builds; its "repository secrets" superseded for this lane by the owner ruling
  on the lane's secrets, #668), ADR-335 (the client, and an upload as artifact publication),
  ADR-350 (the Swift harness's seam), ADR-355 (one Apple job body, two callers), ADR-292 (queued
  runs), ADR-034 (the branch model).
- SPEC-347 R14 (what the app reads) and #625 (the move to the app's scheme).
- `RELEASING.md` (the release path), `.github/workflows/release.yml` (the ancestry step this lane
  repeats).

## Amendments

- #733 (SPEC-405, ADR-419): the release lane also takes an input-free manual dispatch at the tag's
  own ref, through the same guard step, plan and group. D2's "(tag push)" for
  `testflight-release.yml` and its sentence "the release lane keeps its one trigger" are superseded
  by ADR-419, the plan refuses a run that a workflow's own token started (ADR-419 D3), and D7's
  review is unchanged.
