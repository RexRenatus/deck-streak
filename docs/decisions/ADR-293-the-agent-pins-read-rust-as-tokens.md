---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The agent's source pins read Rust as rustc's lexer tokenizes it, and refuse what does not lex

## Context and Problem Statement

SPEC-043's prune pin (A21) reads `crates/agent/src/runs.rs` and its verdict pin (A23) reads
`crates/agent/src/verdict.rs`, each through a lexer written by hand inside the test file. Issue
#404's review reopened the same lexical class in its second, third and fourth rounds, and each fix
added a rule to that lexer. The fourth round found three more members: a raw literal holding a
copy of the call was read as the call; a string or a block comment closing on the declaration's
own line lent the enum the `#[must_use]` it held; and U+200E or U+200F between `enum` and
`Verdict`, which rustc skips as whitespace, stopped the declaration being read, while the same
cause refused a correct enum. How do the pins read a source as rustc reads it, so that the class
ends rather than its next member?

## Decision Drivers

- The pins must see the tokens rustc sees: no text inside a comment or a literal is code, a
  literal's text is its cooked value, and whitespace is what rustc skips between tokens.
- Zero escapes over a population generated from the class's axes and labelled by rustc.
- What a pin cannot decide it refuses, and each such refusal is named in SPEC-043.
- No new crate in the lock and no production dependency.

## Considered Options (the alternatives it was chosen against)

Each option was measured over one generated population: 7,058 members built from the axes of
every round's findings (every literal kind as a holder, raw with none to three hashes, byte and C
strings included; every comment kind, nested and doc forms included; where the holder closes;
rustc's eleven whitespace code points, and NBSP and U+3000 as negatives, in every gap between the
tokens a pin reads; spaces inside paths and macro bangs; copies compiled out or made by a macro).
Each member was labelled by compiling it with rustc 1.97.0, edition 2024: the verdict's truth under
`#![deny(unused_must_use)]`, the prune's through a stand-in `sqlx` that logs each statement it is
handed. rustc rejected 1,540 members, which were dropped; of the 5,518 it accepted, 5,474 are in
the lexical class and 44 belong to issue #444.

- Read Rust as tokens — chosen because over the 5,474 in-class members it lets none escape: each
  source is read through `proc-macro2`'s lexer and each string-like literal through `syn` as the
  value rustc cooks, and each of its 93 refusals of a correct source fails closed on a class
  SPEC-043 names.
- The hand lexer with three more rules — rejected because over the 5,474 in-class members it lets 12
  escape (a prune keyword written with a string escape or a line continuation beside a copy of the
  call that never runs) and refuses 256 correct sources. The rules were round 4's: a table of string
  openings, the eleven `Pattern_White_Space` code points as whitespace, and a guard on the
  declaration's line. The hand lexer before those rules let 470 escape and refused 260.
- rustc as the reader — rejected because it lets 2 escape (a second statement on a path the test
  does not take), needs a toolchain and a second compilation in every test run, and couples the pin
  to sqlx's macro internals: the verdict read under `--emit=metadata` with
  `#![deny(unused_must_use)]`, the prune through a stand-in `sqlx` that logs the statements run. It
  is the direction for the classes issue #444 keeps open, which are not lexical.

## Decision Outcome

Chosen option: "Read Rust as tokens", each source through `proc-macro2`'s lexer and each string-like
literal through `syn` as the value rustc cooks, because over the same 5,474 in-class members it lets
none escape, and each of its 93 refusals of a correct source fails closed on a class SPEC-043 names:
88 are a literal whose cooked text writes the word `delete` beside the one prune, and 5 are a copy
of the enum in code, compiled out, beside the enum. Out of the class it refuses 8 correct sources
(an enum made by a macro, or renamed by a `use` or an alias) and lets 12 escape, each an issue #444
class. Both crates are already in `Cargo.lock` (`proc-macro2` 1.0.107 and `syn` 2.0.119, through the
workspace's derive and query macros); the agent crate takes them as dev-dependencies with no default
feature, so the lock gains two dependency edges and no crate.

### Consequences

- Good, because a comment is no token, a doc comment is a `doc` attribute whose text is prose, a
  literal of any kind is one token whatever it holds, and its text is the value rustc cooks, so an
  escape, a continuation or a raw form reads as rustc reads it.
- Good, because the pins no longer decide what whitespace is: it is whatever separates two tokens.
- Good, because a source that does not lex is refused rather than read.
- Good, because the hand lexer and its helpers are deleted, not kept beside the tokens.
- Bad, because `proc-macro2`'s fallback lexer skips more than rustc does (`char::is_whitespace`,
  NBSP and U+3000 included). rustc refuses those code points between tokens, so a source holding one
  never compiles and never reaches a pin.
- Bad, because a literal whose cooked text writes the word `delete`, beside the one prune, is
  refused: a literal can be handed to `sqlx::query` at run time, and the pin follows no value.
- Bad, because the pins evaluate no `cfg`, expand no macro and read one file each: a copy of the
  enum in code compiled out beside it is a second declaration; an enum made by a macro, declared in
  another file, or renamed by a `use` or an alias is refused; and issue #444's joined-literal,
  other-file and dead-copy-beside-such-a-prune classes stay open.

### Confirmation

The killers in `crates/agent/tests/runs.rs` and `crates/agent/tests/verdict.rs` generate their
members from axis tables and assert the exact count each examined (SPEC-043 section 9). Rows S04331
to S04333, S04335 to S04337 and S04340 to S04355 mutate each place the rule decides code or not
code, whitespace or not, and a spelling, and each is KILLED by the test its row names.

## More Information

Issue #404; issue #444; SPEC-043 sections 9 and 10; ADR-043, which this amends.
