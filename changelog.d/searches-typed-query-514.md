### Added

- Searches are DeckStreak's own: a new crate, `deck-streak-search`, reads a search once into a typed query under a closed grammar, with a skip profile and an operator profile, depth, length and arity limits, and refusals that name what is wrong and never carry the search. Its evaluator selects cards over a collection's rows in Rust, with no SQL and no I/O, and equals every case of the engine's search golden. A read-only parity tool compares it with the engine on a real collection and prints counts only (SPEC-514, ADR-532).

### Changed

- The skip day's configured search is parsed under the skip profile before it is wrapped, so a misspelled key now stops start naming `DECKSTREAK_SKIP_SEARCH`, and the holds are joined to it by structure; a proof in Lean shows no configured search selects a card past them (SPEC-514, ADR-532).
