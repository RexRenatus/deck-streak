# SPEC-082: coins are minted, spent and capped in one wallet, and the shop sells only for coins

- **Wave:** W3. **Issue:** #106, #107 (epic #4). **Context(s):** `deck-streak-economy` (the coin
  ledger, the mint, the daily loss cap, the scaled fine, the debit clip, the wallet's ports, the
  shop's items and the scroll pass's state); `deck-streak-coordination` (the mint step of the fold,
  the shop's use cases, the freeze purchase through the freeze port); `deck-streak-api` (the wallet
  and shop routes); `deck-streak-bot` (`/shop`); the Mini App (`web/app`, the wallet header and the
  shop screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-071 (the mint of each study day
  is settled after its derived XP, and a settled day's value never falls) and ADR-072 (the day's
  base XP is read over both XP tables).
- **Prerequisites:** SPEC-071 (the fold and the current day's reviews), SPEC-072 (the day's base XP),
  SPEC-076 (the freeze port), SPEC-024 (the owner's session) and SPEC-026 (the bot's command table).
  **Mutation band:** `S08200-S08299`.
- **Status:** delivered by E1 in part (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-082.md`, ADR-016): the wallet of #106 in three pull requests. E1b delivers
  R4, A7, the wallet route and header and the daemon wiring; E3 (#107) delivers the shop
  (section 3c).

## 1. The problem, measured

- **What exists.** At `dev` c3d769b, `crates/economy/src/` holds only `lib.rs`. `economy.json`
  declares the coin and shop values the game-economy pack judges, and nothing reads them.
- **The rule that binds it.** Coins are the only confiscable stake (CHARTER 5): every fine, stake
  and tariff of every later wave moves coins through this wallet, and a false fine is worse than a
  missed fine (CHARTER 6), so the floor and the daily loss cap are the safety this SPEC ships before
  any fine exists.
- **What is ported.** The predecessor's `gamification/economy.py` (`mint_for_base_xp`,
  `daily_loss_cap`, `scaled_fine`, `clip_debit`), `database.py:GamifyStore.coin_balance`,
  `coin_balance_before`, `coin_debits_for_day`, `insert_coin_delta_once` and `upsert_coin_grant`,
  the mint of `pipeline.py:GamifyPipeline._recompute_day`, and the shop of
  `pipeline_layers/economy.py:EconomyLayer.buy_item`, `shop_board` and `_active_pass_until`; the
  fine path of `EconomyLayer._debit_fine` is ported as a port whose first caller is the discipline
  wave.
- **Traps a hand port falls into.**
  - The loss cap's base is the wallet at the day's start, the movements of earlier study days only
    (`coin_balance_before`), and the cap's remainder subtracts every coin debited that day,
    purchases included (`coin_debits_for_day` sums every negative movement of the day).
  - The cap rounds its share down (`int(wallet × 0.3)`) and the fine rounds its share up
    (`ceil(wallet × 0.02)`), both over floating point, so the port computes each in `f64` in the
    predecessor's order.
  - The predecessor replaces a day's mint at every recompute (`upsert_coin_grant`), so the mint fell
    whenever the day's base XP fell after the rollover; ADR-071 settles each day once and never
    lowers a settled day, and the mint follows the same rule, or coins would be taken with no fine.
  - The scroll pass's surcharge is `int(price × 1.5)`, an integer of the product, not a rounding.
- **Corrections to the issues.**
  - #107's "activate a held Double-XP token": tokens are the quests context's (SPEC-081), which
    adds its card to this shop; this SPEC builds the freeze and the scroll pass.
  - #107's surcharge "for 48 h": the rung-2 fine sets the surcharge's end 48 hours out
    (`constants.PASS_SURCHARGE_HOURS`, #110); this SPEC stores that end and applies the price rule
    while it lies ahead.
  - #107's gate of "at least one review today" reads the current study day's reviews as of the latest
    recompute (ADR-071); an owner's sync counts reviews made since (ADR-037).
  - The predecessor keeps the active scroll pass as a token row; tokens belong to the quests context
    here, so the pass's end is economy's own state (R12).
- **What the parity oracle proves.** The mint, the cap, the fine and the clip; the wallet at the day's
  start and the day's debits over a synthetic ledger; every verdict of the shop; every constant
  (section 7).
- **Prerequisites.** SPEC-071 and SPEC-072 land first, because the mint reads the day's final base
  XP; SPEC-076 lands first, because a bought freeze is granted through its port.

## 2. Requirements

The wallet (#106)

R1. `coin_ledger` holds every coin movement: its study day, source, reference, a signed whole
    delta and `created_at`, unique on (study day, source, reference). Only the economy context
    writes it, and no other crate names it in a query.
R2. The balance is the sum of every movement. No port writes a movement that would leave the balance
    below `wallet_floor` (0 in `economy.json`): a purchase is refused below its price, a floor-clipped
    debit pays at most the balance, and a capped debit is clipped (R5). Every port runs in one
    `BEGIN IMMEDIATE` write through the kernel's base, so two concurrent debits cannot both pass the
    floor.
R3. The mint of a study day is `min(constants.COIN_MINT_DAILY_CAP, base // constants.COIN_MINT_XP_DIVISOR)`
    (40 and 25), and 0 for a base of 0 or less (golden of `gamification/economy.py:mint_for_base_xp`);
    the base is the day's base XP that progression serves (SPEC-072, the golden of
    `database.py:GamifyStore.day_base_xp`). A day has one mint movement, source `mint`, an empty
    reference.
R4. The mint step registers in phase 6 of SPEC-071's fold (R19 there), after phase 5's derived
    bonuses, so each study day the fold settles is minted from its final base (ADR-071). The current
    study day's mint follows its base at each recompute; a settled day's mint is raised by a later
    recompute and never lowered. The days the first recompute backfills (SPEC-071 R17) are minted
    from their bases too, as the predecessor minted every day of its window: the mint is not a
    today-only rule.
R5. The daily loss cap is 0 when the wallet at the day's start is 0 or less, and
    `min(constants.DAILY_LOSS_CAP_COINS, int(wallet at the day's start × constants.DAILY_LOSS_CAP_WALLET_FRAC))`
    (100 and 0.3) otherwise (golden of `daily_loss_cap`). The wallet at the day's start is the sum of
    the movements of earlier study days (golden of `coin_balance_before`), and the cap's remainder is
    the cap less every coin debited on the day, purchases included (golden of
    `coin_debits_for_day`). A capped debit is `min(requested, wallet, remainder)`, reports whether any
    part was forgiven, and debits nothing for a request of 0 or less (golden of `clip_debit`).
R6. A fine's amount is `max(configured, ceil(wallet × constants.FINE_WALLET_FRAC))` (0.02) for a
    positive configured fine and a positive wallet, the configured fine for an empty wallet, and 0 for
    a configured fine of 0 or less (golden of `scaled_fine`). The capped debit serves the discipline
    wave's fines; this wave raises none.
R7. The ports: `credit(day, source, reference, amount)`, written once (a second credit of one key
    writes nothing); `credit_once(day, source, reference, amount)`, which writes nothing when a
    movement of that source and reference exists on any study day, the predecessor's cross-day guard
    (`database.py:GamifyStore.coin_ref_exists`) for a payout settled on a later day, such as a season
    node's (SPEC-074); `settle_mint(day, amount, closed)` (R4); `purchase(day, source, reference, price)`,
    a verdict that refuses below the price and writes nothing when it refuses;
    `debit_floored(day, source, reference, amount)`, which pays `min(amount, max(0, wallet))` and never
    refuses (the skip day's tariff, SPEC-083); `refund(day, source, reference, amount)`, a positive
    movement; and `debit_capped(day, source, reference, requested)` (R5). No port deletes a movement;
    `settle_mint` alone updates one, the day's mint.
R8. A purchase is never clipped by the loss cap, and it counts toward the day's debits, which the cap's
    remainder for a later fine that day subtracts, as the predecessor's counter does.
R9. The coin and shop constants equal the constants golden, and `economy.json`'s `coins.mint`,
    `coins.loss_cap`, `coins.fines.wallet_fraction`, `coins.wallet_floor`, `shop.freeze.price` and
    `shop.scroll_pass.price` equal them (a test), so the declaration the game-economy pack judges
    cannot drift from the engine.

The shop (#107)

R10. A streak freeze costs `constants.SHOP_FREEZE_PRICE` (150) coins. The purchase is refused, with the
    wallet and the freezes unchanged, first when the language streak holds
    `constants.STREAK_FREEZE_CAP` (3) freezes and then when the wallet is below the price, in the
    predecessor's order (golden of `EconomyLayer.buy_item`). A bought freeze is granted through the
    freeze port with the reason `shop` (SPEC-076) in the same transaction as its coin movement
    (source `shop`, reference `freeze:<instant>`), which coordination holds: a grant the port refuses
    leaves no coin movement.
R11. A scroll pass costs `constants.SHOP_SCROLL_PASS_PRICE` (40) coins for
    `constants.SCROLL_PASS_MINUTES` (30) minutes. It is refused, in the predecessor's order, unless the
    current study day has at least one study review as of the latest recompute, while a pass is active,
    and when the wallet is below its price; while a surcharge's end lies ahead the price is
    `int(price × constants.PASS_SURCHARGE_MULT)` (1.5). Each verdict, price and movement equals the
    golden of `buy_item` (source `shop`, reference `pass:<instant>`).
R12. `economy_state` (one row) holds the active pass's end and the surcharge's end, as epoch
    milliseconds, or none. A pass is active while its end lies ahead of the kernel's clock.
R13. Every refusal names its reason (the freeze cap, too few coins with the price, study first, a pass
    already active) and changes nothing.
R14. The shop's board serves the wallet, the freeze price, the freezes held and their cap, the pass's
    price as it stands (surcharged when it applies) and its minutes, and the active pass's end.

Surfaces

R15. `GET /api/wallet` serves the balance, the day's loss cap and its remainder, and the ledger's
    movements, newest first and paged; `GET /api/shop` serves the board; `POST /api/shop/freeze` and
    `POST /api/shop/pass` buy, each answering its refusal or the new state. Every route answers the
    owner's session only.
R16. The bot's `/shop` shows the board with a button for each item (the predecessor's callback family
    `sh:`, `sh:freeze` and `sh:pass`), and a tap runs the same use case as the Mini App.
R17. The Mini App shows a wallet header on every screen, refreshed after a purchase, and a `/shop`
    screen whose item cards, when disabled, say why with the reasons of R13, with no pressure or
    shaming wording.

Data rights

R18. The migration `migrations/008201_economy_wallet_and_shop.sql` creates `coin_ledger` and
    `economy_state`, each `STRICT` with `created_at` (SPEC-020 R15, R18). `coin_ledger` is exported
    and erased; `economy_state` is one seeded row, reset in place to no pass and no surcharge. Each is
    registered in the context map's ownership register, declared in `privacy.json` and listed in the
    economy data-rights port.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the mint equals the golden of `mint_for_base_xp` for every case (examined count reported, zero refused) | `the_mint_matches_the_parity_golden` |
| A2 | the loss cap, the scaled fine and the debit clip equal their goldens | `the_cap_the_fine_and_the_clip_match_the_parity_goldens` |
| A3 | the wallet at the day's start and the day's debits equal the goldens of `coin_balance_before` and `coin_debits_for_day` over a synthetic ledger, purchases counted | `the_day_start_wallet_and_the_day_debits_match_the_parity_goldens` |
| A4 | a burst of concurrent capped debits and purchases never leaves the balance below zero | `the_wallet_never_goes_negative_under_a_burst` |
| A5 | a credit of one (study day, source, reference) written twice writes one movement | `a_credit_of_one_key_is_written_once` |
| A6 | a settled day's mint is raised by a later settle and never lowered, while the current day's follows its base | `a_settled_days_mint_is_raised_and_never_lowered` |
| A8 | the floor-clipped debit pays what the wallet holds and never refuses | `a_floor_clipped_debit_pays_what_is_held` |
| A9 | no crate but economy names `coin_ledger` in a query, and no migration but economy's names it; a planted fixture that does is refused (examined count reported) | `only_the_wallet_writes_the_coin_ledger` |
| A10 | the coin and shop constants equal the golden, and `economy.json`'s coin and shop values equal them | `the_coin_constants_and_economy_json_match_the_predecessors` |
| A14 | after an erase the coin ledger is empty and the economy row holds its reset values | `an_erase_empties_the_ledger_and_resets_the_state` |
| A19 | a once-ever credit of one source and reference, requested on two study days, writes one movement | `a_once_ever_credit_is_written_once_on_any_day` |

```acceptance
A1: cargo test -p deck-streak-economy --test wallet_goldens -- --exact the_mint_matches_the_parity_golden
A2: cargo test -p deck-streak-economy --test wallet_goldens -- --exact the_cap_the_fine_and_the_clip_match_the_parity_goldens
A3: cargo test -p deck-streak-economy --test wallet_goldens -- --exact the_day_start_wallet_and_the_day_debits_match_the_parity_goldens
A4: cargo test -p deck-streak-economy --test wallet_ports -- --exact the_wallet_never_goes_negative_under_a_burst
A5: cargo test -p deck-streak-economy --test wallet_ports -- --exact a_credit_of_one_key_is_written_once
A6: cargo test -p deck-streak-economy --test wallet_ports -- --exact a_settled_days_mint_is_raised_and_never_lowered
A8: cargo test -p deck-streak-economy --test wallet_ports -- --exact a_floor_clipped_debit_pays_what_is_held
A9: cargo test -p deck-streak-economy --test wallet_census -- --exact only_the_wallet_writes_the_coin_ledger
A10: cargo test -p deck-streak-economy --test wallet_goldens -- --exact the_coin_constants_and_economy_json_match_the_predecessors
A14: cargo test -p deck-streak-economy --test wallet_rights -- --exact an_erase_empties_the_ledger_and_resets_the_state
A19: cargo test -p deck-streak-economy --test wallet_ports -- --exact a_once_ever_credit_is_written_once_on_any_day
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, game-economy, ux-laws and
accessibility packs stay enforced, and no row is deferred for this delivery.

| id | criterion | decided by |
|---|---|---|
| B1 | both tables this delivery creates are declared under a category with their data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/008201_economy_wallet_and_shop.sql` and `crates/economy/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the mint, the loss cap, the fines, the zero floor and the shop's prices still equal the reference economy, only coins are confiscable, and no shop item is random or sold for money, over `economy.json`'s `coins`, `shop` and `unconfiscable` sections | the game-economy pack |
| B3 | the shop's copy carries no confirmshaming and no false urgency, over `web/app/src/routes/shop/+page.svelte`, every file under `web/app/src/lib/economy/` and `crates/bot/src/shop_commands.rs` | the ux-laws pack |
| B4 | the shop screen and the wallet header pass the accessibility audit in both of Telegram's colour schemes, over the `/shop` route that `web/app/src/lib/routes.ts` lists and the layout that renders the header | the accessibility pack |

## 3c. Delivered by the next pull requests

This SPEC lands in three pull requests, in order. This one (E1) delivers the wallet's pure core, its
ports, its tables and its data rights: the criteria of section 3's table that no row below names.
E1b delivers the mint step of the fold, the wallet route and header and the daemon wiring; E3
(#107) delivers the shop, its routes, its command and its screen. The table below holds the
criteria a later pull request delivers, each row naming that pull request, and the lines under it
are their fence lines, each prefixed with that pull request. A later pull request moves each of
its criteria back verbatim: the row into section 3's table, without the `delivered by` column,
and the fence line into the acceptance fence, without the prefix. A15 is split: E1b delivers its
wallet arm and E3 its shop arm.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A7 | the fold mints each settled day after its derived XP, so the mint equals the golden over the day's final base | `the_mint_reads_the_settled_days_final_base` | E1b |
| A11 | the shop's verdicts, prices and movements equal the golden of `buy_item`: the freeze refused at the hold cap with the wallet unchanged, the pass gated on a review, one pass at a time, and the surcharge | `the_shop_verdicts_match_the_parity_golden` | E3 |
| A12 | a bought freeze moves its coins and adds its freeze in one transaction, and a grant the freeze port refuses leaves no movement | `a_bought_freeze_moves_coins_and_the_freeze_together` | E3 |
| A13 | a refused purchase names its reason and leaves the wallet, the freezes and the pass unchanged | `a_refused_purchase_changes_nothing` | E3 |
| A15 | the wallet and shop routes answer the owner's session only, and any other caller gets 401 or 403 and no data | `the_wallet_and_shop_routes_answer_only_the_owner` | E1b |
| A16 | `/shop` shows the board, and its buttons run the same purchase as the Mini App | `shop_shows_the_board_and_its_buttons_buy` | E3 |
| A17 | each disabled item card on the shop screen says why | `disables each item with its reason` | E3 |
| A18 | the wallet header shows the balance in the layout every screen renders | `shows the balance in the layout header` | E1b |

E1b: A7: cargo test -p deck-streak-coordination --test wallet_mint -- --exact the_mint_reads_the_settled_days_final_base
E3: A11: cargo test -p deck-streak-economy --test shop_goldens -- --exact the_shop_verdicts_match_the_parity_golden
E3: A12: cargo test -p deck-streak-coordination --test shop_purchase -- --exact a_bought_freeze_moves_coins_and_the_freeze_together
E3: A13: cargo test -p deck-streak-coordination --test shop_purchase -- --exact a_refused_purchase_changes_nothing
E1b: A15: cargo test -p deck-streak-api --test wallet_routes -- --exact the_wallet_and_shop_routes_answer_only_the_owner
E3: A16: cargo test -p deck-streak-bot --test shop_commands -- --exact shop_shows_the_board_and_its_buttons_buy
E3: A17: pnpm exec vitest run web/app/src/lib/economy/shop.test.ts -t "disables each item with its reason"
E1b: A18: pnpm exec vitest run web/app/src/lib/economy/wallet-header.test.ts -t "shows the balance in the layout header"

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/economy/src/lib.rs` | `deck-streak-economy` | changed: the modules below |
| `crates/economy/src/constants.rs` | `deck-streak-economy` | added: the coin and shop constants, proved by the golden |
| `crates/economy/src/rules.rs` | `deck-streak-economy` | added: the mint, the cap, the scaled fine and the clip, pure |
| `crates/economy/src/wallet.rs` | `deck-streak-economy` | added: the ledger's repository and the wallet's ports |
| `crates/economy/src/shop.rs` | `deck-streak-economy` | added: the items, their prices, the pass's state and the verdicts |
| `crates/economy/src/data_rights.rs` | `deck-streak-economy` | added: the economy data-rights port |
| `crates/economy/Cargo.toml` | `deck-streak-economy` | changed: the workspace dependencies it uses (`sqlx`, `thiserror`, `tokio`); `serde` and `serde_json` as dev-dependencies for the golden reader (SPEC-029 R8) |
| `migrations/008201_economy_wallet_and_shop.sql` | `deck-streak-economy` | added: both tables, `STRICT`, the state row seeded |
| `crates/economy/tests/wallet_goldens.rs` | `deck-streak-economy` | added: A1 to A3, A10 |
| `crates/economy/tests/wallet_ports.rs` | `deck-streak-economy` | added: A4 to A6, A8, A19 |
| `crates/economy/tests/wallet_census.rs` | `deck-streak-economy` | added: A9 |
| `crates/economy/tests/wallet_rights.rs` | `deck-streak-economy` | added: A14 |
| `crates/economy/tests/shop_goldens.rs` | `deck-streak-economy` | added: A11 |
| `crates/coordination/src/recompute/mint.rs` | `deck-streak-coordination` | added: the mint step of the fold |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: registers the mint step in phase 6 of SPEC-071's fold |
| `crates/coordination/src/shop.rs` | `deck-streak-coordination` | added: the purchase use cases, the freeze through the freeze port |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules above |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the economy port joins the registry |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for both tables |
| `crates/coordination/tests/wallet_mint.rs` | `deck-streak-coordination` | added: A7 |
| `crates/coordination/tests/shop_purchase.rs` | `deck-streak-coordination` | added: A12, A13 |
| `crates/api/src/wallet_routes.rs` | `deck-streak-api` | added: the wallet and shop routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes, behind the owner's session |
| `crates/api/tests/wallet_routes.rs` | `deck-streak-api` | added: A15 |
| `crates/bot/src/shop_commands.rs` | `deck-streak-bot` | added: the shop command and its `sh:` buttons |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the shop command joins the command table and the owner's menu |
| `crates/bot/tests/shop_commands.rs` | `deck-streak-bot` | added: A16 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the wallet and the shop joined to coordination |
| `web/app/src/lib/economy/WalletHeader.svelte` | miniapp | added: the balance header |
| `web/app/src/lib/economy/ShopItem.svelte` | miniapp | added: an item card and its disabled reason |
| `web/app/src/lib/economy/wallet.ts` | miniapp | added: the routes' client |
| `web/app/src/lib/economy/shop.test.ts` | miniapp | added: A17 |
| `web/app/src/lib/economy/wallet-header.test.ts` | miniapp | added: A18 |
| `web/app/src/routes/shop/+page.svelte` | miniapp | added: the shop screen |
| `web/app/src/routes/+layout.svelte` | miniapp | changed: the wallet header on every screen |
| `web/app/src/lib/routes.ts` | miniapp | changed: the shop screen's route joins `ROUTES` |
| `tools/parity-oracle/registry/spec_082.py` | repo | added: this SPEC's registrations (SPEC-029's registry) |
| `tools/parity-oracle/goldens/mint_for_base_xp.json` | repo | added: the golden of `gamification/economy.py:mint_for_base_xp` (function) |
| `tools/parity-oracle/goldens/daily_loss_cap.json` | repo | added: the golden of `gamification/economy.py:daily_loss_cap` (function) |
| `tools/parity-oracle/goldens/scaled_fine.json` | repo | added: the golden of `gamification/economy.py:scaled_fine` (function) |
| `tools/parity-oracle/goldens/clip_debit.json` | repo | added: the golden of `gamification/economy.py:clip_debit` (function) |
| `tools/parity-oracle/goldens/coin_balance_before.json` | repo | added: the golden of `database.py:GamifyStore.coin_balance_before` (adapter over a temporary store database) |
| `tools/parity-oracle/goldens/coin_debits_for_day.json` | repo | added: the golden of `database.py:GamifyStore.coin_debits_for_day` (adapter over a temporary store database) |
| `tools/parity-oracle/goldens/buy_item.json` | repo | added: the golden of `pipeline_layers/economy.py:EconomyLayer.buy_item` (adapter over a temporary store database) |
| `tools/parity-oracle/goldens/economy.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S08200-S08299.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains both tables |
| `privacy.json` | repo | changed: the categories `coins` and `economy-state` |
| `PRIVACY.md` | repo | changed: one line for each of the two categories |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-082-coins-are-minted-spent-and-capped-in-one-wallet-and-the-shop-sells-only-for-coins.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-082.md` | docs | added |
| `changelog.d/feat-economy-082.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It raises no fine and keeps no penalty ledger: the capped debit waits for the first fine (#110),
  the confession (#111) and contract breaches (#113).
- It takes no stake and pays no wager or market (#112, #119).
- It pays no kept-window or hard-mode coins (#109, #115); quest and season-node coins arrive through
  its credit port with their features (#100, #78).
- It sets no pass surcharge and gives the pass no effect: the rung-2 fine sets the surcharge's end,
  and the pass makes doomscroll pings free, both in the tripwire (#110).
- It activates no Double-XP token and shows no smoke bomb: both join the shop with the chests
  (#103, #104).
- It charges no skip tariff itself: the skip day charges it through the floor-clipped debit (#108).
- It serves no MCP coin read (#157) and imports none of the predecessor's coin movements (#61).

## 6. Risks

- **A capped debit forgives more or less than the predecessor would.** The cap's share rounds down
  and the fine's rounds up over floating point; the port computes both in `f64` in the predecessor's
  order, and the goldens' cases include wallets whose share lands on a whole number and one coin to
  each side (A2).
- **A settled day's mint falls.** Detected by A6; the fold mints after the day's derived XP (A7), so
  the settled base is final.
- **A purchase splits across two writes,** coins taken with no freeze. Detected by A12.
- **The pass is refused after real study that has not synced.** The gate reads the current day as of
  the latest recompute (ADR-071); the refusal says to study first, and the owner's sync counts new
  reviews (ADR-037). Visible in A11's golden case and on the shop screen (A17).
- **`economy.json` drifts from the engine.** Detected by A10, and judged by the game-economy pack in
  the box run (B2).
- **A later wave writes coins around the ports.** Detected by A9's census, which refuses any other
  crate's query that names the ledger.

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_082.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch milliseconds; no golden holds a calendar date.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/mint_for_base_xp.json` | `gamification/economy.py:mint_for_base_xp` | function | none; cases at and around each multiple of the divisor, at the cap, and at zero and negative bases |
| `goldens/daily_loss_cap.json` | `gamification/economy.py:daily_loss_cap` | function | none; wallets whose share lands on a whole number and one coin to each side, at the absolute cap, zero and negative |
| `goldens/scaled_fine.json` | `gamification/economy.py:scaled_fine` | function | none; configured fines of zero and below, an empty wallet, and wallets around each point where the share passes the configured fine |
| `goldens/clip_debit.json` | `gamification/economy.py:clip_debit` | function | none; returns the allowed amount and whether any part was forgiven |
| `goldens/coin_balance_before.json` | `database.py:GamifyStore.coin_balance_before` | adapter | a temporary store database seeded with the case's synthetic movements, each study day from its epoch day number |
| `goldens/coin_debits_for_day.json` | `database.py:GamifyStore.coin_debits_for_day` | adapter | the same, with purchases, fines and credits on the day and on the days around it |
| `goldens/buy_item.json` | `pipeline_layers/economy.py:EconomyLayer.buy_item` | adapter | a stand-in layer over a temporary store database seeded with the case's movements, freezes held, the day's review count, an active pass and a surcharge end, its clock patched to the case's instant; returns the verdict, the price charged and the movements and freeze events written, every instant as epoch milliseconds |
| `goldens/economy.constants.json` | `constants.py` | constants | `constants.COIN_MINT_XP_DIVISOR`, `COIN_MINT_DAILY_CAP`, `DAILY_LOSS_CAP_COINS`, `DAILY_LOSS_CAP_WALLET_FRAC`, `FINE_WALLET_FRAC`, `SHOP_FREEZE_PRICE`, `SHOP_SCROLL_PASS_PRICE`, `SCROLL_PASS_MINUTES`, `PASS_SURCHARGE_MULT`, `PASS_SURCHARGE_HOURS` and `STREAK_FREEZE_CAP` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `coin_ledger` | `economy` | `migrations/008201_economy_wallet_and_shop.sql` (SPEC-082) | `coin_ledger`, one movement per study day, source and reference, its day as an epoch day | exported and erased |
| `economy_state` | `economy` | the same migration | the active scroll pass's end (a token row of the pass's kind) and the pass surcharge's end (a runtime setting), as epoch milliseconds | reset in place: no pass, no surcharge |

## 9. Mutation rows

The band is `S08200-S08299`, in `scripts/mutation-rows.d/S08200-S08299.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S08201-THE-MINT-DIVIDES-BY-25` | `crates/economy/src/constants.rs` | one coin for each 25 base XP | `wallet_goldens::the_coin_constants_and_economy_json_match_the_predecessors` |
| `S08202-THE-MINT-CAPS-AT-40` | `crates/economy/src/constants.rs` | at most 40 coins minted a day | `wallet_goldens::the_mint_matches_the_parity_golden` |
| `S08203-THE-LOSS-CAP-IS-100` | `crates/economy/src/constants.rs` | at most 100 coins lost a day | `wallet_goldens::the_cap_the_fine_and_the_clip_match_the_parity_goldens` |
| `S08204-THE-CAP-TAKES-30-PERCENT` | `crates/economy/src/constants.rs` | the cap's share of the day-start wallet | `wallet_goldens::the_cap_the_fine_and_the_clip_match_the_parity_goldens` |
| `S08205-A-FINE-TAKES-2-PERCENT` | `crates/economy/src/constants.rs` | a fine keeps its bite as the wallet grows | `wallet_goldens::the_cap_the_fine_and_the_clip_match_the_parity_goldens` |
| `S08206-THE-WALLET-FLOOR-IS-ZERO` | `crates/economy/src/rules.rs` | a clipped debit never takes the balance below zero | `wallet_ports::the_wallet_never_goes_negative_under_a_burst` |
| `S08207-A-SETTLED-MINT-NEVER-FALLS` | `crates/economy/src/wallet.rs` | a settled day's mint is only raised | `wallet_ports::a_settled_days_mint_is_raised_and_never_lowered` |
| `S08208-A-FREEZE-COSTS-150` | `crates/economy/src/constants.rs` | the freeze's price | `shop_goldens::the_shop_verdicts_match_the_parity_golden` |
| `S08209-A-PASS-COSTS-40` | `crates/economy/src/constants.rs` | the scroll pass's price | `shop_goldens::the_shop_verdicts_match_the_parity_golden` |
| `S08210-ONE-MOVEMENT-PER-KEY` | `migrations/008201_economy_wallet_and_shop.sql` | one movement per study day, source and reference, a key held in the migration (a script-mutation row with a cargo killer) | `wallet_ports::a_credit_of_one_key_is_written_once` |

## 10. Amendment, 2026-10-02: what E1 adds beside the manifest, and the criteria it no longer lists

Section 3's table now holds only the criteria this pull request delivers: the rows of A7, A11 to
A13 and A15 to A18 moved out of it, and each stays verbatim in section 3c's table with its fence
line there, as section 3c says a later pull request moves it back. Section 4's table is unchanged.
E1 adds these files, which it does not name:

- `docs/decisions/ADR-308-the-wallet-writes-each-movement-in-one-immediate-transaction-over-a-summed-ledger.md`:
  the wallet writes each movement in one `BEGIN IMMEDIATE` transaction over a summed ledger.
- `docs/schematics/coin-wallet-and-its-ports.md`: the wallet's ports, the ledger and the
  transaction each port runs in.
- `formal/tla/WalletFloor/WalletFloor.tla`, `formal/tla/WalletFloor/MCWalletFloor.cfg` and the four
  configurations under `formal/tla/WalletFloor/witness/`: a model of concurrent callers over one
  coin ledger. It checks that the balance never falls below the floor, that one key holds at most
  one movement, that a once-ever credit is written once over every day, and that a settled day's
  mint never falls. Each witness switches one defect on, and each is caught: the floor read outside
  the write, the once-ever guard read on its own day only, a ledger with no unique key, and a closed
  day's mint that follows its base.
- `formal/lean/Formal/Wallet.lean`: proofs over the integers that the debit clip pays at least 0
  and at most the least of the request, the wallet and the remaining cap, and pays nothing for a
  request of 0 or less, and that the mint lies between 0 and 40 and never falls as the base grows.
  The loss cap and the scaled fine go through floats; their parity is proved by the goldens.
- `formal/lean/Formal/WalletVectors.lean`, one import and one arm in
  `formal/lean/Formal/Vectors.lean`, and one import in `formal/lean/Formal.lean`: the vector writer
  for the proof's ports.
- `formal/vectors/wallet.jsonl`: the vectors, written by the writer and never by hand.
- `crates/economy/tests/formal_vectors_wallet.rs`: `clip_debit` and `mint_for_base_xp` answer every
  vector, with the proof's recorded counterexamples as literal cases.
- `config/formal.json` and `scripts/tests/test_formal_config.py`: the model's time budget, and the
  test that pins it.
- `crates/coordination/tests/relight_order.rs`: the register of every `static` coordination links
  gains the economy data-rights port's, from 14 entries to 15.

The band file section 4 names, `scripts/mutation-rows.d/S08200-S08299.json`, holds E1's eight rows,
S08201 to S08207 and S08210. S08208 and S08209 guard the shop's prices and land with E3. S08210
targets a migration and is killed by a cargo test, so it sits in the band's
`CARGO_KILLED_SCRIPT_MUTATIONS` table, as every other migration row does.

`crates/economy/Cargo.toml` takes its dependencies in a different order and shape than section 4
says:

- `sqlx` arrives with the migration and the data-rights port, and `serde_json` is a dependency
  rather than a dev-dependency, because that port's rows are JSON values;
- `thiserror` arrives with the red tests' stub API, which already names a purchase's refusal;
- `tokio` is a dev-dependency only, because the library spawns nothing: the port tests run their
  concurrent callers on it, and `tempfile`, also a dev-dependency, gives each test its database.

These files section 4 names are left unchanged by E1. The later pull request named on each line adds
or changes them:

- `crates/economy/src/shop.rs`, `crates/economy/tests/shop_goldens.rs` and
  `tools/parity-oracle/goldens/buy_item.json`: E3 delivers them.
- `crates/coordination/src/recompute/mint.rs`, `crates/coordination/src/recompute/mod.rs` and
  `crates/coordination/tests/wallet_mint.rs`: E1b delivers them.
- `crates/coordination/src/shop.rs` and `crates/coordination/tests/shop_purchase.rs`: E3 delivers
  them.
- `crates/coordination/src/lib.rs`, `crates/api/src/wallet_routes.rs`, `crates/api/src/router.rs`
  and `crates/api/tests/wallet_routes.rs`: E1b and E3 deliver them.
- `crates/bot/src/shop_commands.rs`, `crates/bot/src/commands.rs` and
  `crates/bot/tests/shop_commands.rs`: E3 delivers them.
- `crates/daemon/src/wiring.rs`: E1b delivers it.
- `web/app/src/lib/economy/WalletHeader.svelte`, `web/app/src/lib/economy/wallet.ts`,
  `web/app/src/lib/economy/wallet-header.test.ts` and `web/app/src/routes/+layout.svelte`: E1b
  delivers them.
- `web/app/src/lib/economy/ShopItem.svelte`, `web/app/src/lib/economy/shop.test.ts`,
  `web/app/src/routes/shop/+page.svelte` and `web/app/src/lib/routes.ts`: E3 delivers them.

The ports R7 calls `credit` and `credit_once` are named `deposit` and `deposit_once` in code, with their `_on` forms `deposit_on` and `deposit_once_on` and their answer `DepositAnswer` (`Deposited`, `AlreadyDeposited`), because docs/LEXICON.md locks `credit` out of a declaration in deck-streak-economy (ADR-308 ruling 7); R7's prose is unchanged.

## 11. Amendments, 2026-10-02: the manifest of E1

Section 4's table names the files of every pull request of this SPEC. This section is E1's
manifest against it: first the rows E1 leaves to a later pull request, as section 10 splits them,
then the files E1 adds or changes that section 4 does not name.

Section 4's rows that E1 leaves to a later pull request:

- `crates/economy/src/shop.rs`: unchanged by E1; E3 delivers it.
- `crates/economy/tests/shop_goldens.rs`: unchanged by E1; E3 delivers it.
- `tools/parity-oracle/goldens/buy_item.json`: unchanged by E1; E3 delivers it.
- `crates/coordination/src/recompute/mint.rs`: unchanged by E1; E1b delivers it.
- `crates/coordination/src/recompute/mod.rs`: unchanged by E1; E1b delivers it.
- `crates/coordination/tests/wallet_mint.rs`: unchanged by E1; E1b delivers it.
- `crates/coordination/src/shop.rs`: unchanged by E1; E3 delivers it.
- `crates/coordination/tests/shop_purchase.rs`: unchanged by E1; E3 delivers it.
- `crates/coordination/src/lib.rs`: unchanged by E1; E1b and E3 deliver it.
- `crates/api/src/wallet_routes.rs`: unchanged by E1; E1b and E3 deliver it.
- `crates/api/src/router.rs`: unchanged by E1; E1b and E3 deliver it.
- `crates/api/tests/wallet_routes.rs`: unchanged by E1; E1b and E3 deliver it.
- `crates/bot/src/shop_commands.rs`: unchanged by E1; E3 delivers it.
- `crates/bot/src/commands.rs`: unchanged by E1; E3 delivers it.
- `crates/bot/tests/shop_commands.rs`: unchanged by E1; E3 delivers it.
- `crates/daemon/src/wiring.rs`: unchanged by E1; E1b delivers it.
- `web/app/src/lib/economy/WalletHeader.svelte`: unchanged by E1; E1b delivers it.
- `web/app/src/lib/economy/wallet.ts`: unchanged by E1; E1b delivers it.
- `web/app/src/lib/economy/wallet-header.test.ts`: unchanged by E1; E1b delivers it.
- `web/app/src/routes/+layout.svelte`: unchanged by E1; E1b delivers it.
- `web/app/src/lib/economy/ShopItem.svelte`: unchanged by E1; E3 delivers it.
- `web/app/src/lib/economy/shop.test.ts`: unchanged by E1; E3 delivers it.
- `web/app/src/routes/shop/+page.svelte`: unchanged by E1; E3 delivers it.
- `web/app/src/lib/routes.ts`: unchanged by E1; E3 delivers it.

The files E1 adds or changes that section 4 does not name:

- `docs/decisions/ADR-308-the-wallet-writes-each-movement-in-one-immediate-transaction-over-a-summed-ledger.md`: added, the wallet's decision record.
- `docs/schematics/coin-wallet-and-its-ports.md`: added, the wallet's ports and their transactions.
- `formal/tla/WalletFloor/WalletFloor.tla`: added, the model of concurrent callers over one ledger.
- `formal/tla/WalletFloor/MCWalletFloor.cfg`: added, the model's configuration.
- `formal/tla/WalletFloor/witness/a-floor-read-outside-the-write.cfg`: added, a witness.
- `formal/tla/WalletFloor/witness/a-once-ever-guard-read-on-its-own-day.cfg`: added, a witness.
- `formal/tla/WalletFloor/witness/a-ledger-with-no-unique-key.cfg`: added, a witness.
- `formal/tla/WalletFloor/witness/a-closed-day-mint-that-follows-its-base.cfg`: added, a witness.
- `formal/lean/Formal/Wallet.lean`: added, the proofs of the debit clip and the mint.
- `formal/lean/Formal/WalletVectors.lean`: added, the proofs' vector writer.
- `formal/lean/Formal/Vectors.lean`: changed, one import and one arm for the writer.
- `formal/lean/Formal.lean`: changed, one import.
- `formal/vectors/wallet.jsonl`: added, the vectors the writer wrote.
- `crates/economy/tests/formal_vectors_wallet.rs`: added, the Rust rules against the vectors.
- `config/formal.json`: changed, the model's time budget.
- `scripts/tests/test_formal_config.py`: changed, the test that pins the budget.
- `crates/coordination/tests/relight_order.rs`: changed, the economy port joins the static register.
- `scripts/mutation-equivalent.d/deck-streak-economy.json`: added, the economy's equivalent mutants.
