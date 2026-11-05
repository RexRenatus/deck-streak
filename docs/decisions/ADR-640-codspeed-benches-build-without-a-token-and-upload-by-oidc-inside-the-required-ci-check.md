---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-640: CodSpeed benches build without a token and upload by OIDC inside the required ci check

## Context and Problem Statement

DeckStreak holds fixed budgets for the wasm size, the peak memory and the sync time, and measures no
instruction count, so a change that makes a hot path slower passes every check. CodSpeed measures
benches by CPU simulation, instruction by instruction, so its figures do not move with the runner's
load. The aggregate `ci` is the required check and needs every job of `ci.yml` (SPEC-038, ADR-055);
the workflow default token is `contents: read`, and the one OIDC grant in the tree sits on the
release job alone (`release.yml` lines 36-39).

## Decision Drivers

- A bench that fails to build or run must fail the required check, and a pull request that reaches
  no bench must not wait on one.
- No code that a dependency's install script or a build script runs may sit beside a grant that
  can mint a token.
- No secret, and no account act for each accepted regression.
- A slower change that a pull request keeps is explained in public, in a record a check reads.

## Decisions, and the alternatives each was chosen against

### D1. Four jobs inside ci, each with a not-applicable arm

`ci.yml` gains a build job and an upload job per lane (`bench-rust-build`, `bench-rust`,
`bench-web-build`, `bench-web`), and `ci` needs all four. Each job always starts; a plan step,
`scripts/perf-plan.py`, decides, and every measured step carries its `if`, the shape
`mutation-web-shard` already uses, so a job whose steps were skipped concludes `success` and `ci`
needs no new admission arm. A push to dev or main always applies, since the baseline comes from
those runs; a pull request applies when its diff reaches the lane.

- Chosen: jobs inside `ci`, because the required check then always reports and a failed bench build
  reads as itself.
- A path-filtered bench workflow: rejected because a required check that a path filter skipped never
  reports, and a check outside `ci` cannot fail the merge.
- One job for both lanes: rejected because each lane keeps its own arm, and CodSpeed merges the
  uploads of several jobs of one commit.

### D2. A build job with no token, and an upload job that builds and installs nothing

Each lane builds in a `contents: read` job, which runs the toolchain, every build script, proc
macro and install script, and archives the bench binaries or the installed trees with `tar` (an
artifact drops the executable bit) as a one-day artifact. Its upload job holds `contents: read` and
`id-token: write`, unpacks the archive, and runs the pinned action in `mode: simulation` over what
was built; it runs no `cargo build`, no build script and no package install. The upload job still
runs the benches, which carry dependency code; what a token minted there reaches is bounded,
because a token minted in `ci.yml` can sign only as `ci.yml`, and every deploy verifies the release
tarball with `--signer-workflow` pinned to `release.yml` (`deploy/deploy.sh` lines 239-240, held by
`scripts/tests/test_deploy_scripts.py` line 509).

- Chosen: the split, because it takes build scripts, proc macros and install scripts out of the
  job that can mint a token.
- One job per lane holding `id-token: write`: rejected because every build script and install
  script of the dependency tree would then run beside the grant.

### D3. OIDC, never a secret

The upload jobs authenticate by OIDC: GitHub mints a short-lived token for each run, the action
takes no `token` input, and no repository secret is created. The grant is job-scoped, as
`release.yml` already scopes its own, and the workflow default stays `contents: read`; zizmor
audits the new grant in `workflow-lint`, and a finding is cured in the shape, never by an ignore.

- Chosen: OIDC, because nothing long-lived can leak and nothing needs rotating.
- A static CodSpeed token in a repository secret: rejected because it is a long-lived secret every
  upload job could read.
- Tokenless uploads: rejected for now because a failed upload fails the required check, and a
  tokenless arm joins only by its own pull request with a fork run shown green.

### D4. The fork arm

A pull request whose head repository is not this one gets no OIDC token from GitHub. Its plan reads
`upload=false` with the reason `fork PR (GitHub mints no OIDC token for a fork; the push to dev
measures the merged change)`, while `applicable` still follows its diff, so its Rust benches are
still built and its upload jobs conclude `success`. The push to dev after the merge measures the
change.

- Chosen: build and never upload, because a fork pull request then reads not applicable, never a
  red required check.
- Upload from a fork and let it fail: rejected because a failed upload fails `ci`.

