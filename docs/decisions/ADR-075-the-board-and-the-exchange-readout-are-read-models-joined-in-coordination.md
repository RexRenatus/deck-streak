---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat"
---

# The board and the exchange readout are read models joined in coordination

## Context and Problem Statement

SPEC-075 ports two reads the predecessor reached only through its agent read tool: the personal
board of #79 (`pipeline_layers/read_api.py:ReadApiLayer.leaderboard` at `27ee2bc`: the best day
among the most recent rollups, today's score, the streak and the level) and the XP exchange readout
of #80 (`exchange.py:exchange_rates`: XP per graduation, bucketed by source, over a window). Both
are read models: they write nothing. Its planned text placed the readout's two-table read in
coordination, added an api module of its own with a router change, and named no formal entry. At
`dev` a9d2b73 four facts decide where each read lives:

- Progression's census test (`crates/progression/tests/xp_census.rs`,
  `only_progression_writes_xp_settlement_and_only_coordination_settles`, ADR-072) refuses every
  crate but progression whose source names `xp_settlement`.
- The context map gives progression no edge to analytics, and analytics none to progression; only
  coordination depends on both (`docs/CONTEXT-MAP.md`).
- The recompute writes one study day's rollup, its graduations included, and the day's settled XP
  in one `BEGIN IMMEDIATE` write (`crates/analytics/src/rollup.rs`'s module doc: "one day's settle
  is one write"; `docs/schematics/recompute-settles-each-study-day.md`).
- `crates/api/src/badges_routes.rs` already answers `GET /api/records` and
  `crates/api/src/xp_routes.rs` answers `GET /api/level`, each with its own router merged by the
  api's router.

## Decision Drivers

- A rate divides one read by another, so its numerator and denominator must come from one
  committed state of the ledger.
- The census and the context map are binding: a read that needs a table name or an edge they do
  not grant is in the wrong crate.
- The two surfaces are the Mini App's `/records` and `/level` screens; the board shows no one but
  the owner (#79: "The board is single-user and never shows another person").
- The numbers are proved against the predecessor's goldens (ADR-012), and the pure rules (the tie
  order, the bucket, the once-per-day denominator, the defined flag, the capped window) are proved
  in Lean too.

## Considered Options (the alternatives it was chosen against)

- D1, the XP rows read in progression (`exchange::xp_rows`, both tables, one statement each), the graduations in analytics (`rollup::graduations`), and the window and the join in coordination (`exchange_view`): chosen because each read sits in the crate that owns its table, the census stays green, and no edge is added.
- Coordination naming both XP tables in its own query: rejected because the census refuses any crate but progression that names the settled table, so the census test would go red.
- Progression reading `daily_rollup` for the graduations: rejected because progression has no edge to analytics, and the edge is the design; adding one to make the read compile is the move the context map forbids.
- D2, the readout's three reads on one read transaction (`db.reader().begin()`), passed by connection to each repository read: chosen because the recompute writes a day's graduations and its settled XP in one write, so one snapshot never divides one recompute's XP by another's graduations.
- Three separate statements for the readout, each on its own connection: rejected because a recompute can commit between them, and the rate would be torn across that commit.
- D3, the board's rows each from one statement, with no transaction: chosen because no board row combines two reads, so a commit between two rows changes no row's meaning.
- One read transaction for the board too: rejected because it needs a second, connection-taking copy of `SqliteXpLedger::total`'s query for no observable gain.
- The best day read among the 365 most recent rollups on or before today, as `recent_totals` bounds it: chosen because the recompute writes no day after today, so it agrees with the predecessor on every store DeckStreak can hold.
- An unbounded best-day read, for byte parity with a store holding a rollup after today: rejected because DeckStreak cannot reach that state, and the bound is the read every other award already uses.
- D4, `GET /api/board` in `badges_routes.rs` and `GET /api/xp/exchange` in `xp_routes.rs`, each on the module's existing router: chosen because no new module and no router change is needed, and the two screens' other routes already live there.
- A `board_routes.rs` module with its own `router.rs` and api `lib.rs` lines: rejected because it adds a module and two edits to shared files for two handlers that fit beside their screens' routes.
- Folding the board into `GET /api/records`'s body: rejected because it changes SPEC-073's response and its tests for a section the screen can fetch on its own.
- A `/board` bot command: rejected because no issue asks for one, and it would add a message golden and payload rows for a duplicate surface; the agent read tool arrives with the MCP server (#157).
- An `/insights` instrument for the readout: rejected because SPEC-094's instruments are stored and scheduled, while this window is an argument of one read; the readout is a card on `/level` (SPEC-075 R9).
- D5, the constants 365 and 3650 as coordination's `BOARD_ROLLUPS` and `EXCHANGE_WINDOW_CAP`, each pinned by a mutation row: chosen because they are read-window parameters, and the game-economy reference has no key for either.
- 365 and 3650 as keys of `economy.json`: rejected because a SPEC never adds a key to `economy.json`, and its `economy-declared` check refuses one the reference does not name.

## Decision Outcome

Chosen options: D1 to D5, because each keeps the census, the context map and every existing route
module as they are, and the readout reads one committed state.

- Progression owns the pure rules: `board::best_day` and `board::board_rows`, and
  `exchange::bucket` and `exchange::exchange_rates`. It reads the window's XP rows of both tables
  in `exchange::xp_rows`, one `query!` per table, the settled rows appended after the grants.
- Analytics answers `rollup::graduations(connection, window)`, each day's graduations in the
  window by day.
- Coordination's `progression::board_view` maps analytics' `RecentTotals` to progression's own
  `DayScore`, reads the language streak and the XP total, and asks `board_rows`. Its
  `progression::exchange_view` computes the window (`days` of 0 or less is every day; else the
  smaller of `days` and 3650 study days ending today), opens one read transaction, reads the XP rows
  and the graduations on it, and asks `exchange_rates`.
- The api answers both behind the owner's session: 503 `database_not_open` before the database is
  open, 500 `board_unreadable` or `exchange_unreadable` when a read fails, and 400 `invalid_days`
  for a `days` that is not an integer. An undefined rate is JSON null beside `rate_defined` false.
- The Mini App shows the board as a section of `/records` and the readout as a card on `/level`.

### Consequences

- Good, because the census test, the context map and the api's router are unchanged, and the
  readout's rate never mixes two recomputes.
- Good, because the board and the readout reuse the reads `/api/records` and `/api/level` already
  make, so the level the board shows is the level `/api/level` answers.
- Bad, because the readout's window, its fold and its reads live in three crates, so a reader
  follows coordination's view to find all three.
- Bad, because no test can observe a torn read deterministically, so D2 is held by this record and
  by review, not by a mutation row.

### Confirmation

SPEC-075's tests: the board and the readout equal the predecessor's goldens (A1, A3, A5, A7); the
readout reads both XP tables and loses no row (A6, A12); a bucket with no graduation has no rate
(A4, A15); the readout writes nothing (A8); and the Lean entry `formal/lean/Formal/Exchange.lean`
proves the tie order, the bucket, the denominator, the defined flag and the window, with the
readout answering its vectors (A13).

### What would make this wrong

- A board row that combines two reads, such as a Today row computed from a rollup and a separate
  live score: it would then need D3's transaction.
- A reader that acts on what it read, such as a readout that re-prices a source: the readout would
  then be a writer beside the recompute, an interleaving a TLA+ model must check, and re-pricing is
  #281's.
- A census or context-map change that lets coordination name the settled table, or gives
  progression an analytics edge: D1's split would then be a choice, not a constraint.

## More Information

SPEC-075 (R1 to R12, and its sections 10 to 15); SPEC-071 (`recent_totals`, the rollups'
graduations); SPEC-072 and ADR-072 (both XP tables, `level_info`, `/api/level`); SPEC-073 (the
records screen, `badges_routes.rs`); SPEC-076 (the language streak); ADR-012 (the parity oracle);
ADR-016 (promotion); the predecessor's functions named above at `27ee2bc`; #79; #80; #157; #267;
#281.
