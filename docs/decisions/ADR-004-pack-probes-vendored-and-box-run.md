---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Run the packs' probes from a vendored copy in CI, and the phxd-built packs on the box

## Context and Problem Statement

Every pack DeckStreak consumes must be wired into its gate or CI, and green (the definition of
done). The packs live in the owner's private phoenix-v2 repository. Most carry a standard-library
Python probe that judges any tree through `--root`; ten (greenfield, seo-pipeline, web-launch,
vibecode-polish, ux-laws, ui-styles, web-security, cyber-pipeline, auth, ledger-sqlite) are built
into `phxd` and cannot run outside it. The owner decided that the pack runner becomes its own
public open-source repository, which DeckStreak's CI will install at a pinned version; that is a
separate workstream, and until it exists the checks run where they can.

## Decision Drivers

- Public CI may hold no secret, so it cannot clone the private phoenix-v2 repository.
- A pack whose subject is not built yet reads VOID, which must never pass as green.
- A vendored copy must be byte-identical to a named commit, so a stale copy names itself.

## Considered Options (the alternatives it was chosen against)

- Vendor the standard-library probes and their pack data, pinned to one phoenix-v2 commit, and run every row with `scripts/pack-rows.py` against a wiring manifest; run the phxd packs on the maintainer's box with a phxd built from the same commit — chosen: CI proves every vendorable row on every pull request, and nothing is skipped silently.
- Reference phoenix-v2 by path at a pinned commit — rejected because CI cannot read a private repository and a path works only on the box.
- Build phxd in CI — rejected because it needs the private source and a secret to fetch it.
- Wait for the open-source runner — rejected because the skeleton would ship with no pack checks at all.

## Decision Outcome

Chosen option. The four methodology probes sit in `scripts/` (their rows name that path). Every
other probe, and each pack's `SKILL.md`, `checks.json`, data and templates, sit in `.packs/`,
without the packs' `examples/` (phoenix-v2's dogfood, not DeckStreak's subject).
`.packs/VENDORED.json` records the source commit and each file's sha256, and a test holds the
copy byte-identical. `.packs/wiring.json` gives each pack a state (enforced, pending, deferred or
phxd), and each pending or deferred pack or row names the issue that builds its subject.
`scripts/pack-rows.py` runs every tree row, fails on RED and ERROR always and on VOID once a pack
is enforced, and refuses a wiring that forgets a vendored pack. The phxd packs are run by
`scripts/box-packs.sh` on the maintainer's box before each merge into `dev`, and their verdicts
are posted on the pull request. When the open-source runner is published, one pull request
replaces `.packs/` and `box-packs.sh` with the runner at a pinned version.

### Consequences

- Good, because 26 packs' rows run in public CI from the first commit.
- Good, because a pending pack is listed with the issue that will enforce it, so the definition of done's 'every pack wired' is a count, not a belief.
- Bad, because re-pinning to a newer phoenix-v2 commit is a manual copy (one pull request).
- Bad, because the phxd packs are checked on the box, not in CI, until the runner exists.

### Confirmation

`python3 scripts/pack-rows.py` in `scripts/check.sh`; `python3 -m unittest scripts/tests/test_vendored_packs.py` holds `.packs/` equal to `VENDORED.json`.

## What would make this wrong

- A probe changes meaning between phoenix-v2 commits in a way the pin hides (re-pin when a fix lands, as train 84's web-security fixes will).
- The open-source runner exists: this ADR is then superseded.

## More Information

The seat `roles/deckstreak`; `.packs/VENDORED.json`; `.packs/wiring.json`; the addendum's pack runner note.

Superseded in part by ADR-056 (2026-09-28): the owner decided the packs stay box-only and no pack
runner is published, so this arrangement is permanent, not "until the runner exists".
