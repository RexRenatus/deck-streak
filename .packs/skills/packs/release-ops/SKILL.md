---
name: release-ops
description: >-
  Judges a public repository's release path from its tree: main protected by a declared ruleset
  whose required checks always report, a dev to main release pull request merged with a merge
  commit, SemVer versions with a changelog section per release and a changelog fragment per pull
  request, releases published only from SemVer tags on main as drafts that attach assets before
  publishing, deploys only from tags on main with an environment and no racing, and a rollback that
  redeploys the previous tag. Runs on any repository through scripts/release-ops-probe.py --root.
  Use when setting up, reviewing or running releases and deploys, such as DeckStreak's.
requires_phxd_schema: phxd.pack.probe.v1
---

# packs/release-ops

How a public repository with a `dev` branch and a protected `main` releases and deploys, and the
check that keeps it doing so (SPEC-V2-2221 / ADR-V2-2221). Its first user is DeckStreak. The owner's
scope, verbatim: "the dev→main release PR, SemVer tags, and deploy from main tags only; rollback to
the previous tag; changelog fragments; GitHub-hosted CI for a public repository, and branch
protection on main."

`scripts/release-ops-probe.py` is the check. It is standard-library Python 3.11+ and vendorable,
reads only the tree `--root` names, and calls no API. Which seats consume this pack is its catalog
row's `consumes`, the one record of that edge (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack release-ops --root PATH --format json
```

Each row runs `python3 {skills}/../scripts/release-ops-probe.py --root {root} check <class>` under
a 60-second wall. A card shows only each row's exit, so read a red row's findings by running its
class directly: `python3 scripts/release-ops-probe.py --root PATH check <class>`. The protected
branch is `main` and the default branch `dev`; `--protected-branch` and `--default-branch` change
them. `python3 scripts/release-ops-probe.py parse FILE` prints a workflow as the check reads it.

## What composes with this pack

This pack judges the release path. It never copies another pack's rows.

- **greenfield** judges `CHANGELOG.md`'s Keep a Changelog form, that a fragment directory exists,
  that CI runs on pull requests into both long-lived branches, and ships the CI workflow template.
- **cyber-pipeline** judges the CI supply chain: no self-hosted runner in a public repository,
  read-only token permissions, actions pinned by full SHA, and release provenance. Run its white-box
  stage beside this pack (`phxd pack run --pack cyber-pipeline --stage white-box`).
- **data-migration** and **ledger-sqlite** judge what a rollback of the binary cannot undo: a
  database migration.

## The release path

1. **Pull requests go to `dev`.** Each one adds a changelog fragment.
2. **A release is a pull request from `dev` into `main`.** It bumps the version and compiles the
   fragments into a `## [X.Y.Z]` section of `CHANGELOG.md`. It merges with a **merge commit**: a
   squash or a rebase puts commits on `main` that `dev` never had, so every later release pull
   request replays them and conflicts.
3. **An annotated SemVer tag on `main`.** `git tag -a vX.Y.Z`: annotated tags are meant for release,
   and a published tag is never moved.
4. **The tag publishes the release.** The release workflow proves the tag is on `main`, builds once,
   creates the release as a draft, attaches the artifacts, and then publishes it.
5. **The published release deploys.** The deploy job proves the tag is on `main` again, installs the
   artifact built at the tag beside the previous releases, and switches `current` atomically.
6. **Merge `main` back into `dev`.** The release's merge commit reaches `dev`, so the next release
   pull request is up to date with `main`. If `dev` is protected too, the back-merge is a pull
   request from `main` into `dev`, merged with a merge commit as well: a squash would leave `dev`
   without `main`'s release commit.
7. **Rollback is redeploying the previous tag.** A manual dispatch of the deploy workflow with the
   previous tag. The artifact is still attached to its release, and the host still holds its
   directory. A database migration is the part a binary rollback cannot undo, so schema changes
   ship as expand, then contract.

A hotfix branches from `main`, merges into `main` by pull request, is tagged as a patch, and is
merged back into `dev`.

## Branch protection as code

GitHub does not read `.github/rulesets/`: the files there are the rulesets this repository
declares, in the JSON that GitHub's REST API and its ruleset import accept, and the owner applies
them. Create one with `gh api --method POST repos/OWNER/REPO/rulesets --input .github/rulesets/main.json`,
update it with `--method PUT` on `repos/OWNER/REPO/rulesets/ID`, and read the live ones back with
`gh api repos/OWNER/REPO/rulesets` to compare. Reading the live state needs an owner token, so no row
does it.

