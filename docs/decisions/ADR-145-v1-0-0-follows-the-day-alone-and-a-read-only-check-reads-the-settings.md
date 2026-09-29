---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# v1.0.0 follows the day alone, and a check that cannot write reads the repository's settings

## Context and Problem Statement

#64 asks for v1.0.0 tagged on main and deployed by `deploy/deploy.sh`, with CI green on dev and
main and secret scanning, push protection and Dependabot on. ADR-010 and ADR-017 make a release a
tag on main, and ADR-034 keeps main from merging back. The repository's settings are the owner's.
When is v1.0.0 cut, and how is "the settings are on" proved without changing them?

## Decision Drivers

- v1.0.0 claims the product runs alone; that claim must be true when it is tagged.
- Settings are the owner's: a criterion reads them and never changes them.
- No standing credential with write reach over the repository.

## Considered Options (the alternatives it was chosen against)

- The release after the day alone, then a read-only check: chosen, because the tag follows the
  evidence, and the check can only read. The release PR goes dev to main, the tag is on main, the
  deploy is from the tag, and the check (`gh api` GET only) reads the tag, CI and the three
  settings.
- Tagging before the cutover: rejected because v1.0.0 would claim a product that had not yet run
  alone.
- A script that turns the settings on: rejected because the settings are the owner's, and a writing
  script needs an administrator's token.
- A CI job with an administrator's token: rejected because it is a standing credential with write
  reach over the repository, in CI.
- Checking the settings by hand in the web interface: rejected because nothing records it; the
  check's output is the evidence.

## Decision Outcome

Chosen option: v1.0.0 after the day alone, and the read-only release check. SPEC-145 holds it.

### Consequences

- Good, because the check refuses what it cannot read, so an unread setting never passes.
- Bad, because a setting the owner has not turned on holds the release until they do.

### Confirmation

SPEC-145's criteria: the GET-only census of the check, its refusals, and `RELEASING.md`'s order.

## More Information

#64, #164, ADR-010, ADR-017, ADR-034, ADR-035, SPEC-033, SPEC-034, SPEC-145.
