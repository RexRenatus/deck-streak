# SPEC-324: the table censuses read a reserved name joined from literals, and the settle census refuses the owner's own operation

- **Wave:** W3. **Issue:** #460, #445. **Context(s):** `deck-streak-progression` (the ledger census,
  the xp table census and the settle census, all test code); `deck-streak-economy` (the wallet
  census, test code). No production source changes.
- **Decided by:** ADR-325 (a census refuses a reserved name its literals can assemble) and ADR-197
  (its round 9: the owner's own operation is refused unless the owner admits it). Amends SPEC-072
  (section 16) append-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-324.md`. **Mutation band:** `S32400-S32499`.

## 1. The problem, measured

Measured at dev `9affffe6f`.

- Three censuses hold a table to its owner. The ledger census (`xp_ledger`, owner progression,
  SPEC-040 A9) and the wallet census (`coin_ledger`, owner economy, SPEC-082 A9) read each line of
  every crate's `src`: a line starting `//` is skipped, text after ` //` is cut, and the rest is
  searched for the name as written. The xp table census (`xp_settlement`, owner progression,
  SPEC-072 A12) lexes each file once (`strip`), keeps each literal whole and searches for the name as
  written. `find crates/*/src -name '*.rs' | wc -l` reads 248 files in 25 crates; 19 are
  progression's and 5 economy's.
- None of the three decodes an escape, joins two literals, follows an include or folds case, so
  `concat!("xp_", "ledger")`, `"xp\x5fledger"`, `"XP_LEDGER"`, a `const` piece joined in another
  file or crate, and an `include_str!` of a file holding a piece all pass every census today (#460).
  The tests this SPEC adds plant every such spelling outside the owner, and at the base each census
  refuses none of them (`docs/red-first/SPEC-324.md`).
- The real tree holds no such spelling. Over the crate sources outside the owners: 2 `concat!`
  calls (`crates/api/src/health.rs`, `crates/bot/src/commands.rs`), each with an `env!` argument
  beside pieces that are no part of any reserved name; 24 include macros in 6 files, every one with a
  literal path; 2 `#[path]` attributes; 0 `stringify!`.
- The settle census skips every use of `settle` that rustc reports in progression's own package
  (`if used.folder == format!("crates/{OWNER}") {`, row S07263). A wrapper function, a function
  pointer or a generic in progression that calls `settle` is a new operation in the owner's code,
  and SPEC-072 section 12 discloses it as out of the census's reach (#445). The round-6 test
  `the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates` plants exactly such a
  wrapper (`crates/progression/src/inner.rs`, `pub fn own() -> usize { tally() }`) and accepts it.
- rustc reports the probe's deprecation at an import as well as at a call. Measured on a scratch
  library with the probe's attribute: a `use` of `settle`, a `use` inside a function body and a
  re-export a macro writes in its own body are each reported at a span inside the `use`
  declaration (the macro's at its definition, its expansion naming the call); a wrapper, a function
  pointer and a re-export whose path a macro takes as an argument are reported at the call, the
  pointer and the macro's argument.

## 2. Requirements

R1. One reader, `tools/table-census/table_census.rs`, included by path into the three censuses
    (ADR-029's precedent), lexes every crate's `src` outside the owner once, drops comments, and
    decodes every literal as rustc does: strings with every escape and line continuation, raw strings
    with any number of hashes, byte, raw byte and C strings, characters and byte characters; the
    words inside `stringify!` are pieces too.
R2. The reader follows `include!`, `include_str!`, `include_bytes!` and `#[path]` by their literal
    path, as rustc reads each: a file `include!` or `#[path]` reads is lexed as Rust, and a file
    `include_str!` or `include_bytes!` reads is one piece. An include whose path is not one literal
    is refused by name, but for a path joined onto `env!("OUT_DIR")`, which is disclosed (#585). An
    include of a literal path the reader cannot read is refused by name.
R3. The pieces of every file outside the owner, workspace-wide, are one pool, case-folded in ASCII.
    When the pool can assemble the name (a piece that ends with a prefix of it, pieces equal to the
    parts between, and a piece that starts with the rest), or a piece or a word holds it whole in any
    case, every file holding a piece on such a path is refused by name.
R4. A `concat!` whose argument the reader cannot read (anything but a literal, a nested `concat!` or
    a `stringify!`) beside a piece that ends with a proper prefix of the name, or starts with a proper
    suffix of it, of one character or more, is refused by name.
R5. Each census keeps its existing reader and refusals unchanged and adds the reader's, for files its
    own reader did not refuse; each prints the reader's examined counts.
R6. The settle census refuses a use of `settle` that rustc reports in progression's own package, in a
    target that is not a test, a bench or an example, unless the use is written inside a `use`
    declaration (an import or a re-export, which round 6 follows to its callers) or in a file of
    `OWNER_ADMITS`, which admits none.
R7. The real tree's three censuses stay green, and every existing test is unchanged but for one line
    added to the round-6 test's expected list (ruling 99, Q-e): its planted wrapper is refused.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the ledger census refuses every generated spelling of `xp_ledger` planted outside progression, each in a tree of its own, by the name of every file holding a piece, and accepts each spelling planted inside progression; the population's size is printed and asserted | `cargo test -p deck-streak-progression --test ledger_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling` |
| A2 | the same for the xp table census and `xp_settlement`, every spelling in one planted workspace | `cargo test -p deck-streak-progression --test xp_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling` |
| A3 | the same for the wallet census and `coin_ledger`, inside economy | `cargo test -p deck-streak-economy --test wallet_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling` |
| A4 | pieces that do not cover the name (a character or the separator missing) and a `concat!` with an unread argument beside pieces that are no part of the name are not refused | `cargo test -p deck-streak-progression --test ledger_census -- --exact pieces_that_do_not_cover_the_name_are_not_refused` |
| A5 | an unread `concat!` argument beside part of the name, an include by a non-literal path, and an include of a file that is not there are refused by name; an include joined onto `OUT_DIR` is not | `cargo test -p deck-streak-progression --test ledger_census -- --exact a_name_the_census_cannot_resolve_fails_closed` |
| A6 | a wrapper, a function pointer and a generic in progression's library are refused; its imports (grouped, in a function body, after non-ASCII text), its tests and its examples are accepted; a file the owner admits is accepted | `cargo test -p deck-streak-progression --test xp_census -- --exact the_owners_own_operation_is_refused_unless_it_admits_it`, and `an_admitted_file_of_the_owner_is_accepted` |
| A7 | a re-export of `settle` a macro of progression writes in its own body is followed (the member's caller is refused), and one whose path the macro takes as an argument is refused in progression | `cargo test -p deck-streak-progression --test xp_census -- --exact a_reexport_progressions_macro_writes_is_followed_or_refused` |

```acceptance
A1: cargo test -p deck-streak-progression --test ledger_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling
A2: cargo test -p deck-streak-progression --test xp_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling
A3: cargo test -p deck-streak-economy --test wallet_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling
A4: cargo test -p deck-streak-progression --test ledger_census -- --exact pieces_that_do_not_cover_the_name_are_not_refused
A5: cargo test -p deck-streak-progression --test ledger_census -- --exact a_name_the_census_cannot_resolve_fails_closed
A6: cargo test -p deck-streak-progression --test xp_census -- --exact the_owners_own_operation_is_refused_unless_it_admits_it
A6: cargo test -p deck-streak-progression --test xp_census -- --exact an_admitted_file_of_the_owner_is_accepted
A7: cargo test -p deck-streak-progression --test xp_census -- --exact a_reexport_progressions_macro_writes_is_followed_or_refused
```

The population (A1 to A3): for a name of `n` characters, its `n - 1` two-piece splits and its split
into single characters, each planted in nine forms (`concat!`, `+`, `format!`, two constants joined
in one file, a constant in another file, a constant in another crate, a sqlx-style `"..." + "..."`
query, an `include_str!` of a text piece and a `#[path]` module holding a piece), and a family of
fourteen literal spellings (escapes, line continuation, raw hashes, characters, nested `concat!`,
`stringify!` pieces, byte and C strings, other cases). Plain concatenation decides each split and
rustc evaluates each family member, so neither oracle is the reader. The sizes: 95 for `xp_ledger`,
131 for `xp_settlement`, 113 for `coin_ledger`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `tools/table-census/table_census.rs` | test support | added: the shared reader (R1 to R4) |
| `tools/table-census/population.rs` | test support | added: the generated population and the family's oracle |
| `crates/progression/tests/ledger_census.rs` | `deck-streak-progression` | the reader's call; A1, A4, A5 |
| `crates/progression/tests/xp_census.rs` | `deck-streak-progression` | the reader's call, the owner's rule; A2, A6, A7; the round-6 test's one added line |
| `crates/economy/tests/wallet_census.rs` | `deck-streak-economy` | the reader's call; A3 |
| `docs/specs/SPEC-324-the-censuses-read-joined-names-and-refuse-the-owners-own-operation.md` | docs | added |
| `docs/decisions/ADR-325-a-census-refuses-a-reserved-name-its-literals-can-assemble.md` | docs | added |
| `docs/decisions/ADR-197-the-settle-census-reads-progressions-own-reexports.md` | docs | round 9 appended |
| `docs/specs/SPEC-072-xp-is-earned-per-review-and-per-day-settled-once-per-source-with-levels-and-titles.md` | docs | section 16 appended |
| `docs/schematics/source-censuses-and-the-owners-operation.md` | docs | added |
| `docs/red-first/SPEC-324.md` | docs | added |
| `scripts/mutation-rows.d/S32400-S32499.json` | scripts | added |
| `changelog.d/census-joined-names-and-owner-wrappers-460-445.md` | changelog | added |

## 5. What this does NOT do

Four routes stay beyond the reader, each disclosed by kind and tracked by #585: a reserved name held
whole by the owner's exported constant and used through a renamed re-export; a join a procedural
macro writes; build-script output under `OUT_DIR`, including an include joined onto it; and Rust
outside `src` (tests, benches, examples and build scripts), which no census reads (#585).

- The settle census's other disclosed kinds (SPEC-072 sections 12 and 14: the census's cfg read by
  progression's code, doctests and compile tests, procedural macros, other packages' build scripts,
  an `include!` of a recompute file, registry code and builds in mixed debug-assertion states) move
  from #445 to #586, as SPEC-072 section 16 says (#586).
- A unit test inside progression's `src` that calls `settle` is refused, because cargo names a
  library's own test compile with the library's kind; the tree holds none, and a test belongs under
  `tests/` (#586).
- The census's cost in the `rust` job is not changed here: three new census compiles of small
  planted workspaces join it (#508).
- The one-router census (`in_app_feed`, `notification_queue`) keeps its own reader (#297), and the
  agent-runs pin keeps its own (#444).

## 6. Risks

- **A false refusal on the real tree.** The workspace-wide pool could assemble a name from pieces no
  one joins. Detected by the three real-tree tests, which assert no refusal and print the reader's
  examined counts; at this head they refuse none.
- **A vacuous population.** Detected by the size each test asserts, by rustc and concatenation as the
  oracles, and by A4's near misses, which a reader that refuses everything fails.
- **Existing evidence moved.** Every existing test is unchanged but for R7's one line, and every row
  of `xp_census.rs` still finds its text once.
- **Byte offsets against characters.** rustc reports byte offsets; the reader indexes bytes, and A6
  plants an import after non-ASCII text.
- **A loud false refusal of the owner.** A re-export whose path a macro of progression takes as an
  argument is refused (A7), named in ADR-197 round 9 as the price of failing closed.

## 7. The mutation rows

Band `S32400-S32499`, in `scripts/mutation-rows.d/S32400-S32499.json`: S32401 to S32429, each proved
with `python3 scripts/mutation_rows.py prove --row <id>` on a committed tree. S32427 to S32429 pin
R2's three include arms (ruling 108): an include joined onto `env!("OUT_DIR")` is disclosed, only
that variable is, and an include of a literal path the reader cannot read is refused.

## 8. Amendment: what R7 left out (ruling 128 (a))

R7 and the risk "Existing evidence moved" say every existing test is unchanged but for one line.
At `31f45734` three existing parts of `crates/progression/tests/xp_census.rs` changed besides it:

- `the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates`: its comment names the
  wrapper now refused (`80d333ac`), and its two plants of progression's files moved, text and order
  unchanged, into the helper `plant_progressions_aliases` (`b713e00b`, for clippy).
- `s2_population`, which `the_census_refuses_every_caller_the_compiler_finds` judges: the S2 B
  build-script control whose label names the file is now expected refused (`ede1fcdc`, ruling 115
  under ruling 108 (1)), because R2 refuses an include joined onto any variable but `OUT_DIR`.
- `KILLER_POPULATION`: that one verdict re-pins the digest from
  `eaafc8da1c7d34a8bfa4d8794360caf5b930097866d62bf0f88e1e5e5f4e44ca` to
  `f5f615e6ce0c85098b00b2c87b6de5b8fb5bab962b744bf0c316b112d0ea75b3` (`ede1fcdc`). The count stays
  2218.

Every row of `xp_census.rs` still finds its text once.

## 9. Amendments, 2026-10-03: R3, as amended by SPEC-331

R3, as amended by SPEC-331: the pool holds the pieces of two or more characters of every file
outside the owner, workspace-wide. A piece of one character is pooled only in the reach of each
file that holds it, includes it, or names the item holding it.

The reach, the fail-closed arm for an item whose name is a macro's metavariable, and the class this
leaves disclosed (one character carried to its join through a function's return value or argument,
#585) are SPEC-331's R1 to R3 and its section 5, decided by ADR-331 (ruling 158). The killers of
S32411 and S32409 move to SPEC-331's A2, as SPEC-331's section 6 says; their `find`s are unchanged.
