---
status: accepted
decision-makers: "the DeckStreak architect"
---

# A census holds the architecture code map to the crate folders, and every Telegram surface is named beside deprecated

## Context and Problem Statement

ADR-589 made the web app (`web/app`) and the universal iPhone and iPad app (`ios/`) DeckStreak's
surfaces and deprecated the Telegram Mini App and bot, and its consequences name the architecture
among the pages that describe it. At dev at the cut, ARCHITECTURE.md's code map named 24 of the 38
crate folders and had no `ios/` row; it called the two Telegram surfaces DeckStreak's surfaces and
drew them; docs/CONTEXT-MAP.md described identity, notifications, the API, the bot and the front
end in Telegram terms; and no test held either page to the tree or to the deprecation
(SPEC-623 section 1). How should the two pages be held true?

## Decision Drivers

- The code map is a reader's first map of the tree, so a folder it omits is a folder a newcomer
  cannot find.
- ADR-589 keeps the Telegram code in the tree until its removal delivery, so the pages must still
  be able to name it.
- The context map is binding and read by several tests (SPEC-623 F5): its names, edges and
  register cannot move in a documentation change.
- No install runs a Telegram surface (ADR-601).

## Considered Options (the alternatives it was chosen against)

### D1, how the code map is held to the crate folders

- Chosen: a census reads the crate folders from the file system and the code map's first-cell code
  spans, brace groups expanded, and refuses a folder named zero times or twice, a named folder the
  tree lacks, and a named path that is absent without a planned row in the map's fence.
- Against: generate the code map from the fence, because it loses the grouping a reader needs (ten
  domain contexts on one row, the engine's three crates on another).
- Against: fix the text with no test, because that is how 14 crates drifted unseen.

### D2, how a Telegram mention is judged

- Chosen: by unit, a table row, a list item, a paragraph or a fence row's description: a unit that
  names the Mini App, the bot, Telegram or `initData`, with code spans set aside, must also hold the
  stem `deprecat`.
- Against: remove every Telegram mention, because that hides code still in the tree that ADR-589
  keeps until its removal delivery.
- Against: a bare word ban, because it refuses kept identifiers such as `miniapp` and
  `deck-streak-bot`, which ADR-589 leaves to a rename delivery.

### D3, what the drawing shows

- Chosen: only what an install runs. The two apps are drawn with their edges into the service:
  `web[web app] --> api`, backed by the code map's `crates/api/` row ("the HTTPS adapter") and the
  Boundaries line "reaches the backend only through the HTTPS API under `/api` on its own origin";
  and `ios[iPhone and iPad app] --> api`, backed by ADR-335's "calls to the service go through the
  Rust core over FFI". Every other node and edge stays as it was.
- Against: draw the deprecated surfaces dashed, because no install runs them (ADR-601), so a
  dashed node would draw a process that does not exist.

### D4, what may change in the context map

- Chosen: descriptions change; names, path columns, edge lists and the ownership register stay
  byte for byte, so every test that reads them reads the same rows.
- Against: rename `miniapp` and `ios-harness` now, because ADR-589 leaves renames to their own
  delivery and the names feed labels and tests.

### D5, a formal model, decided by surface

- Chosen: none, because two documents and a text census have no protocol, concurrency or state to
  model.

## Decision Outcome

Chosen options: D1 to D5 as each "Chosen:" bullet states, because together they keep the code map
equal to the tree both ways, keep the deprecated code nameable, and move nothing a binding reader
holds.

### Consequences

- Good, because a crate folder added without its code-map row reads red by name.
- Good, because a new sentence that presents a Telegram surface as current reads red by its unit.
- Bad, because a pull request that adds a crate folder now also edits ARCHITECTURE.md.
- Bad, because the unit rule passes a long unit that names a deprecated surface and a current one
  together; the units on both pages are kept short.

### Confirmation

`scripts/tests/test_architecture_map.py` holds each decision, with a planted refusal per rule
(SPEC-623 section 3), and the band S62300-S62399 proves its killers in CI.

### What would make this wrong

- The removal delivery takes the Telegram code out: the deprecated mentions then leave with it,
  and D2's rule still holds over what remains.
- A crate folder that is not a crate (a fixture folder under `crates/`): D1 would then demand a row
  for it, and the census would need the workspace's member list instead.

## More Information

SPEC-623; ADR-589; ADR-601; ADR-335; ADR-336; ADR-002.
