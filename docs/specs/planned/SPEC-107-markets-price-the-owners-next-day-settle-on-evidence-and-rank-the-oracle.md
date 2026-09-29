# SPEC-107: the markets price the owner's next day, settle on evidence, and rank the Oracle

- **Wave:** W5. **Issue:** #119 (self-prediction markets and the Oracle ladder), in epic #6.
  **Context(s):** `deck-streak-markets` (the pricing, the quotes and the board, the trade's and the
  cancel's verdicts, the settlement, the revision, the Brier and the rank, the Oracle's summary,
  digest block, weekly line and calibration series, and `market_positions`);
  `deck-streak-coordination` (the inputs, the trade's and the cancel's transactions, the markets
  step of the sync cycle, the void port and the calibration's read model); `deck-streak-analytics`,
  `deck-streak-progression`, `deck-streak-streaks`, `deck-streak-discipline` and
  `deck-streak-economy` (one read each, below); `deck-streak-bot` (`/predict`, `/oracle`);
  `deck-streak-api` (the markets' routes); the Mini App (`web/app`, the `/markets` screen).
- **Decided by:** ADR-107 (this SPEC's: a lost position is judged again within 7 closed study days
  and paid when its evidence now holds), ADR-104 (a discipline verdict is provisional until its
  evidence settles, the rule ADR-107 carries into markets), ADR-085 (charts render on the client
  from JSON series), ADR-071 (a settled day's rollup changes only with its reviews), ADR-037 (one
  sync a study day) and ADR-012 (the parity oracle).
- **Prerequisites:** SPEC-105 (discipline's engine switch, the booked windows and their judged
  occurrences), SPEC-082 (the wallet's `purchase`, `credit` and `refund`), SPEC-071 (the rollups),
  SPEC-100 (the recent rollups' read), SPEC-072 (the law track's XP in both XP tables), SPEC-076
  (the governor's stored verdict and the freeze markers), SPEC-083 (the skip set), SPEC-090 (the
  adaptive goal), SPEC-084 (the celebration ladder), SPEC-085 (the chart route and component),
  SPEC-041 (the router) and SPEC-023 (the sync cycle). **Mutation band:** `S10700-S10799`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-107.md` (ADR-016).

## 1. The problem, measured

- **What exists.** `crates/markets/src/lib.rs` holds only its doc comment, and the ownership
  register names `market_positions` for markets with no migration behind it. SPEC-085 draws no
  calibration chart and leaves it to the markets (its section 5). After SPEC-105 builds, discipline
  holds the engine switch and the windows' judged occurrences; after SPEC-082, the wallet's ports.
- **What is ported** (at `27ee2bc`):
  - the arithmetic, `gamification/forecast.py:laplace_rate`, `blended_probability`, `price_for`,
    `is_degenerate_rate`, `is_degenerate`, `history_priceable`, `window_priceable`, `payout_for`,
    `survival_probability`, `brier_score`, `rank_index` and `rank_for`;
  - the board and the trades, `pipeline_layers/markets.py:MarketsLayer._closed_rollups`,
    `_candidate_keys`, `_market_quote`, `_reviews_price`, `_retention_price`, `_window_quote`,
    `_streak_sun_quote`, `_law_touch_quote`, `market_board`, `place_position` and `cancel_position`;
  - the settlement, `MarketsLayer._settle_markets`, `_market_truth`, `_day_survived`,
    `_void_position` and `_apply_panic`;
  - the Oracle, `MarketsLayer._brier_rank`, `oracle_summary`, `oracle_digest_block`,
    `oracle_weekly_line` and `settled_positions`, and `charts.py:calibration_chart`;
  - the surfaces, `bot.py:CommandBot._render_market_board`, `_render_oracle`, `_oracle_keyboard` and
    `_callback_market`.
- **The clock.** A market is priced on closed days only and settles at the first sync cycle after
  its outcome day, from the rollup, the windows, the law track's XP, the skip set, the freeze
  markers and the governor. Reviews that sync after that cycle change the day's rollup (ADR-071), so
  a loss can rest on reviews DeckStreak had not read.
- **Deviations from the predecessor, each with its reason.**
  - A lost position is judged again within 7 closed study days and becomes won, paid once, when its
    evidence now holds (ADR-107). The predecessor settles once, so a late sync, or a freeze consumed
    after the settlement, kept the stake of a forecast that came true.
  - The goal market stores the goal's threshold at the trade and settles on it. The predecessor
    reads the goal again at settlement, so a goal that moved overnight judged a position on terms it
    was not sold at.
  - A window market is offered only when its window occurs on the outcome day (SPEC-105 R3). The
    predecessor offers every enabled window, and one that does not occur that day can only lose.
  - A window market whose outcome day holds no judged occurrence (the window was removed) is voided.
    The predecessor's truth reads the missing occurrence as a loss, against its own rule that a
    truth it cannot resolve voids.
  - The trade's row and its stake, a cancel's delete and its refund, and each settlement's status
    and its coins are each one transaction. The predecessor commits the row first and accepts the
    crash window between the two.
  - The buttons carry the outcome day as an epoch day number, never a calendar string.
  - Nothing sends a settlement message or a separate panic line. The predecessor sends both through
    `EconomyLayer._notify_awake`, which drops a message inside quiet hours, where the first cycle
    after the rollover runs by default; the digest's Oracle block reports each settled and voided
    position (#129), and the panic's own notice speaks for the panic (SPEC-106).
  - The calibration chart is a JSON series drawn on the client (ADR-085), not an image.
  - The owner's tapped confidence is named `confidence` in the table and the types: `forecast`
    already names a course's projected completion (SPEC-090).

## 2. Requirements

The arithmetic

R1. The probability and the price equal the golden `market_price`: `laplace_rate(hits, n)` is `(hits
    + 1) / (n + 2)`; `blended_probability` averages the whole window's rate with the outcome
    weekday's rate once that weekday has at least 6 rows, and is the whole window's rate below it;
    `price_for(p)` is `round(100p)`, rounded half to even as Python rounds, clamped to 15..90; and
    `survival_probability` is the product of its rates, 1 for none.
R2. The floors equal the golden `market_degenerate`. `gamification/forecast.py:is_degenerate_rate`
    answers degenerate for a raw hit rate strictly above 0.95 or strictly below 0.05, and priceable
    otherwise, and `is_degenerate` answers degenerate for an empty window; a degenerate market is
    not offered. `history_priceable` needs at least 14 rows and `window_priceable` at least 8 judged
    occurrences.
R3. The payout equals the golden `market_payout`: `min(150, max(stake + 1, stake × 95 div price))`,
    fixed at the trade.

The board (#119)

R4. The candidate markets, in order: `reviews:30`, `reviews:60`, `reviews:100`, `reviews:goal`,
    `retention:85`, one `window:<id>` per booked window by id, `streak_sun` and `law_touch`. Each is
    quoted fresh from its inputs, and a quote is either absent or a label, a price and an outcome
    day. The outcome day is the study day after the current one, except `streak_sun`'s, which is the
    first Sunday strictly after the current study day. For the predecessor's inputs the quotes equal
    the golden `market_quote`.
R5. The rollup markets read the 91 most recent stored rollups before the current study day (R28):
    - `reviews:<T>`: at least 14 rows, the hits the rows with at least T reviews, the raw rate not
      degenerate, the price blended with the rows of the outcome day's weekday;
    - `reviews:goal`: the same with T the adaptive goal (SPEC-090 R4) as `pace_readouts` stores it
      (SPEC-090 R9); absent while the goal is pending or equals 30, 60 or 100;
    - `retention:85`: the rows with at least 20 answered, at least 14 of them, the hits the rows
      whose `true_retention`, a percent, is at least 85.0, priced by `laplace_rate` alone;
    - `streak_sun`: at least 14 rows; for each day from the next study day to the outcome day, the
      `laplace_rate` of the rows of that weekday that hold a review; their product, not offered when
      degenerate.
R6. `window:<id>` reads the window's judged occurrences of the 56 study days before the current one
    (R28): at least 8, the hits the `kept` ones, priced by `laplace_rate`, labelled with the
    window's start as `HH:MM`. It is offered only when the window occurs on the outcome day.
R7. `law_touch` reads the study days on which the law track gained XP (R28): absent until the first
    such day is 14 days behind the current study day; n is that age, at most 30; the hits are the
    distinct such days among the n days before the current study day, priced by `laplace_rate`.
R8. `pipeline_layers/markets.py:MarketsLayer.market_board`'s outcome set is shown or hidden with one
    reason: it is hidden with `disabled` while discipline's engine is off (SPEC-105 R1), else
    `lapse` while the governor's stored verdict holds an open lapse, else `standby` while it is in
    standby (SPEC-076 R16). Otherwise it lists each quoted market with its key, label, price, payout
    multiplier (Python's `round(95 / price, 2)`) and outcome day, the wallet, and the open
    positions, each marked cancellable when its outcome day is after the current study day. It
    equals the golden `market_board`.

The trade and the cancel

R9. `place(key, confidence, stake, outcome day)`
    (`pipeline_layers/markets.py:MarketsLayer.place_position`) refuses, in this order, with
    `governor standby/lapse — no new positions`, `discipline engine disabled — market closed`, `that
    market is not on today's board` (the quote made at the trade is absent), `market locked —
    tomorrow only` (the outcome day is not after the current study day, or is not the quote's),
    `confidence must be a tap choice` (not 60, 70, 80 or 90), `stake must be a preset ≤ <wallet div
    4>` (not 5, 10 or 25, or above a quarter of the wallet), `3 open positions max`, and `already
    holding this market for that day`. An accepted trade records the quote's price, R3's payout, the
    confidence, the stake, the outcome day and the current study day. Both equal the golden
    `market_place`.
R10. The trade is one transaction, which coordination holds through the kernel's base: the reads its
    verdict takes (R9's inputs, the wallet and the open positions), the position's row, and the
    stake's movement `purchase(day, "market_stake", "<position id>", stake)` (SPEC-082 R7). A
    refused trade writes nothing, and the price is never read from the caller.
R11. A `reviews:goal` position stores the goal's threshold at the trade, and its truth reads that
    threshold.
R12. The stake offers are the presets at most a quarter of the wallet, none below 5 answering
    `Wallet too thin — stakes cap at 25% of it.`, and the confidence offers are 60, 70, 80 and 90;
    they equal the golden `market_offers`.
R13. `cancel(id)` (`pipeline_layers/markets.py:MarketsLayer.cancel_position`) refuses with `no such
    open position` (absent or not open) and `locked at rollover — it rides` (the outcome day is not
    after the current study day); otherwise it deletes the row and refunds `refund(day,
    "market_refund", "<id>", stake)` in one transaction. Its verdicts equal the golden
    `market_cancel`. The row's id is never reused, so a refund's reference never names a later
    position.

The settlement

R14. At each sync cycle (SPEC-023 R12), after discipline's steps, the markets step voids every open
    position while discipline's engine is off. Otherwise each open position whose outcome day is
    before the current study day is settled from these inputs: the governor's stored verdict
    (SPEC-076 R16); the skip set (SPEC-083); the outcome day's rollup, its reviews, answered and
    `true_retention` (SPEC-071); the window's judged occurrence on the outcome day (SPEC-105); the
    law track's XP days (R28); the study days whose rollup holds a review, from the day after the
    trade to the outcome day; the freeze markers `consumed` of the days after those (SPEC-076 R6);
    and the position's stored threshold.
    - It is voided first when the governor is in standby or holds an open lapse, when the outcome
      day is a skip day, or when a `retention:85` outcome day has fewer than 20 answered.
    - Otherwise its truth, by market, is: for `reviews:<T>` and `reviews:goal`, the reviews at least
      the threshold; for `retention:85`, `true_retention` at least 85.0; for `window:<id>`, a `kept`
      occurrence on the outcome day; for `streak_sun`, each day from the day after the trade to the
      outcome day survived, by a review, a skip or a freeze consumed on the next day; for
      `law_touch`, law-track XP on the outcome day. A truth that cannot be resolved (a window
      market's outcome day with no judged occurrence) voids.
    - A won position credits `credit(day, "market_win", "<id>", payout)`, and a voided one refunds
      `refund(day, "market_refund", "<id>", stake)`, each in one transaction with its status; a lost
      one moves nothing more. A settlement changes a status only from `open`, and a revision only
      from `lost` to `won` (R15), so each movement is written once.
    The outcome set of `pipeline_layers/markets.py:MarketsLayer._settle_markets` is won, lost and
    voided, and `_market_truth`'s is true, false and unresolvable. For the predecessor's cases the
    outcomes equal the golden `market_settle`.
R15. At each sync cycle, each `lost` position whose outcome day is one of the 7 closed study days
    before the current study day is judged again on its truth, from R14's inputs as they now stand;
    the mercy is not judged again (ADR-107). When its truth now holds it becomes `won`, with its
    revision day, and its payout is credited through `credit(day, "market_win", "<id>", payout)` in
    the same transaction. `won` and `voided` are final.
R16. A won position, settled or revised, whose price is 25 or less raises the celebration
    `quest_all` with the key `market:<id>`. The Brier rank is computed before and after the step,
    and a rank after the step that is the first or a higher band raises `quest_all` with the key
    `oracle_rank:<name>`. Both go through SPEC-084's ladder on the router (SPEC-041), and neither
    moves a coin. The markets step records each raise pending (R25), as SPEC-106 R17 does: when the
    cycle carries a router (the owner's sync), the step raises it through that router at once;
    otherwise it waits for `discipline_tick` (SPEC-105 R12), which raises it through the router on
    its next run; either path clears it as SPEC-106 R17 does.
R17. `void_open_positions(day)`, which coordination's markets module holds, runs in its caller's
    transaction: each open position becomes `voided` with its settled day, and its stake is refunded
    through `refund(day, "market_refund", "<id>", stake)`; it answers the number voided. SPEC-106's
    panic calls it.

The Oracle

R18. The Brier is the mean of `(confidence / 100 − outcome)²` over the 30 most recent settled
    positions, won or lost, ordered by settled day and then id, newest first, summed as Python's
    compensated `sum()` sums (markets carries its own port: it depends on the kernel only).
    `gamification/forecast.py:rank_for` answers no rank below 10 settled positions in all, or with
    no Brier, and otherwise the first band whose floor the Brier reaches, by `rank_index`; the bands
    are Coin-Flipper at 0.25 or more, Apprentice at 0.18, Forecaster at 0.12, Oracle at 0.06, and
    Delphic below. They equal the golden `market_brier`.
R19. The Oracle's summary is the rank, the Brier, the settled count, the rank's floor of 10, the
    wins, the losses, the lifetime net of the sources `market_stake`, `market_win` and
    `market_refund` (R28), the wallet and the open positions, equal to the golden `market_oracle`.
R20. The digest's Oracle block for a study day lists that outcome day's won, lost and voided
    positions, the rank or `Unranked` with the settled count over 10, and the open count; the weekly
    line reports the wins and losses settled in the 7 days before, the net of the three sources
    since then, the rank and the week's Brier. Each is empty when nothing settled and nothing is
    open, and each equals its golden, `market_digest_block` and `market_weekly_line`.
R21. The Oracle's calibration series holds, for each confidence with a settled position, the
    observed win rate and the count; the rolling Brier of each settled position over its 30 most
    recent, those settled within 90 days of the last kept; the bands' floors; and the empty state.
    It equals the golden `market_calibration`. `oracle_calibration` joins SPEC-085's closed set of
    charts, and coordination's read model serves the settled positions by settled day and then id.

The surfaces

R22. The bot keeps `/predict` (the board, `pm:mkt:<key>`, then `pm:conf:<key>:<confidence>:<day>`,
    then `pm:go:<key>:<confidence>:<stake>:<day>`) and `/oracle` (`pm:cx:<id>`, `pm:chart` and
    `pm:new`), `<day>` an epoch day number. Each button's data is checked before any use case, and
    malformed data reaches none. A refusal's reason is relayed unchanged in the callback's answer.
    `pm:chart` answers with a button that opens the Mini App's `/markets` screen and sends no image
    (SPEC-085 R10).
R23. The API serves, behind the owner's session: `GET /api/markets/board`, `POST
    /api/markets/positions`, `DELETE /api/markets/positions/{id}` and `GET /api/markets/oracle`; the
    calibration series is `GET /api/charts/oracle_calibration` (SPEC-085 R6).
R24. The Mini App's `/markets` screen, listed in the route table, shows the board's cards (label,
    price and payout multiplier), one trade ticket (the confidence taps and the offered stakes), the
    open positions with a cancel while cancellable, the Oracle card (the rank, or unranked with the
    settled count over 10, the Brier, the record and the net) and the calibration chart through
    SPEC-085's chart component with its table of values. The server prices every trade again.

Tables, rights and constants

R25. The migration `migrations/010701_markets_positions.sql` creates `market_positions`, `STRICT`
    with `created_at` (SPEC-020 R15, R18): an `AUTOINCREMENT` id, the key, the threshold (present
    exactly for `reviews:goal`), the outcome day, the price (15..90), the confidence (60, 70, 80 or
    90), the stake (above 0), the payout (above the stake), the status (`open`, `won`, `lost`,
    `voided`), the settled day, the revision day, the trade's study day, the pending `market:<id>`
    raise of R16, and the pending `oracle_rank:<name>` raise of R16 (the rank's name, carried on the
    last position the step settled), with a unique index on the key and the outcome day.
R26. Markets' data-rights port exports and erases `market_positions`, which owes SPEC-021's six
    files.
R27. The constants equal the golden `markets.constants`. `economy.json` gains no markets section:
    the game-economy pack's reference stays as it is.
R28. The reads this SPEC adds to other contexts, each that context's own and a read only:
    progression's study days with law-track XP over both XP tables in a range, and the first of
    them; streaks' freeze markers of a range of study days; discipline's judged occurrences of a
    window in a range, and each booked window's mask and start; and economy's net of a set of
    sources on or after a study day. The goal, the governor's stored verdict, the skip set, the
    engine switch, the wallet and a range of rollups are read through the reads SPEC-090, SPEC-076,
    SPEC-083, SPEC-105, SPEC-082 and SPEC-071 already give, and the most recent stored rollups
    before a study day, newest first, through analytics' `RollupStore::recent_before(day, count)`,
    which SPEC-100 adds.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the probability and the price equal the golden `market_price`, at hundredfold prices of 14, 15, 42.5, 62.5, 90 and 91.5, and weekday samples of 5 and 6 | `the_prices_match_the_parity_golden` |
| A2 | the floors equal the golden `market_degenerate`, at raw rates of exactly 0.95 and 0.05 and just past each, an empty window, 13 and 14 rows, and 7 and 8 occurrences | `the_floors_match_the_parity_golden` |
| A3 | the payout equals the golden `market_payout`, at the floor of the stake plus one and at 148 and 158 before the cap of 150 | `the_payout_matches_the_parity_golden` |
| A4 | the Brier, the rank's index and the rank equal the golden `market_brier`, at each band's floor exactly and just below it, at 9 and 10 settled, with nothing settled, and over 10 settled whose plain running sum differs from Python's in the last bit | `the_brier_and_rank_match_the_parity_golden` |
| A5 | each market's quote equals the golden `market_quote`, the current study day on each weekday, the goal pending, equal to 60 and not a preset, and 19 and 20 answered on a row | `the_quotes_match_the_parity_golden` |
| A6 | the board equals the golden `market_board`: each hidden reason and its precedence, and the payout multiplier of every price from 15 to 90 | `the_board_matches_the_parity_golden` |
| A7 | the trade's refusals, their order and an accepted trade's record equal the golden `market_place`, at wallets of 39 and 40 with a stake of 10, with 2 and 3 open, and the outcome day equal to the current study day and one after | `the_trade_matches_the_parity_golden` |
| A8 | the cancel's verdicts equal the golden `market_cancel`, the outcome day equal to the current study day and one after | `the_cancel_matches_the_parity_golden` |
| A9 | the settlement's outcomes equal the golden `market_settle`: each mercy void, each market's truth, a price of 25 and 26 won, a freeze consumed on the day after the outcome day, and a rank from 9 to 10 settled | `the_settlement_matches_the_parity_golden` |
| A10 | the Oracle's summary equals the golden `market_oracle`, over 31 settled positions two of which share a settled day | `the_oracle_matches_the_parity_golden` |
| A11 | the digest's Oracle block equals the golden `market_digest_block` | `the_digest_block_matches_the_parity_golden` |
| A12 | the weekly line equals the golden `market_weekly_line`, a position settled 7 days back and one 8 days back | `the_weekly_line_matches_the_parity_golden` |
| A13 | the calibration series equals the golden `market_calibration`, with nothing settled, one confidence with no sample, and positions settled 90 and 91 days before the last | `the_calibration_matches_the_parity_golden` |
| A14 | the stake and confidence offers equal the golden `market_offers` at wallets of 19, 20, 39, 40, 99 and 100 | `the_stake_offers_match_the_parity_golden` |
| A15 | the constants equal the golden `markets.constants` | `the_market_constants_match_the_predecessors` |
| A16 | a window market is quoted only when its window occurs on the outcome day | `a_window_market_needs_an_occurrence_on_its_day` |
| A17 | a goal position settles on the threshold stored at its trade after the goal has changed | `a_goal_position_settles_on_its_stored_threshold` |
| A18 | a lost position is judged again only while its outcome day is one of the 7 closed study days back (the 7th judged, the 8th not), becomes won only when its truth holds, and a won or voided one is never judged again | `a_lost_position_is_judged_again_for_seven_closed_days` |
| A19 | a window market whose outcome day holds no judged occurrence is voided, never lost | `an_unjudged_window_day_voids_its_position` |
| A20 | a second position on one market and outcome day is refused and writes nothing | `a_second_position_on_a_market_and_day_is_refused` |
| A21 | `market_positions` is exported and erased by markets' port, and a cancelled position's id is not reused after it | `the_positions_are_exported_and_erased` |
| A22 | the board gathers every input: the 91 most recent rollups before the current study day, the goal, the booked windows with their masks and starts, each window's judged occurrences of 56 study days, the law track's XP days, the governor, the engine switch, the wallet and the open positions | `the_board_gathers_every_input` |
| A23 | an accepted trade writes its row and its `purchase` in one transaction, is priced from the inputs as they stand at the trade and not from the caller, and a refused one writes neither | `a_trade_writes_its_row_and_stake_together` |
| A24 | a cancel deletes the row and refunds the stake in one transaction, and a second cancel refunds nothing | `a_cancel_refunds_once` |
| A25 | two trades raced from separate connections with 2 open leave 3 open at most, and each stake is debited once | `raced_trades_never_open_a_fourth_position` |
| A26 | the settlement gathers every input: the engine switch, the governor's stored verdict, the skip set, each outcome day's rollup, each window's judged occurrence on its outcome day, the law track's XP days, the study days holding a review from the day after each trade, the freeze markers of the days after those, and each position's stored threshold; and a won position is paid once with its status over two cycles | `the_settlement_gathers_every_input_and_pays_once` |
| A27 | while the engine is off every open position is voided and refunded once | `an_engine_off_voids_every_open_position` |
| A28 | `void_open_positions` voids and refunds each open position once in its caller's transaction, and answers the count | `the_void_port_refunds_each_open_position_once` |
| A29 | a lost position whose truth now holds on a later cycle within 7 closed study days is won and paid once, and one outside them is not | `a_lost_position_is_paid_when_its_evidence_now_holds` |
| A30 | a long shot and a rank-up each raise their celebration once through the router, and neither moves a coin | `a_long_shot_and_a_rank_up_celebrate_once` |
| A31 | the calibration read model serves the settled positions by settled day and then id, under `oracle_calibration` | `the_calibration_read_model_serves_the_settled_positions` |
| A32 | the law track's XP days of a range and the first of them are read from both XP tables, a day of 0 XP excluded | `the_law_track_days_are_read_from_both_xp_tables` |
| A33 | the freeze markers of a range of study days are read back as the streak wrote them | `the_freeze_markers_of_a_range_are_read_back` |
| A34 | a window's judged occurrences of a range are read back with their verdicts, and each booked window with its mask and start | `the_judged_occurrences_of_a_range_are_read_back` |
| A35 | the net of a set of sources on or after a study day sums only those sources' movements from that day | `the_net_of_a_set_of_sources_since_a_day` |
| A36 | the commands and buttons call their use cases through a stub port, carry the day as an epoch day, relay a refusal's reason unchanged, and malformed data reaches no use case | `market_commands_run_the_use_cases` |
| A37 | the calibration button opens the markets screen and sends no image | `the_calibration_button_opens_the_markets_screen` |
| A38 | the markets' routes and the calibration chart answer the owner's session only | `the_market_routes_answer_only_the_owner` |
| A39 | the screen trades from the board through one ticket, cancels while cancellable, and shows the Oracle card and the calibration chart with its table | `trades from the board and shows the oracle` |
| A40 | a scheduled sync (no router) records the markets step's raises pending and sends nothing, the next `discipline_tick` delivers each exactly once, and a second tick sends nothing, and an owner's sync delivers them at once and, outside quiet hours (a quiet-hours withhold keeps them pending, SPEC-106 R17), leaves nothing pending | `a_scheduled_sync_leaves_the_markets_raises_pending_for_the_tick` |

```acceptance
A1: cargo test -p deck-streak-markets --test market_goldens -- --exact the_prices_match_the_parity_golden
A2: cargo test -p deck-streak-markets --test market_goldens -- --exact the_floors_match_the_parity_golden
A3: cargo test -p deck-streak-markets --test market_goldens -- --exact the_payout_matches_the_parity_golden
A4: cargo test -p deck-streak-markets --test market_goldens -- --exact the_brier_and_rank_match_the_parity_golden
A5: cargo test -p deck-streak-markets --test market_goldens -- --exact the_quotes_match_the_parity_golden
A6: cargo test -p deck-streak-markets --test market_goldens -- --exact the_board_matches_the_parity_golden
A7: cargo test -p deck-streak-markets --test market_goldens -- --exact the_trade_matches_the_parity_golden
A8: cargo test -p deck-streak-markets --test market_goldens -- --exact the_cancel_matches_the_parity_golden
A9: cargo test -p deck-streak-markets --test market_goldens -- --exact the_settlement_matches_the_parity_golden
A10: cargo test -p deck-streak-markets --test market_goldens -- --exact the_oracle_matches_the_parity_golden
A11: cargo test -p deck-streak-markets --test market_goldens -- --exact the_digest_block_matches_the_parity_golden
A12: cargo test -p deck-streak-markets --test market_goldens -- --exact the_weekly_line_matches_the_parity_golden
A13: cargo test -p deck-streak-markets --test market_goldens -- --exact the_calibration_matches_the_parity_golden
A14: cargo test -p deck-streak-markets --test market_goldens -- --exact the_stake_offers_match_the_parity_golden
A15: cargo test -p deck-streak-markets --test market_goldens -- --exact the_market_constants_match_the_predecessors
A16: cargo test -p deck-streak-markets --test market_rules -- --exact a_window_market_needs_an_occurrence_on_its_day
A17: cargo test -p deck-streak-markets --test market_rules -- --exact a_goal_position_settles_on_its_stored_threshold
A18: cargo test -p deck-streak-markets --test market_rules -- --exact a_lost_position_is_judged_again_for_seven_closed_days
A19: cargo test -p deck-streak-markets --test market_rules -- --exact an_unjudged_window_day_voids_its_position
A20: cargo test -p deck-streak-markets --test positions_store -- --exact a_second_position_on_a_market_and_day_is_refused
A21: cargo test -p deck-streak-markets --test positions_store -- --exact the_positions_are_exported_and_erased
A22: cargo test -p deck-streak-coordination --test markets_trade -- --exact the_board_gathers_every_input
A23: cargo test -p deck-streak-coordination --test markets_trade -- --exact a_trade_writes_its_row_and_stake_together
A24: cargo test -p deck-streak-coordination --test markets_trade -- --exact a_cancel_refunds_once
A25: cargo test -p deck-streak-coordination --test markets_trade -- --exact raced_trades_never_open_a_fourth_position
A26: cargo test -p deck-streak-coordination --test markets_settle -- --exact the_settlement_gathers_every_input_and_pays_once
A27: cargo test -p deck-streak-coordination --test markets_settle -- --exact an_engine_off_voids_every_open_position
A28: cargo test -p deck-streak-coordination --test markets_settle -- --exact the_void_port_refunds_each_open_position_once
A29: cargo test -p deck-streak-coordination --test markets_settle -- --exact a_lost_position_is_paid_when_its_evidence_now_holds
A30: cargo test -p deck-streak-coordination --test markets_settle -- --exact a_long_shot_and_a_rank_up_celebrate_once
A31: cargo test -p deck-streak-coordination --test markets_calibration -- --exact the_calibration_read_model_serves_the_settled_positions
A32: cargo test -p deck-streak-progression --test track_days -- --exact the_law_track_days_are_read_from_both_xp_tables
A33: cargo test -p deck-streak-streaks --test freeze_markers_read -- --exact the_freeze_markers_of_a_range_are_read_back
A34: cargo test -p deck-streak-discipline --test window_occurrences_read -- --exact the_judged_occurrences_of_a_range_are_read_back
A35: cargo test -p deck-streak-economy --test wallet_sources -- --exact the_net_of_a_set_of_sources_since_a_day
A36: cargo test -p deck-streak-bot --test markets_commands -- --exact market_commands_run_the_use_cases
A37: cargo test -p deck-streak-bot --test markets_commands -- --exact the_calibration_button_opens_the_markets_screen
A38: cargo test -p deck-streak-api --test markets_routes -- --exact the_market_routes_answer_only_the_owner
A39: pnpm exec vitest run web/app/src/lib/markets/markets.test.ts -t "trades from the board and shows the oracle"
A40: cargo test -p deck-streak-coordination --test markets_settle -- --exact a_scheduled_sync_leaves_the_markets_raises_pending_for_the_tick
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, game-economy, telegram-platform,
accessibility, ux-laws and notifications-policy packs stay enforced, and no row is deferred or
lifted for this delivery, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | `market_positions` is declared under the category `markets-positions` with data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010701_markets_positions.sql` and `crates/markets/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the economy's declaration is unchanged and still names every asset but coins as never confiscated, over `economy.json` | the game-economy pack |
| B3 | every `pm:` button's data fits the platform's limit and every callback is answered, over `crates/bot/src/markets_commands.rs` | the telegram-platform pack |
| B4 | the markets screen passes the accessibility audit in both colour schemes, its chart carrying its text alternative, over the `/markets` route that `web/app/src/lib/routes.ts` lists | the accessibility pack |
| B5 | the markets' copy carries no countdown pressure and no shame, and says a stake is coins, over every file under `web/app/src/lib/markets/` and `crates/bot/src/markets_commands.rs` | the ux-laws pack |
| B6 | every markets message goes through the one router, over every file under `crates/coordination/src/markets/` | the notifications-policy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/markets/src/lib.rs` | `deck-streak-markets` | changed: the modules below |
| `crates/markets/src/constants.rs` | `deck-streak-markets` | added: the market constants, proved by the golden |
| `crates/markets/src/forecast.rs` | `deck-streak-markets` | added: the probability, the price, the floors, the payout, the Brier and the rank, pure, with the port of Python's compensated sum |
| `crates/markets/src/quote.rs` | `deck-streak-markets` | added: the candidates, the quotes and the board, pure |
| `crates/markets/src/trade.rs` | `deck-streak-markets` | added: the trade's and the cancel's verdicts and the offers, pure |
| `crates/markets/src/settle.rs` | `deck-streak-markets` | added: the mercy, the truth, the settlement, the revision and the rank's rise, pure |
| `crates/markets/src/oracle.rs` | `deck-streak-markets` | added: the summary, the digest block, the weekly line and the calibration series, pure |
| `crates/markets/src/store.rs` | `deck-streak-markets` | added: the repository over `market_positions`, through the kernel's base, and the pending raises of R16 |
| `crates/markets/src/data_rights.rs` | `deck-streak-markets` | added: the markets data-rights port |
| `crates/markets/Cargo.toml` | `deck-streak-markets` | changed: the workspace dependencies it uses (`sqlx`, `thiserror`, `tokio`); `serde` and `serde_json` with `float_roundtrip` as dev-dependencies for the golden reader |
| `crates/markets/tests/market_goldens.rs` | `deck-streak-markets` | added: A1 to A15 |
| `crates/markets/tests/market_rules.rs` | `deck-streak-markets` | added: A16 to A19 |
| `crates/markets/tests/positions_store.rs` | `deck-streak-markets` | added: A20, A21 |
| `crates/progression/src/ledger.rs` | `deck-streak-progression` | changed: the law track's XP days of a range and the first, over both XP tables |
| `crates/progression/tests/track_days.rs` | `deck-streak-progression` | added: A32 |
| `crates/streaks/src/store.rs` | `deck-streak-streaks` | changed: the freeze markers of a range of study days, read for the markets and for SPEC-106's stakes |
| `crates/streaks/tests/freeze_markers_read.rs` | `deck-streak-streaks` | added: A33 |
| `crates/discipline/src/windows.rs` | `deck-streak-discipline` | changed: a window's judged occurrences of a range, and each booked window's mask and start |
| `crates/discipline/tests/window_occurrences_read.rs` | `deck-streak-discipline` | added: A34 |
| `crates/economy/src/wallet.rs` | `deck-streak-economy` | changed: the net of a set of sources on or after a study day |
| `crates/economy/tests/wallet_sources.rs` | `deck-streak-economy` | added: A35 |
| `crates/coordination/src/markets/mod.rs` | `deck-streak-coordination` | added: the inputs, the board, the trade and the cancel, and the void port |
| `crates/coordination/src/markets/settle.rs` | `deck-streak-coordination` | added: the markets step, its inputs, its coins and its celebrations |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the markets module |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the markets step after discipline's, raising its pending raises through the cycle's router when it carries one (R16) |
| `crates/coordination/src/discipline/tick.rs` | `deck-streak-coordination` | changed: the markets' pending raises (`market:<id>`, `oracle_rank:<name>`) raised at the tick through SPEC-084's ladder (R16, A40) |
| `crates/coordination/src/charts/mod.rs` | `deck-streak-coordination` | changed: `oracle_calibration` joins the closed set |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `market_positions` registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row of `market_positions` |
| `crates/coordination/tests/markets_trade.rs` | `deck-streak-coordination` | added: A22 to A25 |
| `crates/coordination/tests/markets_settle.rs` | `deck-streak-coordination` | added: A26 to A30 and A40 |
| `crates/coordination/tests/markets_calibration.rs` | `deck-streak-coordination` | added: A31 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the markets' use cases joined to the bot and the API |
| `crates/bot/src/markets_commands.rs` | `deck-streak-bot` | added: /predict, /oracle and the `pm:` buttons |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the two commands join the command table |
| `crates/bot/tests/markets_commands.rs` | `deck-streak-bot` | added: A36, A37 |
| `crates/api/src/markets_routes.rs` | `deck-streak-api` | added: the markets' routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes, behind the owner's session |
| `crates/api/tests/markets_routes.rs` | `deck-streak-api` | added: A38 |
| `web/app/src/lib/markets/markets.ts` | miniapp | added: the markets' client |
| `web/app/src/lib/markets/BoardCard.svelte` | miniapp | added: a market's card |
| `web/app/src/lib/markets/TradeTicket.svelte` | miniapp | added: the confidence taps and the offered stakes |
| `web/app/src/lib/markets/OracleCard.svelte` | miniapp | added: the rank, the record and the calibration chart |
| `web/app/src/lib/markets/markets.test.ts` | miniapp | added: A39 |
| `web/app/src/routes/markets/+page.svelte` | miniapp | added: the markets screen |
| `web/app/src/lib/routes.ts` | miniapp | changed: /markets joins the route table |
| `migrations/010701_markets_positions.sql` | repo | added: `market_positions` (SPEC-020 R15, R18) |
| `tools/parity-oracle/registry/spec_107.py` | repo | added: this SPEC's registrations |
| `tools/parity-oracle/goldens/market_price.json` | repo | added: the golden of `laplace_rate`, `blended_probability`, `price_for` and `survival_probability` (pure) |
| `tools/parity-oracle/goldens/market_degenerate.json` | repo | added: the golden of `is_degenerate_rate`, `is_degenerate`, `history_priceable` and `window_priceable` (pure) |
| `tools/parity-oracle/goldens/market_payout.json` | repo | added: the golden of `payout_for` (pure) |
| `tools/parity-oracle/goldens/market_brier.json` | repo | added: the golden of `brier_score`, `rank_index` and `rank_for` (pure) |
| `tools/parity-oracle/goldens/market_quote.json` | repo | added: the golden of `MarketsLayer._market_quote` (adapter) |
| `tools/parity-oracle/goldens/market_board.json` | repo | added: the golden of `MarketsLayer.market_board` (adapter) |
| `tools/parity-oracle/goldens/market_place.json` | repo | added: the golden of `MarketsLayer.place_position` (adapter) |
| `tools/parity-oracle/goldens/market_cancel.json` | repo | added: the golden of `MarketsLayer.cancel_position` (adapter) |
| `tools/parity-oracle/goldens/market_settle.json` | repo | added: the golden of `MarketsLayer._settle_markets` (adapter) |
| `tools/parity-oracle/goldens/market_oracle.json` | repo | added: the golden of `MarketsLayer.oracle_summary` (adapter) |
| `tools/parity-oracle/goldens/market_digest_block.json` | repo | added: the golden of `MarketsLayer.oracle_digest_block` (adapter) |
| `tools/parity-oracle/goldens/market_weekly_line.json` | repo | added: the golden of `MarketsLayer.oracle_weekly_line` (adapter) |
| `tools/parity-oracle/goldens/market_calibration.json` | repo | added: the golden of `charts.py:calibration_chart` (adapter) |
| `tools/parity-oracle/goldens/market_offers.json` | repo | added: the golden of `CommandBot._callback_market`'s offers (adapter) |
| `tools/parity-oracle/goldens/markets.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S10700-S10799.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `market_positions`'s migration |
| `privacy.json` | repo | changed: the category `markets-positions` |
| `PRIVACY.md` | repo | changed: one line for the category |
| `docs/schematics/market-position-lifecycle.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-107-markets-price-the-owners-next-day-settle-on-evidence-and-rank-the-oracle.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-107.md` | docs | added |
| `changelog.d/feat-markets-107.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It places no Oracle block in the daily digest and no Oracle line in the weekly report: it renders
  both, and the digest (#129) and the weekly report (#130) carry them.
- It sends no settlement message: the digest's Oracle block reports each position (#129).
- It flushes no held celebration when quiet hours end: the router's flush does (#291).
- It stakes nothing but coins, no rank pays a coin, and nothing moves real money (#116).
- It changes no sync cadence: a position settles at whatever cadence the owner keeps (#164).
- It writes no engine switch and applies no panic: the panic calls its void port (#114).
- It imports none of the predecessor's positions (#61).

## 6. Risks

- **A trade at a stale price.** The quote is made again inside the trade's transaction, from the
  inputs as they stand, and the caller's price is never read (A23).
- **A loss rests on reviews DeckStreak had not read.** It is judged again within 7 closed study days
  and paid when the evidence now holds (A18, A29).
- **Coins paid, taken or refunded twice.** Every movement is written in one transaction with the
  status change that allows it once, and a raced trade still debits once (A23 to A28).
- **A fourth open position.** The verdict and the row share one immediate transaction, so a second
  connection waits for the first (A25).
- **A market that can only lose.** A window market is offered only when the window occurs, and an
  unjudged window day voids (A16, A19).
- **The rank drifts from the predecessor's by a bit.** The Brier sums as Python's compensated sum
  does, and the golden holds a run where a plain sum differs (A4).
- **Market pressure on a hard day.** The board hides in a lapse, in standby and while the engine is
  off, and a mercy void refunds a skip day's position (A6, A9).

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_107.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number; no golden
holds a calendar date or a personal value.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/market_price.json` | `gamification/forecast.py:laplace_rate`, `blended_probability`, `price_for`, `survival_probability` | pure | hits and windows from 0; weekday samples of 5 and 6; probabilities whose hundredfold is 14, 15, 42.5, 62.5, 90 and 91.5; products of 0, 1 and 7 rates |
| `goldens/market_degenerate.json` | `gamification/forecast.py:is_degenerate_rate`, `is_degenerate`, `history_priceable`, `window_priceable` | pure | raw rates of 0.95, 0.05 and just past each; hits over an empty window; 13 and 14 rows; 7 and 8 occurrences |
| `goldens/market_payout.json` | `gamification/forecast.py:payout_for` | pure | each stake at each price from 15 to 90, which holds the floor of the stake plus one, 148 and the cap at 158 |
| `goldens/market_brier.json` | `gamification/forecast.py:brier_score`, `rank_index`, `rank_for` | pure | no settled forecast; runs of 1, 10 and 30, one of 10 whose plain running sum differs from Python's; each band's floor exactly and just below; 9 and 10 settled |
| `goldens/market_quote.json` | `pipeline_layers/markets.py:MarketsLayer._market_quote` | adapter | a stub store of synthetic rollups, window events and law XP rows, and a stub goal; each key and an unknown one, the current study day on each weekday, 13 and 14 rows, 19 and 20 answered, 7 and 8 events, a law track 13 and 14 days old and 30 and 31, and the goal pending, equal to 60 and at 45; every window occurs daily |
| `goldens/market_board.json` | `MarketsLayer.market_board` | adapter | a stub governor, the engine switch and a stub wallet; disabled with a lapse, lapse, standby, armed, and a board of markets at every price from 15 to 90 |
| `goldens/market_place.json` | `MarketsLayer.place_position` | adapter | a temporary store and a stub governor; each refusal in turn, wallets of 39 and 40 with a stake of 10, 2 and 3 open, a held market, confidences of 55 and 60, the outcome day equal to the current study day and one after, and an accepted trade |
| `goldens/market_cancel.json` | `MarketsLayer.cancel_position` | adapter | a temporary store; no row, a settled row, the outcome day equal to the current study day and one after |
| `goldens/market_settle.json` | `MarketsLayer._settle_markets`, `_market_truth`, `_day_survived` | adapter | a temporary store, a stub governor, a skip set, study days and freeze markers; each mercy void, 19 and 20 answered, each market's truth, a price of 25 and 26 won, a freeze consumed on the day after the outcome day, and 9 settled before a tenth; the goal is unchanged between trade and settlement |
| `goldens/market_oracle.json` | `MarketsLayer.oracle_summary`, `_brier_rank` | adapter | a temporary store with a stub ledger; 31 settled positions two of which share a settled day, voided and open ones |
| `goldens/market_digest_block.json` | `MarketsLayer.oracle_digest_block` | adapter | a temporary store; nothing, a won, a lost and a voided position on the day, ranked and unranked, with and without open ones |
| `goldens/market_weekly_line.json` | `MarketsLayer.oracle_weekly_line` | adapter | a temporary store with a stub ledger; nothing, positions settled 7 and 8 days back, and open ones only |
| `goldens/market_calibration.json` | `charts.py:calibration_chart` | adapter | a recording figure in place of the image; nothing settled, one confidence with no sample, and positions settled 90 and 91 days before the last |
| `goldens/market_offers.json` | `bot.py:CommandBot._callback_market` | adapter | a stub pipeline answering the wallet and a recording sender; wallets of 19, 20, 39, 40, 99 and 100 |
| `goldens/markets.constants.json` | `constants.py` | constants | every `MARKET_` constant |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `market_positions` | `markets` | `migrations/010701_markets_positions.sql` (SPEC-107) | `market_positions`, its days as epoch days, its forecast as the confidence, a `reviews:goal` row's threshold the goal read at the import, and no revision day, and no pending raise | exported and erased |

## 9. Mutation rows

The band is `S10700-S10799`, in `scripts/mutation-rows.d/S10700-S10799.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039). No
row's mutant makes a loop or a wait unbounded.

| row | target | what it guards | killer |
|---|---|---|---|
| `S10701-THE-PRICE-FLOOR-IS-15` | `crates/markets/src/forecast.rs` | the lower clamp | `market_goldens::the_prices_match_the_parity_golden` |
| `S10702-THE-PRICE-CAP-IS-90` | `crates/markets/src/forecast.rs` | the upper clamp | `market_goldens::the_prices_match_the_parity_golden` |
| `S10703-THE-PRICE-ROUNDS-HALF-TO-EVEN` | `crates/markets/src/forecast.rs` | Python's rounding at a half | `market_goldens::the_prices_match_the_parity_golden` |
| `S10704-THE-WEEKDAY-JOINS-AT-SIX` | `crates/markets/src/forecast.rs` | the weekday sample's inclusive floor | `market_goldens::the_prices_match_the_parity_golden` |
| `S10705-A-DEGENERATE-RATE-IS-STRICT` | `crates/markets/src/forecast.rs` | 0.95 exactly is offered | `market_goldens::the_floors_match_the_parity_golden` |
| `S10706-FOURTEEN-ROWS-PRICE-A-MARKET` | `crates/markets/src/forecast.rs` | the rollup markets' cold start | `market_goldens::the_floors_match_the_parity_golden` |
| `S10707-EIGHT-OCCURRENCES-PRICE-A-WINDOW` | `crates/markets/src/forecast.rs` | the window markets' cold start | `market_goldens::the_floors_match_the_parity_golden` |
| `S10708-THE-PAYOUT-BEATS-THE-STAKE` | `crates/markets/src/forecast.rs` | the floor of the stake plus one | `market_goldens::the_payout_matches_the_parity_golden` |
| `S10709-THE-PAYOUT-CAP-IS-150` | `crates/markets/src/forecast.rs` | the payout's cap | `market_goldens::the_payout_matches_the_parity_golden` |
| `S10710-TEN-SETTLED-TO-RANK` | `crates/markets/src/forecast.rs` | the rank's floor | `market_goldens::the_brier_and_rank_match_the_parity_golden` |
| `S10711-A-BAND-FLOOR-IS-INCLUSIVE` | `crates/markets/src/forecast.rs` | a Brier on a floor takes that band | `market_goldens::the_brier_and_rank_match_the_parity_golden` |
| `S10712-THE-BRIER-SUMS-AS-PYTHON-DOES` | `crates/markets/src/forecast.rs` | the compensated sum | `market_goldens::the_brier_and_rank_match_the_parity_golden` |
| `S10713-A-QUARTER-OF-THE-WALLET` | `crates/markets/src/trade.rs` | the stake's inclusive cap | `market_goldens::the_trade_matches_the_parity_golden` |
| `S10714-THREE-OPEN-AT-MOST` | `crates/markets/src/trade.rs` | the open positions' cap | `market_goldens::the_trade_matches_the_parity_golden` |
| `S10715-ONE-POSITION-PER-MARKET-AND-DAY` | `migrations/010701_markets_positions.sql` | the unique index, a key held in the migration (a script-mutation row with a cargo killer) | `positions_store::a_second_position_on_a_market_and_day_is_refused` |
| `S10716-TWENTY-ANSWERS-JUDGE-RETENTION` | `crates/markets/src/settle.rs` | the retention market's mercy floor | `market_goldens::the_settlement_matches_the_parity_golden` |
| `S10717-A-LONG-SHOT-IS-25-OR-LESS` | `crates/markets/src/settle.rs` | the long shot's inclusive price | `market_goldens::the_settlement_matches_the_parity_golden` |
| `S10718-A-LOSS-IS-JUDGED-SEVEN-DAYS-BACK` | `crates/markets/src/settle.rs` | the revision's window | `market_rules::a_lost_position_is_judged_again_for_seven_closed_days` |
| `S10719-A-WIN-IS-PAID-WITH-ITS-STATUS` | `crates/coordination/src/markets/settle.rs` | a won position's credit | `markets_settle::the_settlement_gathers_every_input_and_pays_once` |
