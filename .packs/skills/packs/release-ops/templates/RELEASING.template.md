# Releasing

A release is a pull request from `dev` into `main`, an annotated SemVer tag on `main`, and a deploy
of that tag. Nothing deploys from a branch.

## 1. Prepare the release on dev

1. Choose the version by SemVer: a breaking change bumps the major, a feature the minor, a fix the
   patch. Set it in the workspace `Cargo.toml` and in the web `package.json`.
2. Compile the fragments under `changelog.d/` into a new `## [X.Y.Z]` section of `CHANGELOG.md`,
   and delete them.

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

The tag runs the release workflow, which checks that the tag is on `main`, builds once, and
publishes the release. The published release runs the deploy.

## 4. Merge main back into dev

```sh
git switch dev && git pull --ff-only
git merge --no-ff origin/main
git push origin dev
```

## 5. A hotfix

Branch from `main`, open a pull request into `main`, tag the patch version on `main`, and merge
`main` back into `dev` as in step 4.

## 6. Rollback

Re-deploy the previous tag. The artifact built at that tag is still attached to its release, and the
host keeps the previous release beside the current one.

```sh
gh workflow run deploy.yml -f tag=vX.Y.W
```

A release whose database migration removed or rewrote data cannot be rolled back by the binary
alone: ship a schema change as expand, then contract, and follow the data-migration runbook's
rollback for the database.
