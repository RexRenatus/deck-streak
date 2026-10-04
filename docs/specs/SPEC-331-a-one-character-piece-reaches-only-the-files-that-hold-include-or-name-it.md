# SPEC-331: a one-character piece reaches only the files that hold, include or name it, so one unrelated literal no longer refuses the workspace

- **Wave:** W3. **Issue:** #604 (the census defect); #585 (the routes the shared reader does not
  reach). **Context(s):** `deck-streak-progression` (the ledger census and the xp table census, test
  code); `deck-streak-economy` (the wallet census, test code). No production source changes.
- **Decided by:** ADR-331 (a one-character piece is pooled only in the reach of a file that holds,
  includes or names it). Amends SPEC-324 R3 append-only (section 8 below); ADR-325's rejection of "a
  pool per crate" stands.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-331.md`. **Mutation band:** `S33100-S33199`.

## 1. The problem, measured

Measured at dev `e2eb20d7` on a `git archive` export. The shared reader
(`tools/table-census/table_census.rs`) was compiled std-only with rustc in a scratch harness, by
`#[path]` as the censuses include it, beside each census's own naming rule (a code word or literal
holding the name; comments excluded).

- SPEC-324 R3 pools the pieces of every file outside the owner workspace-wide, single characters
  included, folded to lower case, and refuses every file holding a piece on a path that assembles the
  name. At dev the three censuses refuse nothing: `xp_ledger` 0, `xp_settlement` 0, `coin_ledger` 0.
- **The defect.** dev's tree with #600's `crates/ingest/src` laid over it (`86c222e1`): the xp table
  census refuses 133 files, none of which spells `xp_settlement`. By crate: coordination 21, ingest 15,
  vault 14, api 12, daemon 9, agent 9, notifications 8, kernel 8, bot 8, readings 6, quests 5,
  curriculum 4, and the rest. The cause is one character literal,
  `date.strip_prefix('M')` at `crates/ingest/src/skip_write.rs:1066`, which folds to the piece `m`.
  It is the one middle piece the pool lacked. Control: the same tree with that literal written
  `char::from(77_u8)` refuses 0.
- **The fragility census.** One lone character literal, `text.strip_prefix('<c>')`, was planted in an
  unrelated file outside every owner (`crates/ingest/src/skip.rs`) for each of the 63 characters
  `a-z`, `A-Z`, `0-9` and `_`, and the files each census refuses were counted. The file was restored
  byte for byte after each plant (sha256 `23d27ce1…` before and after).

  | character | `xp_ledger` | `xp_settlement` | `coin_ledger` |
  |---|---|---|---|
  | `l`, `L` | 90 | 0 | 96 |
  | `m`, `M` | 0 | 131 | 0 |
  | the other 59 | 0 | 0 | 0 |

  The counts are the same when the character sits in a `fn new` body or a `From::from` body. Each name
  is one letter from complete in today's pool: `xp_settlement` lacks only `m`, and `xp_ledger` and
  `coin_ledger` lack only `l`. Any lone literal of that letter, anywhere outside the owner, completes a
  path.
- **The residual, multi-character.** A lone string literal of each run of 2 to `n - 1` characters of
  the three names (141 runs) was planted the same way. At dev, 22 runs trip `xp_ledger`, 34 trip
  `xp_settlement` and 34 trip `coin_ledger`, the shortest of 2 characters (`le`, `me`, `_l`). Under
  this SPEC, 8, 11 and 5 runs trip, and none is shorter than 5 characters (`_ledg`, `_settlem`,
  `oin_ledg`): each is a long run of the name itself.
- **What a narrower pool must keep.** SPEC-324's own population (A1 to A3: 95, 131 and 113 spellings,
  each planted outside and inside the owner, and the one-workspace shape; A4's near misses and
  controls; A5's fail-closed cases) is judged exactly as at dev by this SPEC's reader. A minimum piece
  length alone loses 12 spellings per name: the `file`, `crate`, `text` and `module` forms of the
  splits that leave a single character.
- **The real joins a narrower pool must keep** (section 3 A2) are refused by this SPEC's reader with the
  same file sets as dev's: 13 join plants, and 5 further joins (multi-character pieces through a
  function's return value or argument, pieces in a `static` struct field, and a one-character `const`
  whose name a `macro_rules!` takes as an argument). One shape dev refuses is not refused by this
  SPEC's reader: a one-character piece that reaches its join only through a function's return value or
  argument (3 plants, A3, disclosed in section 5). dev refuses those only because its pool holds every
  character.

