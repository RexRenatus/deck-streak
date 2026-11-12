---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# The real-money rung is not built until the owner rules, and a census holds that no path posts

## Context and Problem Statement

The parity row `beeminder-money-rung` (#116) is the predecessor's top discipline rung: once a study
day it posts the previous day's reviews and defections to an outside pledge service, whose goals
carry pledges in real money (`pipeline_layers/discipline.py:DisciplineLayer._post_beeminder` and
`_beeminder_send`, predecessor `27ee2bc`). It posts nothing until a token is configured and its
setting is on. The issue calls the rung detached in the predecessor's production.

DeckStreak's rule is that coins are the only confiscable stake. A post to a do-less goal can make
the outside service charge the owner, and a charge in money cannot be reversed by a refund of
coins, so the rung sits outside every reversal ADR-103 and ADR-104 build. Money is the owner's
decision and an irreversible class.

Does W5 build the rung, and in what form?

## Decision Drivers

- Coins are the only confiscable stake, and no path moves real money without an owner ruling.
- A false fine is worse than a missed fine: every stake DeckStreak holds is revisable (ADR-104).
- SPEC-001 R1 and A2: every predecessor feature id stays in the parity matrix, built or excluded
  with its reason, and every exclusion carries an owner decision issue.
- A path that exists can be enabled by a setting or a mistake; a path that does not exist cannot.

## Considered Options (the alternatives it was chosen against)

- Build nothing that posts, hold the absence with a census test, and put the rung to the owner — chosen, because no code path can then move real money before the owner rules, and the parity row stays open for the owner's answer.
- Build the rung off by default behind a token and a setting, as the predecessor does — rejected because a setting is not a ruling, and a shipped path is one configuration away from an irreversible charge.
- Build it behind a compile-time feature that release builds leave off — rejected because the code would still be reviewed, tested and carried as if decided, and a build flag is as easy to flip as a setting.
- Build only the predecessor's warning that a suspended rail leaves a stake blind — rejected because with no rung there is no blind stake to warn about, and the warning alone would imply the rung exists.
- Exclude the parity row now — rejected because an exclusion carries the owner's decision (SPEC-001 A2), and the owner may want the rung.

## Decision Outcome

Chosen option: "build nothing that posts, with a census", held by SPEC-106.

- **What W5 builds.** No crate and no screen posts to an outside pledge service, and no setting
  enables one. SPEC-106 R25 and its criterion A31 hold the absence over every source file of the
  crates and the Mini App. The predecessor's `beeminder_posts` maps to nothing.
- **What stays open.** The parity row keeps its disposition and #116 stays open as the owner's
  question. When the owner rules to build it, a later SPEC builds the rung under that ruling and
  removes the census; when the owner rules against it, the row is excluded with the ruling as its
  reason.
- **What the other stakes keep.** Wagers, contracts, hard mode and the rail stake coins only, each
  refunded when its evidence fails (ADR-103, ADR-104).

### Consequences

- Good, because nothing DeckStreak ships can charge the owner money.
- Good, because the owner's answer, either way, lands on a clean slate rather than on a dormant
  path.
- Bad, because the discipline ladder has no top rung until the owner rules, so a parity row stays
  open past W5.
- Bad, because the census must name the service's host and route, and must be kept when a new
  surface appears.

### Confirmation

SPEC-106's criterion A31, `no_path_posts_to_a_pledge_service`, over every file under
`crates/*/src/` and `web/app/src/`.

## What would make this wrong

- The owner rules that the rung is wanted: the census goes, and a SPEC builds the rung with the
  owner's terms for enabling it.
- The outside service stops holding pledges in money: the rung would then stake nothing
  irreversible, and building it off by default would be the right call.

## More Information

Cites ADR-103 (the fine and its reversal), ADR-104 (a verdict is revisable), SPEC-001 (the parity
matrix) and SPEC-106. The owner's question is #116. The schematic is
`docs/schematics/wager-and-contract-lifecycle.md`.
