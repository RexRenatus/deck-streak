---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-355: One Apple job body, called by a change caller and a tag caller

## Context and Problem Statement

ADR-335 decides that "a macOS CI job builds the Apple and FFI paths when they change and on every
tag" (its Builds section), and ADR-345 D4 decides that the job is a workflow of its own, admitted
to the macOS runner by its file name. On `dev`, `.github/workflows/xcframework.yml` is that
workflow, and it runs on a pull request into `dev` that changes the adapter, the lockfile or the
workflow itself, and on a dispatch. No tag starts it (SPEC-344 section 1.1).

Neither ADR-335 nor ADR-340 settles how the tag run is added, and the census settles part of it
by construction. A workflow a tag push can start, and every workflow it calls, is in the release
class: its concurrency group reads `github.ref` alone, it never cancels and it queues every run
(`test_workflow_concurrency.py`, `release_problems` and `release_class_problems`). A workflow with
a `pull_request` trigger must hold exactly the pull-request group, which reads `github.event_name`
and `github.run_id`, and cancels a superseded pull-request run. One block cannot meet both rules.
So the open questions are where the tag trigger goes, how the census admits it, whether the run
gates a merge, which tags start it, how a tag run is proved before any tag exists, and which paths
are the Apple paths.

ADR-340 settles the sync unit's render and its budget entry, and SPEC-337 and SPEC-340 deliver the
tests, so this ADR decides nothing about them. The `allow-git` list already admits every git
source the lockfile holds (SPEC-344 section 1.3), so no decision about it remains either.

## Decision Drivers

- The census's two concurrency rules stay as strict as they are: no rule is weakened to admit the
  tag run.
- The macOS runner stays admitted to one workflow file by name (ADR-345 D4).
- One job body, so a tag run builds what a change run builds and the two cannot drift.
- A required check must always report, and a path-filtered workflow does not.
- A release tag is protected and cannot be deleted or moved, so no test may cut one.
- `release.yml` is not touched.
- The path filter is a measured population, not a guess.

## Considered Options (the alternatives it was chosen against)

- D1, the job body stays in `xcframework.yml`, which takes `workflow_call` and `workflow_dispatch`
  and holds no concurrency block, called by two thin workflows, `apple-on-change.yml` (the
  pull-request rule's block) and `apple-on-tag.yml` (the release class's block) - chosen, because
  each caller's block meets the one rule its trigger is held to, the body and the runner admission
  stay in the one file ADR-345 D4 admits, and both events run one body.
- D1, a `push: tags:` trigger added to `xcframework.yml` - lost: the file would then be held to
  both concurrency rules, which no single block meets, so one rule would have to be weakened.
