---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Public text describes DeckStreak only

## Context and Problem Statement

The repository is public (CHARTER 11): everything pushed to it, from the tree to a commit message
or a pull request body, is published at once. SECURITY.md sends every vulnerability report to a
private advisory, never to a public issue or pull request. The scrubber's deny list keeps values
out: addresses, host names, secret names. DeckStreak runs on one small host shared with other
services, and a sentence can describe that host without naming a single value. What may public
text say about where DeckStreak runs?

## Decision Drivers

- A description of a host is reconnaissance whether or not it names a value: what else runs there
  and when, what the host can hold, what it exposes, and at which version.
- SECURITY.md keeps weaknesses out of public issues; the same fact in a document is just as public.
- Builders write most of the prose, so the rule must be one a builder can apply before a push and a
  check can read.
- DeckStreak's own design stays reviewable: its units, its budgets and the versions it pins are the
  product, and its code states them anyway.

## Considered Options (the alternatives it was chosen against)

- Public text describes DeckStreak only, and the host in one fixed form — chosen: one rule a builder can apply before a push and a check can read, and every fact of DeckStreak's own design stays public.
- A values-only scrub — rejected because the deny list matches literals, and a sentence that describes a host's capacity or its other services contains none.
- Case-by-case review of each passage — rejected because it rests on each reviewer's reading, and a passage one reviewer misses is published the moment it is pushed.
- Describing the host publicly, with the findings kept private — rejected because the public description is itself the reconnaissance the private findings protect.

## Decision Outcome

Chosen option. Public text means the tree, every commit message and every pull request body.

- It describes the host only as one small host shared with other services, within a stated budget
  (CHARTER 3), and it may state DeckStreak's own resource budgets (ADR-032,
  `deploy/host-budget.json`).
- It never names or describes another service on the host, the predecessor included as a running
  service: not its schedule, its users, its units, its ports or its services.
- It never states the capacity, load or state of a host that runs DeckStreak, or of the
  maintainer's own machines. The published specifications of hosted CI runners are public facts,
  and may be stated.
- It never describes a weakness of a host or of a service.
- It never states the version of a service reachable from outside the host.
- It may state a dependency version DeckStreak itself pins, and it may cite the predecessor as code:
  its functions, goldens and formulas, as `module.py:function` at its commit.
- A finding about a host goes to the maintainer's private notes; public text cites only the owner
  gate that tracks such findings (#167).

### Consequences

- Good, because a builder can hold each sentence against one short rule before pushing it.
- Good, because DeckStreak's own design stays whole in public: every unit, budget and pinned version of its own.
- Bad, because a reader cannot learn from the repository what else shares the host; a design that depends on it states its own requirement instead, such as a budget or a slot kept free.

### Confirmation

The public-prose check reads the tree, the commit messages and each pull request body for the shapes this rule forbids (capacity figures, another service's name or schedule, port and firewall exposure, the versions of externally reachable services) and refuses a match. The builder rule: before each push, a builder reads its added lines, its commit message and its pull request body for the same shapes, and scrubs the body with `scripts/public-scrub.py`.

## What would make this wrong

- A design cannot be reviewed in public without a host fact this rule withholds.
- DeckStreak gets a host of its own: the host's one public form changes, and the other rules stand.

## More Information

CHARTER 3 and 11; SECURITY.md; ADR-032; `scripts/public-scrub.py`; #167.
