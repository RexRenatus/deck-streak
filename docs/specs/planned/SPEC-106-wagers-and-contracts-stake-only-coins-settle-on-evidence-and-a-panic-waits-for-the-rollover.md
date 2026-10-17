# SPEC-106: wagers and contracts stake only coins, settle on evidence, and a panic waits for the rollover

- **Wave:** W5. **Issue:** #112 (the streak wager), #113 (commitment contracts), #114 (panic, pardon
  and re-arm) and #116 (the money rung, which this SPEC holds unbuilt under ADR-106), in epic #6.
  **Context(s):** `deck-streak-discipline` (the wager's arming, settlement and revision, the
  contract's authoring, verdict, weakening and revision, the pardon, the panic, the engine switch's
  writes and the Sunday review's rule); `deck-streak-coordination` (the stakes' step of the sync
  cycle, the panic's application across discipline, markets and economy, the stakes' messages at
  the discipline tick, and the active wager's stake passed to the evening job); `deck-streak-bot`
  (`/wager`, `/contract`, `/panic`, `/pardon`, `/discipline_on`); `deck-streak-api` (the stakes'
  routes); the Mini App (`web/app`, the stakes' cards on the `/discipline` screen).
- **Decided by:** ADR-106 (this SPEC's: the money rung is not built until the owner rules), ADR-104
  (a verdict settles after its evidence and is revised within 7 closed study days: a contract's day
  and a lost wager), ADR-105 (the discipline tick, which raises the stakes' messages), ADR-103 (the
  fine and its reversal), ADR-037 (one sync a study day), ADR-012 (the parity oracle) and ADR-071 (a
  settled day's rollup changes only with its reviews).
- **Prerequisites:** SPEC-105 (discipline's floor: `discipline_state` and its engine switch, the
  kind `discipline`, the tick, and the kept windows of a study day), SPEC-104 (the rail's defections
  of a study day), SPEC-103 (`fine` and `reverse_fine`), SPEC-107 (markets' `void_open_positions`
  port, and the streak's read of the freeze markers of a range of study days), SPEC-100 (the evening
  job and its stakes preview's wager line), SPEC-082 (the wallet's `purchase` and `credit`),
  SPEC-076 (the governor's stored verdict and the freeze markers), SPEC-083 (the skip set), SPEC-071
  (the rollup's reviews of a study day), SPEC-084 (the celebration ladder), SPEC-041 (the router)
  and SPEC-023 (the sync cycle).
  **Mutation band:** `S10600-S10699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-106.md` (ADR-016).

## 1. The problem, measured

- **What exists.** After SPEC-103, SPEC-104, SPEC-105 and SPEC-107 build, discipline holds its state
  row with an engine switch and a panic's study day that nothing writes, the rail and the windows;
  economy holds the fine port and its reversal; markets holds a port that voids its open positions.
  No wager or contract table exists, and SPEC-082 takes no stake and pays no wager (its section 5).
- **What is ported** (at `27ee2bc`):
  - the wager, `pipeline_layers/discipline.py:DisciplineLayer.arm_wager` and `_settle_wagers`, with
    the offers of `bot.py:CommandBot._process_update` (`/wager`) and `_callback_wager`;
  - the contract, `DisciplineLayer.build_contract`, `_contract_verdict`, `_settle_contracts`,
    `queue_contract_weakening`, `cancel_contract_weakening` and `_apply_contract_changes`, with
    `database.py:GamifyStore.stake_override_for_day` and the builder of
    `bot.py:CommandBot._contract_builder_keyboard` and `_callback_contract`;
  - the pardon and the panic, `DisciplineLayer.pardon_latest_breach`, `panic`, `cancel_panic` and
    `_apply_panic`, with `database.py:GamifyStore.use_pardon`;
  - the Sunday review, `DisciplineLayer._sunday_stake_review` and `telegram.py:render_stake_review`.
- **The clock.** The stakes' verdicts read the rollup, the kept windows, the rail's defections, the
  freeze markers and the governor, so each settles at the first sync cycle after its study day
  closes (ADR-104), in the predecessor's order: the wager, then the panic, then the weakenings, then
  the contract days. Their messages cannot be raised there: the scheduled settle runs inside quiet
  hours by default, and the router withholds a nudge there (SPEC-041 R4, rule 4). The discipline
  tick raises them outside quiet hours (ADR-105), as it raises SPEC-105's reminders.
- **The money rung (#116).** The predecessor posts a day's reviews and pings to an outside pledge
  service whose pledges are real money (`DisciplineLayer._post_beeminder`). Money is the owner's
  decision and an irreversible class: ADR-106 builds nothing that posts, and the question goes to
  the owner.
- **Deviations from the predecessor, each with its reason.**
  - A lost wager is revised (ADR-104): when a day of the gap its streak break closed turns out to
    hold a study review that synced late, the wager is voided and its stake refunded. The streak
    stays as SPEC-076 settled it, which never rejoins a late review (SPEC-076, section 6), so the
    loss would otherwise rest on reviews DeckStreak had not read.
  - A breached contract day is revised within 7 closed study days on its metric, the skip set and
    the freeze markers as they now stand (ADR-104). The governor is read once, at the first
    settlement, because its state keeps no past verdict (SPEC-076 R16).
  - Authoring refuses terms outside the builder's offer, which #113 asks for; the predecessor
    accepts any positive threshold and stake. Its own comment names the reason: a button's data is
    written by the client.
  - Arming a wager, authoring a contract and scheduling a panic are refused while the engine is off.
    The predecessor checks the governor only, so a contract authored while off would be judged,
    after the re-arm, on the days the engine was off, against #114's rule that penalties resume only
    after the re-arm.
  - The weakening's horizon is 72 hours, the predecessor's only value: its per-contract column has
    no writer (inert in v9), so the landing day's rule is kept with the horizon as a constant.
  - A message is raised by the tick outside quiet hours, once. The predecessor sends a lost wager's
    message and the panic's notice at once, at night, and drops a breach's message inside quiet
    hours (`EconomyLayer._notify_awake`), so the pardon's offer rarely reached the owner.
  - The panic's cancel button answers the port's refusal; the predecessor's answers "cancelled"
    whatever the port answered.
  - Every day is an epoch day: the pardon's month is keyed by the epoch day of its first day, and
    the review's week by its Monday's, never by a calendar string.

## 2. Requirements

The wager (#112)

R1. `arm_wager(stake, days)` refuses, in this order, with `engine off` while discipline's engine is
    off, and then in the golden `arm_wager`'s order: `governor standby/lapse — no new stakes` when
    the governor's stored verdict is not armed, `stake must be 1..<half> (50% of wallet)` for a
    stake below 1 or above half the wallet rounded down, `duration must be 7/14/30`, and `a wager is
    already active` (one active wager, a partial unique index). Otherwise it records the wager,
    starting on the current study day and ending `days` study days later, and debits the stake
    through SPEC-082's `purchase(day, "wager_stake", "<wager id>", stake)` in the same transaction,
    which coordination holds (as SPEC-082 R10's freeze does).
R2. The bot and the Mini App offer the stakes 25, 50 and 100 that are at most half the wallet
    rounded down, and the durations 7, 14 and 30 days, equal to the golden `wager_offers`; with no
    stake offered, `/wager` says that a wager needs coins and names the wallet.
R3. At each sync cycle (SPEC-023 R12), the active wager settles from these inputs: the governor's
    stored verdict (SPEC-076 R16); the language track's freeze markers `consumed` and `streak_break`
    of the study days after its start up to the earlier of the current study day and its end plus 3
    (SPEC-076 R6); the skip set (SPEC-083) after its start up to its end; and the current study day.
    Over the outcome set `active`, `won`, `won_early`, `lost` and `voided`, and equal to the golden
    `wager_settle`:
    - the governor not armed: `voided`, and the stake is refunded through SPEC-082's `refund(day,
      "wager_refund", "<wager id>", stake)`;
    - otherwise its flat spots are the consumed markers and the skip days counted above, its
      extensions the smaller of 3 and the flat spots, its effective end its end plus the extensions,
      and the stored extensions follow them;
    - a `streak_break` marker after the start up to the effective end: `lost`;
    - else more than 3 flat spots: `won_early`, paying the stake plus half the stake rounded down
      (`credit(day, "wager_win", "<wager id>", amount)`);
    - else a current study day after the effective end: `won`, paying twice the stake the same way;
    - else it stays `active`.
    A settlement's status and its coin movement are written in one transaction, and only an
    `active` wager settles, so each is written once.
R4. When a wager settles `lost`, it records its break's study day and the first day of the gap the
    break closed: the earliest day of the run of days before the break's study day that held no
    study review in SPEC-071's rollup, skip days bridged. At each sync cycle while the break's study
    day is the current study day or one of the 7 closed study days before it, the loss is judged
    again: when a day of that gap now holds a study review, the wager becomes `voided` with the
    reason `revision` and its stake is refunded through `refund(day, "wager_refund", "<wager id>",
    stake)` in the same transaction as the status, so a stake is refunded once whatever voids it
    (ADR-104). Its streak stays as SPEC-076 settled it. `won`, `won_early` and `voided` are final.
R5. A void records its reason, one of `standby` (R3), `panic` (R15) and `revision` (R4).

The contract (#113)

R6. `build_contract(metric, threshold, stake, weeks)` refuses, in this order: in the golden
    `build_contract`'s order, `metric must be reviews/windows/pings`, then `bad terms` for a
    threshold or stake of 0 or less or weeks not 1, 2 or 4; then `not an offered contract` for terms
    outside the offer (R7, #113); then `engine off`; then, in the golden's order again, `governor
    standby/lapse — author when strong`, `max 2 concurrent contracts`, and `an identical contract is
    already running` for an active contract with the same metric and threshold. Otherwise the
    contract starts on the current study day and ends 7 times `weeks` study days later; its stake is
    the fine of each breached day, and no coin moves when it is authored.
R7. The offer is the metrics reviews 30, reviews 60, windows 1 and pings 3, each with the stake and
    length pairs 20 coins for 2 weeks, 50 for 2 weeks and 50 for 4 weeks, equal to the golden
    `contract_offers`.
R8. A contract day's verdict is a pure function equal to the golden `contract_verdict` over the
    outcome set `clean`, `breach` and `flat` (the pardon alone writes `pardoned`, R13): `flat` when
    the day is a skip day, a lapse is open, the governor is in standby, or a freeze was consumed on
    the next day; otherwise, by metric, `clean` when the rollup's reviews of the day reach the
    threshold (`reviews`), when its kept windows reach it (`windows`), or when its defections are at
    most the threshold (`pings`); otherwise `breach`.
R9. At each sync cycle, after the weakenings land (R12), each active contract's days from the later
    of its start and the 7th closed study day back to the earlier of the last closed study day and
    its end, not yet judged, are judged once (unique on contract and study day) from these inputs:
    the rollup's reviews (SPEC-071); the study day's kept windows (SPEC-105); its defections, the
    rail's events of verdict `defection` (SPEC-104); the skip set (SPEC-083); the freeze markers
    `consumed` (SPEC-076 R6); and the governor's stored verdict (SPEC-076 R16). For the
    predecessor's cycle the step equals the golden `contract_settle`. A breach fines through
    SPEC-103's `fine(current study day, "contract:<contract id>:<epoch day>", stake in force)`,
    where the stake in force on a day is the old stake of the earliest applied weakening that landed
    after that day, or the contract's stake when none did, equal to the golden `contract_stake_on`.
    At the first cycle whose current study day is after its end, the contract becomes `done` and
    leaves the celebration `quest_all` with the key `contract_done:<contract id>` pending on its row
    for the tick (R17). While the engine is off no contract day is judged.
R10. At each sync cycle, every `breach` of a contract, whatever its status, on one of the 7 closed
    study days before the current one, whose fine is not reversed, is judged again on R8's rule with
    the rollup, the kept windows, the defections, the skip set and the freeze markers as they now
    stand and with the governor taken as armed, as it was when the breach was judged. When it is now
    `clean` or `flat`, the day takes that verdict and its fine is reversed through SPEC-103's
    `reverse_fine(reference, current study day, "revision")` in the same transaction (ADR-104). A
    `clean` or `flat` day is never judged a breach later, and a `pardoned` day is final.
R11. `queue_contract_weakening(contract, new stake)` refuses, in the golden `contract_weakening`'s
    order, with `no such active contract`; `weakening must lower the stake (floor 10 🪙)` for a new
    stake below 10, the smaller of the weakening stakes 20 and 10, or not below the current stake;
    `contract ends before the horizon — ride it out or /panic` when the landing day falls after the
    contract's end; and `a weakening is already queued for this contract` (one pending change per
    contract, a partial unique index). The landing day is the current study day plus 1 plus the
    72-hour horizon's hours plus 23, divided by 24 and rounded down: 4 study days later. The bot and
    the Mini App offer the weakening stakes below the current stake. `cancel_contract_weakening`
    cancels a pending change at once, and refuses with `no weakening queued`.
R12. At each sync cycle, after the panic (R15), each pending weakening whose landing day is on or
    before the current study day lands: the stake of its contract becomes the new stake when the
    contract is active and the change becomes `applied`, or else the change becomes `cancelled`,
    over the outcome set `pending`, `applied` and `cancelled`, equal to the golden
    `contract_changes_apply`.

The pardon, the panic and the re-arm (#114)

R13. `pardon_latest_breach()` refuses, in the golden `pardon_breach`'s order, with `no breach to
    pardon` when no contract day holds `breach`; `older than 48h` when the newest breach's day is
    more than 2 study days before the current one; and `pardon already used this month` when the
    current study day's calendar month, keyed by the epoch day of its first day, has used its one
    pardon. Otherwise, in one transaction, it claims the month's pardon, sets the day's verdict to
    `pardoned`, and reverses its fine through `reverse_fine(reference, current study day,
    "pardon")`.
R14. `panic()` stores the next study day as the panic's study day, and `cancel_panic()` clears it;
    while the engine is off both refuse with `already disabled — /discipline_on re-arms`. For the
    predecessor's cases both equal the golden `panic_schedule`; its panic while off, which schedules
    a disable that voids nothing and repeats its notice, is the recorded deviation.
R15. At the first sync cycle whose current study day is on or after the panic's study day, after the
    wager settles (R3) and before the weakenings (R12), in one transaction: the engine is switched
    off and the panic's day cleared; the active wager is voided with the reason `panic` and its
    stake refunded through `refund(day, "wager_refund", "<wager id>", stake)`; each active contract
    becomes `revoked`, so its pending weakening is cancelled at R12; markets' `void_open_positions`
    (SPEC-107) voids each open position and refunds its stake; and the panic's notice is left
    pending for the tick (R17). Discipline's part equals the golden `panic_apply`. Nothing is voided
    before that cycle, so a stake armed on the panic's eve still stands for it.
R16. `/discipline_on`, the notice's re-arm button (`ct:rearm`) and the Mini App's switch turn the
    engine on. While it is off, no contract day is judged (R9), an open fines nothing (the rail's
    input, SPEC-104 R6), and arming, authoring and a panic are refused (R1, R6, R14); from the
    re-arm on, each resumes.

The messages (ADR-105)

R17. A settlement leaves its message pending on its row, and at each tick outside quiet hours
    (SPEC-105 R12) the pending messages are raised through the router under the kind `discipline`
    (ADR-104), each with its per-incident key: a lost or voided wager (`wager-settled:<wager id>`),
    a wager revised (`wager-revised:<wager id>`), a breach with the pardon's offer
    (`breach:<contract id>:<epoch day>`), a breach revised with its refund
    (`breach-revised:<contract id>:<epoch day>`), a landed weakening (`weakened:<change id>`), and
    the panic's notice with the re-arm button (`panic:<epoch day>`). A message is cleared when the
    router sends it, or withholds it with `already_recorded`, `nudges_disabled` or `lapse`; any
    other withhold leaves it for the next tick. The router's per-incident key makes each message
    once, however many ticks run at once. A `won` or `won_early` wager leaves the celebration
    `quest_all` with the key `wager:<wager id>` pending on its row at its settlement, as a `done`
    contract leaves `contract_done:<contract id>` (R9); the tick raises each beside the messages,
    through SPEC-084's ladder, and clears it as it clears a message or when the router defers it,
    so a scheduled sync, which carries no router, loses neither.
R18. At each tick outside quiet hours on a study day that is a Sunday, when an active contract, an
    active wager or tonight's booked hard-mode night exists (SPEC-105) and the week's review has not
    been recorded, the review is raised under `discipline` with the key `stakes:<the week's Monday
    epoch day>`, and the week is recorded in `discipline_state` only when the router sends it or
    withholds it with `already_recorded`, `nudges_disabled` or `lapse`. It lists each contract (its
    metric, threshold, stake, clean and breached days, end, and a pending weakening's stake and
    landing day), the wager (its stake and end) and tonight's night (its start and duration), and
    the two exits: a weakening waits for its horizon, and a panic disables everything at the next
    rollover. Which study days and weeks raise it equals the golden `stake_review`.

Surfaces, tables and rights

R19. The bot keeps the predecessor's commands and buttons: `/wager` (`wg:stake:<stake>`, then
    `wg:go:<stake>:<days>`); `/contract` (the active contracts with `ct:wk:<id>`,
    `ct:wkgo:<id>:<stake>` and `ct:wkcancel:<id>`, and `ct:new`; the builder's
    `ct:<metric>:<threshold>`, then `ct:go:<metric>:<threshold>:<stake>:<weeks>`); `/panic` with
    `ct:cancelpanic`; `/pardon`; and `/discipline_on` with `ct:rearm`. Each button's data is checked
    again by its use case, never trusted, and every refusal names its reason.
R20. The API serves, behind the owner's session: `GET /api/discipline/stakes` (the wager, the
    contracts with their days and pending weakenings, the engine, a scheduled panic and whether this
    month's pardon is used), `POST /api/discipline/wager`, `POST /api/discipline/contracts`, `POST
    /api/discipline/contracts/{id}/weakening`, `DELETE /api/discipline/contracts/{id}/weakening`,
    `POST /api/discipline/panic`, `DELETE /api/discipline/panic`, `POST /api/discipline/pardon` and
    `POST /api/discipline/rearm`.
R21. The Mini App's `/discipline` screen gains the wager card (the offered stakes and durations, and
    the active wager's day count and effective end), the contracts card (the builder from the offer,
    each contract's days as a calendar of verdicts, and a pending weakening's landing day, never a
    ticking countdown) and the engine card (the switch, a scheduled panic with its cancel, the
    re-arm, and whether this month's pardon is used).
R22. The migration `migrations/010601_discipline_stakes.sql` creates `wagers` (one `active` row at
    most), `contracts`, `contract_days` (unique on contract and study day), `contract_changes` (one
    `pending` row per contract at most) and `pardons` (one row per month), each `STRICT` with
    `created_at`, with `CHECK`s on the statuses, the verdicts, the metrics, the durations and the
    void reasons, and the pending-message flag of each wager, contract, contract day and change (a
    won wager's and a done contract's celebration among them, R17); and it adds to
    `discipline_state` the last reviewed week and the pending panic notice (SPEC-020 R15, R18).
R23. Discipline's data-rights port exports and erases the five tables, and the reset of
    `discipline_state` (SPEC-105 R24) clears the two new fields; the five tables owe SPEC-021's six
    files.
R24. The weakening stakes equal the constants golden `stakes.constants`; the horizon, the durations,
    the offers, the payouts and the day limits are proved by the goldens of R1 to R18. The wager's
    stake options, durations, stake fraction, multipliers, extension cap and void states equal
    `economy.json`'s `wager` section, the game-economy pack's reference, which this SPEC does not
    change.
R25. No crate and no screen posts to an outside pledge service or names it, and no setting enables
    one (ADR-106, #116).
R26. The evening job (SPEC-100 R14) passes the active wager's stake, in whole coins, read from
    discipline's store of wagers, as the stakes preview's wager input (SPEC-100 R15). It passes none
    while no wager is `active`, so a won, lost or voided wager draws no line. The stake is the only
    input this SPEC adds to that job.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | arming's refusals and their order equal the golden `arm_wager`, at a stake of half the wallet and one more, and at each duration and 10 | `the_arming_matches_the_parity_golden` |
| A2 | the offered stakes and durations equal the golden `wager_offers` at wallets of 49, 50, 99, 100, 199 and 200 | `the_wager_offers_match_the_parity_golden` |
| A3 | the settlement equals the golden `wager_settle` over its outcomes, at 3 and 4 flat spots, a break on the start day and on the effective end, a consumed marker on the end plus 3 and plus 4, and the current study day on the effective end and one after | `the_wager_settlement_matches_the_parity_golden` |
| A4 | arming writes the wager and the stake's debit in one transaction, and a failed write leaves neither | `arming_debits_the_stake_in_one_write` |
| A5 | a second active wager is refused with its reason, whatever the order of two arms at once | `a_second_wager_is_refused` |
| A6 | the settlement reads each of R3's inputs from its source (the governor's stored verdict, the consumed and break markers, the skip set and the current study day), each proved by flipping that source alone, and pays or refunds once | `the_wager_settlement_gathers_every_input` |
| A7 | a lost wager whose gap gains a late study review in the rollup within 7 closed study days is voided with the reason `revision` and refunded once; a skip day in the gap is bridged, and one past the window is not revised | `a_lost_wager_is_voided_when_its_gap_proves_studied` |
| A8 | authoring's refusals and their order equal the golden `build_contract`, at weeks 1, 2, 3 and 4, a third contract and an identical one | `the_authoring_matches_the_parity_golden` |
| A9 | the offer equals the golden `contract_offers` | `the_contract_offer_matches_the_parity_golden` |
| A10 | terms outside the offer are refused with `not an offered contract`, each metric and pair tried | `terms_outside_the_offer_are_refused` |
| A11 | the verdict equals the golden `contract_verdict` over its outcomes, at each threshold, one below and one above, and in its flat order | `the_contract_verdict_matches_the_parity_golden` |
| A12 | the days judged equal the golden `contract_settle`, at the 7th and the 8th closed study day back, the start day, the end day, a judged day and a finished contract | `the_contract_settlement_matches_the_parity_golden` |
| A13 | the stake in force equals the golden `contract_stake_on`, with a weakening landing on the day and on the day after | `the_stake_in_force_matches_the_parity_golden` |
| A14 | the contract days read each of R9's inputs from its source (the rollup's reviews, the kept windows, the defections, the skip set, the consumed markers and the governor's stored verdict), each flipped alone, and a breach fines the stake in force once under its reference | `the_contract_days_gather_every_input` |
| A15 | a breach is judged again on the rollup's reviews, the kept windows, the defections, the skip set and the consumed markers as they now stand: a late review that meets the metric revises it to clean and refunds it once; a clean day is never re-judged; a pardoned day is final; one past the window stands | `a_breach_is_revised_when_its_evidence_fails` |
| A16 | the weakening's refusals, its order and its landing day equal the golden `contract_weakening`, at a new stake of 10, 9 and the current one, and a landing day on the end and one after | `the_weakening_matches_the_parity_golden` |
| A17 | landing equals the golden `contract_changes_apply`, a revoked contract's change cancelled | `the_landing_matches_the_parity_golden` |
| A18 | the pardon's refusals and order equal the golden `pardon_breach`, at 2 and 3 study days back and with the month's pardon used | `the_pardon_matches_the_parity_golden` |
| A19 | a second pardon in a month is refused with its reason, and the next month's first is not | `a_second_pardon_in_a_month_is_refused` |
| A20 | a pardon sets the day pardoned and refunds its fine once through the fine port | `a_pardon_refunds_the_breach_once` |
| A21 | the panic's schedule and cancel equal the golden `panic_schedule` | `the_panic_schedule_matches_the_parity_golden` |
| A22 | discipline's part of the panic equals the golden `panic_apply` | `the_panic_application_matches_the_parity_golden` |
| A23 | nothing is voided before the rollover; at the first cycle on the panic's day the wager settles first, then it is voided and refunded, the contracts revoked, their weakenings cancelled and the open positions voided and refunded | `nothing_is_voided_before_the_rollover` |
| A24 | while the engine is off an open fines nothing and no contract day is judged, and after the re-arm both resume | `penalties_resume_only_after_the_rearm` |
| A25 | arming, authoring and a panic are refused with their reasons while the engine is off | `stakes_are_refused_while_the_engine_is_off` |
| A26 | the review's study days and weeks equal the golden `stake_review` | `the_stake_review_matches_the_parity_golden` |
| A27 | the review is raised once a week outside quiet hours, recorded only when sent, retried after a missing transport, and not raised with no stakes | `the_sunday_review_is_raised_once_a_week_after_a_send` |
| A28 | each pending message is raised once outside quiet hours, left pending after a missing transport, and cleared when the setting is off | `each_stake_message_is_raised_once_outside_quiet_hours` |
| A29 | a won wager and a finished contract each leave their celebration pending at the sync, which sends nothing, the next tick outside quiet hours raises each once through the ladder, and a second tick raises nothing | `wins_and_finished_contracts_celebrate_once` |
| A30 | the constants equal the constants golden | `the_stake_constants_match_the_predecessors` |
| A31 | no file under `crates/*/src/` or `web/app/src/` names the pledge service or its host, and no setting key enables one | `no_path_posts_to_a_pledge_service` |
| A32 | the wager's constants equal `economy.json`'s `wager` section, key for key | `the_wager_constants_equal_the_economy_declaration` |
| A33 | the commands and buttons call their use cases through a stub port and relay a refusal's reason unchanged, and a malformed button's data reaches no use case | `stake_commands_run_the_use_cases` |
| A34 | the stakes' routes answer the owner's session only | `the_stake_routes_answer_only_the_owner` |
| A35 | the screen arms a wager from the offers and shows a pending weakening's landing day | `arms a wager and shows the landing day` |
| A36 | an erase empties the five tables and clears the two new state fields | `an_erase_empties_the_stake_tables` |
| A37 | the evening job passes the active wager's stake to the stakes preview, and none with no wager or with a won, lost or voided one | `the_evening_job_passes_the_active_wagers_stake` |

```acceptance
A1: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_arming_matches_the_parity_golden
A2: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_wager_offers_match_the_parity_golden
A3: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_wager_settlement_matches_the_parity_golden
A4: cargo test -p deck-streak-coordination --test discipline_stakes -- --exact arming_debits_the_stake_in_one_write
A5: cargo test -p deck-streak-discipline --test stake_rules -- --exact a_second_wager_is_refused
A6: cargo test -p deck-streak-coordination --test discipline_stakes -- --exact the_wager_settlement_gathers_every_input
A7: cargo test -p deck-streak-coordination --test discipline_stakes -- --exact a_lost_wager_is_voided_when_its_gap_proves_studied
A8: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_authoring_matches_the_parity_golden
A9: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_contract_offer_matches_the_parity_golden
A10: cargo test -p deck-streak-discipline --test stake_rules -- --exact terms_outside_the_offer_are_refused
A11: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_contract_verdict_matches_the_parity_golden
A12: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_contract_settlement_matches_the_parity_golden
A13: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_stake_in_force_matches_the_parity_golden
A14: cargo test -p deck-streak-coordination --test discipline_stakes -- --exact the_contract_days_gather_every_input
A15: cargo test -p deck-streak-coordination --test discipline_stakes -- --exact a_breach_is_revised_when_its_evidence_fails
A16: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_weakening_matches_the_parity_golden
A17: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_landing_matches_the_parity_golden
A18: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_pardon_matches_the_parity_golden
A19: cargo test -p deck-streak-discipline --test stake_rules -- --exact a_second_pardon_in_a_month_is_refused
A20: cargo test -p deck-streak-coordination --test discipline_stakes -- --exact a_pardon_refunds_the_breach_once
A21: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_panic_schedule_matches_the_parity_golden
A22: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_panic_application_matches_the_parity_golden
A23: cargo test -p deck-streak-coordination --test discipline_panic -- --exact nothing_is_voided_before_the_rollover
A24: cargo test -p deck-streak-coordination --test discipline_panic -- --exact penalties_resume_only_after_the_rearm
A25: cargo test -p deck-streak-discipline --test stake_rules -- --exact stakes_are_refused_while_the_engine_is_off
A26: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_stake_review_matches_the_parity_golden
A27: cargo test -p deck-streak-coordination --test discipline_stake_messages -- --exact the_sunday_review_is_raised_once_a_week_after_a_send
A28: cargo test -p deck-streak-coordination --test discipline_stake_messages -- --exact each_stake_message_is_raised_once_outside_quiet_hours
A29: cargo test -p deck-streak-coordination --test discipline_stake_messages -- --exact wins_and_finished_contracts_celebrate_once
A30: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_stake_constants_match_the_predecessors
A31: cargo test -p deck-streak-discipline --test money_rung_census -- --exact no_path_posts_to_a_pledge_service
A32: cargo test -p deck-streak-discipline --test stake_goldens -- --exact the_wager_constants_equal_the_economy_declaration
A33: cargo test -p deck-streak-bot --test stake_commands -- --exact stake_commands_run_the_use_cases
A34: cargo test -p deck-streak-api --test stake_routes -- --exact the_stake_routes_answer_only_the_owner
A35: pnpm exec vitest run web/app/src/lib/discipline/stakes.test.ts -t "arms a wager and shows the landing day"
A36: cargo test -p deck-streak-discipline --test stake_rights -- --exact an_erase_empties_the_stake_tables
A37: cargo test -p deck-streak-coordination --test evening_wager_line -- --exact the_evening_job_passes_the_active_wagers_stake
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, ux-laws, accessibility,
game-economy and notifications-policy packs stay enforced, and no row is deferred or lifted for
this delivery, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | the five tables are declared under the category `discipline-stakes` with data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010601_discipline_stakes.sql` and `crates/discipline/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the stakes' copy carries no countdown pressure and no shame, and says that coins are the only stake, over every file under `web/app/src/lib/discipline/` and `crates/coordination/src/discipline/` | the ux-laws pack |
| B3 | the discipline screen with its stakes' cards passes the accessibility audit in both colour schemes, over the `/discipline` route that `web/app/src/lib/routes.ts` lists | the accessibility pack |
| B4 | the economy's declaration still names every asset but coins as never confiscated, over `economy.json` | the game-economy pack |
| B5 | every stakes message goes through the one router, over every file under `crates/coordination/src/discipline/` and `crates/bot/src/` | the notifications-policy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/discipline/src/lib.rs` | `deck-streak-discipline` | changed: the modules below, and its doc comment no longer names the money rung (ADR-106) |
| `crates/discipline/src/constants.rs` | `deck-streak-discipline` | changed: the weakening stakes, the horizon, the offers and the coin sources join SPEC-105's |
| `crates/discipline/src/state.rs` | `deck-streak-discipline` | changed: the engine switch's writers, the panic's day, the pending panic notice and the last reviewed week |
| `crates/discipline/src/wager.rs` | `deck-streak-discipline` | added: arming's rules, the offers, the settlement and the revision, pure |
| `crates/discipline/src/contract.rs` | `deck-streak-discipline` | added: the offer, authoring's rules, the verdict, the days to judge, the stake in force and the landing day, pure |
| `crates/discipline/src/stakes.rs` | `deck-streak-discipline` | added: the store of wagers, contracts, their days, their changes and the pardons, and the pending messages and celebrations (R17) |
| `crates/discipline/src/panic.rs` | `deck-streak-discipline` | added: the schedule, its cancel, its application and the re-arm |
| `crates/discipline/src/review.rs` | `deck-streak-discipline` | added: the Sunday review's rule and its lines, pure |
| `crates/discipline/src/data_rights.rs` | `deck-streak-discipline` | changed: the five tables join discipline's data-rights port |
| `migrations/010601_discipline_stakes.sql` | `deck-streak-discipline` | added: the five tables, `STRICT`, and the two state fields |
| `crates/discipline/tests/stake_goldens.rs` | `deck-streak-discipline` | added: A1 to A3, A8, A9, A11 to A13, A16 to A18, A21, A22, A26, A30, A32 |
| `crates/discipline/tests/stake_rules.rs` | `deck-streak-discipline` | added: A5, A10, A19, A25 |
| `crates/discipline/tests/stake_rights.rs` | `deck-streak-discipline` | added: A36 |
| `crates/discipline/tests/money_rung_census.rs` | `deck-streak-discipline` | added: A31 |
| `crates/coordination/src/discipline/mod.rs` | `deck-streak-coordination` | changed: the stakes' modules |
| `crates/coordination/src/discipline/stakes.rs` | `deck-streak-coordination` | added: the wager's, the weakenings' and the contract days' step, their inputs, their coins, their revision and their pending messages and celebrations (R9, R17) |
| `crates/coordination/src/discipline/panic.rs` | `deck-streak-coordination` | added: the panic's application across discipline, markets and economy, and the re-arm |
| `crates/coordination/src/discipline/tick.rs` | `deck-streak-coordination` | changed: the stakes' pending messages and celebrations (R17) and the Sunday review |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the stakes' step after the rail's and the windows' steps |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | unchanged: discipline's port is registered by SPEC-105; listed under SPEC-021's six-file rule |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for the five tables |
| `crates/coordination/tests/discipline_stakes.rs` | `deck-streak-coordination` | added: A4, A6, A7, A14, A15, A20 |
| `crates/coordination/tests/discipline_panic.rs` | `deck-streak-coordination` | added: A23, A24 |
| `crates/coordination/tests/discipline_stake_messages.rs` | `deck-streak-coordination` | added: A27 to A29 |
| `crates/coordination/src/nudges/evening.rs` | `deck-streak-coordination` | changed: the active wager's stake passed to the stakes preview (R26) |
| `crates/coordination/tests/evening_wager_line.rs` | `deck-streak-coordination` | added: A37 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the stakes' use cases joined to the bot and the API, and markets' void port to the panic |
| `crates/bot/src/discipline_commands.rs` | `deck-streak-bot` | changed: the five commands and the `wg:` and `ct:` buttons join SPEC-105's |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the five commands join the command table |
| `crates/bot/tests/stake_commands.rs` | `deck-streak-bot` | added: A33 |
| `crates/api/src/discipline_routes.rs` | `deck-streak-api` | changed: the stakes' routes join SPEC-105's |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes, behind the owner's session |
| `crates/api/tests/stake_routes.rs` | `deck-streak-api` | added: A34 |
| `web/app/src/lib/discipline/discipline.ts` | miniapp | changed: the stakes' routes join the client |
| `web/app/src/lib/discipline/WagerCard.svelte` | miniapp | added: the offers and the active wager |
| `web/app/src/lib/discipline/ContractsCard.svelte` | miniapp | added: the builder, the verdict calendar and the weakening |
| `web/app/src/lib/discipline/EngineCard.svelte` | miniapp | added: the switch, the panic, the re-arm and the pardon |
| `web/app/src/lib/discipline/stakes.test.ts` | miniapp | added: A35 |
| `web/app/src/routes/discipline/+page.svelte` | miniapp | changed: the three cards join the discipline screen |
| `tools/parity-oracle/registry/spec_106.py` | repo | added: this SPEC's registrations |
| `tools/parity-oracle/goldens/arm_wager.json` | repo | added: the golden of `DisciplineLayer.arm_wager` (adapter) |
| `tools/parity-oracle/goldens/wager_offers.json` | repo | added: the golden of `CommandBot._process_update` on /wager and `_callback_wager` (adapter) |
| `tools/parity-oracle/goldens/wager_settle.json` | repo | added: the golden of `DisciplineLayer._settle_wagers` (adapter) |
| `tools/parity-oracle/goldens/build_contract.json` | repo | added: the golden of `DisciplineLayer.build_contract` (adapter) |
| `tools/parity-oracle/goldens/contract_offers.json` | repo | added: the golden of `CommandBot._contract_builder_keyboard` and `_callback_contract` (adapter) |
| `tools/parity-oracle/goldens/contract_verdict.json` | repo | added: the golden of `DisciplineLayer._contract_verdict` (adapter) |
| `tools/parity-oracle/goldens/contract_settle.json` | repo | added: the golden of `DisciplineLayer._settle_contracts` (adapter) |
| `tools/parity-oracle/goldens/contract_stake_on.json` | repo | added: the golden of `GamifyStore.stake_override_for_day` (adapter) |
| `tools/parity-oracle/goldens/contract_weakening.json` | repo | added: the golden of `DisciplineLayer.queue_contract_weakening` and `cancel_contract_weakening` (adapter) |
| `tools/parity-oracle/goldens/contract_changes_apply.json` | repo | added: the golden of `DisciplineLayer._apply_contract_changes` (adapter) |
| `tools/parity-oracle/goldens/pardon_breach.json` | repo | added: the golden of `DisciplineLayer.pardon_latest_breach` and `GamifyStore.use_pardon` (adapter) |
| `tools/parity-oracle/goldens/panic_schedule.json` | repo | added: the golden of `DisciplineLayer.panic` and `cancel_panic` (adapter) |
| `tools/parity-oracle/goldens/panic_apply.json` | repo | added: the golden of `DisciplineLayer._apply_panic` (adapter) |
| `tools/parity-oracle/goldens/stake_review.json` | repo | added: the golden of `DisciplineLayer._sunday_stake_review` (adapter) |
| `tools/parity-oracle/goldens/stakes.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S10600-S10699.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains the five tables |
| `privacy.json` | repo | changed: the category `discipline-stakes` |
| `PRIVACY.md` | repo | changed: one line for the category |
| `docs/schematics/wager-and-contract-lifecycle.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-106-wagers-and-contracts-stake-only-coins-settle-on-evidence-and-a-panic-waits-for-the-rollover.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-106.md` | docs | added |
| `changelog.d/feat-discipline-stakes-106.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It posts nothing to an outside pledge service and carries no setting for one: the money rung waits
  for the owner's ruling (#116).
- It prices, places and settles no forecast; the panic calls markets' void port (#119).
- It adds no stakes line to the evening nudge (#117), and none to the daily digest or the weekly
  report (#129, #130).
- It flushes no held celebration when quiet hours end: the router's flush is (#291).
- It changes no sync cadence: every verdict settles at whatever cadence the owner keeps (#164).
- It carries no settings screen for the discipline notices' setting: W7's settings screen does
  (#57).
- It imports none of the predecessor's wager, contract, pardon or panic rows (#61).

## 6. Risks

- **A loss or a breach rests on reviews DeckStreak had not read.** Both are judged again within 7
  closed study days and refunded when the evidence fails (A7, A15).
- **Coins are paid, taken or refunded twice.** Every movement is written in one transaction with the
  status change that allows it once (A4, A6, A14, A20).
- **A panic lands early, late or twice.** It is applied only at the first cycle on its study day,
  after the wager settles, in one transaction (A23), and it is refused while the engine is off
  (A25).
- **A weakening lands before its horizon.** The landing day's rule is proved at its boundary (A16).
- **A breach's message arrives at night, or never.** The tick raises it outside quiet hours and
  keeps it pending after a missing transport (A28).
- **Real money moves without a ruling.** Nothing that posts is built, and a census holds it (A31,
  ADR-106).

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_106.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch milliseconds; no golden holds a calendar date or a personal value.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/arm_wager.json` | `pipeline_layers/discipline.py:DisciplineLayer.arm_wager` | adapter | a stub governor and a temporary store; wallets of 99 and 100 with a stake of 50, stakes of 0 and 1, durations 7, 10, 14 and 30, the governor in standby, and a second arm |
| `goldens/wager_offers.json` | `bot.py:CommandBot._process_update` (`/wager`), `_callback_wager` | adapter | a stub pipeline answering the wallet and a recording sender; wallets of 0, 49, 50, 99, 100, 199 and 200, and a stake's duration step |
| `goldens/wager_settle.json` | `DisciplineLayer._settle_wagers` | adapter | a stub governor, a skip set and freeze markers over a temporary store; 3 and 4 flat spots, a break on the start day and on the effective end, a consumed marker on the end plus 3 and plus 4, the current study day on the effective end and one after, and the governor in standby |
| `goldens/build_contract.json` | `DisciplineLayer.build_contract` | adapter | a stub governor and a temporary store; each metric and an unknown one, a threshold and a stake of 0, weeks 1, 2, 3 and 4, a third contract and an identical one |
| `goldens/contract_offers.json` | `bot.py:CommandBot._contract_builder_keyboard`, `_callback_contract` | adapter | a recording editor; the builder and each metric's stake step |
| `goldens/contract_verdict.json` | `DisciplineLayer._contract_verdict` | adapter | stub inputs; each metric at its threshold, one below and one above, a skip day, a lapse, standby and a consumed freeze on the next day, each alone and together |
| `goldens/contract_settle.json` | `DisciplineLayer._settle_contracts` | adapter | stub verdict inputs and a recording fine; contracts started 10 days back, a day already judged, the 7th and 8th closed study day back, the end day, a finished contract and the engine off |
| `goldens/contract_stake_on.json` | `database.py:GamifyStore.stake_override_for_day` | adapter | a temporary store; no change, one change landing on the day and on the day after, and two changes in sequence |
| `goldens/contract_weakening.json` | `DisciplineLayer.queue_contract_weakening`, `cancel_contract_weakening` | adapter | a temporary store; new stakes of 9, 10, 20 and the current one, a landing day on the end and one after, a second queue, a cancel and a cancel with nothing queued |
| `goldens/contract_changes_apply.json` | `DisciplineLayer._apply_contract_changes` | adapter | a temporary store and a silent notifier; a change due today, one due tomorrow, and one of a revoked contract |
| `goldens/pardon_breach.json` | `DisciplineLayer.pardon_latest_breach`, `GamifyStore.use_pardon` | adapter | a temporary store; no breach, breaches 2 and 3 study days back, two breaches of different days, and a second pardon in one month |
| `goldens/panic_schedule.json` | `DisciplineLayer.panic`, `cancel_panic` | adapter | a temporary store; a panic, a cancel, and a cancel while disabled |
| `goldens/panic_apply.json` | `DisciplineLayer._apply_panic` | adapter | a temporary store and a recording notifier; no panic, a panic for tomorrow and for today, with and without an active wager and contracts |
| `goldens/stake_review.json` | `DisciplineLayer._sunday_stake_review` | adapter | a stub clock, a recording notifier that succeeds or fails; each weekday, a Sunday with no stakes, each stake alone, a failed send and a second Sunday send |
| `goldens/stakes.constants.json` | `constants.py` | constants | `CONTRACT_WEAKEN_STAKES` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `wagers` | `discipline` | `migrations/010601_discipline_stakes.sql` (SPEC-106) | `wagers`, its days as epoch days; a voided row's reason read as `standby`, and its message and celebration read as sent | exported and erased |
| `contracts` | `discipline` | the same migration | `contracts`, its days as epoch days, its horizon dropped and its celebration read as sent | exported and erased |
| `contract_days` | `discipline` | the same migration | `contract_days`, one row per contract and study day, its pending message read as sent | exported and erased |
| `contract_changes` | `discipline` | the same migration | `contract_changes`, its landing day as an epoch day and its message read as sent | exported and erased |
| `pardons` | `discipline` | the same migration | `pardons`, its month keyed by the epoch day of its first day | exported and erased |

The predecessor's `beeminder_posts` maps to nothing: the money rung is not built (ADR-106).

## 9. Mutation rows

The band is `S10600-S10699`, in `scripts/mutation-rows.d/S10600-S10699.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S10601-A-STAKE-IS-HALF-THE-WALLET-AT-MOST` | `crates/discipline/src/wager.rs` | the stake's inclusive cap | `stake_goldens::the_arming_matches_the_parity_golden` |
| `S10602-THREE-EXTENSIONS-AT-MOST` | `crates/discipline/src/wager.rs` | the extensions' cap | `stake_goldens::the_wager_settlement_matches_the_parity_golden` |
| `S10603-A-FOURTH-FLAT-SPOT-PAYS-ONE-AND-A-HALF` | `crates/discipline/src/wager.rs` | the early payout | `stake_goldens::the_wager_settlement_matches_the_parity_golden` |
| `S10604-SURVIVAL-PAYS-DOUBLE` | `crates/discipline/src/wager.rs` | the payout on survival | `stake_goldens::the_wager_settlement_matches_the_parity_golden` |
| `S10605-ONE-ACTIVE-WAGER` | `migrations/010601_discipline_stakes.sql` | the partial unique index on the active wager, a key held in the migration (a script-mutation row with a cargo killer) | `stake_rules::a_second_wager_is_refused` |
| `S10606-A-STUDIED-GAP-VOIDS-A-LOSS` | `crates/discipline/src/wager.rs` | a loss is revised by a late review in its gap | `discipline_stakes::a_lost_wager_is_voided_when_its_gap_proves_studied` |
| `S10607-FLAT-BEFORE-THE-METRIC` | `crates/discipline/src/contract.rs` | a skip, lapse, standby or freeze day never breaches | `stake_goldens::the_contract_verdict_matches_the_parity_golden` |
| `S10608-PINGS-AT-THE-THRESHOLD-ARE-CLEAN` | `crates/discipline/src/contract.rs` | the pings metric's inclusive bound | `stake_goldens::the_contract_verdict_matches_the_parity_golden` |
| `S10609-SEVEN-CLOSED-DAYS-OF-CATCH-UP` | `crates/discipline/src/contract.rs` | the catch-up's first day | `stake_goldens::the_contract_settlement_matches_the_parity_golden` |
| `S10610-THE-STAKE-IN-FORCE-IS-THE-OLD-ONE` | `crates/discipline/src/contract.rs` | a weakening's stake applies only after it lands | `stake_goldens::the_stake_in_force_matches_the_parity_golden` |
| `S10611-A-WEAKENING-LANDS-FOUR-DAYS-ON` | `crates/discipline/src/contract.rs` | the horizon's landing day | `stake_goldens::the_weakening_matches_the_parity_golden` |
| `S10612-A-WEAKENING-ONLY-LOWERS` | `crates/discipline/src/contract.rs` | the reduction-only rule and its floor | `stake_goldens::the_weakening_matches_the_parity_golden` |
| `S10613-TWO-CONTRACTS-AT-MOST` | `crates/discipline/src/contract.rs` | the concurrent contracts' cap | `stake_goldens::the_authoring_matches_the_parity_golden` |
| `S10614-A-PARDON-REACHES-TWO-DAYS-BACK` | `crates/discipline/src/panic.rs` | the pardon's inclusive reach | `stake_goldens::the_pardon_matches_the_parity_golden` |
| `S10615-ONE-PARDON-A-MONTH` | `migrations/010601_discipline_stakes.sql` | the month's key, a key held in the migration (a script-mutation row with a cargo killer) | `stake_rules::a_second_pardon_in_a_month_is_refused` |
| `S10616-THE-PANIC-WAITS-FOR-ITS-DAY` | `crates/discipline/src/panic.rs` | the panic applies on its study day, not before | `discipline_panic::nothing_is_voided_before_the_rollover` |
| `S10617-A-FAILED-BREACH-IS-REFUNDED` | `crates/coordination/src/discipline/stakes.rs` | the breach's revision reverses its fine | `discipline_stakes::a_breach_is_revised_when_its_evidence_fails` |
