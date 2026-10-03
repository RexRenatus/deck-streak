# Schematic: what the table censuses read, and how the settle census judges the owner's own use

Kind: data flow. Read at DeckStreak dev `9affffe6f`. Decided by ADR-325 and ADR-197 (round 9); built
by SPEC-324.

## The shared reader, called by each table census

```mermaid
flowchart TD
  members[every crate under crates/] --> census[a census: ledger, xp table or wallet]
  census --> own[its own reader: each line, or one lexer, the name as written]
  own -->|names the table outside the owner| refusedOwn[refused: names the table]
  own -->|files it named| reader[table_census.rs]
  members -->|src files outside the owner, not already named| reader
  reader --> lex[lex once, comments dropped]
  lex --> decode[decode each literal: escapes, line continuation, raw hashes, byte, C, char]
  lex --> words[words, and the words inside stringify!]
  lex --> includes{include!, include_str!, include_bytes! or #91;path#93;}
  includes -->|one literal path, a Rust file| lex
  includes -->|one literal path, another file| piece[one piece]
  includes -->|joined onto OUT_DIR| disclosed[counted, disclosed by #35;585]
  includes -->|any other path| refusedInclude[refused: includes a file the census cannot name]
  includes -->|a literal path it cannot read| refusedRead[refused: cannot read it]
  decode --> pool[one pool, workspace-wide, ASCII case-folded]
  piece --> pool
  words -->|a word holding the name whole| pool
  pool --> assemble{a start piece, exact middle pieces and an end piece cover the name?}
  assemble -->|yes| refusedSpell[refused: every file holding a piece on a covering path]
  assemble -->|no| quiet[nothing]
  lex --> concat{a concat! with an argument it cannot read?}
  concat -->|beside a piece ending with a proper prefix or starting with a proper suffix| refusedJoin[refused: joins a value the census cannot read]
  reader -->|prints its examined counts| census
```

The assembly is a reachability over the positions of the name: position `i` is reached when a piece
ends with the name's first `i` characters, or a reached position `k` is followed by a piece equal to
the characters from `k` to `i`; the name is covered when a reached position, or the start, is
followed by a piece that starts with the rest. A piece may serve more than once, and pieces join in
any order, so the rule holds `concat!`, `+`, `format!`, constants joined in one file, across files
and across crates, without a parser.

## The settle census's owner branch

```mermaid
flowchart TD
  rustc[a use of settle rustc reports, with its spans] --> package{the package cargo compiled it in}
  package -->|not progression| before[round 6 to 8 rules, unchanged]
  package -->|progression| kind{its target is a test, a bench or an example?}
  kind -->|yes| accepted[accepted]
  kind -->|no| innermost[the innermost span in the tree: its file and byte offset]
  innermost --> usedecl{inside a use declaration, read by the shared lexer in bytes?}
  usedecl -->|yes, an import or a re-export| followed[accepted here; its callers are judged where rustc reports them]
  usedecl -->|no| admitted{its file in OWNER_ADMITS?}
  admitted -->|yes| accepted
  admitted -->|no| refused[refused: calls settle inside progression's own code]
```

A re-export a macro of progression writes in its own body is reported at the macro's definition,
inside its `use`, so it is followed. A re-export whose path the macro takes as an argument is
reported at the macro's call, outside any `use`, so it is refused. `OWNER_ADMITS` admits no file.
