---
status: "accepted"
---

# ADR-657: three parent crates come from forks until their releases drop paste and basic-toml

## Status

Accepted, with SPEC-055's amendment (§9).

## Context

Two unmaintained notices reach the workspace through crates we do not own: `paste`
(RUSTSEC-2024-0436) through `frankenstein` 0.52.1 and `uniffi_bindgen` 0.29.1, and `basic-toml`
(RUSTSEC-2026-0374) through `rinja_derive` 0.3.5, which the bindings generator reaches through
`rinja`. No release of `frankenstein` drops `paste`. `uniffi_bindgen` declares `paste` and no source
file names it; upstream dropped it after 0.29.1, and ADR-345 pins `uniffi` at `=0.29.1`. askama
replaced `basic-toml` with `toml` on its main branch, with no release yet. An ignore entry is not a
fix, so each parent comes from a fork we keep, as ADR-513 and ADR-583 do.

## Decision

- D1. `frankenstein` comes from RexRenatus/frankenstein at the 0.52.1 release plus one commit that
  takes `pastey` 0.2.3 for `paste`, by `rev`, kept fetchable by branch `deckstreak-pastey`.
- D2. The seven `uniffi` crates come from RexRenatus/uniffi-rs at the 0.29.1 release plus one commit
  that drops `uniffi_bindgen`'s unused `paste`, by `rev`, kept fetchable by branch
  `deckstreak-no-paste`; the version stays `=0.29.1`.
- D3. `rinja`, `rinja_derive` and `rinja_parser` come from RexRenatus/askama at the `rinja` 0.3.5
  release plus one commit that parses the config with `toml` 0.5, by `rev`, kept fetchable by branch
  `deckstreak-rinja-toml`.
- D4. Each fork's crates are already-locked successors, so no crate enters the lock. Each entry goes
  when a release of its parent, one this workspace can take, drops the crate; each fork owes one
  upstream pull request once this repository is green.

## Options rejected

| option | why not |
|---|---|
| an ignore entry per crate | an exception, not a fix |
| a narrowed `unmaintained` scope | an exception by another name |
| a `uniffi` upgrade | ADR-345 pins the generator; an upgrade is its own decision |
| removing the bot | its own decision; the fork cures the edge at less cost |

## Consequences

`uniffi_internal_macros`, which `uniffi_meta` names by path, comes from the same uniffi commit at
0.29.1, the version that release names, in place of the 0.29.5 the registry resolved (ADR-345); no
other locked version moves.

`allow-git` names three more sources (A1), and each fork's branch must stay fetchable. Read with
ADR-026, ADR-345, ADR-513, ADR-583 and ADR-058.

## What would make this wrong

- A release of `frankenstein`, of `rinja`, or of `uniffi` that ADR-345 admits drops its crate,
  and that parent's entries stay.
- A fork's branch or the fork goes, so its commit can no longer be fetched.
- A fork's commit moves more than its one dependency, so its crates no longer match the
  published release they name.
