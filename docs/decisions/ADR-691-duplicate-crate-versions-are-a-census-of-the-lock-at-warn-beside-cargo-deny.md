# ADR-691: duplicate crate versions are a census of the lock at warn, beside cargo-deny's own warning

- **Status:** accepted, with SPEC-668.
- **Follows:** ADR-450 (a new rule starts at warn and is promoted by its own delivery, D6).

## Context

`deny.toml`'s `[bans]` table sets `multiple-versions = "warn"`. cargo-deny's bans check, run by
the `rust` job's `audit-rust` stage, then prints `warning[duplicate]: found <n> duplicate entries
for crate '<name>'` for each duplicated crate, with the chain that includes each version, and the
check still passes. By its own documentation it leaves out a crate reached only through
dev-dependencies unless `multiple-versions-include-dev` is set, and its `skip` and `skip-tree`
entries silence a named version or a whole subtree. Nothing in the tree says which duplicates are
held on purpose, so the warning cannot tell an old duplicate from a new one. The box that builds
deliveries runs no cargo, so a check whose arms are proved there reads the lock as text.

## Decision Drivers

- No rule already in force is lowered: cargo-deny's warning stays as it is.
- A new rule starts at warn and moves to fail only by its own delivery (ADR-450 D6).
- Every check prints what it examined and fails on an empty population, which is VOID.
- Today's sites are listed, not cured.

## Decisions

**D1. A census of `Cargo.lock` under `scripts/tests/`.** `test_lock_duplicates.py` reads every
`[[package]]` entry by `tomllib`, as `test_xp_crate_graph.py` reads the lock, and each of its arms
is proved over a planted lock read as text (SPEC-668 R1).

**D2. Two forms, and every entry stays in.** A name is a site at two or more versions, or at one
version from two or more sources. The lock records no dependency kind, so a version reached only
through a dev or a build dependency stays in the population (SPEC-668 R2, R3).

**D3. A listing by name, with a reason.** Each site at the base is listed by name with the reason
each version is there. An unlisted site warns at `warn` and fails at `fail`; a listing entry that
names no site, and a dependency string that names no one entry, fail at both levels (SPEC-668 R4
to R7).

**D4. Warn first.** `LEVEL` is `warn`; the promotion is ADR-450 D6's rule, by its own delivery,
and a demotion is a weakening (SPEC-668 R8).

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| raise `multiple-versions` to `deny` with a `skip` entry per listed name | it changes `deny.toml`, a gate's configuration, and its arms can only be proved by running cargo-deny, which the box does not; it stays a question for its own delivery |
| key the listing by name and version | every version bump of a listed name would read as one stale entry and one new site, though the duplicate is the same |
| read every manifest to drop the versions reached only through dev-dependencies | it narrows the population to cargo-deny's default scope by code the lock cannot check, and a dev-only duplicate is still two versions in the lock |
| resolve the graph with `cargo metadata` | the census would need cargo and the network, and its planted cases could not run on the box |

## Consequences

- A new duplicate prints one `WARN` line in the `hygiene` job's log while the level is `warn`.
- A listed name that leaves the lock fails the census until its entry goes, in the same change.
- cargo-deny keeps printing its own warnings; the two reports differ where its scope does.

## What would make this wrong

- cargo writes a dependency string in a form the reader does not know: every such string fails
  as `UNDECIDED`, so the census fails rather than passing blind.
- The listing's churn on dependency updates outweighs its signal: ADR-450 D6's live precision
  falls below 0.8 with at least five hits, and D6 proposes a demotion to the owner.
