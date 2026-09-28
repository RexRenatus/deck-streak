# SPEC-075: the personal board and the XP exchange readout are read from the ledgers

- **Wave:** W3. **Issue:** #79, #80 (epic #4). **Context(s):** `deck-streak-progression` (the board's
  rows, the exchange readout's buckets and rates); `deck-streak-coordination` (the two read models,
  joining analytics' rollups, streaks' state and both XP tables); `deck-streak-api` (their routes);
  the Mini App (`web/app`) (the board on the records screen, the readout on the level screen).
- **Decided by:** ADR-012 (the parity oracle proves the math) and ADR-072 (XP lives in two tables,
  and every reader of XP reads both).
- **Prerequisites:** SPEC-071 (the rollups, their scores and graduations), SPEC-072 (both XP tables,
  the level and its title, the level screen), SPEC-073 (the records screen the board joins), SPEC-076
  (the language streak and its longest). **Mutation band:** `S07500-S07599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-075.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` c3d769b, nothing reads a personal best, and nothing joins XP to
  graduations. Both features were reachable in the predecessor only through its agent read tool.
- **What is ported** (predecessor `27ee2bc`, names only):
  - #79: `pipeline_layers/read_api.py:ReadApiLayer.leaderboard`: the best score among the most recent
    rollups with its day, today's score, the streak's current and longest, and the level with its
    title;
  - #80: the readout `exchange.py:exchange_rates`, with its buckets `exchange.py:normalize_source`
    and its JSON rows `pipeline_layers/economy.py:EconomyLayer.xp_exchange_rates`, and the window its
    read tool computes (`server.py:create_server`, the tool `get_xp_exchange_rates`).
- **What is not ported.** The readout's second half, a per-source re-pricing of future grants
  (`exchange.py:apply_multiplier`, `pipeline_layers/economy.py:EconomyLayer.grant_priced_xp`), has no
  production caller in the predecessor. It is excluded with the reason "inert in v9", and the owner
  decides whether to revive it (#267); #80's own note keeps it dormant unless the owner asks.
- **Traps a hand port falls into.**
  - The readout's denominator is each day's graduations, counted once per bucket and day, never a sum
    of a mature-card snapshot.
  - A bucket that bought no graduation has an undefined rate, reported as no rate and
    `rate_defined` false, never as 0; a renderer that writes "rate or 0" turns it into a real zero.
  - A day's XP with no rollup still counts, with no graduation (the predecessor's left join).
  - DeckStreak keeps XP in two tables (ADR-072), so the readout reads both, or it loses rows.
- **Corrections to the issues, read in the predecessor's code.**
  - #79: the best day is the highest score among the 365 most recent rollup rows, not 365 calendar
    days, and a tie goes to the most recent of them; the golden proves both.
  - #80: a window of 0 or less reads every day, and a window of N reads the smaller of N and 3650
    study days ending today; there is no clamp to 1. The golden proves the bound.
- **What the parity oracle proves.** The board over synthetic rollups, a streak and a level; the
  readout over synthetic ledger and rollup rows; the bucket of a source; the window.
- **Prerequisites.** SPEC-071, SPEC-072, SPEC-073 and SPEC-076, above. This SPEC registers no step in
  SPEC-071's fold: both features are read models that write nothing.

## 2. Requirements