## 2. Requirements

R1. A piece of one character, after decoding and ASCII case folding, no longer enters the
    workspace-wide pool. A piece of two or more characters enters it exactly as SPEC-324 R3 says.
R2. Each file outside the owner has a reach. The reach holds pieces of every length:
    - the file's own pieces;
    - the pieces of every file it includes by `include!`, `include_str!`, `include_bytes!` or
      `#[path]`, followed transitively;
    - the pieces of every `const` or `static` item whose name the file, or a file it includes, writes
      as a word or as a format placeholder's name.

    A name is matched as written, unqualified. An include is recorded for every includer, also when
    the included file was already read. A `const` or `static` item whose name is a macro's
    metavariable (`const $name: ..`) has a name the reader cannot read, so its one-character pieces
    stay in the workspace-wide pool, failing closed as every piece did before. dev's tree holds no
    such item.
R3. The name is covered when the pool covers it, by SPEC-324 R3's rule unchanged, or when one file's
    reach covers it by the same rule. Every file holding a piece on a covering path is refused by name,
    with the existing message: `{file} spells {TABLE} from literals, joined or in another case, and
    only {OWNER}'s code may`.
R4. SPEC-324 R1, R2, R4 and R5 are unchanged. R4's one-character threshold for an unread `concat!`
    argument is not raised (ruling 99, Q-j). The reader's examined line also prints the count of
    one-character pieces kept out of the pool.
R5. On every tree, every file the new reader refuses, dev's reader refuses too, so no tree that is
    green today turns red.
R6. The real tree's three censuses stay green. Every existing test is unchanged. The killers of S32411
    and S32409 are re-pointed (sections 6 and 7), and the weakening table names each move.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the population A1 builds is planted for each census's own name. At each position `i` of the name, the pieces before and after `i` are lone literals in two files, and the character at `i` is a lone literal in a third. The third file holds it in each of five lone shapes: a method argument (#600's `strip_prefix` line among them), a `matches!` arm, a format template, a named `const` used alone, and a split set. A letter is planted in both cases. No file is refused, and the population's size is printed and asserted | `cargo test -p deck-streak-progression --test ledger_census -- --exact a_lone_character_unjoined_in_its_file_completes_no_path`, and the same test in `xp_census` and in `deck-streak-economy`'s `wallet_census` |
| A2 | each join below is refused by the exact set of files holding its pieces: `concat!("xp_", "settlement")` in one file; a `const` holding `"xp_sett"` in one file joined with `"lement"` in another; `"XP_Settlement"`; a `stringify!` split; a char-by-char spelling in one file, as an array and as a push sequence; a one-character `const` or char array joined from another file or crate (`format!`, push, inline capture, glob import, renamed import); a one-character piece in a `static` struct field; a one-character `const` whose name a `macro_rules!` takes as an argument; multi-character pieces carried through a function's return value or argument; and a multi-character piece in another case, returned by a function in another crate, so that only the pool's case folding joins it | `cargo test -p deck-streak-progression --test xp_census -- --exact the_joins_the_census_names_stay_refused` |
| A3 | a one-character piece that reaches its join only through a function's return value or argument, alone or beside a multi-character piece carried the same way, is not refused (the disclosed class) | `cargo test -p deck-streak-progression --test xp_census -- --exact a_character_carried_by_a_function_is_disclosed` |
| A4 | the real tree: each census refuses nothing | `cargo test -p deck-streak-progression --test ledger_census -- --exact only_the_grant_port_writes_the_xp_ledger`, `cargo test -p deck-streak-progression --test xp_census -- --exact only_progression_writes_xp_settlement_and_only_coordination_settles`, `cargo test -p deck-streak-economy --test wallet_census -- --exact only_the_wallet_writes_the_coin_ledger` |
| A5 | SPEC-324's population is judged as before: every spelling refused outside the owner and accepted inside it, near misses and controls accepted, fail-closed cases refused | `cargo test -p deck-streak-progression --test ledger_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling` and its `xp_census` and `wallet_census` twins, `pieces_that_do_not_cover_the_name_are_not_refused`, `a_name_the_census_cannot_resolve_fails_closed` |