- D1, a second workflow file holding a copy of the job body for tags - lost: it copies the whole
  body (and the harness jobs #656 adds), so the two can drift; the runner would need a second
  admission, against ADR-345 D4's one file; and it still needs a census change.
- D1, a push to `main` in place of a tag - lost: it is not a tag run, and a tag can name any
  commit of `main`, so the commit a tag names may never have been built. It needs no census change,
  so it is the fallback if the seat refuses D2.
- D1, a `workflow_run` trigger chained on `release.yml` - lost: the workflow audit `ci.yml` runs
  reports `workflow_run` as a dangerous trigger, and such a run reads its workflow file and its ref
  from the default branch, so it would not build the commit the tag names.
- D1, `release.yml` calling or dispatching the build - lost: `release.yml` is not touched, and a
  dispatch needs a write token.
- D2, the pin census admits a call job's `uses` when it is pinned by a full commit SHA, as today,
  or when it fullmatches `LOCAL_CALL`, the pattern the concurrency census already reads calls with,
  moved into `test_ci_workflows.py` and imported back so one pattern serves both; every other
  `uses` stays pinned, and the census judges the same population with the same refusal text -
  chosen, because a local call runs this repository's file at the caller's own commit, so it has
  nothing to pin, and the release-class rule already refuses a call to a file that is not here or
  does not take `workflow_call`.
- D2, the pin census unchanged - lost: it refuses every local call, so D1 cannot land.
- D2, a second copy of the whole call rule (file here, takes `workflow_call`) in the pin census -
  lost: two copies of one rule are free to drift, and the release-class rule already walks every
  call in every workflow.
- D2, the pin census importing from `test_workflow_concurrency.py` - lost: that module imports
  from `test_ci_workflows.py`, so the import would be circular.
- D3, the Apple build stays advisory: in no `needs` of the aggregate `ci` job and in no ruleset -
  chosen, because a path-filtered workflow leaves a required check pending, and the macOS cost
  ADR-335 names is paid only by a change that touches an Apple path.
- D3, `xcframework` as a required context - lost: a pull request that changes no Apple path never
  reports it, so the pull request can never merge.
- D3, the build called from `ci.yml` behind a path-detecting job, its skip admitted by the
  aggregate - lost: SPEC-290 R8 admits a skip from two named legs only, and every lockfile change
  (84 commits in `dev`'s history touch `Cargo.lock`) would hold `ci` on a macOS build.
- D4, the tag filter is `release.yml`'s own SemVer filter - chosen, because the build then runs on
  exactly the tags that publish a release.
- D4, `v*`, the tag ruleset's pattern - lost: it admits tags that publish nothing, such as `vfoo`.
- D4, every tag (`**`) - lost: a tag that is not a release would pay for a macOS build.
- D5, a tag run proved without a tag: the census reads the tag caller as GitHub reads it (its tag
  events, its filter against `release.yml`'s names, its place in the release class), one body
  serves both events and reads no pull-request-only context, and the change caller's run on the
  build's own pull request runs that body - chosen, because it proves everything but the trigger
  without a write, and the trigger is the census's reading of GitHub's documented filter.
- D5, cutting a throwaway tag - lost: release tags are protected from deletion and update, and the
  build may never cut, push or delete one.
- D5, a dispatch on a tag ref - lost: it is a write to GitHub, and a dispatch is not the push event
  the tag caller takes.
- D6, the change caller's paths: `crates/ffi/**`, `ios/**`, `Cargo.lock`, `Cargo.toml`,
  `rust-toolchain.toml` and the two workflow files - chosen, because each is an input of the build:
  `Cargo.toml` holds the engine pin, the profiles and the patch entry, and one commit in `dev`'s
  history changed it without the lockfile; `rust-toolchain.toml` is the compiler the iOS targets
  are added to.
- D6, `crates/**` - lost: the adapter depends on the engine alone (ADR-345 D1), so a change to
  another crate cannot change what the build builds unless it changes the lockfile, which is in
  the filter.
- D6, the filter `dev` carries, unchanged - lost: it misses `Cargo.toml` and `rust-toolchain.toml`.

## Decision Outcome

Chosen options: D1 one body called by a change caller and a tag caller, D2 a call job admitted by
the concurrency census's own pattern, D3 an advisory build, D4 `release.yml`'s tag filter, D5 a tag
run proved by the census and the shared body, and D6 the measured path population. Together they
add the tag run without weakening either concurrency rule, keep the runner's admission to one
file, keep the build out of the merge gate, and leave `release.yml`, `deny.toml` and the deploy
templates untouched.

### Consequences

- Good, because a release tag builds the XCFramework, and the harness jobs once #656 lands, with
  the same steps a change run takes.
- Good, because both concurrency rules and the runner admission are unchanged; the pin census
  admits one more shape, a local call, and refuses everything it refused before.
- Good, because a change to `Cargo.toml` or `rust-toolchain.toml` now runs the build.
- Bad, because the build's check names gain the caller's job name, `<caller job> / xcframework`.
  No ruleset reads them.
- Bad, because the first tag run is seen only when a real release is tagged; until then the
  trigger is proved by the census's reading of GitHub's documented filter, not by a run.
- Bad, because a tag run is advisory: a red Apple build does not stop the release.
- Bad, because `xcframework.yml` joins the release class, so the parser-schema reading of
  `release_class_problems` now judges its whole body, and a key the reader does not model there
  fails the release-class test, not the Apple build.
- Bad, because #656 edits the same `on:` block; whichever lands second resolves it, and its
  `ios/**` path is already the change caller's.

### Confirmation

SPEC-344 A1 holds the change caller's paths over planted changes; A2 holds the tag caller's
filter equal to `release.yml`'s over admitted and refused names; A3 holds the one body; A4 holds the
pin census's call-job rule beside plants it refuses; A5 holds the release class's membership; A6
holds `--locked`. The rows of band `S344` prove each criterion fails on its mutant.

## What would make this wrong

- GitHub starts evaluating a path filter on a tag push, or gives a called workflow a context of its
  own: D1 and D5 then need re-reading.
- A release has to wait on the Apple build: D3 is then reversed by a ruleset change and a gated
  release job, which is #634's decision.
- The adapter gains a dependency on another workspace crate: D6's population is then re-measured.

## More Information

SPEC-344; ADR-335 (Builds), ADR-340, ADR-345 (D1, D4); SPEC-190 for the concurrency census;
SPEC-290 R8 for the aggregate's skips; SPEC-334 R20, rows 1.2 and 1.5.

## Amendment

The callers use the self-repository form, `$/.github/workflows/xcframework.yml`, and the local-call
admission of the pin census reads that form only. Chosen against keeping the `./` form with a
linter ignore, which is a weakening of the gate, and against pinning the linter's version, which
is an environment pin and leaves the finding in place. The `$/` form is not subject to runtime
file-system state and is read as a form of pinning, so D1 to D6 stand unchanged.
