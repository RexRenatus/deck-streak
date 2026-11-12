# SPEC-108: a smoke bomb cancels one day's pending coin fines, and a chest lock makes the day's first chest Common

- **Wave:** W5. **Issues:** #279 (the smoke bomb's spend) and the chest lock's effect on a chest,
  which SPEC-104 leaves to this SPEC for #110, in epic #6. **Context(s):** `deck-streak-quests` (the
  locked first chest, the take of a smoke bomb, `smoke_bomb_spends`); `deck-streak-coordination`
  (the chest lock's port over discipline's, the spend's use case, the Perfect Week's text);
  `deck-streak-economy` and `deck-streak-discipline` (their ports join a caller's transaction);
  `deck-streak-api` and `deck-streak-bot` (the offer, the confirm and the spend); the Mini App (the
  shop's offer).
- **Decided by:** ADR-104 (a fine stays revisable for the current study day and the 7 closed study
  days before it, and the smoke bomb reads that closed set), ADR-103 (a reversal refunds the
  recorded amount once), ADR-081 (two draws per chest, stored with its pity in one write), ADR-071
  (each study day is settled once, in order), ADR-012 (the parity oracle) and ADR-016 (a planned
  SPEC is promoted by its delivery).
- **Prerequisites:** SPEC-081 (the chest grant, the Perfect Week's smoke bomb and `inventory`, the
  shop's token card and smoke bombs held), SPEC-082 (the wallet, the shop's answer and screen, and a
  cross-context write coordination holds), SPEC-103 (`reverse_fine`, the reason `smoke_bomb` and
  `standing_fines`), SPEC-104 (the chest lock's port and the revision window), SPEC-071 (the fold),
  SPEC-026 (the bot's callbacks and the owner gate), SPEC-028 (the Mini App shell), SPEC-021 (the
  six files of a table), SPEC-020 (migrations), SPEC-029 (the goldens) and SPEC-038 (its section 8
  ruling (i), under which SPEC-081 is amended). **Mutation band:** `S10800-S10899`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-108.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` af0693f, `crates/quests/src/` holds only `lib.rs`. SPEC-081 (planned)
  earns the Perfect Week's smoke bomb into `inventory`, at most 2 held, and shows the count, and its
  R15 makes every text say a smoke bomb has no use yet, because the predecessor promises a spend it
  never built; its section 5 leaves the spend to #268 and the chest lock to #110. The owner's
  decision at #268 revived the spend, which #279 specifies. SPEC-103 (planned) keeps the penalty
  ledger, reverses a fine once with the reason `smoke_bomb` (the coin source `smoke_bomb_refund`)
  and lists a study day's standing fines. SPEC-104 (planned) sets a chest lock on the study day
  after a rung-1 fine, lifts it by ransom, and leaves its effect on a chest to this SPEC (its R14
  and section 5).
- **What is ported** (at `27ee2bc`):
  - the chest lock's effect, the locked branch of
    `pipeline_layers/loot.py:LootLayer._grant_session_chests` (the ransom is SPEC-104's,
    `LootLayer._chest_lock_active`);
  - the first sentence of the Perfect Week's text, `LootLayer._maybe_earn_smoke_bomb`;
  - no spend: no code of the predecessor spends a smoke bomb (#279), so the spend is new. It moves
    coins only through SPEC-103's reversal, whose goldens (`database.py:GamifyStore.reverse_penalty`
    and `penalties_total_for_day`) prove the once-only refund and a day's standing total.
- **Measured in the predecessor: the locked branch's outcome set.** `_chest_lock_active` is read
  once a study day, after the return on a declared skip day and the return on a day with no
  eligible session, and before the first session. The loop then breaks at the day's cap (the
  default 3) and skips a session that has a chest of its key or one within a session gap, before
  the lock applies. The first session that reaches the roll takes its rarity draw; under the lock
  it is stored `common`, its payout is drawn as a Common's with its second draw, and the pity
  counters advance as after a Common (both gain one). The lock is then cleared, and every later
  chest of the day rolls as it would unlocked. For a chest the outcome is `locked Common` or
  `rolled`; for the lock it is `consumed` (a chest reached the roll), or `kept` (a skip day, no
  eligible session, the cap reached first, or every session skipped).
- **Traps a hand port falls into.**
  - The rarity draw is taken even when the lock discards it: SPEC-081 R5's two draws a chest stand,
    so a lock that skips the draw moves every later payout onto the wrong draw.
  - The pity follows the STORED rarity: a lock over the chest that would be the guaranteed Epic
    (the 14th since the last Epic) stores a Common, and the Epic counter keeps growing.
  - A cap, a skip day or a skipped session consumes nothing: the lock bites the first chest the
    day actually grants.
  - A day's pending fines are not a stored state. A fine the loss cap forgave in full records 0
    (SPEC-103 R4) and took no coin, so it is never offered.
  - A spend recorded without its reversals takes a bomb for nothing, and reversals without the
    record let a retry take a second bomb: the record, the bomb and the reversals are one write.
- **Deviations.**
  - The lock is consumed in the chest's own write, not before it. The predecessor clears it before
    its insert, so an insert that finds the session's chest already written consumes the lock with
    no Common chest; here the lock, the chest and its pity land together or not at all (ADR-081).
  - The Perfect Week's text names what a spend does. The predecessor's second sentence, "One tap
    cancels a whole night's pending penalties", promises penalties (a lock, a surcharge, a rung) and
    one tap; the spend cancels coin fines only, behind one confirm (#279).
  - A night, #279's word, is a study day (the LEXICON's), so it turns over at the rollover.
  - The predecessor has no window. A fine is pending here while ADR-104 may still revise it: on the
    current study day or one of the 7 closed study days before it. An older fine is final, and
    ADR-104 names the smoke bomb among the readers of that closed set.
  - SPEC-081's R15 and A14 give way to R9 and A13, by an insert-only amendment at this delivery
    (R10).

## 2. Requirements

The chest lock's effect (#110)

R1. The chest step (SPEC-081 R2) passes quests' grant a chest-lock port, which coordination
    implements over discipline's lock port (SPEC-104 R14), so quests names no discipline type. The
    grant asks it once for a study day whether a lock stands: after the skip-day and
    eligible-session checks and before the day's first session. On a declared skip day, or a day
    with no eligible session, it does not ask.
R2. While the lock stands, the first chest the grant rolls that day takes both of its draws as
    SPEC-081 R5 takes them and is stored Common: its payout is a Common's payout of its payout draw
    (SPEC-081 R4), and the pity counters advance as after a Common (SPEC-081 R6). Every later chest
    of the day, and every chest while no lock stands, rolls unchanged. A session the grant skips (a
    chest of its key, or one within a session gap) and a day at its cap consume nothing. What the
    grant writes, and whether it asks and consumes the lock, equal
    `goldens/session_chests_locked.json`.
R3. The lock is consumed through the port in the same transaction as the chest row and the pity it
    writes (SPEC-081 R5): a write that fails leaves the lock standing and no chest, and a failed
    draw consumes nothing.

The spend (#279)

R4. A night is a study day. Its pending fines are the fines the penalty ledger records on that
    study day (SPEC-103 R1) that are not reversed and whose debited amount is above 0, read through
    SPEC-103 R8's `standing_fines`. A night is on offer while it lies in the revision window of
    SPEC-104 R20 (the current study day and the 7 closed study days before it), read from the one
    declaration in `crates/discipline/src/revision.rs`, and holds a pending fine.
R5. `smoke_bomb_spends` holds one row per night spent: the night, which is its key, the instant, the
    number of fines reversed, the coins refunded and `created_at`. It is `STRICT`, created by
    `migrations/010801_quests_smoke_bomb_spends.sql` (SPEC-020 R15 and R18), and written by quests'
    repository.
R6. `spend_smoke_bomb(night)` is a coordination use case in
    `crates/coordination/src/chests/smoke_bomb.rs`. It runs in one `BEGIN IMMEDIATE` transaction it
    holds through the kernel's repository base, checks in this order, and answers one outcome:
    - `already spent`: the night has a row; it answers that row and changes nothing;
    - `outside the window`: the night lies before or after the revision window (R4); refused;
    - `no smoke bomb`: `inventory` holds none; refused;
    - `no pending fine`: the night holds none (R4); refused;
    - `spent`: it takes one smoke bomb from `inventory`, reverses each pending fine through
      `reverse_fine(reference, current study day, smoke_bomb)` (SPEC-103 R5), and writes the
      night's row with the count and the coins those reversals refunded; it answers each fine
      reversed with its coins, the coins refunded and the smoke bombs left.
    A refusal changes nothing.
R7. A spend reverses only what was pending when it ran: a fine levied on the night after its spend
    stands, and a second spend of the night answers `already spent`. A reversed fine is never levied
    again (SPEC-103 R7), so settling the night again, or revising it (SPEC-104 R20), books nothing
    and refunds nothing twice.
R8. SPEC-103's `reverse_fine` and `standing_fines`, SPEC-104's consumption of a lock and quests'
    take of a smoke bomb each join a transaction their caller holds, as SPEC-082 R10's freeze and
    its coin movement do.

The copy (#279)

R9. The Perfect Week's celebration (SPEC-081 R15's event and key) reads the predecessor's first
    sentence and a new second one, joined by one space:
    `💨 <b>Perfect Week</b> — a Smoke Bomb joins your inventory.`
    `Spend it in the shop to cancel one day's pending coin fines.`
    Every screen and message that shows smoke bombs (SPEC-081 R19 and R20) says that a smoke bomb,
    spent in the shop, cancels the pending coin fines of one day among the last 8 study days. No
    text says that it cancels a penalty, a lock, a surcharge or a rung, that it lasts no longer, or
    that one tap spends it.
R10. SPEC-081 is amended insert-only (SPEC-038 section 8, ruling (i)): `~~` around A14 in its
    criteria table, A14's fence line set apart in a `retired` fence, a line after R15 saying that
    R9 of this SPEC supersedes its "no use yet", and a dated amendment section naming each
    insertion. The test of A14 gives way to A13's.

Surfaces

R11. The API serves `GET /api/smoke-bombs` (the smoke bombs held, and each night on offer, newest
    first, with its pending fines' count and coins) and `POST /api/smoke-bombs/spend` (a night as an
    epoch day; it answers R6's outcome), in `crates/api/src/chests_routes.rs`. Each answers the
    owner's session only.
R12. The bot's shop answer (SPEC-082, with SPEC-081 R19's lines) gains, while a smoke bomb is held,
    one line per night on offer, newest first, with its date, its pending fines' count and coins and
    a button whose data is `sh:bomb:<epoch day>`. The tap answers one confirm naming the night, the
    fines and the coins, with a button whose data is `sh:bombgo:<epoch day>`. The confirm spends,
    and its answer names each fine cancelled, the coins refunded and the smoke bombs left, or the
    refusal's reason. Both callbacks are behind the owner gate (SPEC-026).
R13. The Mini App's shop screen (SPEC-082, with SPEC-081 R20's smoke bombs held) gains the same
    offer: each night on offer with its count and coins and a Spend button, one confirm, and an
    answer naming the fines cancelled and the coins refunded. Its strings live in the message
    catalogs.

Rights

R14. `smoke_bomb_spends` is declared in quests' data-rights port as exported and erased, under
    SPEC-081's inventory category in `privacy.json` and `PRIVACY.md`, with its own-tables row in the
    context map and a seeded row in coordination's symmetry test (SPEC-021).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the locked grant's chests, pity, question and consumption equal the golden of `_grant_session_chests` with the lock standing, for every case (examined count reported, zero refused) | `the_locked_grant_matches_the_predecessors_golden` |
| A2 | the grant asks the lock port at most once a study day, and never on a declared skip day or a day with no eligible session | `the_lock_port_is_asked_once_a_day_at_most` |
| A3 | a chest write that fails under a standing lock leaves the lock standing, no chest and the pity unchanged, and a failed draw consumes nothing | `a_failed_chest_write_leaves_the_lock_standing` |
| A4 | a lock set by a rung-1 fine makes the next study day's first chest Common at the recompute and is consumed, and a ransomed lock leaves the chest as rolled | `the_chest_step_consumes_the_lock_through_discipline` |
| A5 | with two fines on one night and one on another, a spend on the first reverses both once with the reason `smoke_bomb` on the current study day, leaves the other night's fine standing, takes one of two smoke bombs and answers both fines and their coins | `a_spend_reverses_the_nights_fines_and_takes_one_bomb` |
| A6 | a second spend of a night answers `already spent`, reverses nothing and takes no bomb, and a fine levied on the night after the spend stays standing | `a_second_spend_of_a_night_changes_nothing` |
| A7 | a spend with no smoke bomb is refused `no smoke bomb` before a night with no pending fine is refused `no pending fine`, and a night whose only fine was forgiven in full, or whose fines are all reversed, is refused; each leaves the fines and the bombs unchanged | `a_spend_without_a_bomb_or_a_pending_fine_changes_nothing` |
| A8 | a spend on the 7th closed study day before the current one is spent, and one on the 8th or on the next study day is refused `outside the window` | `a_spend_outside_the_revision_window_is_refused` |
| A9 | after a spend, the fine port requested again with each reversed reference, and the rail's settlement and revision (SPEC-104 R10, R20) run again over the night, book nothing and refund nothing | `a_spent_night_settled_again_levies_nothing` |
| A10 | a reversal that fails inside a spend leaves no row, the bomb held and every fine standing | `the_spend_and_its_reversals_are_written_together` |
| A11 | two spends of one night run at once reverse each fine once and take one bomb | `concurrent_spends_of_a_night_take_one_bomb` |
| A12 | the Perfect Week's celebration reads R9's two sentences | `the_perfect_week_celebration_names_the_spend` |
| A13 | every smoke-bomb string in the message catalogs and every smoke-bomb text in coordination and the bot names coin fines and one day, and none names a penalty, a lock, a surcharge, a rung, a lasting effect or one tap (examined count reported) | `test_the_smoke_bomb_copy_says_what_the_spend_does` |
| A14 | the two routes answer the owner's session only; the GET lists the bombs held and the nights on offer newest first with their counts and coins, and the POST answers the spend's outcome | `the_smoke_bomb_routes_answer_only_the_owner` |
| A15 | the shop's answer offers each night on offer with its date, count, coins and `sh:bomb:` button while a bomb is held, the tap answers one confirm with `sh:bombgo:`, and the confirm's answer names each fine cancelled, the coins refunded and the bombs left; no bomb or no night shows no button | `the_shop_offers_the_spend_behind_one_confirm` |
| A16 | the shop screen offers each night on offer behind one confirm and names the fines cancelled and the coins refunded | `offers each night on the shop behind one confirm and names the fines cancelled` |
| A17 | quests' data-rights port exports `smoke_bomb_spends`, and after an erase it is empty | `the_spends_are_exported_and_erased` |
| A18 | a second row for a night is refused by the table's key and leaves the first | `a_night_holds_one_spend` |

```acceptance
A1: cargo test -p deck-streak-quests --test chests_locked -- --exact the_locked_grant_matches_the_predecessors_golden
A2: cargo test -p deck-streak-quests --test chests_locked -- --exact the_lock_port_is_asked_once_a_day_at_most
A3: cargo test -p deck-streak-quests --test chests_locked -- --exact a_failed_chest_write_leaves_the_lock_standing
A4: cargo test -p deck-streak-coordination --test chests_lock_step -- --exact the_chest_step_consumes_the_lock_through_discipline
A5: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact a_spend_reverses_the_nights_fines_and_takes_one_bomb
A6: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact a_second_spend_of_a_night_changes_nothing
A7: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact a_spend_without_a_bomb_or_a_pending_fine_changes_nothing
A8: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact a_spend_outside_the_revision_window_is_refused
A9: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact a_spent_night_settled_again_levies_nothing
A10: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact the_spend_and_its_reversals_are_written_together
A11: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact concurrent_spends_of_a_night_take_one_bomb
A12: cargo test -p deck-streak-coordination --test smoke_bomb_spend -- --exact the_perfect_week_celebration_names_the_spend
A13: python3 -m unittest discover -s scripts/tests -p test_smoke_bomb_copy.py -k test_the_smoke_bomb_copy_says_what_the_spend_does
A14: cargo test -p deck-streak-api --test smoke_bomb_routes -- --exact the_smoke_bomb_routes_answer_only_the_owner
A15: cargo test -p deck-streak-bot --test smoke_bomb_shop -- --exact the_shop_offers_the_spend_behind_one_confirm
A16: pnpm exec vitest run web/app/src/lib/chests/smoke-bomb.test.ts -t "offers each night on the shop behind one confirm and names the fines cancelled"
A17: cargo test -p deck-streak-quests --test smoke_bomb_spends -- --exact the_spends_are_exported_and_erased
A18: cargo test -p deck-streak-quests --test smoke_bomb_spends -- --exact a_night_holds_one_spend
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, ux-laws and accessibility packs stay
enforced, and no row is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | `smoke_bomb_spends` is declared under a category with its data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010801_quests_smoke_bomb_spends.sql` and `crates/quests/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the smoke-bomb copy carries no false urgency and no confirmshaming, and promises nothing the spend does not do, over the smoke-bomb strings in `web/app/messages/*.json`, every file under `web/app/src/lib/chests/` and `crates/coordination/src/chests/`, and `crates/bot/src/shop_commands.rs` | the ux-laws pack |
| B3 | the shop screen with the smoke-bomb offer and its confirm, in both colour schemes, meets WCAG 2.2 AA, over `web/app/src/routes/shop/` and `web/app/src/lib/chests/SmokeBombOffer.svelte` | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/quests/src/chests.rs` | `deck-streak-quests` | changed: the grant takes the chest-lock port, asks it once and stores the day's first chest Common while a lock stands (R1, R2) |
| `crates/quests/src/chest_store.rs` | `deck-streak-quests` | changed: the chest's write consumes the lock in its transaction (R3) |
| `crates/quests/src/inventory.rs` | `deck-streak-quests` | changed: the take of one smoke bomb inside a caller's transaction (R6, R8) |
| `crates/quests/src/smoke_bombs.rs` | `deck-streak-quests` | added: the repository over `smoke_bomb_spends` (R5) |
| `crates/quests/src/lib.rs` | `deck-streak-quests` | changed: the module above |
| `crates/quests/src/data_rights.rs` | `deck-streak-quests` | changed: `smoke_bomb_spends`, exported and erased |
| `migrations/010801_quests_smoke_bomb_spends.sql` | `deck-streak-quests` | added: the table, `STRICT`, keyed on the night |
| `crates/quests/tests/chests_locked.rs` | `deck-streak-quests` | added: A1 to A3 |
| `crates/quests/tests/smoke_bomb_spends.rs` | `deck-streak-quests` | added: A17, A18 |
| `crates/economy/src/fines.rs` | `deck-streak-economy` | changed: `reverse_fine` and `standing_fines` join a caller's transaction (R8) |
| `crates/discipline/src/lock.rs` | `deck-streak-discipline` | changed: the lock's consumption joins a caller's transaction (R3, R8) |
| `crates/discipline/src/revision.rs` | `deck-streak-discipline` | changed: the revision window answered by one public function, which the spend reads (R4) |
| `crates/coordination/src/recompute/chests.rs` | `deck-streak-coordination` | changed: the chest-lock port over discipline's, passed to the grant (R1, R3) |
| `crates/coordination/src/chests/smoke_bomb.rs` | `deck-streak-coordination` | added: the nights on offer, the spend and its outcomes (R4, R6, R7) |
| `crates/coordination/src/chests/mod.rs` | `deck-streak-coordination` | changed: the module above |
| `crates/coordination/src/chests/messages.rs` | `deck-streak-coordination` | changed: the Perfect Week's text (R9) |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `smoke_bomb_spends` joins quests' entry |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `smoke_bomb_spends` |
| `crates/coordination/tests/chests_lock_step.rs` | `deck-streak-coordination` | added: A4 |
| `crates/coordination/tests/smoke_bomb_spend.rs` | `deck-streak-coordination` | added: A5 to A12 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the spend's use case joined to the API and the bot |
| `crates/api/src/chests_routes.rs` | `deck-streak-api` | changed: the two smoke-bomb routes (R11) |
| `crates/api/tests/smoke_bomb_routes.rs` | `deck-streak-api` | added: A14 |
| `crates/bot/src/shop_commands.rs` | `deck-streak-bot` | changed: the offer, the confirm and the spend's answer (R12) |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the callback prefixes `sh:bomb:` and `sh:bombgo:` registered |
| `crates/bot/tests/smoke_bomb_shop.rs` | `deck-streak-bot` | added: A15 |
| `web/app/src/routes/shop/+page.svelte` | miniapp | changed: the smoke-bomb offer (R13) |
| `web/app/src/lib/chests/SmokeBombOffer.svelte` | miniapp | added: the nights on offer, Spend and its confirm |
| `web/app/src/lib/chests/smoke-bomb.ts` | miniapp | added: the two API calls |
| `web/app/src/lib/chests/smoke-bomb.test.ts` | miniapp | added: A16 |
| `web/app/messages/*.json` | miniapp | changed: the smoke-bomb strings, in each locale's catalog (R9, R13) |
| `scripts/tests/test_smoke_bomb_copy.py` | repo | added: A13 |
| `scripts/tests/test_chests_copy.py` | repo | changed: the test of SPEC-081's retired A14 removed (R10) |
| `tools/parity-oracle/registry/spec_108.py` | repo | added: this SPEC's registration (SPEC-029's registry) |
| `tools/parity-oracle/goldens/session_chests_locked.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._grant_session_chests` with the lock standing (§7) |
| `scripts/mutation-rows.d/S10800-S10899.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `smoke_bomb_spends` |
| `privacy.json` | repo | changed: `smoke_bomb_spends` joins the inventory category |
| `PRIVACY.md` | repo | changed: the inventory category's line names the spends |
| `docs/specs/SPEC-081-session-chests-are-rolled-once-with-pity-and-tokens-and-smoke-bombs-keep-their-word.md` | docs | changed: the insert-only amendment of R10 |
| `docs/schematics/smoke-bomb-spend-and-the-chest-lock.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `docs/specs/SPEC-108-a-smoke-bomb-cancels-one-days-pending-coin-fines-and-a-chest-lock-makes-the-days-first-chest-common.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-108.md` | docs | added |
| `changelog.d/feat-quests-smoke-bomb-108.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It lifts no chest lock, surcharge or rung: a spend cancels coin fines only, and a reversed fine's
  lock and surcharge stand, as SPEC-104 R20 rules for a revised one (#110).
- It sets, ransoms or announces no chest lock: the doomscroll rail's first rung does (#110).
- It changes no fine's rule and no revision window: each fine's rules stay with its feature
  (#110, #111, #113).
- It spends no smoke bomb by itself: only the owner's confirmed spend does, never a job or a
  recompute (#279).
- It sells no smoke bomb and earns none in a new way: the Perfect Week earns them, and no
  randomized reward is sold (#172).
- It imports nothing: the predecessor spent no smoke bomb, and its inventory is SPEC-081's import
  (#61).

## 6. Risks

- **A lock bites twice.** It is consumed in the chest's own write (R3, A3), so a retried recompute
  finds both the chest and the consumed lock.
- **A spend takes a bomb for nothing.** The row, the bomb and the reversals are one write (A10), and
  a refusal changes nothing (A7).
- **A fine is refunded twice.** SPEC-103's reversal refunds once, the night's key holds one spend
  (A18), and two spends at once take one bomb (A11).
- **A spend and a revision race.** Both reverse through SPEC-103's port inside `BEGIN IMMEDIATE`
  writes, so whichever runs first refunds the fine and the other refunds nothing.
- **The copy drifts back into a promise.** Detected by A13 and by the ux-laws pack (B2).

## 7. Parity goldens

The golden is registered in `tools/parity-oracle/registry/spec_108.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch milliseconds; no golden holds a calendar date.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/session_chests_locked.json` | `pipeline_layers/loot.py:LootLayer._grant_session_chests` | adapter | SPEC-081's stand-in layer for `session_chests_granted` (a stub store holding the day's chests, settings, pity, buffs and skip day, the recompute's clock and quiet state, and `random.SystemRandom` patched to return the case's listed draws), with `_chest_lock_active` patched to a recorder that answers the case's lock; returns the chests inserted, the pity after, whether the lock was asked and whether it was cleared. Cases: `lock_first` (two eligible sessions whose draws both roll a Rare: the first stored Common, the second a Rare), `lock_epic_guarantee` (the Epic counter at 13), `lock_legendary` (the first draw rolls a Legendary), `lock_common` (the first draw rolls a Common), `lock_near` (the first session within a session gap of an existing chest), `lock_cap` (the day already at its cap of 3), `lock_skip` (a declared skip day), `lock_none` (no eligible session) and `unlocked` (the recorder answers no lock) |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `smoke_bomb_spends` | `quests` | `migrations/010801_quests_smoke_bomb_spends.sql` (SPEC-108) | none: the predecessor spent no smoke bomb, so there is nothing to import | exported and erased |

## 9. Mutation rows

The band is `S10800-S10899`, in `scripts/mutation-rows.d/S10800-S10899.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S10801-A-LOCKED-CHEST-IS-COMMON` | `crates/quests/src/chests.rs` | a standing lock stores the day's first chest Common (`lock_first`) | `chests_locked::the_locked_grant_matches_the_predecessors_golden` |
| `S10802-A-LOCK-BITES-ONE-CHEST` | `crates/quests/src/chests.rs` | only the first chest is locked (`lock_first`'s second chest stays a Rare) | `chests_locked::the_locked_grant_matches_the_predecessors_golden` |
| `S10803-LOCKED-PITY-ADVANCES-AS-COMMON` | `crates/quests/src/chests.rs` | the pity follows the stored rarity (`lock_epic_guarantee`, `lock_legendary`) | `chests_locked::the_locked_grant_matches_the_predecessors_golden` |
| `S10804-A-LOCKED-CHEST-TAKES-BOTH-DRAWS` | `crates/quests/src/chests.rs` | the payout comes from the second draw (`lock_first`) | `chests_locked::the_locked_grant_matches_the_predecessors_golden` |
| `S10805-A-SKIPPED-SESSION-KEEPS-THE-LOCK` | `crates/quests/src/chests.rs` | the lock applies after the key and gap skips (`lock_near`) and not past the cap (`lock_cap`) | `chests_locked::the_locked_grant_matches_the_predecessors_golden` |
| `S10806-THE-LOCK-IS-ASKED-ONCE` | `crates/quests/src/chests.rs` | one question a study day, none on a skip day | `chests_locked::the_lock_port_is_asked_once_a_day_at_most` |
| `S10807-THE-LOCK-GOES-WITH-THE-CHEST` | `crates/quests/src/chest_store.rs` | the consumption rides the chest's transaction | `chests_locked::a_failed_chest_write_leaves_the_lock_standing` |
| `S10808-ONE-SPEND-PER-NIGHT` | `migrations/010801_quests_smoke_bomb_spends.sql` | the key on the night, a key held in the migration (a script-mutation row with a cargo killer) | `smoke_bomb_spends::a_night_holds_one_spend` |
| `S10809-A-SPEND-TAKES-ONE-BOMB` | `crates/quests/src/inventory.rs` | a spend takes exactly one smoke bomb | `smoke_bomb_spend::a_spend_reverses_the_nights_fines_and_takes_one_bomb` |
| `S10810-A-SPEND-KEEPS-TO-ITS-NIGHT` | `crates/coordination/src/chests/smoke_bomb.rs` | only the night's fines are reversed | `smoke_bomb_spend::a_spend_reverses_the_nights_fines_and_takes_one_bomb` |
| `S10811-A-FORGIVEN-FINE-IS-NOT-PENDING` | `crates/coordination/src/chests/smoke_bomb.rs` | a fine of 0 coins is not pending | `smoke_bomb_spend::a_spend_without_a_bomb_or_a_pending_fine_changes_nothing` |
| `S10812-NO-BOMB-NO-SPEND` | `crates/coordination/src/chests/smoke_bomb.rs` | the refusal without a smoke bomb | `smoke_bomb_spend::a_spend_without_a_bomb_or_a_pending_fine_changes_nothing` |
| `S10813-THE-WINDOW-HOLDS-SEVEN-CLOSED-DAYS` | `crates/coordination/src/chests/smoke_bomb.rs` | the 7th closed study day is inside the window and the 8th outside | `smoke_bomb_spend::a_spend_outside_the_revision_window_is_refused` |
| `S10814-A-REPEAT-SPENDS-NOTHING` | `crates/coordination/src/chests/smoke_bomb.rs` | a spent night answers its row first | `smoke_bomb_spend::a_second_spend_of_a_night_changes_nothing` |