```acceptance
A1: cargo test -p deck-streak-progression --test ledger_census -- --exact a_lone_character_unjoined_in_its_file_completes_no_path
A1: cargo test -p deck-streak-progression --test xp_census -- --exact a_lone_character_unjoined_in_its_file_completes_no_path
A1: cargo test -p deck-streak-economy --test wallet_census -- --exact a_lone_character_unjoined_in_its_file_completes_no_path
A2: cargo test -p deck-streak-progression --test xp_census -- --exact the_joins_the_census_names_stay_refused
A3: cargo test -p deck-streak-progression --test xp_census -- --exact a_character_carried_by_a_function_is_disclosed
A4: cargo test -p deck-streak-progression --test ledger_census -- --exact only_the_grant_port_writes_the_xp_ledger
A4: cargo test -p deck-streak-progression --test xp_census -- --exact only_progression_writes_xp_settlement_and_only_coordination_settles
A4: cargo test -p deck-streak-economy --test wallet_census -- --exact only_the_wallet_writes_the_coin_ledger
A5: cargo test -p deck-streak-progression --test ledger_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling
A5: cargo test -p deck-streak-progression --test xp_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling
A5: cargo test -p deck-streak-economy --test wallet_census -- --exact a_reserved_name_joined_from_literals_is_refused_in_every_spelling
A5: cargo test -p deck-streak-progression --test ledger_census -- --exact pieces_that_do_not_cover_the_name_are_not_refused
A5: cargo test -p deck-streak-progression --test ledger_census -- --exact a_name_the_census_cannot_resolve_fails_closed
```

Oracles. A1's trees are built from the name by position, so the planted set decides the expected
verdict: none refused. A2's and A3's expected sets are the planted files. Neither oracle is the reader.
The design harness measured A1's population at 85 trees for `xp_ledger`, 125 for `xp_settlement` and
105 for `coin_ledger` (letters in both cases, `_` once, five shapes). dev's reader refuses every one
of those trees (235, 355 and 295 files), so A1 is red at the base for its own reason; this SPEC's
reader refuses none. A3 is red at the base too: dev refuses 2, 2 and 3 files. A2 is green at the base:
it pins what dev already refuses, and the red-first record says so (`not red`).

## 4. File manifest

| file | context | change |
|---|---|---|
| `tools/table-census/table_census.rs` | test support | R1 to R4: the one-character pieces leave the pool; each file's reach; the reach pools judged |
| `crates/progression/tests/ledger_census.rs` | `deck-streak-progression` | A1 |
| `crates/progression/tests/xp_census.rs` | `deck-streak-progression` | A1, A2, A3 |
| `crates/economy/tests/wallet_census.rs` | `deck-streak-economy` | A1 |
| `scripts/mutation-rows.d/S32400-S32499.json` | scripts | the killers of S32411 and S32409 re-pointed to A2 (amendment rows in the weakening table) |
| `scripts/mutation-rows.d/S33100-S33199.json` | scripts | added: the new rows |
| `docs/specs/SPEC-331-a-one-character-piece-reaches-only-the-files-that-hold-include-or-name-it.md` | docs | added |
| `docs/decisions/ADR-331-a-one-character-piece-is-pooled-only-in-a-files-reach.md` | docs | added |
| `docs/specs/SPEC-324-the-censuses-read-joined-names-and-refuse-the-owners-own-operation.md` | docs | a section appended: R3 as amended |
| `docs/schematics/source-censuses-and-the-owners-operation.md` | docs | the pool and the reach drawn |
| `docs/red-first/SPEC-331.md` | docs | added |
| `formal/lean/Formal/TableCensusReach.lean`, its vectors writer `formal/lean/Formal/TableCensusReachVectors.lean` with its one-line arm in `formal/lean/Formal/Vectors.lean`, and `formal/vectors/table-census-reach.jsonl` | formal | added (FORMAL REQUIRED; build brief); `Vectors.lean` gains the arm |
| `changelog.d/table-census-reach-604.md` | changelog | added |

## 5. What this does NOT do

