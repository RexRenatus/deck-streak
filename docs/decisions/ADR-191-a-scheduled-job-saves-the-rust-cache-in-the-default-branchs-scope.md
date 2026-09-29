---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A scheduled job saves the Rust cache in the default branch's scope, so a run on any ref can restore it

## Context and Problem Statement

A workflow run restores a cache only from its own ref, its pull request's base branch and the
default branch. SPEC-038 R2 lets only a push to `dev` or `main` save the Rust cache, and `dev` is
not the default branch, so every entry saved so far sits in `dev`'s scope. A `workflow_dispatch`
run on a feature branch restores none of them and compiles every dependency from nothing (#370).
Where does an entry come from that a run on any ref can restore, without changing any check?

## Decision Drivers

- The entry must sit in the default branch's scope, the one scope every ref reads.
- No check may examine less: the change moves where a build's inputs come from and nothing else.
- No new paid storage, no larger runner and no repository setting: those are the owner's.
- The entry must be exactly the one `ci.yml` and the weekly battery ask for, so no consumer changes.

## Considered Options (the alternatives it was chosen against)

- Chosen, because a scheduled run executes the default branch's copy of the workflow and so runs in
  its scope: a new `rust-cache.yml`, on `schedule` and `workflow_dispatch`, that checks out `dev`,
  looks `ci.yml`'s exact key up with `lookup-only` and stops on a hit, and on a miss builds what
  `ci.yml`'s `rust` job builds without running a test, cleans the workspace's own artifacts and saves
  under the exact key.
- Making `dev` the default branch: rejected, because it is a repository setting and the owner's call,
  and this issue does not ask for it.
- A `workflow_run` trigger after `ci` on `dev`: rejected, because the workflow linter flags that
  trigger and it too runs from the default branch's copy, so it would take effect no earlier than a
  schedule does.
- Saving from every pull request's run into its own scope: rejected, because no other ref can
  restore that entry and storage would grow with every pull request.
- Stopping `ci.yml` saving in `dev`'s scope now: rejected, because a push to `dev` would lose its
  cache until this job runs from `main`; it is a follow-up once the job is measured live there.

## Decision Outcome

`.github/workflows/rust-cache.yml` runs every two hours (`43 */2 * * *`) and on dispatch, with a
read-only token and a queueing concurrency group. Its one job checks out `dev`, computes `ci.yml`'s
key over that tree (the same expression, which a test compares character for character with the
`ci.yml` and `mutation-weekly.yml` restore steps), looks it up with `lookup-only: true` and skips
every later step on a hit. On a miss it installs what `ci.yml`'s `rust` job installs, restores by
`ci.yml`'s prefix, runs `cargo clippy --workspace --all-targets --locked`, `cargo nextest run
--workspace --locked --no-run` and `cargo test --doc --workspace --locked --no-run`, runs `cargo
clean --workspace` and saves under the key.

It takes effect when the release pull request carries the workflow to `main`, because a schedule runs
only from the default branch's copy.

### Consequences

- Good, because a run on any ref restores a warm cache once `main` carries the workflow.
- Good, because every consumer's key is unchanged and no gate examines less.
- Bad, because the workflow does nothing on its schedule until `main` has it; a dispatch on a branch
  saves into that branch's scope only, which the live proof deletes.
- Bad, because SPEC-038 R2's test needs one admitted shape (a scheduled save that runs only on a
  lookup miss); SPEC-191 R9 and A7 bound it and refuse the variants.

## What would make this wrong

- `dev` becoming the default branch: then a push to `dev` saves in the scope every ref reads, and
  this job would be redundant.
- A build in the stages whose dependency fingerprints differ from the three commands here: the entry
  would restore and still recompile, and the live proof of a later `ci.yml` run would show it.

## More Information

SPEC-191, SPEC-038 R1 and R2, ADR-055, issue #370.