The personal board (#79)

R1. The board equals `goldens/leaderboard.json` (`pipeline_layers/read_api.py:ReadApiLayer.leaderboard`),
    in its order: when any rollup exists, "Best day" (the highest score among the 365 most recent
    rollups on or before today, with its day; the most recent of them on a tie) and "Today" (today's
    live score, 0 when today has no rollup); then "Streak" (the language streak's current length, with
    its longest, SPEC-076); then "Level" (the level, with its title, SPEC-072). Each row's label and
    emoji are the golden's.
R2. `GET /api/board` answers the owner's session only (SPEC-024), with 401 or 403 and no data to any
    other caller; the board names no one but the owner.
R3. The Mini App's `/records` (SPEC-073) gains the board as a section, its days shown as the server's
    study days.

The XP exchange readout (#80)

R4. The readout equals `goldens/exchange_rates.json` (`exchange.py:exchange_rates`): for each bucket
    (a source's text up to and including its first colon, or the whole source when it has none,
    `goldens/exchange_normalize_source.json`, `exchange.py:normalize_source`), the total XP; the
    graduations of the days that bucket paid on, each day's graduations taken from analytics' rollup
    (SPEC-071) and counted once per bucket and day, a day with no rollup counting none; the rate of XP
    per graduation; and whether the rate is defined. A bucket with no graduation has no rate and
    `rate_defined` false. The buckets are ordered by name.
R5. The readout reads `xp_ledger` and `xp_settlement` together, and loses no row: over a window, the
    totals of its buckets sum to the XP of both tables in that window.
R6. The window equals `goldens/exchange_window.json` (`server.py:create_server`, its tool
    `get_xp_exchange_rates`): a window of 0 or less reads every day, and a window of N reads the
    smaller of N and 3650 study days ending on the current study day, both ends included.
R7. `GET /api/xp/exchange?days=N` answers the owner only, with the readout as JSON; an undefined rate
    is JSON null beside `rate_defined` false, never 0.
R8. The readout is read-only: it writes nothing, and nothing in it changes the amount of any grant or
    settled row.
R9. The Mini App's `/level` (SPEC-072) gains the readout as a card, which renders an undefined rate as
    undefined, never as 0.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the board built from synthetic rollups, a streak and a level equals the golden (examined count reported, zero refused) | `the_board_matches_the_parity_golden` |
| A2 | the board and exchange routes answer only the owner (401 or 403, no data) | `the_board_and_exchange_routes_answer_only_the_owner` |
| A3 | the readout over synthetic ledger and rollup rows equals the golden | `the_exchange_readout_matches_the_parity_golden` |
| A4 | a bucket with no graduation has no rate and `rate_defined` false | `a_bucket_with_no_graduation_has_an_undefined_rate` |
| A5 | a source's bucket equals the golden | `the_bucket_of_a_source_matches_the_parity_golden` |
| A6 | the readout reads both XP tables, and its bucket totals sum to both tables' XP in the window | `the_readout_reads_both_xp_tables_and_loses_no_row` |
| A7 | the window equals the golden | `the_exchange_window_matches_the_parity_golden` |
| A8 | a readout leaves every XP row as it was | `the_readout_writes_nothing` |
| A9 | the records screen's board section shows the best day, today, the streak and the level | `shows the best day, today, the streak and the level` |
| A10 | the readout card renders an undefined rate as undefined, never zero | `renders an undefined rate as undefined, never zero` |

```acceptance
A1: cargo test -p deck-streak-coordination --test board_view -- --exact the_board_matches_the_parity_golden
A2: cargo test -p deck-streak-api --test board_routes -- --exact the_board_and_exchange_routes_answer_only_the_owner
A3: cargo test -p deck-streak-progression --test exchange_rates -- --exact the_exchange_readout_matches_the_parity_golden
A4: cargo test -p deck-streak-progression --test exchange_rates -- --exact a_bucket_with_no_graduation_has_an_undefined_rate
A5: cargo test -p deck-streak-progression --test exchange_rates -- --exact the_bucket_of_a_source_matches_the_parity_golden
A6: cargo test -p deck-streak-coordination --test exchange_view -- --exact the_readout_reads_both_xp_tables_and_loses_no_row
A7: cargo test -p deck-streak-coordination --test exchange_view -- --exact the_exchange_window_matches_the_parity_golden
A8: cargo test -p deck-streak-coordination --test exchange_view -- --exact the_readout_writes_nothing
A9: pnpm exec vitest run web/app/src/lib/records/BoardSection.test.ts -t "shows the best day, today, the streak and the level"
A10: pnpm exec vitest run web/app/src/lib/level/ExchangeCard.test.ts -t "renders an undefined rate as undefined, never zero"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The accessibility pack stays enforced; no row is
deferred for this delivery.

| id | criterion | decided by |
|---|---|---|
| B1 | the accessibility checks pass over `web/app/src/lib/records/BoardSection.svelte` and `web/app/src/lib/level/ExchangeCard.svelte`, examining every element of the section and the card, the undefined rate's text alternative included | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/src/board.rs` | `deck-streak-progression` | added: the board's rows from a best score, today's score, a streak and a level |
| `crates/progression/src/exchange.rs` | `deck-streak-progression` | added: the buckets, the rate and its defined flag |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the modules above |
| `crates/progression/tests/exchange_rates.rs` | `deck-streak-progression` | added: A3 to A5 |
| `crates/coordination/src/progression/board_view.rs` | `deck-streak-coordination` | added: the board from analytics' rollups, streaks' state and the level |
| `crates/coordination/src/progression/exchange_view.rs` | `deck-streak-coordination` | added: the readout over both XP tables and the rollups' graduations, and its window |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules above |
| `crates/coordination/tests/board_view.rs` | `deck-streak-coordination` | added: A1 |
| `crates/coordination/tests/exchange_view.rs` | `deck-streak-coordination` | added: A6 to A8 |
| `crates/api/src/board_routes.rs` | `deck-streak-api` | added: the board and exchange routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the two routes |
| `crates/api/tests/board_routes.rs` | `deck-streak-api` | added: A2 |
| `web/app/src/lib/records/BoardSection.svelte` | miniapp | added |
| `web/app/src/lib/records/board.ts` | miniapp | added: the board's types and its fetch |
| `web/app/src/lib/records/BoardSection.test.ts` | miniapp | added: A9 |
| `web/app/src/routes/records/+page.svelte` | miniapp | changed: the board section |
| `web/app/src/lib/level/ExchangeCard.svelte` | miniapp | added |
| `web/app/src/lib/level/exchange.ts` | miniapp | added: the readout's types and its fetch |
| `web/app/src/lib/level/ExchangeCard.test.ts` | miniapp | added: A10 |
| `web/app/src/routes/level/+page.svelte` | miniapp | changed: the readout card |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `tools/parity-oracle/registry/spec_075.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/leaderboard.json` | repo | added: the golden of `pipeline_layers/read_api.py:ReadApiLayer.leaderboard` (adapter: a stub store) |
| `tools/parity-oracle/goldens/exchange_rates.json` | repo | added: the golden of `exchange.py:exchange_rates` (adapter: a temporary store database seeded from the case) |
| `tools/parity-oracle/goldens/exchange_normalize_source.json` | repo | added: the golden of `exchange.py:normalize_source` (function) |
| `tools/parity-oracle/goldens/exchange_window.json` | repo | added: the golden of `server.py:create_server`'s window (adapter: a stub pipeline recording the window) |
| `scripts/mutation-rows.d/S07500-S07599.json` | repo | added: the rows of section 9 |
| `docs/specs/SPEC-075-the-personal-board-and-the-xp-exchange-readout-are-read-from-the-ledgers.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-075.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It re-prices no XP source: the predecessor's per-source re-pricing has no production caller, so it
  is inert in v9 and waits for the owner's decision (#267).
- It shows no one but the owner: DeckStreak serves one owner per deployment (#172).
- It adds no bot command and no agent read tool: the predecessor reached both features only through
  its agent read tool, which arrives with the MCP server (#157).
- It creates no table and imports nothing: both are read from tables other SPECs create (#61).

## 6. Risks

- **An undefined rate renders as zero.** A reader of `rate` alone collapses "no graduation" into a
  real 0. Detected by A4 and A10, and by row S07503.
- **The readout loses the settled XP.** A readout over `xp_ledger` alone would drop every derived
  source. Detected by A6 and row S07504.
- **A graduation day is counted once per row, not once per bucket and day.** The denominator would
  grow with the row count. Detected by A3's golden, whose cases hold several rows of one bucket on
  one day, and by row S07505.
- **The board's best day reads a different window.** Detected by A1's cases, which hold more than 365
  rollups, and by row S07501.

## 7. Parity goldens

Every golden is generated on the owner's machine from the predecessor at `27ee2bc` (SPEC-029), with
synthetic inputs only, and registered in `tools/parity-oracle/registry/spec_075.py`.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `leaderboard` | `pipeline_layers/read_api.py:ReadApiLayer.leaderboard` | adapter | a stand-in layer over a stub store holding the case's rollups (more than 365 in some cases, ties in others), a streak state and a total XP, with today's study day; each row returned as its label, value and detail, a day as its epoch day |
| `exchange_rates` | `exchange.py:exchange_rates` | adapter | a temporary store database it creates from the case's synthetic ledger rows (several rows of one bucket on one day, days with no rollup) and rollup rows, each day an epoch day turned into the store's form inside the adapter; each bucket returned with its total, graduations, rate and defined flag |
| `exchange_normalize_source` | `exchange.py:normalize_source` | function | literal sources, prefixed sources, several colons, an empty prefix |
| `exchange_window` | `server.py:create_server`, the tool `get_xp_exchange_rates` | adapter | the server built over a stub pipeline that records the window it is asked for and answers a fixed study day as today, with synthetic settings it never binds and a stub drill gate; each case's window returned as its first and last day relative to today, or none for every day |

## 8. Tables and the v9 import

This SPEC creates no table. The board reads analytics' rollups, streaks' state and progression's
level; the readout reads both XP tables and analytics' rollups.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S07501-THE-BOARD-WINDOW` | `crates/coordination/src/progression/board_view.rs` | the best day is read among the 365 most recent rollups | `board_view::the_board_matches_the_parity_golden` |
| `S07502-THE-LATEST-BEST-DAY-ON-A-TIE` | `crates/progression/src/board.rs` | a tie for the best score goes to the most recent day | `board_view::the_board_matches_the_parity_golden` |
| `S07503-AN-UNDEFINED-RATE` | `crates/progression/src/exchange.rs` | a bucket with no graduation has no rate | `exchange_rates::a_bucket_with_no_graduation_has_an_undefined_rate` |
| `S07504-BOTH-XP-TABLES` | `crates/coordination/src/progression/exchange_view.rs` | the readout reads the settled XP beside the grants | `exchange_view::the_readout_reads_both_xp_tables_and_loses_no_row` |
| `S07505-ONE-GRADUATION-DAY-PER-BUCKET` | `crates/progression/src/exchange.rs` | a day's graduations count once per bucket | `exchange_rates::the_exchange_readout_matches_the_parity_golden` |
| `S07506-THE-BUCKET-PREFIX` | `crates/progression/src/exchange.rs` | a source's bucket ends at its first colon | `exchange_rates::the_bucket_of_a_source_matches_the_parity_golden` |
| `S07507-THE-WINDOW-CAP` | `crates/coordination/src/progression/exchange_view.rs` | a window reads at most 3650 study days | `exchange_view::the_exchange_window_matches_the_parity_golden` |
