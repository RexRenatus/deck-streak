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
- `k` and the signers path are fixed at first commit and the axioms can only shrink, so they are
  settled before the file merges.

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
- Good, because a change to `k`, an added axiom or a changed signers path is a weakening the checker
  reports, so the review sees it.
- Bad, because a mutation row removed while its target file stays now needs the owner's signed
  ruling (SPEC-295 R4). Measured cost: over the development branch's whole history no merged pull
  request removed a mutation row whose target stayed, so no delivery so far would have been held.
- Bad, because until the first entry lands the checker has nothing to examine and reads the
  population as empty, which is a void and not a pass.

### Confirmation

SPEC-295's A1 to A3 (`scripts/tests/test_formal_config.py`).

## What would make this wrong

- A field the checker starts to read that the file lacks: if it starts to require that field, it
  would refuse the file as incomplete. A1 pins the file to the fields R1 lists, so the change is an
  amendment and not silent drift.
- A value of `k` that is too small or too large for the development branch's pace of merges: it is
  a ratchet, so changing it is a signed ruling.

## More Information

SPEC-295, issue #468.

## Addendum, 2026-09-30: the repository names the checker's toolchain by identity (#504)

The formal checker carries its own tool pin, and a repository may name that pin's identity in
`config/formal.json` as `"toolchain": {"identity": "<64 lowercase hex digits>"}`. The decision: the
repository names the identity and commits no pin file, so the checker uses its own pin when the two
are equal, refuses a checker built with another pin as drift, and refuses a tree that names an
identity and also commits a pin file. The considered options:

- Name the checker's toolchain by its identity in `config/formal.json`: chosen, because the pin
  stays in one place, the checker, and the repository states which build it was written against, so
  a change of build is a reviewed change here (#504).
- Commit a pin file beside the settings: rejected, because it duplicates what the checker already
  carries and drifts from it silently, and a tree that commits one while naming an identity is
  refused as two sources (#504).
- Name nothing: rejected, because every TLA+ and Lean check then stays at its tool stage and no
  model is ever checked (#504).
- Name a version string instead of a digest: rejected, because a version cannot tell two builds of
  one version apart (#504).

Cost: a checker rebuilt with another pin refuses every check as drift until this field is changed,
and that change is reviewed here. Confirmation: SPEC-295's A1, A3 and A6 to A8; the test reads the working tree, so an untracked pin file turns it red, which is fail-closed by design.


Addendum, round 2 (#504). Confirmation: SPEC-295's A7 reads the pin path as the tree stores it,
so a link at the pin path is a pin file and is never followed, and under A8 every refusal of the
settings file (a link at the file or at its directory, through which the tree holds no file, an
absent file, bytes that are not JSON, bytes that are not UTF-8, a nesting past the
parser's depth and an integer past its digit limit) fails by assertion and never as an error. The
test reads the working tree, so an untracked file, directory or link on disk turns it red, which is
fail-closed by design. A committed file missing from the work tree is only dirty locally and is
never judged here, because a CI checkout is the committed tree.

Addendum, round 3 (#504). Confirmation: SPEC-295's A8 judges every path the checker reads as the tree
stores it, at every component from the repository root to the file: a link at any of them, of any
kind (relative, absolute, a chain, to a directory, dangling, to itself), holds no file for the
checker and is refused by assertion naming the component, by one walk both test modules use. The
population is derived from the components and the kinds, so a new or deeper path joins it by itself.
The one exception is the pin path's own last component, which is the pin file, a second source, and
is listed and never followed. Chosen against refusing only the settings file's directory, which
closes the one shape measured and leaves every other component open.
