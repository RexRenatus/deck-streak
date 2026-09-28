---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The packs stay box-only, and no pack runner is published

## Context and Problem Statement

ADR-004 vendored the packs' standard-library probes into this repository and ran the packs built
into `phxd` on the maintainer's box. It left one question open: the owner had planned a public,
open-source pack runner that CI would install at a pinned version, and ADR-004 treated today's
arrangement as the state "until it exists". Several later documents lean on that premise:
- ADR-030 expects the runner to replace `scripts/box-packs.sh`;
- ADR-039 expects it to end vendoring;
- ADR-043 expects it to ship a proxy client with no private name;
- SPEC-002 and SPEC-030 exclude work "until the open-source pack runner exists" (#60).
- `scripts/pack-rows.py` and `.packs/VENDORED.json` describe the vendored probes as standing in
  for that runner until it exists.

On 2026-09-28 the owner decided the question: the packs stay box-only. How does DeckStreak run its
packs from now on?

## Decision Drivers

- The owner's decision: the runner is not published.
- Public CI can hold no secret, so it can neither fetch nor build the maintainer's private runner.
- Every pack DeckStreak consumes must still be wired and green (the definition of done, #60).

## Considered Options (the alternatives it was chosen against)

- Vendored probes in CI, runner-built packs on the box: chosen, because it is the only arrangement
  the owner's decision leaves that still judges every pack. CI runs DeckStreak's own gate, which
  includes the vendored probes' rows, and the box runs `scripts/box-packs.sh` against the
  maintainer's build at the vendored commit.
- Wait for a published runner: rejected because the owner decided none will be published.
- Build the runner in CI from a private source: rejected because public CI would need a secret to
  fetch it, which the repository's CI must never hold (ADR-017).
- Drop the runner-built packs from DeckStreak's definition of done: rejected because every pack
  DeckStreak consumes must be wired and green, and the box can run them.

## Decision Outcome

Chosen option.
- **Public CI** runs `bash scripts/check.sh` and nothing that needs a private build. Its pack stage
  judges the vendored probes' rows, as ADR-004 set up.
- **The maintainer's box** runs `scripts/box-packs.sh` for the runner-built packs, with the build
  pinned to the commit `.packs/VENDORED.json` names (ADR-030). Each expected red there names an
  open issue.
- **Vendoring** stays the way the probes reach this repository (ADR-039). It is refreshed with
  `scripts/vendor-packs.py` at each re-pin.
- **#60** keeps its goal, every pack wired and green, reached through box runs rather than through a
  published runner.

This supersedes ADR-004's premise that today's arrangement lasts "until it exists", meaning the
runner. The rest of ADR-004 stands. It also retires the "what would make this wrong" premises of
ADR-030, ADR-039 and ADR-043 that assumed a published runner, and the "until the open-source pack
runner exists" wording of the judged SPEC-002 and SPEC-030: each now reads as "while the packs stay
box-only". SPEC-033's exclusion, no private literal in CI, is unconditional and stands as written.
`scripts/pack-rows.py` and `.packs/VENDORED.json` now state the box-only arrangement.

### Consequences

- Good, because the owner's private runner is never published, and nothing in public CI needs it.
- Good, because every pack is still judged: the vendored rows in CI, and the runner-built packs on
  the box at the pinned commit.
- Bad, because the runner-built packs are judged only when the maintainer's box runs them. A pull
  request's public checks do not show those rows, so the maintainer's verification of each pull
  request includes a box run.

### Confirmation

- `scripts/box-packs.sh` reads `BOX PACKS OK` on each pull request's head in the maintainer's
  verification.
- `.github/workflows/` reads no secret and fetches no private build. Until the test #216 asks for
  makes this mechanical, a pull request that changes a workflow is reviewed against ADR-017, which
  allows no secret in a pull-request workflow.

## What would make this wrong

- The owner publishes the runner after all. A new ADR would then replace this one.

## More Information

ADR-004 (partly superseded), ADR-017, ADR-030, ADR-039, ADR-043; issues #60 and #216.