- **`main`**: `deletion`, `non_fast_forward`, `pull_request` with `allowed_merge_methods: ["merge"]`,
  and `required_status_checks` with `strict_required_status_checks_policy: true`. A solo owner
  cannot approve their own pull request, so `required_approving_review_count` is 0 while a pull
  request is still required. Do not add `required_linear_history` to `main`: it refuses the merge
  commit a release pull request needs.
- **Release tags**: a `tag` ruleset on `refs/tags/v*` with `deletion`, `non_fast_forward` and
  `update`, and a `tag_name_pattern` that admits SemVer tags only. Also enable immutable releases in
  the repository settings: a published release's tag and assets are then locked.
- **Bypass**: leave `bypass_actors` empty, or give an actor `bypass_mode: pull_request` only.

**A required check must always report.** A workflow skipped by a path filter, a branch filter or a
commit message leaves its check Pending, and the pull request cannot merge. A job skipped by its
`if` reports Success without having run. A job whose need failed is skipped. So require one
aggregate job that always runs and fails when any job it needs did not succeed:

```yaml
  ci:
    if: ${{ always() }}
    needs: [rust, web]
    runs-on: ubuntu-24.04
    steps:
      - env:
          RESULTS: ${{ join(needs.*.result, ' ') }}
        run: for result in $RESULTS; do test "$result" = success || exit 1; done
```

Require the aggregate, never a matrix job by its bare name: a matrix reports one check per
combination, `name (…)`.

## GitHub-hosted CI for a public repository

Standard GitHub-hosted runners are free for a public repository; larger runners are billed even
there. Pin release and deploy jobs to a versioned image such as `ubuntu-24.04`: `ubuntu-latest`
moves to a new Ubuntu release on GitHub's schedule, and a tag rebuilt later should build on the same
image. Self-hosted runners in a public repository are cyber-pipeline's refusal.

## Environments

Every deploy job names an environment, `production`. In its settings, restrict deployment branches
and tags to the tag pattern `v*`, add the owner as a required reviewer, and disallow administrators
bypassing the rules. Environment secrets, such as the deploy key, reach only jobs that name the
environment. These are repository settings, so no row reads them.

## The rows

Twenty-two rows, all `tree`-scoped. The script exits 0 when green, 1 on a finding, 2 on a usage
error, and 3 when VOID: nothing to examine, which is never a pass. Every class prints one line per
finding and `examined N`.

