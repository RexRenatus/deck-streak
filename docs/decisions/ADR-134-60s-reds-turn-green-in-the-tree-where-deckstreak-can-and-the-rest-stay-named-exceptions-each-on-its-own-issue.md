---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# #60's reds turn green in the tree where DeckStreak can make them so, and the rest stay named exceptions, each bound to its own issue

## Context and Problem Statement

#60 asks for every box-run pack to be enforced (SPEC-056, ADR-069). Most packs wait on the
delivery that gives them a subject, and each is named by its issue. Some reds that #60 owns are
different: the pack reads a declaration no public file may carry, reads a policy where DeckStreak
does not put one, reads text DeckStreak must not change, or carries rows written for a kind of
repository DeckStreak is not. Which of #60's reds does the delivery turn green in the tree, and
what does it do with the rest?

## Decision Drivers

- Public text names packs, never a row id, a probe path or the private wiring (ADR-059, SPEC-056).
- No exclusion is silent: every one names its reason and an issue.
- An exception must not read as a green: the box run keeps it as an expected red with its issue.
- DeckStreak's own properties are held by DeckStreak's own tests wherever a pack's row cannot run.

## Considered Options (the alternatives it was chosen against)

For the threat model's trace:

- Plan the STRIDE model and keep its trace a named exception on its own issue: chosen, because
  the model and its citations (`path:line:quote` spans, declared by the front matter the
  cyber-pipeline pack reads) are checked in the tree, and nothing of the hub is published.
- Carry the pack's row identifier in public front matter: rejected because it puts a hub identifier
  in a public repository.
- Exclude the trace silently: rejected because the house forbids an exclusion with no record.

For the strict script policy:

- A named exception until the packs read the page's own policy: chosen, because the Mini App's
  static build declares its script policy in the page, where the packs do not look today.
- Template the build's script hashes into the Caddy block: rejected because the hash list would live
  in two places and drift at every build.
- Allow `'unsafe-inline'`: rejected because it would admit any injected script.

For the rest:

- The license placeholders stay a named exception: chosen, because they are the AGPL text's own
  appendix, which DeckStreak must carry unchanged.
- The ledger-sqlite pack's five blocking rows stay a named exception: chosen, because their forms
  cannot run in a consuming repository, and DeckStreak holds four of their properties with its own
  tests (every table `STRICT` with `created_at`, concurrent writers serialised, one sync at a time,
  and an edited applied migration refused).
- Turn those packs off for DeckStreak: rejected because an off pack examines nothing, which the box
  run already reads as no verdict.

## Decision Outcome

Chosen option: "green in the tree where DeckStreak can make it so, and four named exceptions, each
bound to one DeckStreak issue filed when the build lands", because every red is then either gone or
stated with its reason and its issue, and no public file carries a hub identifier.

- **Green in the tree.** The STRIDE model and its checker (`scripts/threat_model.py`), the page
  template's language and direction, and the kernel's test that an edited applied migration is
  refused.
- **Named exceptions.** The threat model's trace, the strict script policy, the license
  placeholders, and the ledger-sqlite pack's five rows. Each has its issue, filed at build, and
  the pack change that would turn it green goes to the pack's authors as feedback.
- **Still pending.** The pack that examines nothing stays pending on its own issue, filed at build,
  and the pack that #61 enforces stays pending on #61 (W8).

### Consequences

- Good, because #60 closes with every pack enforced or named, and no red counted as green.
- Bad, because four exceptions outlive #60 until their packs change; each issue stays open until
  then.

### Confirmation

SPEC-134 §3 (the threat model, the page template and the migration criteria), §3a, and its rows in
`S13400-S13499`.

## What would make this wrong

- A pack that starts reading a neutral declaration, the page's own policy, or a consuming
  repository's form: its exception turns green, and the box run's expectation becomes stale, which
  the run refuses by name until the expectation is removed.
- A red that DeckStreak could fix in the tree after all: it moves from the exceptions to the green
  list.

## More Information

SPEC-134, SPEC-056, SPEC-030, ADR-004, ADR-030, ADR-056, ADR-059, ADR-069.
