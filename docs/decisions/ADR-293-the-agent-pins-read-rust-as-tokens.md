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

## Amendment (2026-09-30): the source as rustc receives it, and one set of skipped groups

PR #414's sixth review (issue #404) found two members of the lexical class this decision closes
still open, both in how the tokens were taken rather than in the choice of tokens. rustc removes a
first line that opens with `#!` and is not an inner attribute (a shebang) before it lexes, and
`proc-macro2`'s lexer has no such step, so a copy of the call, or a `#[must_use]`, written on that
line was read as code. And the run count read the tokens of a `doc` attribute that the word count
skips. The pins now read the source as rustc receives it: one byte order mark is removed, and a
source that then opens with `#!` and not `#![` is refused. Every count reads one token set and skips
the same groups: a `doc` attribute, outer or inner, its name raw or not (SPEC-043 section 11).

Two sentences above claim more than the pins read. Each stays where it is, and is read as follows.

**The first decision driver**, "The pins must see the tokens rustc sees", reads: the pins must read
the tokens rustc's lexer produces for the source as rustc receives it, or refuse the source; no text
inside a comment or a literal is code, a literal's text is its cooked value, whitespace is what
rustc skips between tokens, a first line rustc removes as a shebang is refused, and every count
reads the same tokens and skips the same groups. What rustc does after it lexes (expanding a macro,
evaluating a `cfg`, running one path and not another) is not read, and issue #444 tracks it.

**The first Bad consequence**, "rustc refuses those code points between tokens, so a source holding
one never compiles and never reaches a pin", reads: rustc refuses those code points between the
tokens it lexes, so a source holding one there never compiles; inside a comment or a literal both
lexers read them alike; and on a first line rustc removes as a shebang, where rustc accepts them,
they are never read, because the pins refuse a source that opens with `#!` and not `#![`.

**Options for this amendment.**

- Narrow and disclose, chosen because over every population measured it lets no in-class member
  escape: the pins keep one reader, refuse the first line rustc removes as a shebang, and skip the
  same groups in every count; the sentences above are narrowed to that lexical claim, and what lies
  past the lexer is issue #444's.
- Re-derive rustc's shebang lookahead in the pins, rejected because it rewrites a rule of rustc's
  lexer by hand, the approach this decision retired, to spare refusals that already fail closed: at
  most the 105 measured members the refusal turns away although rustc builds them correctly (92 of
  the new members and 13 of the review's). No source in this repository opens with `#!`.
- rustc as the reader, declined again because it is the option this decision rejected, and nothing
  new outweighs its measured cost: a toolchain and a second compilation in every test run, a
  coupling to sqlx's macro internals, and 2 escapes of its own over the same population.

**Measured.** Each member was labelled by compiling it with rustc 1.97.0, edition 2024, as above.
After this amendment the pins let 0 in-class members escape in every population: the 7,058 above
(5,518 accepted), the review's 1,369 (1,039 accepted; 37 escaped before, each on a shebang line),
its two sets of doc-attribute members (64, 48 accepted; 24 escaped before, the counts reading
different groups), 8 members that spell the doc attribute `r#doc` (4 escaped before), the fifth
round's 83 planted sources, and 860 new members generated from the Rust Reference's input format,
from every group one count might skip and the other read, and from raw identifiers (741 accepted; 92
escaped before). Every escape out of the class is an issue #444 shape: a keyword the count cannot
read (9), a copy that never runs beside a prune the count cannot read (21), a macro that captures a
doc attribute's text or tokens (18), and a copy that never runs, with or without a prune beside it
(17).

**Consequences of the amendment.**

- Good, because a first line rustc removes before it lexes is never read as code.
- Good, because the run count and the word count read one token set and skip the same groups, so
  they cannot disagree about a group.
- Bad, because every source that opens with `#!` and not `#![` is refused, including one rustc reads
  as an inner attribute because whitespace or a comment follows `#!`; each such refusal fails
  closed.
- Bad, because a macro that captures a doc attribute's text or tokens is not followed: a prune made
  that way is refused, failing closed, and a second statement made that way is not seen (issue
  #444).

**Confirmation.** `runs::a_first_line_rustc_removes_as_a_shebang_is_refused`,
`verdict::a_first_line_rustc_removes_as_a_shebang_is_refused`,
`runs::a_query_written_inside_a_doc_attribute_is_neither_a_run_nor_a_word`,
`runs::each_count_names_the_number_it_read` and
`verdict::a_copy_of_the_enum_compiled_out_beside_it_is_refused_as_a_second_declaration` decide
SPEC-043's A24 and A25. Rows S04356 to S04365 mutate the shebang refusal, the byte order mark, the
group skip, the raw `r#doc` and each comparison of the counts, and each is KILLED by the test its
row names.
