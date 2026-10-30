---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# License: AGPL-3.0-or-later for the whole repository

## Context and Problem Statement

The owner chose AGPL-3.0 for the public repository. The predecessor's code is the owner's own,
"all rights reserved", and porting its behaviour into an AGPL repository is the owner's
relicensing act. The vendored pack probes come from the owner's private repository, which the
owner decided will become open source.

## Decision Drivers

- The owner's decision: AGPL-3.0, recorded in `LICENSE` and SPDX identifiers.
- Anki's Rust engine (a candidate dependency, ADR-009) is AGPL-3.0-or-later.
- SPDX list 3.x deprecates the bare `AGPL-3.0` identifier.

## Considered Options (the alternatives it was chosen against)

- `AGPL-3.0-or-later` for every file, recorded in `LICENSE`, `LICENSES/AGPL-3.0-or-later.txt`, `REUSE.toml` and every manifest — chosen: the owner's licence, spelled as a current SPDX id, and compatible with Anki's engine.
- `AGPL-3.0-only` — rejected because it would not combine with a later AGPL version of a dependency and adds nothing the owner asked for.
- A permissive licence — rejected by the owner.

## Decision Outcome

Chosen option. The owner relicenses the predecessor's behaviour and the vendored probes into
this repository under AGPL-3.0-or-later by the decisions recorded in the architect's brief; no
predecessor source file is copied verbatim (behaviour is ported, and its tests are rewritten with
synthetic data). The network-use source offer (AGPL section 13) is the README's source section
and a link from the Mini App's about screen.

### Consequences

- Good, because one licence covers the whole tree.
- Bad, because a contributor must accept the AGPL's network clause; CONTRIBUTING says so.

### Confirmation

greenfield's license rows on the box (`license-matches-manifest`, `license-spdx-current`, `readme-license`, `agpl-source-offer`); cyber-pipeline's `cp.license-present` and `cp.license-osi`.

## What would make this wrong

- The owner changes the licence decision.

## More Information

LICENSE; REUSE.toml; README's License and Source code sections.

## Amendment: the brand files keep their own licence (ADR-598)

The decision above, `AGPL-3.0-or-later` for every file, is amended by ADR-598 for exactly the
files `scripts/brand-files.json` lists, which come from the DeckStreak brand source; its text above
is kept as it was. Those files keep the brand source's own licence, recorded by one `REUSE.toml`
annotation with `precedence = "override"` that names each listed path, and by its licence text in
`LICENSES/LicenseRef-DeckStreak-Brand.txt`. Every other file stays under
`AGPL-3.0-or-later`, and the code that reads or serves the brand files is the project's own.

- Chosen: the logo keeps its own licence beside the AGPL code, because a project's mark is its identity, not part of the program others may change and redistribute under the AGPL
- Licensing the logo under the AGPL with every other file: rejected because the brand source records the files under its own licence, and the files keep it
