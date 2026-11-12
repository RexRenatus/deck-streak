---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat, rulings 155 and 158 on the census defect (design-CEN1)"
---

# A one-character piece is pooled only in the reach of a file that holds, includes or names it

## Context and Problem Statement

SPEC-324 R3 and ADR-325 pool the decoded, case-folded pieces of every file outside a table's owner
workspace-wide, and refuse every file holding a piece on a path that assembles the name. ADR-325
named the risk itself: "a real-tree refusal of pieces no one joins, which would show the
workspace-wide pool is too wide for the tree as it grows".

That risk is now measured at dev `e2eb20d7`. Each of the three names is one letter from complete in
the pool: `xp_settlement` lacks only `m`, and `xp_ledger` and `coin_ledger` lack only `l`. One lone
character literal of that letter, in any file outside the owner, completes a path:

- #600's `date.strip_prefix('M')` refuses 133 files;
- a planted lone `l` or `L` refuses 90 files for `xp_ledger` and 96 for `coin_ledger`;
- a planted lone `m` or `M` refuses 131 files for `xp_settlement`.

None of those files spells a name. How does the census stop a verdict on a hundred files turning on
one unrelated character, and keep refusing every join SPEC-324 names?

## Decision Drivers

- SPEC-324's threat (#460) is a name joined from literals: `concat!`, an escape, another case, a
  `const` piece joined in another file or crate, an included file. Every spelling of SPEC-324's
  population must stay refused, with the same files named.
- A census is test code. It adds no dependency edge and no parser (ADR-325, ADR-197 round 3).
- Narrowing what a census refuses is a weakening. The narrowing must be the smallest that cures the
  defect, measured, and named in the weakening table.
- R4's one-character threshold for an unread `concat!` argument is not raised without a ruling
  (ruling 99, Q-j).
- #600 may not evade the census, and may not edit it or be admitted into it (ruling 155).

## Considered Options (the alternatives it was chosen against)

Every option was measured in one scratch harness over dev's export, with dev's reader as the control.
"Escapes" names plants dev refuses and the option does not refuse. B9 is a one-character `const`
whose name a `macro_rules!` takes as an argument. (d) and (d+fn) refuse it through (d)'s fail-closed
arm (below). (a) and (b) let it escape as specified, and (c) lets it escape even with that arm, since
it has no global pool.

| option | #600's tree | lone `l`/`L`/`m`/`M` | A1 trees refused (85, 125, 105) | SPEC-324 population | join plants (P1–P13, B4–B7, B9) | one character carried by a function (B1, B2, B8) |
|---|---|---|---|---|---|---|
| dev (the control) | 133 | 90, 96, 131 | all | exact | all refused | refused |
| (a) a one-character piece counts only where its own file joins it | 0 | 0 | 39, 63, 51 | exact | P6, P9, B9 escape | escape |
| (b) a minimum piece length of 2, plus a per-file char-by-char refusal | 0 | 0 | 0 | 12 spellings lost per name | P7–P13, B7, B9 escape | escape |
| (c) a pool per file, plus a cross-file `const` join detector (no global pool) | 0 | 0 | 0 | exact | B4, B5, B9 escape | escape |
| **(d) chosen: a one-character piece is pooled only in a file's reach** | 0 | 0 | 0 | exact | all refused | escape (disclosed) |
| (d+fn) as (d), also following `fn` items by name | 0 | 0 | 0 | exact | all refused | refused |

- (d), a one-character piece pooled only in a file's reach: chosen, because it refuses every join
  the others keep, and no lone character trips it. The reach is the file's own pieces, the files it
  includes and the items it names.
- (a), a one-character piece counts only where its own file joins it: lost, because of two reasons.
  Its file joins it inside `concat!`, `format!`, a `+`, an array of two or more, or a `const`.
  The defect survives in shapes as ordinary as #600's: a format template (`format!("M{n}")`), a
  named const used alone, and a split set (`split(['M', '.'])`). A1 still refuses 39, 63 and 51
  trees. A char-by-char push sequence in one file (P6) and a character pushed between two other
  files' constants (P9) escape.
- (b), a minimum piece length of two plus a per-file char-by-char refusal: lost, because it drops
  SPEC-324's own spellings. The `file`, `crate`, `text` and `module` forms of every split
  that leaves a single character escape, 12 per name. So do the one-character constants joined from
  another file or crate (P7 to P13, B7).
- (c), a pool per file plus a cross-file `const` join detector: lost, because it drops real
  multi-character joins. A piece returned by a function in another crate (B4) and one passed as an
  argument (B5) escape, the shape ADR-325 gave for rejecting a per-crate pool.
- (d+fn), (d) with `fn` items followed by name as well: lost, because of its width. It keeps B1, B2
  and B8 refused, but names are matched unqualified, so `new`, `from` and `fmt` collide.
  At dev, each file's reach draws on a mean of 44.6 holder files (max 133, the defect's own scale)
  against (d)'s 2.9 (max 26), and holds 13.06 one-character pieces against 3.18. It rebuilds a
  near-workspace pool keyed by common names, and it still misses a character carried through two
  calls.
