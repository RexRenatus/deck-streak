---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Internal TestFlight builds come from a dispatch on dev, numbered by dev's first-parent count

## Context and Problem Statement

The owner chose that internal TestFlight builds run throughout the campaign, not only at its end:
the lane starts right after the Swift harness, and every later iPhone and iPad delivery puts a
build on internal TestFlight with the owner as the only tester (SPEC-334 R17). `RELEASING.md` says
nothing deploys from a branch, and CHARTER 3 says binaries reach the host only from a SemVer tag
on `main`. A TestFlight upload installs nothing on the host: it publishes an artifact to the
owner's own devices, as ADR-335 records. Two things are left to decide: what starts a build, and
what number each build carries. App Store Connect refuses an upload whose build number is not
unique, and expects a higher one within the same marketing version.

## Decision Drivers

- Builds run throughout the campaign, so the trigger cannot wait for a release.
- A build exists only when the owner wants one on a device; nothing uploads unasked.
- The upload credential is never exposed to code that has not reached `dev`.
- The build number is unique and rises with every build, and it can be derived again from the tree
  alone.
- Two iPhone and iPad deliveries in flight at once never collide on a number.
- A build from `dev` syncs to the staging sync user, never to the owner's own data (REL-01). Real
  data reaches a device only from a SemVer tag on `main`, and only after the real-data gates clear:
  both owner rulings on `dev`, and the sync cutover of ADR-340.

## Considered Options (the alternatives it was chosen against)

### The trigger

- A manual dispatch of the build workflow on `dev`, run after each iPhone and iPad delivery lands, uploading under the dev app id — chosen because it runs throughout the campaign, builds only reviewed code that has reached `dev`, and uploads only when someone asks for a build.
- A SemVer tag on `main`, as host binaries are released — rejected because it cannot run throughout the campaign: `main` moves only by a release, and the owner chose builds after every iPhone and iPad delivery.
- An automatic build on every push to `dev` — rejected because it uploads on merges nobody asked to install, and spends the credential and a macOS run on every merge.
- A build on every pull request — rejected because it builds code that has not reached `dev`, and would expose the upload credential to pull-request runs.

### The build number

- `git rev-list --count --first-parent HEAD` on `dev`, a plain integer, from a full-history checkout — chosen because `dev` advances only by merged pull requests and refuses a force-push (CHARTER 21, ADR-017), so the count rises by at least one with every merge and can be derived again from the commit alone.
- The CI run number — rejected because it is CI state, not the tree's: it restarts when the workflow is renamed or recreated, and a re-run of the same run reuses it.
- A timestamp — rejected because it cannot be derived again from the commit.
- A number kept in a file and raised by hand — rejected because two deliveries in flight at once raise it to the same value and collide.
- The release tag — rejected because `dev` carries no tag.

## Decision Outcome

Proposed option: a manual dispatch on `dev`, numbered by `dev`'s first-parent count.

- **The trigger.** The iPhone and iPad build workflow runs on `workflow_dispatch` only, and refuses
  any ref other than `dev`. Each iPhone and iPad delivery's hand-off asks for one dispatch after it
  lands; the owner or the orchestrator runs it. No push, pull request or schedule starts it.
- **The app id.** Builds from `dev` use a dev app id, distinct from the release app id, so a dev
  build never replaces a release build on a device.
- **The tester.** The build goes to internal TestFlight only, with the owner as the only tester.
  No external group, no public link and no App Store submission.
- **The sync user.** A dev build is configured for the staging sync user, which holds a scrubbed
  copy (ADR-340). Real data reaches a device only from a release build, after the real-data gates
  clear.
- **The build number.** The workflow checks out full history, refuses a shallow clone, and sets
  `CFBundleVersion` to `git rev-list --count --first-parent HEAD`. The marketing version
  (`CFBundleShortVersionString`) is the workspace version. Dispatching the same commit twice yields
  the same number, which App Store Connect refuses as already uploaded: the right outcome, since
  that build exists.
- **Release builds.** A later release builds from a SemVer tag on `main` under the release app id,
  numbered by `main`'s own first-parent count; its own ADR decides its lane.
- **The credential.** The App Store Connect upload credential and the signing material are the
  owner's to place as repository secrets, available to this workflow only; they never enter the
  tree. Until the owner places them, the workflow builds and stops before the upload, and says
  so.
- **Not a deploy.** The upload is artifact publication (`RELEASING.md`), so CHARTER 3, which
  governs host binaries, does not apply to it.

### Consequences

- Good, because the owner gets a build on a device after every iPhone and iPad delivery, from code
  already reviewed on `dev`.
- Good, because a build number can be derived again from its commit, and two deliveries in flight
  never collide.
- Bad, because a build needs a dispatch: a delivery that lands without one leaves the device a
  delivery behind until the next.
- Bad, because a shallow clone would give a small, colliding number, so the workflow must refuse
  one.

### Confirmation

- The workflow's own tests, in the delivery that adds it: it refuses a non-`dev` ref and a shallow
  checkout, and it computes the build number from first-parent history.
- The first upload's record: the build in internal TestFlight under the dev app id, its number
  equal to the count at its commit.

## What would make this wrong

- App Store Connect starts requiring build numbers unique across every marketing version, and a
  release build's count on `main` falls below a dev build's; the release lane would then use a
  separate range, decided in its own ADR.
- `dev` stops refusing a force-push, so first-parent history could shrink; the number would then
  need a source outside the tree.

## More Information

- SPEC-334 (R17; rows 1.1 and 1.5).
- ADR-335 (the client, and an upload as artifact publication), ADR-340 (the staging sync user),
  ADR-017 and ADR-034 (the branch model), `RELEASING.md`, CHARTER 3 and 21.
