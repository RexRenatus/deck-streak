---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat, ruling 99 on the design pass for #460 and #445"
---

# A census refuses a reserved name its literals can assemble

## Context and Problem Statement

Three censuses hold a table to its owner: the ledger census (`xp_ledger`), the xp table census
(`xp_settlement`) and the wallet census (`coin_ledger`). Each searches crate sources for the name as
written, so a name joined from literals (`concat!("xp_", "ledger")`), spelled with an escape
(`"xp\x5fledger"`), in another case, from a constant in another file or crate, or through an
included file passes every census (#460). How does a census refuse every such spelling outside the
owner without refusing what no one joins, and without a parser?

## Decision Drivers

- A census is test code: it may add no dependency edge to a crate (ADR-197 round 3 ruled out `syn`
  for the settle census on the same ground).
- Each census's existing reader and refusals are evidence already landed; removing or narrowing one
  is a weakening.
- A census must fail closed: what it cannot read is refused, not passed, but a refusal of the real
  tree is a red build for every pull request.
- The real tree measured at dev `9affffe6f`: 2 `concat!` calls outside the owners, each with an
  `env!` argument beside pieces that are no part of a reserved name; every include has a literal
  path.

## Considered Options (the alternatives it was chosen against)

- One shared reader that decodes every literal, follows includes by literal path, pools every piece
  outside the owner workspace-wide in ASCII case, and refuses when the pool can assemble the name,
  added beside each census's own reader (chosen).
- A follower that joins only adjacent literals and the arguments of one `concat!`, in order: lost,
  because it misses `format!`, `+` joins, constants joined in another file or crate, and pieces
  written in another order.
- Refusing every `concat!` with an argument that is not a literal: lost, because it refuses both
  real-tree calls (`crates/api/src/health.rs`, `crates/bot/src/commands.rs`), which join no
  reserved name.
- A `syn` parser that evaluates constant expressions: lost, because it is a new dependency edge for
  test code, and it still could not see a join across crates or through an include.
- Replacing the ledger and wallet censuses' line reader with the new lexer: lost, because the line
  reader refuses a name inside a block comment and the lexer drops comments, so the swap would
  narrow an existing check.
- A pool per crate rather than workspace-wide: lost, because a `pub const` piece in one crate joined
  in another is a real shape, and the wider pool measured no false refusal on the real tree.

## Decision Outcome

Chosen option: "one shared reader, added beside each census's own", because one rule over a pool of
decoded pieces refuses every join shape at once, and a pool needs no parser.

- `tools/table-census/table_census.rs`, included by path from the three censuses (ADR-029's
  precedent), lexes each file once by byte offset, drops comments (nested blocks too), and decodes
  every literal as rustc does. The words inside `stringify!` are pieces. Numbers, `true` and `false`
  are pieces, since `concat!` joins them.
- `include!`, `include_str!`, `include_bytes!` and `#[path]` are followed by their literal path,
  relative to the including file's folder, as rustc reads each: a file `include!` or `#[path]` reads
  is lexed as Rust, and a file `include_str!` or `include_bytes!` reads is one piece. A path
  joined onto `env!("OUT_DIR")` is counted and disclosed (#585): build-script output is a route the
  reader does not reach, and the settle census's own tests plant that shape. Any other non-literal
  path, and a literal path the reader cannot read, is refused by name.
- The pool holds the pieces of every file outside the owner that the census's own reader did not
  already refuse, case-folded in ASCII. The name is covered when a piece ends with its first `i`
  characters, pieces equal its next characters, and a piece starts with its rest, or when a piece or
  a word holds it whole. A piece may serve more than once. Every file holding a piece on a covering
  path is refused: `{file} spells {TABLE} from literals, joined or in another case, and only
  {OWNER}'s code may`.
- A `concat!` with an argument that is not a literal, a nested `concat!` or a `stringify!` beside a
  piece that ends with a proper prefix of the name, or starts with a proper suffix of it, is refused:
  `{file} joins a value the census cannot read beside part of {TABLE}, and only {OWNER}'s code may`.
  One character suffices, and the threshold is never raised without a ruling (ruling 99, Q-j).
- Each census calls the reader once, after its own walk of the crates, as a statement of its own,
  and adds its refusals; it prints the reader's examined counts.

### Consequences

- Good, because every join shape the issue names is refused by one rule, and the reader is one file
  every census tests.
- Good, because the existing readers and their refusals stand, so no landed evidence moves.
- Bad, because the rule over-approximates: pieces that only could join are refused, and a piece of
  one character beside an unread `concat!` argument is refused. The real tree measured none.
- Bad, because four routes stay beyond the reader (#585): a whole name held by the owner's exported
  constant and used through a renamed re-export, a procedural macro's join, build-script output
  under `OUT_DIR`, and Rust outside `src`.
- Neutral, because the reader runs over files the census already walks, so its cost is one more
  lexing pass of text.

### Confirmation

`crates/progression/tests/ledger_census.rs` (A1, A4, A5), `crates/progression/tests/xp_census.rs`
(A2), `crates/economy/tests/wallet_census.rs` (A3); rows S32401 to S32421 and S32427 to S32429.

## What would make this wrong

- A real-tree refusal of pieces no one joins, which would show the workspace-wide pool is too wide
  for the tree as it grows.
- A census of another table (#297's one-router census) needing a case-sensitive or a per-crate rule,
  which the shared reader would then have to carry as a parameter.

## More Information

SPEC-324; ADR-029 (one file included by path); ADR-197 (round 9); SPEC-040 A9; SPEC-072 A12;
SPEC-082 A9; issues #460, #585.
