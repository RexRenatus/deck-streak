---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The public achievement page is served by DeckStreak from a directory it can empty, so an unpublish and an erase withdraw it

## Context and Problem Statement

#156 asks for public achievement publishing: opt-in, scrubbed, an explicit allow-list of fields.
The predecessor composed a stats document and badge files (`publish.py:compose_stats`,
`publish.py:build_badges`) and checked them with `publish.py:scrub_public` before they left.
DeckStreak's erase must remove what DeckStreak made public, and an unpublish must withdraw it
(the W7 plan's binding decision 4). Where the page is hosted is the owner's question (#348). Where
does DeckStreak put the page, so that withdrawing it is possible at all?

## Decision Drivers

- Export and erase stay symmetric: whatever is published can be listed and withdrawn (SPEC-021).
- Off by default, and nothing but the allow-list leaves.
- No secret, address or forward-looking date on anything public.
- No new service holds the owner's data.

## Considered Options (the alternatives it was chosen against)

- DeckStreak writes static files into a directory it is configured with, and its own Caddy block
  serves them under `/public/`: chosen, because a withdrawal is a file removal DeckStreak performs
  and verifies.
- A public repository: rejected because history cannot be withdrawn, and that breaks erase symmetry.
- A third-party static host: rejected because a withdrawal would depend on another service's
  deletion and its caches.
- Pages inside the Mini App: rejected because its host carries `noindex` and every page there needs
  the owner's session.
- A public API route that renders from the database: rejected because every request would read the
  owner's tables, where a static file holds only what passed the allow-list.

## Decision Outcome

Chosen option: "static files in a configured directory, served by DeckStreak's own Caddy block",
because it is the only option in which every published byte is recorded where DeckStreak can list
it for export and remove it for an unpublish or an erase.

- **The page.** Typed pages whose fields are the allow-list; catalogs, study days and the scrubber
  refuse by name; a page with no wired section is not written.
- **The write.** One `BEGIN IMMEDIATE` transaction records each file with its hash; an unchanged
  page writes nothing.
- **The switch.** `public_achievements`, off by default; switching on is refused while the
  directory is unset; switching off withdraws every file and deletes the records.
- **The erase.** Both erase paths withdraw every recorded file first and refuse `withdraw_failed`
  before erasing anything when a file cannot be removed.
- **The serving.** `handle_path /public/*` with no fallback and its own content policy, so a
  withdrawn file answers 404.

### Consequences

- Good, because the owner's page lives and dies with DeckStreak's own records.
- Bad, because the page is only as public as the host: until the owner has a domain (#168) and
  answers #348, it is served on the Mini App's host, which asks search engines not to index it.

### Confirmation

SPEC-137 §3 (the switch, the job, the withdrawal and the erase criteria), §3a, and its rows in
`S13700-S13799`.

## What would make this wrong

- The owner answers #348 with a host DeckStreak does not run: a withdrawal would then need that
  host's deletion, and the choice would need its own decision about what an erase can promise.
- A page that must be indexed by search engines: it would need its own host (#168), outside the
  Mini App's `noindex`.

## More Information

SPEC-137, SPEC-021, SPEC-130, ADR-012, ADR-027, ADR-059, and the W7 schematic
`docs/schematics/w7-publishing-and-unpublishing.md`.
