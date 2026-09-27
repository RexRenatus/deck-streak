# Releasing

A release is a pull request from `dev` into `main`, an annotated SemVer tag on `main`, a release
built by CI from that tag, and a deploy of that tag run from the maintainer's machine. Nothing
deploys from a branch, and nothing deploys from CI (ADR-010, ADR-017).

## 1. Prepare the release on dev

1. Choose the version by SemVer: a breaking change bumps the major, a feature the minor, a fix the
   patch. Set it in the workspace `Cargo.toml` and in every `package.json`.
2. Compile the fragments under `changelog.d/` into a new `## [X.Y.Z] - YYYY-MM-DD` section of
   `CHANGELOG.md`, and delete them.
3. Run `bash scripts/check.sh` and `bash scripts/box-packs.sh` green on the release head.

## 2. The release pull request

Open the release pull request from `dev` into `main`, and merge it with a merge commit once its
checks are green. A squash or a rebase would replay the commits of `dev` on every later release.

```sh
gh pr create --base main --head dev --title "release: vX.Y.Z" --body-file release-notes.md
```

## 3. Tag it on main

```sh
git switch main && git pull --ff-only
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

The tag runs the release workflow, which checks that the tag is on `main`, builds the release
binary and the Mini App once, and publishes them on a draft release before publishing it.

## 4. Deploy the tag

From the maintainer's machine, with the private deploy rail's configuration loaded:

```sh
bash deploy/deploy.sh vX.Y.Z
```

The script proves the tag is on `main` (`git merge-base --is-ancestor vX.Y.Z origin/main`),
downloads the release's artifacts and checks their digests, installs them beside the previous
releases on the host, switches `current` atomically, restarts the units and waits for readiness.
The first deploy, and any change to a unit, Caddy or the firewall, needs the owner's go.

## 5. Merge main back into dev

```sh
git switch dev && git pull --ff-only
git merge --no-ff origin/main
git push origin dev
```

If `dev` is protected, this is a pull request from `main` into `dev`, merged with a merge commit.

## 6. A hotfix

Branch from `main`, open a pull request into `main`, tag the patch version on `main`, deploy it,
and merge `main` back into `dev` as in step 5.

## 7. Rollback

Re-deploy the previous tag. Its artifacts are still attached to its release, and the host keeps the
previous release beside the current one.

```sh
bash deploy/rollback.sh vX.Y.W
```

A release whose database migration removed or rewrote data cannot be rolled back by the binary
alone: ship a schema change as expand, then contract, and follow the data-migration runbook's
rollback for the database.
