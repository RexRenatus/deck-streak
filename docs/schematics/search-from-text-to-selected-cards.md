# Schematic: a search, from its text to the cards it selects

Kind: data flow. Read at the delivery's base `bae990a2`, and at the pinned engine `65e053b8` for the syntax and the
predicates it ports (`rslib/src/search/parser.rs`, `rslib/src/search/sqlwriter.rs`,
`rslib/src/scheduler/timing.rs`). Decided by ADR-532; built by SPEC-514.

```mermaid
flowchart TD
  text["a search's text (a configured skip search, or a golden's search)"] --> limits{"within the limits? (length 1024 bytes, depth 8, arity 32)"}
  limits -- no --> refusal["a Refusal: TooLong, TooDeep or TooWide"]
  limits -- yes --> grammar["the closed grammar: terms, and, or, -, ( ), quotes, key:value"]
  grammar -- "a shape it cannot read" --> structural["a structural Refusal: Blank, UnopenedGroup, UnclosedGroup, EmptyGroup, UnclosedQuote, EmptyQuote, MisplacedOperator, MissingKey, BadEscape"]
  grammar -- "a key outside the grammar or the profile" --> named["a Refusal by name: UnknownKey, Unsupported(name), NotInProfile(operator), BadValue(operator)"]
  grammar --> profile{"Profile: skip or operator"}
  profile --> query["the typed Query: Term, Not, And, Or"]
  query --> skip["SkipQuery: And(configured, -is:new, -is:learn, prop:due=0, -is:suspended, -is:buried, -deck:filtered)"]
  skip --> skiptext["the wrapped text the engine runs: (configured) -is:new -is:learn followed by the holds"]
  query --> eval["eval: each term a predicate over one card and its note"]
  rows["Collection rows: decks, note types, notes, cards, review log"] --> eval
  day["EngineDay: days elapsed, next day, now (the study day crate's rule)"] --> eval
  eval --> order["the engine's order: a top-level deck name lists by deck, queue, due; else by card id"]
  order --> cards["the selected cards' indices"]
  structural --> start["at start: the settings refuse naming DECKSTREAK_SKIP_SEARCH, never its value"]
  named --> start
  refusal --> start
```

## The parity tool

```mermaid
flowchart LR
  source["a collection file"] -- "Db::open_foreign_read_only (mode=ro, query_only)" --> read["the read: decks, note types, fields, templates, notes, cards, review log"]
  read --> dseval["eval over the read rows"]
  source -- "a copy, never the source" --> scratch["the scratch copy"]
  scratch --> engine["the engine's day and its search"]
  engine --> dseval
  engine --> compare["the two id sets"]
  dseval --> compare
  compare --> counts["match=N engine-only=N ds-only=N, and nothing else"]
```

## What each edge carries

- **Text to the limits.** The byte length is read before any character: a search past 1024 bytes is refused whole.
  The depth is counted at each `(` and the arity at each group's end, so the parser's recursion is bounded by the
  depth limit.
- **The grammar to the profile.** Each `key:value` is matched against the grammar's own keys first (`deck`, `tag`,
  `is`, `prop`, `card`, `note`, `added`, `rated`); an engine key the grammar does not implement is refused by its own
  name from a fixed list; any other key is a field search only under a profile that admits field searches, and an
  unknown key everywhere else. No refusal carries the search's text.
- **The query to the skip wrap.** The configured query is one operand of a top-level `and` whose other operands are
  the holds; the wrapped text parses back to the same query (`crates/search/tests/holds.rs`), and the Lean entry
  `formal/lean/Formal/SearchHolds.lean` proves no configured query selects a card past the holds.
- **Rows and the day to the evaluator.** The evaluator reads the rows it is given and the day it is given; it opens
  nothing and reads no clock. Names are matched in Rust: the engine's `LIKE` for text and field values, its globs
  with Unicode case folding for deck, note type, template, field and tag names.
- **The source to the parity tool.** DeckStreak's side reads the source only through the kernel's read-only open;
  the engine runs only on a scratch copy, so the source is never opened writable and never written.
