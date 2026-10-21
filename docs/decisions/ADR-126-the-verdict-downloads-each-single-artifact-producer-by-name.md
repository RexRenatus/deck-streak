---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The verdict downloads each single-artifact producer by name and the shards by a merged pattern, so its layout never depends on a count

## Context and Problem Statement

`actions/download-artifact` v8.0.1 extracts into `path` itself when it downloads by `name`, when
`merge-multiple` is true, or when a pattern matched exactly one artifact, and into `path/<name>` only
for two or more matches without `merge-multiple`. The mutation verdict downloaded `pattern:
mutation-*` and read `reports/mutation-plan/plan.json`. On a pull request whose Rust and rows jobs
upload nothing, and when `mutation-web` (not in the verdict's `needs:`) has not uploaded yet, one
artifact matched, the plan landed flat, and the verdict exited `VOID no plan` (#351). Where does the
verdict's layout come from, so that no count of artifacts can change it?

## Decision Drivers

- The verdict must be a function of the plan and the reports, never of the order two jobs finish in.
- A red that a re-run clears trains readers to re-run, which is the failure a gate must not teach.
- `mutation-web` is the slowest job and the verdict has no use for its report.
- The shard count is 1 to 256, so a pattern for the shards meets the same rule at one shard.

## Considered Options (the alternatives it was chosen against)

- Chosen, because the layout is then fixed by the step and not by a count: one `name:` download per
  single-artifact producer (`mutation-plan`, and `mutation-rows` when the plan says it ran), each
  into `reports/<name>`, and the shards by `pattern: mutation-rust-shard-*` with `merge-multiple:
  true`, each shard's artifact carrying its own top directory.
- Adding `mutation-web` to the verdict's `needs:`: rejected, because it makes the verdict wait for
  the slowest job to read a report it does not use, and the pattern would still lay the plan out by
  count in a run where the other jobs upload nothing.
- The pattern `mutation-*` with `merge-multiple: true` and unique file names: rejected, because it
  loses the directory a shard's `outcomes.json` and `cargo-mutants.exit` are read by, and the
  plan's, the rows' and `mutation-web`'s `plan.json` and report files would have to be renamed.
- A reader that accepts both layouts, flat and per-name: rejected, because it hides the defect: the
  judge would find a plan wherever it landed and the next layout change would pass unseen.
- `continue-on-error` on the whole download step: rejected, because it turns a missing plan into a
  green step that the judge must then discover, and it hides every real download failure.
- The pattern kept for the shards without `merge-multiple`: rejected, because a diff needing one
  shard matches one artifact, which extracts flat; the plan's artifact used to make the count two.

## Decision Outcome

`mutation-verdict` downloads `mutation-plan` by name into `reports/mutation-plan`; downloads
`mutation-rows` by name into `reports/mutation-rows` under `if: needs.mutation-plan.outputs.rows ==
'true'`, the condition under which `mutation-rows` makes its report; and downloads
`mutation-rust-shard-*` with `merge-multiple: true` into `reports`. `mutation-rust` writes each
shard's report under `shards/mutation-rust-shard-<i>/` and uploads `shards/`, so every shard lands
at `reports/mutation-rust-shard-<i>/` whatever the shard count. The judge's command lines are
unchanged.

### Consequences

- Good, because a docs-only pull request and a Rust pull request lay out the same paths, so a re-run
  is never the fix.
- Good, because the verdict no longer reads or waits on anything of `mutation-web`.
- Bad, because the rows' download carries a condition that must track the rows job's own; A3 pins
  that no step fails when the rows never uploaded.

### Confirmation

SPEC-126's A1 to A4, and the pull request's own `mutation-verdict` run, whose download log lines are
quoted in the pull request.

## What would make this wrong

- A version of the action that lays a pattern out independently of the count: then the pattern
  would do, and this ADR would be superseded by the decision that bumps the action.

## More Information

SPEC-126, SPEC-039 R3 and R18, ADR-057, issue #351.