- A one-character piece carried to its join through runtime value flow, such as a function's return
  value, a function's argument, or a value built at run time, is not refused. The reader follows
  includes and the names of `const` and `static` items, never calls. A3 pins the class, and it joins
  the routes the reader does not reach (#585).
- A one-character piece carried to its join through a SECOND step is not refused either: a `const`
  naming another `const` across files (an alias chain, also across crates), a re-export renamed in a
  third file, and an `include!` or `include_str!` inside a named item's initialiser. The reader
  follows one step (a file's own pieces, the files it includes, and the items it names), and the
  included text of an item's initialiser belongs to no item. dev refuses each of these (2 files
  each) only because its pool holds every character. A3 pins four plants of the class (O1, O2, O3
  and O7) beside B1, B2 and B8, so the escape is counted, and the controls O4, O5 and O6 stay refused
  in A2. Following a second step is #585's work, not this SPEC's (#585).
- A lone literal of two or more characters still completes a path through the workspace-wide pool:
  measured, 8, 11 and 5 runs of the three names, each 5 characters or longer. That is ADR-325's
  accepted over-approximation, unchanged here (#585).
- The function items a file names are not followed. Following them keeps the disclosed class
  refused, but it widens each reach to a mean of 44.6 holder files (max 133) against 2.9 (max 26).
  ADR-331 records the measurement (#585).
- No production source changes, and #600's code is neither edited, admitted nor exempted by this
  delivery (#600).
- R4's one-character threshold for an unread `concat!` argument is unchanged (#460).
- The one-router census keeps its own reader (#297), and the agent-runs pin keeps its own (#444).

## 6. Risks

- **A real one-character join escapes.** The disclosed class (section 5). Detected by A3, which pins
  exactly that class, so widening or narrowing it moves a test.
- **A reach widened by a common item name.** A name is matched unqualified, so a file that writes a
  word equal to some `const` name pulls that const's pieces into its reach. Measured at dev under the
  reader's rule: each reach holds a mean of 3.18 one-character pieces (median 2, max 36) from a mean
  of 2.9 holder files (max 26). Detected by A4 on the real tree.
- **Existing evidence moved.** Under the new reader, S32411's mutant (the pool cleared per member) is
  no longer killed by SPEC-324's population, because the reach finds the crate form. Its killer moves
  to A2, which holds multi-character pieces joined across crates through a function, and the
  weakening table names the move. S32409's mutant (the pool no longer folded to lower case) is no
  longer killed by SPEC-324's population either: every member it spells in another case lies in one
  file, or in files that include or name each other, and each reach folds its own pieces. Measured
  at the build: the mutant dies at dev's reader (`f007 "XP_LEDGER"`) and passes every census test at
  this reader. Its killer moves to A2, which gains a plant only the fold refuses: a multi-character
  piece in another case returned by a function in another crate (P14). Every other row on
  `table_census.rs` keeps its `find`, once (S32409's `.entry(part.to_ascii_lowercase())` included),
  and is expected KILLED; each is re-proved at the build.
- **Cost.** The reader judges one reach per file: about 240 small pools per census. Measured in the
  harness (std-only, optimised), all three censuses read in 0.25 s against dev's 0.13 s.

## 7. The mutation rows

Band `S33100-S33199`, in `scripts/mutation-rows.d/S33100-S33199.json`, each proved with
`python3 scripts/mutation_rows.py prove --row <id>` on a committed tree. Each row's mutant and the
test that kills it:

- a one-character piece enters the pool again (A1);
- the reach follows no include (SPEC-324 A1's `text` and `module` forms);
- the reach follows no named item (SPEC-324 A1's `file` and `crate` forms);
- no reach pool is judged (SPEC-324 A1);
- a format placeholder's name is no mention (A2's glob-imported const captured inline);
- a word is no mention (SPEC-324 A1);
- an item named by a macro's metavariable sends its one-character pieces nowhere global (A2's
  macro-named `const`);
- S33107, added in the fix round: a file read gets no reach of its own, so a joining file that holds
  no literal is never judged (AD2's no-own-piece join, killed by its own test).

The first seven die in the design harness and at the build. The killers of S32411 and S32409 are
re-pointed to A2, as section 6 says.

## 8. SPEC-324 R3, as amended

SPEC-324 gains a section: "R3, as amended by SPEC-331: the pool holds the pieces of two or more
characters of every file outside the owner, workspace-wide. A piece of one character is pooled only
in the reach of each file that holds it, includes it, or names the item holding it."
