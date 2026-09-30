---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The repository declares its formal-check settings in one file, judged against the development branch

## Context and Problem Statement

The formal checker for TLA+ models and Lean statements reads its settings from one file in the tree
it judges, at the head commit. With that file absent it refuses at once and no model is ever
checked (#468). Where do the settings live, and against which commit is a delivery judged?

## Decision Drivers

- A setting that guards against weakening must be reviewable in the pull request that sets it.
- A delivery is judged on what it changes, so its base is the branch it lands on.
- The file carries only what the checker reads, so no field is dead text.
- A value fixed at first commit (`k`, the axioms, the signers path) is a ratchet, so it is settled
  before the file merges.

## Considered Options (the alternatives it was chosen against)

- The repository carries its own settings file, judged against the development branch: chosen,
  because the file is reviewable here, its ratchet starts at its first commit, and the base is the
  branch every delivery lands on (#468).
- No file, so every formal check stays void and no model is ever checked: rejected, because a
  delivery whose surface owes a model could then never be checked, and each would stop before its
  push (#468).
- Settings supplied by the checker's own installation: rejected, because they are not available to
  the repository, and a change to them would not be reviewable in a pull request here (#468).
- Judging against `main`, the release branch: rejected, because it would judge every change since
  the last release instead of the delivery (#468).

## Decision Outcome

Chosen option: the repository carries `config/formal.json`, holding the fields the checker reads and
no other, with `k` 20, the budgets of SPEC-295's R1, the three core axioms, the trust file's path
and one model-checker slot. Formal checks are judged against the development branch's live commit.
The trust file itself is not part of this decision: it lands in a separate, owner-signed change.

### Consequences

- Good, because the checker reads the repository's settings, so a check runs over the tree and no
  longer refuses on a missing file.
- Good, because every change to `k`, the axioms or the signers path is a weakening the review sees.
- Bad, because a mutation row removed while its target file stays now needs the owner's signed
  ruling (SPEC-295 R4). Measured cost: over the development branch's whole history no merged pull
  request removed a mutation row whose target stayed, so no delivery so far would have been held.
- Bad, because until the first entry lands the checker has nothing to examine and reads the
  population as empty, which is a void and not a pass.

### Confirmation

SPEC-295's A1 to A3 (`scripts/tests/test_formal_config.py`).

## What would make this wrong

- A field the checker starts to read that the file lacks: it would refuse the file as incomplete.
  A1 pins the file to the fields R1 lists, so the change is an amendment and not silent drift.
- A value of `k` that is too small or too large for the development branch's pace of merges: it is
  a ratchet, so changing it is a signed ruling.

## More Information

SPEC-295, issue #468.
