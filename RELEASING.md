# Releasing

A release is a pull request from `dev` into `main`, an annotated SemVer tag on `main`, a release
built by CI from that tag, and a deploy of that tag run from the maintainer's machine. Nothing
deploys from a branch, and nothing deploys from CI (ADR-010, ADR-017).

Every change reaches `dev` by a pull request, and `main` only by a release pull request from `dev`.
Nobody pushes to `dev` or `main` directly; the rulesets refuse it, and `base-is-dev` fails a pull
request into `main` from any other branch (ADR-017, ADR-034).

## 1. Prepare the release on dev

The preparation is a pull request into `dev`, from a branch such as `chore/release-X.Y.Z`:

1. Choose the version by SemVer: a breaking change bumps the major, a feature the minor, a fix the
   patch. Set it in the workspace `Cargo.toml` and in every `package.json` that declares one.
2. Compile the fragments under `changelog.d/` into a new `## [X.Y.Z] - YYYY-MM-DD` section of
   `CHANGELOG.md`, and delete them. The edit to `CHANGELOG.md` is what satisfies `fragment`.
3. Run `bash scripts/check.sh` and `bash scripts/box-packs.sh` green on the release head, then
   merge the pull request once its checks are green.

## 2. The release pull request

Open the release pull request from `dev` into `main`, and merge it with a merge commit once its
checks are green. A squash or a rebase would replay the commits of `dev` on every later release.

The release pull request's required `ci` is its own run, and the push run on `dev` reports as
`ci (push)`.

`main`'s ruleset does not require an up-to-date head. Only this repository's `dev` can reach
`main`, and GitHub keeps one open pull request per head and base, so `main` cannot move between the
release pull request's checks and its merge. Those checks run on the pull request's merge ref,
which already contains `main` (ADR-034). `dev`'s ruleset does require an up-to-date head, because
many pull requests land there.

Measured before `v0.1.0` was tagged (SPEC-034 R6, probe pull request #193):
- **The release pull request merges.** `dev` lacked `main`'s release merge commit (behind by 1,
  ahead by 3), and a release pull request from `dev` still read mergeable and `CLEAN`, not
  `BEHIND`.
- **Update-branch is refused.** GitHub's update-branch on that pull request answered HTTP 422,
  "Changes must be made through a pull request", and left `dev` unchanged. Nothing needs it.

```sh
gh pr create --base main --head dev --title "release: vX.Y.Z" --body-file release-notes.md
```

**A shard that never reported.** A hosted runner can be shut down in the middle of a mutation shard:
its log ends with "The runner has received a shutdown signal", and the shard's `always()` upload
step never runs. The release pull request's `mutation-verdict` counts every shard the plan promised
(SPEC-039 R18), so it names the missing shard, reads it VOID and fails. The remedy is "Re-run failed
jobs", which runs that shard again and the jobs that need it. Never re-run the whole workflow, which
repeats every shard of the release's run, and never merge while a shard is missing.

## 3. Tag it on main

```sh
git switch main && git pull --ff-only
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

The tag runs the release workflow (`.github/workflows/release.yml`). It checks that the tag is
SemVer and annotated and that its commit is on `main`, builds the release binary and the Mini App
once, attests the tarball's build provenance, and attaches the tarball and its `SHA256SUMS` to a
draft release that its last step publishes.

## 4. Deploy the tag

From the maintainer's machine, with the private deploy rail's configuration loaded:

```sh
bash deploy/deploy.sh vX.Y.Z
```

The script refuses unless the tag is an annotated SemVer tag whose commit is on `origin/main`
after a fetch (`git merge-base --is-ancestor vX.Y.Z origin/main`). It downloads the release, verifies the tarball's attestation
(`gh attestation verify --repo` this repository, `--signer-workflow` its release workflow) and its
digest against `SHA256SUMS`, and only then reaches the host: it unpacks beside the previous
releases, switches `current` with one `mv -T`, installs the units byte for byte, restarts them and
waits for the API's readiness. A release that does not become ready is switched back from, and the
deploy names the unit. The host keeps the current release and the two before it.
The first deploy, and any change to a unit, Caddy or the firewall, needs the owner's go.

## 5. Nothing is merged back into dev

`main` receives only release pull requests from `dev`, merged with a merge commit, so it holds
nothing `dev` lacks except those merge commits, and the next release pull request merges cleanly. A
pull request from `main` into `dev` would carry no change, and `fragment` would refuse it (ADR-034).

## 6. A hotfix

A hotfix is a pull request into `dev` like any other change, with its fragment. It is followed at
once by a patch release: steps 1 to 4 with the patch version. No branch is cut from `main`. If `dev`
holds work that must not ship yet, revert it on `dev` first (ADR-034).

## 7. Rollback

Roll back to the previous tag. The host keeps it beside the current release, so the rollback is a
switch and a restart with no download; a release the host no longer keeps is deployed anew, through
the same verification a deploy takes. The tag must be an annotated SemVer tag on `main`.

```sh
bash deploy/rollback.sh vX.Y.W
```

A release whose database migration removed or rewrote data cannot be rolled back by the binary
alone: ship a schema change as expand, then contract, and follow the data-migration runbook's
rollback for the database.