- Editing #600's literal, admitting it, or exempting its file: lost, because the defect is the
  census's (ruling 155). The next lone `l` or `m` in any crate would trip it again, as the
  fragility census shows.
- Raising R4's one-character threshold: lost, because it is not the defect's rule. R4 judges an
  unread `concat!` argument beside part of a name, and its threshold is never raised without a
  ruling (ruling 99, Q-j).
- A per-crate pool: lost, because ADR-325 rejected it for the reason (c) loses here, B4 and B5. It
  stays rejected.

## Decision Outcome

Chosen option: "(d) a one-character piece is pooled only in the reach of a file that holds, includes
or names it". It cures the defect for every character (0 refusals in the fragility census and in
A1), and it keeps every spelling of SPEC-324's population and every join plant refused with dev's
file sets.

- A piece of two or more characters enters the workspace-wide pool as SPEC-324 R3 says, unchanged.
- A piece of one character enters no global pool. It is held in its file's local pieces.
- A file's reach holds pieces of every length:
  - its own pieces;
  - those of every file it includes (`include!`, `include_str!`, `include_bytes!`, `#[path]`),
    transitively, recorded for every includer even when the included file was already read;
  - those of every `const` or `static` item whose name the file, or a file it includes, writes as a
    word or as a format placeholder's name, matched unqualified.
- A `const` or `static` item whose name is a macro's metavariable (`const $name: ..`) has a name the
  reader cannot read. Its one-character pieces stay in the workspace-wide pool, failing closed as
  every piece did before. dev's tree holds no such item, so the arm costs no refusal there.
- The name is covered when the pool covers it, or when any one file's reach covers it, by SPEC-324's
  `covering` rule unchanged. Every holder of a piece on a covering path is refused with the existing
  message.
- On every tree the new refusals are a subset of dev's. The global pool is a subset of dev's pool,
  and each reach is a subset of it, attribution included. `covering` only grows with its pool. So no
  green tree turns red.

### Consequences

- Good, because a verdict no longer turns on one unrelated character:
  - #600's tree reads 0;
  - every one of the 63 characters reads 0, also inside a `fn new` or `From::from` body;
  - A1's 315 trees read 0.
- Good, because every SPEC-324 spelling (95, 131 and 113), A4's near misses, A5's fail-closed cases,
  the five plants the brief names, and 13 further joins stay refused, with the same files.
- Bad, because a one-character piece carried to its join only through a function's return value or
  argument is no longer refused (B1, B2, B8). That is a real join. The reader follows names of items
  and includes, never calls, and dev caught it only by pooling every character. It is disclosed and
  pinned (SPEC A3), among #585's routes.
- Bad, because a lone literal of two or more characters still completes a path through the global
  pool. That is ADR-325's over-approximation. At dev, 8, 11 and 5 runs of the names trip, each 5
  characters or longer, against 22, 34 and 34 runs from 2 characters up.
- Bad, because S32411's mutant (the pool cleared per member) is no longer killed by SPEC-324's
  population: the reach finds the crate form. Its killer moves to SPEC A2.
- Bad, because S32409's mutant (the pool not folded to lower case) is no longer killed by SPEC-324's
  population either: each of its members in another case lies within one reach, and a reach folds
  its own pieces. Its killer moves to SPEC A2 too, which gains a cross-crate join in another case
  (P14) that only the pool's fold refuses.
- Neutral, because the reader judges one small pool per file: 0.25 s for all three censuses against
  dev's 0.13 s in the harness.

### Confirmation

- SPEC A1 to A5: the lone-character population, the joins, the disclosed class, the real tree, and
  SPEC-324's population.
- The band's eight new rows (S33107 added in fix round 2), and S32411 and S32409 re-proved against their new killer.
- A Lean entry, `TableCensusReach`: on every tree the reader's refusals are within dev's, and a lone
  file is never refused. A lone file's pieces are all of one character, none held by an item a
  macro's metavariable names; it includes no file and no file includes it; it names no other file's
  item and no other file names its items; its own pieces do not spell the name; and it neither
  writes the name nor fails closed. Its witness is dev's reader refusing such a file.

## What would make this wrong

- A real join in the tree that carries a single character through a function. It would show the
  disclosed class is live, and (d+fn), or a typed follower, would be owed.
- A real-tree refusal from a reach widened by a common `const` or `static` name.
- A lone run of five or more characters refusing the real tree. It would show the multi-character
  residual needs the same treatment for short pieces, which (c) gives at the cost of B4 and B5.
- A tree that grows an item named by a macro's metavariable and holding one character. The
  fail-closed arm pools that character workspace-wide, so such an item could bring the defect back
  for its own letter. dev holds 0 such items.

## More Information

SPEC-331; SPEC-324 (R3 amended); ADR-325; ruling 99 (Q-j); issues #460, #585, #600.