### D5. The CodSpeed check stays outside every ruleset

CodSpeed posts its own check on branches and pull requests. That check joins no ruleset; the merge
step reads it: it merges when the check reads success, or when the check is absent and both upload
jobs printed not applicable, or when the check reads failure and the pull request carries a
regression record naming every regressed benchmark and the change has been accepted. An absent
check on an applicable run is a hold.

- Chosen: the merge step reads the check, because an accepted regression then needs no account act
  in CodSpeed, and CodSpeed's uptime is no ruleset dependency.
- The CodSpeed check as a required check: rejected because each accepted regression would need an
  acknowledgement only a CodSpeed admin can make.
- A `ci` step that polls CodSpeed and parses its text: rejected because it parses a vendor's text,
  races its asynchronous analysis and adds that analysis to what the required check waits on.

### D6. A regression record, checked

A pull request that keeps a change CodSpeed reads slower adds `perf/regressions.d/<branch-slug>.json`
of the closed schema `deckstreak.perf-regression.v1`: the branch, each regressed benchmark with its
change, base, head and instrument, the cause, why, the SPEC requirement it serves, the alternatives
tried and the remediation. `scripts/perf-record.py check` refuses an unknown key, a blank cause or
why, a change of 0 or less, a benchmark no bench defines and a branch other than the pull
request's own. The record never carries a verdict.

- Chosen: a public record in the tree, because the reason for a slower change then sits beside the
  change, and a check holds its shape.
- A comment on the pull request: rejected because no check reads it and it leaves the tree.

### D7. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani: the plan and the checker are standard-library scripts, and no
protocol, state machine, interleaving or engine path gains or changes.

## Consequences

- Good, because a slower hot path is seen on its pull request, instruction by instruction.
- Good, because no secret exists, and no build or install script runs beside the token.
- Bad, because the upload is a CodSpeed dependency inside the required check: an upload outage
  turns `ci` red until a re-run.
- Bad, because a fork pull request is measured only after its merge.

## Confirmation

- `scripts/tests/test_perf_plan.py` and `scripts/tests/test_perf_record.py`, run by the gate python
  stage, hold the plan with its fork arm and the record checker with each refusal arm (SPEC-618).
- The delivery pull request's first bench run is read by name before it lands.

## What would make this wrong

- GitHub mints OIDC tokens for fork pull requests: the fork arm could then upload.
- CodSpeed's analysis becomes part of the upload's exit: the check would then sit inside `ci`.
- A relying party outside the tree trusts a token by repository name alone: the split's bound on
  what a minted token reaches would need re-reading.

## More Information

SPEC-618; SPEC-038 and ADR-055 (the gate jobs and the aggregate `ci`); ADR-017 (hosted runners);
ADR-589 (the deprecated code the exclusions name); ADR-613 (coverage stays advisory).

## Amendment for the second part of the delivery, and what it was chosen against

Three facts of the second part differ from the text above, and each is recorded here; no decision
above is withdrawn.

The Rust bench lane installs one tool, in its build job. D2 says an upload job builds and installs
nothing; `bench-rust-build`, which holds `contents: read` alone, installs `cargo-codspeed` at its
pinned version by `cargo install --locked` and carries the binary in `bench-rust.tar`, and
`bench-rust` puts the unpacked binary's folder on its path, so no install and no compile runs
beside the token. Neither upload job runs a package install or a build script (SPEC-618 R14).

The record check runs on each pull request in `bench-rust-build`, which holds `contents: read`
only, and checks only the records that pull request adds or changes under `perf/regressions.d/`
(SPEC-618 R11). A refused record fails the build job and so `ci`.

The benchmark list the check passes is `perf/benchmarks.txt`, one `<file stem>::<bench>` name per
line, and a test holds it equal to the names the bench files define (SPEC-618 R12).

- Carry the `cargo-codspeed` binary in the build archive: taken. `taiki-e/install-action` at its
  pinned commit lists no `cargo-codspeed` manifest (the manifest reads not found), so with
  `fallback: none` it cannot install the tool, and its default fallback may compile it in the upload
  job beside the token.
- Check every record in `perf/regressions.d/` on each pull request: rejected, because a record kept
  for an earlier change names that change's branch, so the branch arm would refuse it on every
  later pull request.
- Derive the benchmark names inside the record step: rejected, because the step would then run a
  parser the gate does not test, while the committed list is held equal to the bench files by a
  test the gate runs.