Stage `protection`: 6 rows (5 blocking, 1 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `ro.main-ruleset` | block | `main-unprotected` | no ruleset is declared (VOID); a file is not JSON; a rule type or a `pull_request` rule's required parameter is one the API refuses; a ruleset covering `main` is not `active`; or no active branch ruleset covers `refs/heads/main` with `deletion`, `non_fast_forward`, `pull_request` and a non-empty `required_status_checks` (rulesets aggregate; a bare `main` matches no ref) |
| `ro.release-merge-method` | block | `release-merge-rewrites` | `main`'s effective `allowed_merge_methods` (the intersection across rulesets) is not exactly `merge`, or `main` requires linear history |
| `ro.required-checks-exist` | block | `required-check-unreported` | a required check (other than one another app reports) names no job of a pull-request workflow, only a job whose workflow never runs on pull requests into `main`, or a matrix job by its bare name |
| `ro.required-checks-not-skipped` | block | `required-check-skippable` | a required job's workflow filters pull requests by paths, omits `synchronize` from its types, gives the job an `if` other than `always()` or `!cancelled()`, needs a job without `always()`, or runs `always()` without reading `needs.*.result` |
| `ro.tag-ruleset` | block | `release-tag-mutable` | no active tag ruleset covers `refs/tags/v1.2.3`, or the tags carry no `deletion` or no `update` rule |
| `ro.ruleset-bypass` | advisory | `bypass-always` | a ruleset guarding `main` or the tags lets an actor bypass it at any time |

Stage `version`: 3 rows (2 blocking, 1 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `ro.versions-semver` | block | `version-not-semver` | a Cargo.toml, package.json or tauri.conf.json version is not SemVer 2.0.0 (`v1.2.3`, `01.2.3`, `1.2` and `1.2.3-01` are not; an inherited `version.workspace` is read as the workspace's) |
| `ro.changelog-has-version` | block | `version-without-notes` | the product's version (the workspace's, else the root package's) is a release and `CHANGELOG.md` has no level-2 section for exactly it |
| `ro.versions-agree` | advisory | `versions-disagree` | the manifests declare more than one version |

Stage `changelog`: 2 rows (2 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `ro.fragment-shape` | block | `fragment-unparseable` | there is no fragment directory, or a fragment does not parse in its scheme: in `changelog.d/`, only `###` headings of the six Keep a Changelog types, each with a bullet; with towncrier, `<issue>.<type>` for a configured type; with changesets, a frontmatter of `package: major, minor or patch` bumps and a summary |
| `ro.fragment-check-in-ci` | block | `fragment-unenforced` | no pull-request workflow runs `towncrier check`, `changeset status`, or a `git diff` over the fragment directory |

Stage `release`: 5 rows (2 blocking, 3 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `ro.release-runbook` | block | `release-undocumented` | there is no `RELEASING.md` (at the root, in `docs/` or in `.github/`), or it never names the release pull request from `dev` into `main`, tags with `git tag -a`, `-s` or `-m`, merges `main` back into `dev`, or has a rollback section |
| `ro.release-on-tag` | block | `release-off-tag` | a workflow that publishes a release runs on a branch push, every push, a pull request, a schedule or anything but a tag push, a published release or a manual dispatch |
| `ro.tag-filter-semver` | advisory | `tag-filter-loose` | a release or deploy workflow's tag filter admits a non-release tag (`vfoo`, `v1.2`, `v1.2.3.4`, `latest`) or no `v1.2.3` |
| `ro.release-draft-first` | advisory | `release-published-before-assets` | a release job attaches assets to a release it does not create as a draft and publish after the last upload |
| `ro.hosted-runner-pinned` | advisory | `runner-unpinned` | a release or deploy job runs on a moving `-latest` label, or on a label that is not a standard hosted one |

Stage `deploy`: 6 rows (3 blocking, 3 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `ro.deploy-from-tags` | block | `deploy-off-tag` | a deploy job (one with an environment other than `github-pages`, named deploy, or running a `deploy/` script) runs from anything but a tag push, a published release or a manual dispatch |
| `ro.deploy-tag-on-main` | block | `deploy-ancestry-unchecked` | neither a deploy job nor a job it needs runs `git merge-base --is-ancestor <tag> origin/main` on a checkout with the history for it |
| `ro.rollback-entrypoint` | block | `rollback-missing` | there is no `deploy/` rollback script and no manual dispatch with a tag input in a workflow that deploys |
| `ro.deploy-environment` | advisory | `deploy-without-environment` | a deploy job names no environment |
| `ro.deploy-concurrency` | advisory | `deploy-races` | a deploy job has no concurrency group, or cancels a deploy in flight |
| `ro.rollback-keeps-previous` | advisory | `rollback-rebuilds` | a `deploy/` script replaces the release in place, or switches releases without an atomic rename (`mv -T`) |

## Templates

| template | copy to |
|---|---|
| [main ruleset](templates/main.ruleset.template.json) | `.github/rulesets/main.json` |
| [release tag ruleset](templates/release-tags.ruleset.template.json) | `.github/rulesets/release-tags.json` |
| [fragment check](templates/changelog.template.yml) | `.github/workflows/changelog.yml` |
| [release workflow](templates/release.template.yml) | `.github/workflows/release.yml` |
| [deploy workflow](templates/deploy.template.yml) | `.github/workflows/deploy.yml` |
| [deploy script](templates/deploy.template.sh) | `deploy/deploy.sh` |
| [release runbook](templates/RELEASING.template.md) | `RELEASING.md` |

The workflow templates spell pinned action SHAs as `{{CHECKOUT_ACTION_SHA}}`, the way greenfield's
templates do. Add the aggregate `ci` job above to greenfield's CI workflow, so the ruleset's two
required checks, `ci` and `fragment`, both exist.

## How DeckStreak adopts this

1. Copy the templates, add the aggregate `ci` job, and apply the two rulesets with `gh api`.
2. Create the `production` environment with its tag policy and reviewer, and enable immutable
   releases.
3. Run this pack, cyber-pipeline's white-box stage and greenfield against the tree until all are
   green.
4. Release by `RELEASING.md`.

## References

SemVer 2.0.0: https://semver.org/spec/v2.0.0.html. Keep a Changelog: https://keepachangelog.com/en/1.1.0/.
towncrier: https://towncrier.readthedocs.io/. changesets: https://github.com/changesets/changesets.
GitHub: https://docs.github.com/en/rest/repos/rules ·
https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets ·
https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/troubleshooting-required-status-checks ·
https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions ·
https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases.
git: https://git-scm.com/docs/git-tag · https://git-scm.com/docs/git-merge-base. Release
engineering: https://12factor.net/build-release-run · https://sre.google/sre-book/release-engineering/.
The full study, with every practice mapped, is SPEC-V2-2221 §4.2.
