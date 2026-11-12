# ADR-532: Searches are read once into a typed query under a closed grammar, and evaluated over rows in Rust

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-514 |
| builds on | ADR-029, ADR-515, ADR-516 (none amended) |
| measured at | the delivery's base `bae990a2`, and the pinned engine `65e053b8` |

## Context

DeckStreak passes search text to the engine and has no reading of its own: the skip day's configured search is
checked by a balance count and wrapped as text (`crates/ingest/src/skip.rs:62-108`), so a misspelled key reaches the
engine as a field search that selects nothing, and nothing in DeckStreak can say what a search selects without the
engine. The engine's search has goldens over synthetic collections (G-L6-1, SPEC-498). This ADR decides where the
search lives, what its grammar admits, how a refusal is named, how the skip's holds are joined, how the evaluator
matches and orders, and how a real collection is compared without being written.

## D1. Where the search lives, and what it was chosen against

- **Chosen: a new crate, `deck-streak-search` at `crates/search`, depending on the study day crate alone.** The
  search is a pure function of text, rows and a day; a crate with no kernel edge can be linked by either client, and
  ingest depends on it.
- **A module of ingest:** lost, because ingest links the engine, the kernel's database and the runtime, so no client
  could link the search, and the grammar would sit beside the code it is meant to replace.
- **A module of the kernel:** lost, because the search is one context's reading of the engine's language, not a
  concept two contexts share (`docs/CONTEXT-MAP.md`'s kernel rule).
- **The engine core:** lost, because it holds the engine itself, which the evaluator is held against, not built on.

## D2. One typed query, within limits

- **Chosen: `Query` is `Term`, `Not`, `And` and `Or`, with `and` binding before `or` and a group keeping its own
  node; every parse is held to 1024 bytes, 8 nested groups and 32 terms a group.** The shape is the engine's SQL
  precedence, so a query means what the engine would select. The limits bound the parser's recursion and every
  refusal is whole: a search past a limit is refused, never truncated.
- **A flattened `and` over groups:** lost, because the skip's wrapped text must parse back to the wrapped query
  exactly, which a flattening would break for a configured `and`.

## D3. A closed grammar, its profiles, and refusal by name

- **Chosen: the grammar's own keys are `deck`, `tag`, `is`, `prop`, `card`, `note`, `added` and `rated`; any other
  key is a field search only under a profile that admits field searches, and unknown under every other; each engine
  key the grammar does not implement is refused by its own name from a fixed list.** A `Refusal` carries no text of
  the search: a key the user typed is part of the search, so an unknown key is refused unnamed, and only names from
  the grammar's fixed lists appear in a refusal.
- **`Profile::SKIP` admits text, `deck`, `tag`, `is`, `prop`, `card` and `note`; `Profile::OPERATOR` admits every
  operator.** The skip selects a study day's due review cards and reads no field and no history, so a field-like key
  in its setting is a misspelling, and start refuses it naming `DECKSTREAK_SKIP_SEARCH` (A2). On the golden skip
  searches the skip profile agrees with the one-expression check (A1).
- **Admitting every key as a field search everywhere:** lost, because no key could then be unknown, and a misspelled
  skip search would keep selecting nothing in silence.

## D4. The skip's holds are joined by structure

- **Chosen: `SkipQuery` parses the configured search under the skip profile and joins it and the six holds as the
  operands of one `and`; the text the engine runs is the predecessor's wrap followed by the holds, byte for byte,
  and parses back to that query.** A configured text that closes the wrap's group is refused at the parse, before
  any wrap. Lean's `holds_inescapable` proves the joined query selects no card past a hold for any configured query;
  the Rust property checks the text reads back as the query over seeded random predicates.
- **Keeping the balance check:** lost, because it admits unknown keys and proves nothing about what the wrapped text
  means.

## D5. The engine's day from the study day's rule

- **Chosen: `EngineDay::at` counts the calendar days from the creation's local date to the study day of now with
  `deck-streak-study-day`'s rule, and the next rollover from the same study day.** That is the pinned engine's
  computation for a collection whose creation offset is set, and both timing goldens hold it (A3). The golden's own
  cases are evaluated at the instant the driver's run gave the engine: its time of day read in the host's zone.

## D6. Matching and order in Rust

- **Chosen: text and field values match by the engine's `LIKE` (ASCII letters folded); names by its globs with
  Unicode case folding; the order is the engine's for the goldens' shapes.** Names are never compared in SQL, since
  the collection's name columns use a collation SQLite does not ship.

## D7. Kani: VOID until a run is pinned

- **Chosen: K-operator-profile is VOID, unratcheted, and nothing runs, until the formal checker pins a Kani run for
  this tree.** The profile's closure is held meanwhile by its Rust census (`crates/search/tests/profile.rs`).

## D8. The parity tool reads only through the read-only open

- **Chosen: `search_parity` reads the source through `Db::open_foreign_read_only`, runs the engine only on a
  scratch copy, refuses a scratch path that is the source, and answers three counts.** The engine opens a collection
  writable and, in client mode, rewrites a configured offset, so it never touches the source; counts carry no card,
  deck, field or search text.

## Consequences

- A skip search with a misspelled key now stops start; one that the old check admitted and the skip profile refuses
  is a configuration error the owner sees at once.
- `SearchRefusal` gains `OutsideTheSkipProfile`.
- The context map gains the crate, ingest's edge to it, and its edge to the study day crate.
