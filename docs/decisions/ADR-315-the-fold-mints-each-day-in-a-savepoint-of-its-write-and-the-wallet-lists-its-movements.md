---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The fold mints each study day in a savepoint of the day's write, and the wallet lists its movements newest first on a screen of its own

## Context and Problem Statement

SPEC-082 R4 has the fold of SPEC-071 mint each study day it evaluates from the day's final base:
the step registers in phase 6, after phase 5's derived bonuses; the current day's mint follows its
base at each recompute; a settled day's mint is raised and never lowered; and the days the first
recompute backfills are minted too. E1 (#543) built the port, `settle_mint_on`, which takes the
caller's `Transaction` so no caller can read outside the write lock (ADR-308 ruling 1). The fold
hands each step a bare `&mut SqliteConnection` inside the day's `BEGIN IMMEDIATE` write
(`recompute/mod.rs::DayStep`). Two cycles reach the fold, the scheduled one and the owner's
recompute, in either order. #106 also asks for a cheap history view of the ledger beside the
wallet header, which R15 serves as movements, newest first and paged, and R17 names no screen for.
Where does the mint read its base, how does it reach the port inside the fold's write, and where
does the owner read the movements?

## Decision Drivers

- Phase 2's XP step is the only writer of a day's base: phase 5 writes only `consistency` and
  `ascendant`, and both are in `economy.json`'s `day_base_excludes`, so the base the mint reads is
  final once phase 4 has run (measured at dev `340d8967`; the predecessor's
  `pipeline.py:_recompute_day` also mints after `_apply_day_bonuses`).
- A settled day's base is raised and never lowered by progression's settle (`settle.rs::settle`),
  so a mint taken from the day's rows on a closed day never falls; a mint taken from one cycle's
  own reading of the reviews can.
- `settle_mint_on`'s `Transaction` parameter is ADR-308 ruling 1's compile-time guarantee, and
  `tla/WalletFloor` covers the function's span.
- The fold runs in four kinds of write (`Fold::run`): the first recompute's backfill, one write per
  settled day, the current day, and the revisit of every past day. A crash can fall between any two.
- The header is on every screen, so whatever it renders is paid on every screen.

## Considered Options (the alternatives it was chosen against)

- A phase-6 step that opens a savepoint on the fold's connection with `sqlx::Connection::begin` and calls `settle_mint_on` inside it: chosen, because inside the fold's `BEGIN IMMEDIATE` the pinned sqlx issues `SAVEPOINT _sqlx_savepoint_1`, so the mint commits or rolls back with the day's write while the port keeps its signature and `tla/WalletFloor` its covers (#106).
- `settle_mint_on` taking a bare `&mut SqliteConnection`: rejected, because it removes ADR-308 ruling 1's compile-time guarantee that no caller reads outside the write lock, which is a weakening (#106).
- `DayStep::evaluate` handing each step a `Transaction` instead of a connection: rejected, because it is churn across eleven `DayStep` impls (six in the crates' sources, five in their tests) and the fold body that CO1 (#558) is changing at the same time (#106).
- The mint read on the bot's or the route's read path: rejected, because a read would then write, and a day would be minted only when someone looks at the wallet (#106).
- A mint job of its own after the fold, in a transaction of its own: rejected, because it is a second cycle to model, and it reads a base the fold can change under it between the two writes (the witness `a-mint-written-in-a-transaction-of-its-own` of `tla/MintReadsTheFinalBase`) (#106).
- The mint step registered before phase 2: rejected, because it reads the day's base before the XP step writes it (the witness `a-mint-read-before-its-days-base-is-written`) (#106).
- The mint taken from the base this cycle computes from its own reading of the reviews: rejected, because a stale cycle's reading of a closed day can sit below the base its rows already hold, so the mint would fall (the witness `a-mint-from-the-cycles-own-reading`) (#106).
- The mint for the current day only: rejected, because R4 mints every day the first recompute backfills, as the predecessor minted every day of its window (the witness `a-backfill-that-writes-no-mint`) (#106).
- A /wallet screen of its own, linked from the header, listing the movements newest first a page at a time: chosen, because the header stays one line on every screen and the history costs a read only when the owner opens it (#106).
- The movements listed inside the header itself: rejected, because the header renders on every screen, so every screen would read and lay out the ledger (#106).
- The movements' page query as one constant statement run by `sqlx::query_as` in the wallet: chosen, because a mutant of its `ORDER BY` compiles, so a mutation row proves the order the owner reads, and the read's own test proves its columns in place of the offline query cache (#106).
- The page query as a `sqlx::query!` statement: rejected, because a mutant of a `query!` statement does not compile against the offline query cache, so no mutation row could prove the order it serves (#106).
- Pages by offset (`?page=N`): rejected, because a movement written while the owner reads would shift every later page by one, showing a movement twice or skipping one; a cursor on the last movement shown does not move (#106).

## Decision Outcome

Chosen option: a phase-6 mint step in a savepoint of the day's write, and a /wallet screen fed by
GET /api/wallet, with these rulings.

1. **The step is registered once, in phase 6.** `recompute/mint.rs::MintStep` declares
   `Phase::CoinMint`, and the composition root registers it after phase 5's step. It opens no
   connection and no transaction of its own: it opens a savepoint on the connection the fold hands
   it, so a fold that fails after phase 6 leaves no mint, and no other connection reads the mint
   before the fold commits. A mint taken between phases 4 and 5 would equal this one, because phase
   5 writes no base source, so the model's witnesses are a mint before phase 2, in a write of its
   own, from the cycle's own reading, and none at all on a backfilled day.
2. **The base is the day's rows.** The step reads the day's settled XP rows and sums them the way
   phase 5 does (`consistency::day_base_xp`, which leaves out `day_base_excludes`), then mints
   `mint_for_base_xp` of that sum. It runs for every evaluation the fold makes, backfill, settle,
   current and revisit, and reports the day `closed` for every evaluation but the current day's, so
   `settle_mint_on` raises a settled day's mint and lets the current day's follow its base.
3. **The history view.** The header shows the balance and links to the /wallet screen. That
   screen lists the movements newest first (by study day, then by the order they were written), one
   page at a time, from GET /api/wallet. The page size is named once, in the wallet's page query,
   which is one constant statement run by `sqlx::query_as`, and the cursor is the last movement
   shown, passed back as `before`. Each line shows its study day, its source in plain words and its
   signed amount. No line urges, counts down or shames.

### Consequences

- Good, because the mint lands in the same commit as the base it reads, under both cycles and any
  crash between writes.
- Good, because `settle_mint_on`, its covers and ADR-308's guarantee are untouched.
- Good, because the history costs nothing on the screens that do not show it.
- Bad, because the revisit write re-mints every past day of the window at every recompute. Each is
  one read of the day's rows and at most one update, and a settled day's mint only moves up.
- Bad, because a savepoint is one more statement pair per evaluated day inside the write.

### Confirmation

`tla/MintReadsTheFinalBase` models the two cycles, each write of the fold as one action and a crash
between any two writes, and checks that a settled day's mint equals the mint of its final base, that
it never falls, and that the current day's mint follows its base after each recompute. Its five
witnesses are each caught, and the built configuration is clean at its state floor.
`wallet_mint::the_mint_reads_the_settled_days_final_base` judges a population of backfilled days, a
settled day raised by a later recompute and the current day re-minted down and up against the parity
golden of `mint_for_base_xp` over each day's final base, and the same file proves the savepoint: a
fold that fails after phase 6 leaves no mint, and a second connection reads none before the fold
commits. `wallet-history.test.ts` judges the order of the history view's lines.

## What would make this wrong

- A later phase-5 or phase-6 step that writes a base source: the mint would read a base that is not
  final. The model's `covers` lines on the phase declaration and the step read `STALE` when they move.
- A step that hands the port a connection outside the fold's write: the savepoint would then open a
  transaction of its own. The step takes only the connection the fold gives it.
- A source of a coin movement with no plain words in the history view: the line would show the
  source's code. The view's words are read from the source codes the ledger holds.

## More Information

Issue #106; SPEC-082 R4, R15 and R17, A7, A15, A18 and A20; ADR-071 (the fold settles each day once,
in order), ADR-072 (the day's base over both XP tables) and ADR-308 (the wallet's ports); the
schematic `docs/schematics/coin-wallet-and-its-ports.md`, whose fold edge this step realises.
