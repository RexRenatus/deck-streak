# Schematic: the TestFlight lane, its job graph, and the credential's path from the environment to one step

Companion to SPEC-352 and ADR-363. Everything here is public: the credential's parts are named
by role, never by secret name, and no value, identifier or host appears.

## The components

| component | where | holds |
|---|---|---|
| internal trigger | `.github/workflows/testflight-internal.yml` | `workflow_dispatch` only; jobs `plan`, `framework`, `app`; the internal environment on `app` |
| internal trigger, amended (SPEC-352 R22) | `.github/workflows/testflight-internal.yml` | also a push to `dev` whose `paths` name every crate the XCFramework links, `ios/**`, `Cargo.lock`, `Cargo.toml`, `rust-toolchain.toml`, the two workflow files the lane runs and the two scripts its steps call; no other trigger |
| release trigger | `.github/workflows/testflight-release.yml` | a push of a SemVer tag only; the same jobs; the release environment on `app` |
| release trigger, amended (SPEC-405) | `.github/workflows/testflight-release.yml` | also a manual dispatch at the tag's own ref, with no input, through the same guard step, plan and group; the plan refuses a run a workflow's own token started (ADR-419) |
| framework call | `.github/workflows/xcframework.yml` | already takes `workflow_call` (SPEC-344); its jobs (the XCFramework, the harness, the wire sweep) run inside the caller's run; its `harness` job gains the icon step |
| lane script | `scripts/ios_lane.py` | `plan`, `preflight`, `build-unsigned`, `sign`, `upload-to-testflight`, `clean`, `summary`; standard library only |
| icon generator | `scripts/ios_icon.py` | `write <path>`: the 1024-pixel opaque icon the asset catalog lists, written in CI before the project is generated, into a path git ignores; never committed |
| internal environment | repository setting (the owner's) | the six parts; deployment admitted from the branch `dev` only |
| release environment | repository setting (the owner's, after the real-data gates) | the six parts; tags `v*` only; the owner as required reviewer; no administrator bypass |
| hardening test | `scripts/tests/test_ci_workflows.py` | one admission table of the lanes' reads by file, job and step; the macOS image admitted by file and job to the two `app` jobs; the two `framework` calls in the exact call list; refuses every other shape by name |

The six parts, by role: the upload key, its key id, its issuer id, the distribution certificate
(a password-protected bundle), the certificate's password, and the lane's App Store profile. The
team and the app id are read from the profile.

## The job graph

```
 internal: workflow_dispatch (refs/heads/dev)         release: push of tag vX.Y.Z
 internal (R22): or a push to dev that changes a filtered path
                     |                                            |
                     v                                            v
 +----------------------------------------------------------------------------------+
 | plan       versioned Ubuntu image; no environment; no secret                      |
 |            checkout, full history, no persisted token                             |
 |            [release only] tag is SemVer, annotated, its commit on main             |
 |            ios_lane.py plan --lane L: refuse event, ref, shallow, version          |
 |            outputs: lane, marketing version, build number (first-parent count)     |
 +----------------------------------------------------------------------------------+
                     | needs (a refusal here starts no macOS job)
                     v
 +----------------------------------------------------------------------------------+
 | framework  uses: $/.github/workflows/xcframework.yml   (no secrets passed)        |
 |            xcframework -> harness (simulator tests) -> harness-wire                |
 |            artifact `xcframework` of THIS run                                      |
 +----------------------------------------------------------------------------------+
                     | needs
                     v
 +----------------------------------------------------------------------------------+
 | app        admitted macOS image; environment = the lane's; contents: read          |
 |   1 checkout (no persisted token)                                                 |
 |   2 download this run's `xcframework`; place it; write the icon; generate project |
 |   3 preflight          reads six presence booleans  -> all | none | fail          |
 |   4 build-unsigned     if none: Release, generic iOS device, signing off,          |
 |                        placeholder app id (`invalid.`) -> "stopped before upload"  |
 |   5 sign               if all: certificate, password, profile                      |
 |   6 upload-to-testflight  if all: key, key id, issuer                              |
 |   7 clean              always()                                                    |
 |   8 summary            always(): lane, sha, version, number, placed, outcome       |
 +----------------------------------------------------------------------------------+
```

Each lane file queues its runs in one group per lane and ref (`testflight-internal-` or
`testflight-release-` followed by `github.ref`) that never cancels a run in progress and never
replaces a waiting one, up to `queue: max`'s hundred waiting runs (ADR-292). No job restores or
saves a cache, and no job uploads an artifact of its own.

The internal lane's queue is amended by SPEC-352 R22: its group is `testflight-internal-`, then
`github.event_name`, then `-` and `github.ref`, with `cancel-in-progress: false` and
`queue: single`, so a dispatch and a push never share a group, no run in progress is cancelled,
and a newer run replaces the waiting run of its own event. SPEC-352 A25 holds the push filter's
crates to the XCFramework's build closure, read from the workspace's manifests.

## The credential's path, and nowhere else

```
 the owner's environment secret (one per part, per lane)
        |
        |  GitHub releases it only to a job that names the environment,
        |  on a ref the environment admits (dev | v* tags)
        v
 app job ------------------------------------------------------------------------------
   step 3 preflight      ${{ secrets.<part> != '' }} x6  -> booleans only, never a value
   step 5 sign           env: certificate, password, profile
        |  ios_lane.py sign:
        |    decode the profile in memory -> team, app id, name, UUID
        |    ::add-mask:: each identifier, the keychain password and the re-wrap passphrase
        |    check: App Store profile, unexpired, for this certificate, app id's first
        |           label not `invalid` (the placeholder's)
        |    write certificate -> TMP/lane/original.p12 (0600); password -> openssl pkcs12 on stdin,
        |    re-wrapped under a run-time passphrase -> security import -x -T codesign
        |    -> every certificate file REMOVED
        |    keychain TMP/lane/lane.keychain-db (generated password, search list set)
        |    profile -> the user's profiles directory (removed by step 7)
        |    include file ios/Config/<ignored include> (project-level: every native
        |    target, never a package target; removed by step 7)
        |    drop every secret variable from the environment handed to any child
        |    xcodebuild archive  -> TMP/lane/app.xcarchive   (output -> TMP/lane/xcodebuild-archive.log)
        v
   step 6 upload-to-testflight   env: key, key id, issuer
        |  ios_lane.py upload-to-testflight:
        |    mkdir TMP/lane/key-XXXX (0700); write AuthKey file (0600)
        |    write TMP/lane/export-options.plist (app-store-connect, manual, upload, internal only)
        |    xcodebuild -exportArchive ... -authenticationKeyPath <that file>
        |                (output -> TMP/lane/xcodebuild-exportArchive.log; only error lines printed, redacted)
        |    on ANY exit: remove TMP/lane/key-XXXX
        v
   step 7 clean (always)  remove key dir, profile, include file, keychain; restore search list
```

`TMP` is the runner's temporary directory. Nothing in the path is written to the checkout except
the ignored include file, which step 7 removes; nothing is uploaded as an artifact; the runner is
discarded when the job ends.

| part | readable by | lives | removed by |
|---|---|---|---|
| upload key | step 6 only | one `0600` file in a `0700` directory, during step 6 | step 6 on any exit; step 7 |
| key id, issuer id | steps 3 and 6 | step 6's arguments to `xcodebuild` | the step's end |
| certificate | steps 3 and 5 | one file until `security import -x`, then a non-extractable key in the job keychain | step 5 after import (the file); step 7 (the keychain) |
| certificate password | steps 3 and 5 | `openssl pkcs12`'s standard input only; `security import` gets the run-time passphrase | the step's end |
| profile | steps 3 and 5 | the user's profiles directory | step 7 |
| team, app id, identity, profile name and UUID | steps 5 to 8, masked | the include file, the export options, tool arguments | step 7 (the include file); masked in every log line |

## A run's outcomes

| state | when | `app` result | summary says |
|---|---|---|---|
| refused | wrong event or ref, shallow, a bad tag, a version mismatch | `plan` fails; no macOS job runs | no summary: the app job never runs; the plan step's error line names the reason |
| stopped | no part placed | success | "stopped before the upload: the credential is not placed" |
| half-placed | some parts placed | failure at step 3 | credential: half placed; outcome: stopped at the preflight (its error line names the absent parts by role) |
| profile refused | a development, expired or foreign profile, or one whose app id's first label is `invalid` | failure at step 5, before the archive | outcome: not uploaded: the signing or the upload failed (the step's error lines name the cause, redacted) |
| awaiting the owner | release lane, parts placed | waits at the environment's review | (nothing until approved) |
| uploaded | all placed, signing and upload succeed | success | "uploaded", with the number |
| upload refused | App Store Connect refuses | failure at step 6 | outcome: not uploaded: the signing or the upload failed (the step's error lines name the cause, redacted) |
