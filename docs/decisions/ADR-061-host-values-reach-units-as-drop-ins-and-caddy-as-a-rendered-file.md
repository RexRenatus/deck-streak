---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Host values reach the public units as systemd drop-ins, and the Caddy block as a rendered file imported by one line

## Context and Problem Statement

ADR-032 keeps every deploy template valid as committed, with neutral values (an example release
root, UTC calendars, credential ids with no path, Caddy's own `{$VAR}` placeholders), "which the
private rail replaces at deploy", and it has the rail supply the Caddy block's values when it
installs the block. It did not decide how the rail replaces a unit's values, or how it fills the
block. Caddy substitutes `{$VAR}` from its own process environment whenever it parses its
configuration, at start and at every reload. How do the host's concrete values reach the installed
units and the Caddy block, so that what runs is what the gate judged, and nothing of the host's
Caddy changes but DeckStreak's block?

## Decision Drivers

- The file systemd loads should be the file the packs and the tests judged (ADR-032).
- A value the rail forgot to replace must be caught before a unit starts, not after.
- DeckStreak adds its site block to the host's Caddy configuration and nothing else: no setting of
  Caddy's own unit or environment changes for it (ADR-007).
- Every change has a one-step rollback (gate 2).

## Considered Options (the alternatives it was chosen against)

- Drop-ins for the units, and a rendered file plus one `import` line for Caddy: chosen, because the installed unit is byte for byte the tested template, `systemctl cat` shows the host's values in one named file, a rollback deletes that file, and Caddy's own unit and environment are never touched.
- Rewriting the unit files' text before install (with `sed` or `envsubst`): rejected because the installed file is then not the file the gate judged, and a value the rewrite missed is silent (ADR-032's consequence).
- Keeping full private copies of the units in the rail: rejected because they drift from the public templates the packs judge, and every template change must be made twice.
- Giving Caddy's own process the placeholders' values through its environment: rejected because it changes Caddy's unit for DeckStreak's sake, with a drop-in and a daemon reload, and makes every later start or reload of Caddy depend on DeckStreak's variables.
- Loading DeckStreak's block through Caddy's admin API: rejected because a configuration loaded that way is replaced by the next reload from the Caddyfile.

## Decision Outcome

Chosen option.
- **Units.** The rail installs each template from the release byte for byte into the system's unit
  directory, and writes one drop-in per unit, `<unit>.d/10-rail.conf`, holding every value
  `deploy/rail-contract.json` lists for that unit: the release root in `ExecStart=`, the environment
  file's path, the owner's zone in each `OnCalendar=` (reset, then set), and the vault writer's and
  the AI route's additions when those are enabled (SPEC-063, SPEC-065). `effective-check.py` refuses
  a unit whose effective configuration still carries a neutral value (SPEC-061 R7).
- **Caddy.** `deploy/scripts/render-caddy.py` replaces the block's three `{$DECKSTREAK_*}`
  placeholders with the host's values, refuses any `{$` left and any upstream that is not loopback,
  and writes the rendered block as its own file. The install adds one `import` line for that file to
  the host's Caddyfile, after `caddy validate` and `caddy adapt --validate` pass on a copy of the
  whole configuration; Caddy's graceful reload keeps the running configuration when the new one
  fails. The rollback removes the line first and the file second (an `import` of a missing file is an
  error), validates and reloads.
- This amends ADR-032's sentence on Caddy's placeholders: they stay in the committed template, valid
  as written, and the rail fills them by rendering the block at install, so Caddy's environment
  carries none of their values. The rest of ADR-032 stands.

### Consequences

- Good, because what runs is what was judged, and each host value is visible in one place per unit.
- Good, because Caddy's own unit and environment are never edited, and its configuration gains one
  line and one file, both DeckStreak's.
- Bad, because the rendered Caddy file holds the host's name, so it lives only on the host and in the
  rail, never in the repository.
- Bad, because a template change that adds a neutral value needs a matching line in the rail
  contract; A3 of SPEC-061 fails until it has one.

### Confirmation

SPEC-061's A3 to A5 and SPEC-062's A7; at gate 2, `effective-check.py` over every installed unit and
`caddy validate` on the copy, recorded privately.

## What would make this wrong

- Caddy's configuration on the host stops using the Caddyfile's `import` (for instance, it becomes a
  JSON configuration managed through the API). DeckStreak's block would then need that form's own
  mechanism, decided in a new ADR.
- A pack gains a way to judge a unit merged with its drop-ins as installed; the effective check could
  then be that pack's row.

## More Information

ADR-007; ADR-010; ADR-032 (amended in its Caddy sentence); ADR-038; SPEC-061; SPEC-062; the Caddy
documentation on `import`, `{$ENV}` placeholders, `caddy validate`, `caddy adapt` and `caddy reload`,
read through Context7; systemd.unit(5) on drop-in directories.
