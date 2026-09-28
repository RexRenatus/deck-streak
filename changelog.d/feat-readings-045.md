### Added

- The readings' day set. Each study day, every reading topic's new cards come from Anki's own queue
  of today's new cards: one query per top-level deck, so child decks share their parent's new-card
  budget, and each card attributed to its original deck. Topics come from a private taxonomy file,
  and each topic ends the day in exactly one state: no new cards, could not tell (with a class and a
  closed reason), or paused after two study days without study. The readings resolve only after a
  sync that succeeded, however old the collection file is, and a resolution has a 30-second budget.
  Each run and each topic's state are exported and erased with the rest of the owner's data.
