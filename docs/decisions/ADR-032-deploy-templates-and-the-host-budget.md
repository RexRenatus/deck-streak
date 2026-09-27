---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Deploy templates carry neutral, lint-valid values the private rail replaces, and DeckStreak's share of the host is 640 MiB with a ceiling per unit

## Context and Problem Statement

ADR-010 decides hardened systemd units per role, parameterised "by environment variables with
placeholder values", with `MemoryHigh` and `MemoryMax` from `deploy/host-budget.json`; ADR-007 adds
a Caddy block "parameterised by environment variables". The repository is public (CHARTER 11), so no
concrete host, path or secret may appear; but the durable-services and web-security packs judge the
committed files as written, and a placeholder such as `@ROOT@` is not a valid absolute path to
systemd. And no ADR gives the budget a number. How are the templates parameterised, and what is the
budget?

## Decision Drivers

- The packs read the templates as committed; a placeholder must still be a valid value.
- CHARTER 11: nothing private enters the tree; the private deploy rail holds the concrete values.
- The host: 1.9 GiB of RAM, the predecessor's unit and a co-hosted stack beside DeckStreak until
  cutover (ADR-011); disk is the binding limit (ADR-010).
- Each unit fails on its own ceiling rather than the kernel's global OOM killer choosing
  (durable-services `resources.budget`).

## Considered Options (the alternatives it was chosen against)

- Templates holding neutral values that are valid as written (an example release root under `/usr/local`, UTC calendars, credential ids with no path, Caddy's own `{$VAR}` placeholders), which the private rail replaces at deploy — chosen: every pack judges a real unit, and nothing private is committed.
- Committed concrete values — rejected because the host's paths, names and zone are private (CHARTER 11).
- A template engine that renders the units at deploy (Jinja, or `envsubst` over non-valid placeholders) — rejected because the packs could only judge the unrendered text, which is not a unit systemd would load, so the gate would judge nothing real.
- One slice whose `MemoryMax` caps the whole stack — rejected as the only control because a runaway job would then throttle the API and the bot inside the same slice; a ceiling per unit keeps each failure its own, and the per-unit sum is what the budget row checks.

## Decision Outcome

Chosen option. The budget, which `deploy/host-budget.json` records and every unit's `MemoryHigh=` and
`MemoryMax=` must equal:

| unit | memory_high | memory_max | why |
|---|---|---|---|
| `deck-streak-api.service` | 96M | 128M | an axum service bounded to 64 requests and 2 MB bodies |
| `deck-streak-bot.service` | 64M | 96M | one poll loop and one transport |
| `deck-streak-job@.service` | 320M | 384M | the sync's budget is 256 MiB of resident memory (ADR-022), a fifth below `MemoryHigh` |
| `deck-streak-alert@.service` (SPEC-031) | 32M | 48M | a shell script and one HTTPS request |
| `deck-streak-slo.service` (SPEC-031) | 48M | 64M | a standard-library Python evaluator over the journal |
| `deck-streak-memory-watch.service` (SPEC-031) | 32M | 48M | a shell script over the units' memory accounting |

The share is `"memory": "640M"`: the long-running units' ceilings (224 MiB) plus the largest oneshot's
(384 MiB) are 608 MiB, which fits with room to spare. Why 640 MiB: with the predecessor's ceiling and
the co-hosted stack beside it on a 1.9 GiB host, a larger share would leave the kernel no headroom
when every ceiling is reached at once; the memory watch measures the real figures from the first
deploy, and a change is a new ADR.

Caddy's native `{$DECKSTREAK_HOST}`, `{$DECKSTREAK_WEB_ROOT}` and `{$DECKSTREAK_API_UPSTREAM}` read the
unit environment Caddy runs with, which the private rail sets.

### Consequences

- Good, because the gate judges exactly the units that will run, and the public tree stays clean.
- Good, because a job's spike throttles and fails the job alone.
- Bad, because the rail must replace every neutral value; a missed one shows as a unit that cannot
  find its binary or a timer that fires in UTC, which the first start and the liveness job's drift
  check reveal.

### Confirmation

SPEC-032's acceptance tests; durable-services `resources.*` rows, enforced; the memory watch's first
week on the host (SPEC-031, W2).

## What would make this wrong

- The memory watch shows a unit living at its `MemoryHigh` in normal use (its ceiling is too low), or
  the host's available memory with both services running falls below the share (a resize is an
  owner decision, ADR-011).
- A pack gains a way to judge rendered templates, which would allow real placeholders.

## More Information

ADR-007; ADR-010; ADR-011; ADR-022; ADR-025; SPEC-032; `docs/schematics/deployment.md`; the
durable-services and observability packs.
