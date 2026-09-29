# SPEC-138: future grants are re-priced per source, from the next study day, inside the grant port

- **Wave:** W7. **Issue:** #281 (per-source re-pricing of future grants) (epic #8).
  **Context(s):** `deck-streak-progression` (the table `xp_price_changes`, the multiplier, the
  price in force on a study day, pricing inside the grant and the settle); `deck-streak-api` (the
  price routes); `deck-streak-coordination` (the table's data-rights registration and its symmetry
  seed); `deck-streak-bot` (a test that no command changes a price); `deck-streak-daemon` (the api
  role hands the price store to the routes); the Mini App (`web/app`, the prices' rows on the
  settings screen).
- **Decided by:** ADR-138 (this SPEC's: a re-price takes effect from the next study day and is
  applied inside the grant port), ADR-072 (a closed day's settled XP never falls), ADR-012 (the
  parity oracle proves the math), ADR-130 (the settings screen writes stored runtime settings only).
- **Prerequisites:** SPEC-024 (the owner's session and the CSRF bound), SPEC-040 (the grant port and
  `xp_ledger`), SPEC-072 (`xp_settlement` and its `settle`), SPEC-075 (the buckets and the exchange
  readout) and SPEC-130 (the settings screen the rows join). SPEC-072, SPEC-075 and SPEC-130 are
  unlanded. **Mutation band:** `S13800-S13899`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-138.md` (ADR-016).

## 1. The problem, measured

- **Revived by the owner.** The predecessor's re-pricing (`exchange.py:apply_multiplier`,
  `exchange.py:clamp_multiplier`, `pipeline_layers/economy.py:EconomyLayer.grant_priced_xp`, at
  `27ee2bc`) had no production caller, and SPEC-075 excluded it as inert in v9. The owner revived
  it as #281 (#267); SPEC-001 §14's amendment records the revival.
- **The predecessor's rule.** A multiplier per bucket, clamped to `[0.1, 5.0]`
  (`exchange.py:MULTIPLIER_MIN`, `exchange.py:MULTIPLIER_MAX`), with `1.0` for any bucket that has
  none (`exchange.py:DEFAULT_MULTIPLIER`) and for a non-finite value. A positive base is priced as
  `max(1, round(base * multiplier))`, and a non-positive base passes unchanged. A grant whose
  `(day, source)` already has a row is refused (`FrozenLedgerRowError`), so a re-price never
  restates a granted row. The buckets are SPEC-075's (`exchange.py:normalize_source`).
- **The predecessor priced by the multiplier current at the call.** A change made in the middle of
  a day would price that day's later grants differently from its earlier ones, and a recompute of
  an open day could change its own price. Binding decision 5 of the W7 plan asks for an effective
  study day instead.
- **Python's `round` is half to even.** `round(2.5)` is 2 and `round(3.5)` is 4, so a Rust port
  that rounds half away from zero differs on every `.5` product (`f64::round_ties_even` is the
  port).
- **No table holds a price.** `xp_ledger` (SPEC-040) and `xp_settlement` (SPEC-072) store amounts;
  nothing on dev stores a multiplier.

## 2. Requirements

The price

R1. `progression::pricing::clamp_multiplier` and `progression::pricing::apply_multiplier` equal the
    goldens `repricing_clamp_multiplier` and `repricing_apply_multiplier`: the full outcome set of
    `exchange.py:clamp_multiplier` (a non-finite value gives 1.0; below 0.1 gives 0.1; above 5.0 gives
    5.0; otherwise the value) and of `exchange.py:apply_multiplier` (a base of 0 gives 0; otherwise
    `max(1, round_ties_even(base * clamp(multiplier)))`). The constants equal `repricing.constants`.
R2. Progression owns `xp_price_changes` (`bucket`, `multiplier`, `effective_study_day`, `created_at`),
    created `STRICT` by `migrations/013801_progression_xp_price_changes.sql`, with
    `UNIQUE (bucket, effective_study_day)` and a `CHECK` that the multiplier lies in `[0.1, 5.0]`.
    Only progression names the table in a query or a migration.
R3. The multiplier in force for a bucket on a study day is the multiplier of that bucket's row with
    the latest `effective_study_day` on or before that day, or 1.0 when there is none. The choice is
    made in Rust over the bucket's rows, never in a query's comparison, so a hand-proved row can
    mutate it (§9).
R4. A price change for a bucket written at an instant takes effect from the study day after the
    instant's study day, by the kernel's rule and its 04:00 rollover. The write is one
    `BEGIN IMMEDIATE` transaction that:
    - writes nothing when the multiplier equals the one in force on the effective day;
    - replaces a pending row (one whose effective day is that same day), since only one change per
      bucket can wait;
    - removes a pending row when the new multiplier equals the one in force on the write's own day;
    - and answers the effective study day.
R5. A multiplier outside `[0.1, 5.0]`, or a bucket that is not a SPEC-075 bucket of a valid grant
    source, is refused (`multiplier_out_of_range`, `bucket_invalid`) and writes nothing.

Pricing inside the port (ADR-138)

R6. `GrantPort::grant` prices the request's amount by the multiplier in force for the source's bucket
    on the request's study day, and writes the priced amount; `GrantAnswer::Granted` carries it. No
    caller prices an amount, so XP still moves through the port alone (SPEC-040 R10).
R7. A request whose key is already granted answers `AlreadyGranted` with the stored amount and writes
    nothing, whatever the price now is: a re-price never restates a granted row (the predecessor's
    refusal, answered as the port's replay).
R8. `settle` prices its amount the same way, by the multiplier in force on the settled study day.
    Because a change takes effect only from the next study day, a day's price is fixed before its
    first grant, so a recompute of a closed day meets the price its first settle met, and ADR-072's
    rule (a closed day's settled XP never falls) holds unchanged.
R9. `grant_priced_xp`'s golden (`repricing_grant_priced_xp`) holds through the port: for a fresh key
    the stored amount equals the golden's priced amount, and for a key already granted the golden's
    refusal is the port's `AlreadyGranted`.

The owner's surface

R10. `GET /api/xp/prices`, behind SPEC-024 R7's `OwnerSession`, answers each bucket in SPEC-075's
     readout with its multiplier in force today, a pending multiplier and its effective study day
     when one waits, and the readout's rate and its defined flag. `PUT /api/xp/prices/{bucket}`
     (JSON, SPEC-024 R9's CSRF bound) writes R4's change and answers the effective study day, or
     R5's refusal with 422.
R11. The settings screen (SPEC-130) gains one row per bucket: its multiplier, the pending one marked
     "from the next study day", and its current rate. The screen renders on the client and stores
     nothing on the device (SPEC-028).
R12. No bot command reads or changes a price.

Rights and rules

R13. `xp_price_changes` owes SPEC-021's six files (§4). An export lists every row; an erase deletes
     them all, which returns every bucket to 1.0 from that instant.
R14. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no faucet
     (a multiplier is capped at 5.0 and applies from the next study day only) and no dishonest copy
     (the screen shows the price in force and the pending one apart).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `apply_multiplier` equals its golden, the `.5` products and a floor at 1 included | `apply_multiplier_matches_the_parity_golden` |
| A2 | `clamp_multiplier` equals its golden: 0.1, 0.09, 5.0, 5.01 and each non-finite value | `clamp_multiplier_matches_the_parity_golden` |
| A3 | a grant is priced by the multiplier in force on its own study day, and 1.0 when none is | `a_grant_is_priced_by_the_multiplier_in_force_on_its_study_day` |
| A4 | a change written at 03:59 and one at 04:00 take effect from the next study day of each, on an injected clock | `a_change_takes_effect_from_the_next_study_day_across_the_rollover` |
| A5 | a granted key answers `AlreadyGranted` with its stored amount after a re-price | `a_granted_key_is_never_repriced` |
| A6 | a closed day's settled XP does not fall when a later change lowers its bucket's multiplier | `a_closed_days_settled_xp_never_falls_after_a_reprice` |
| A7 | a repeated change writes nothing, a second pending change replaces the first, and a change back to the price in force removes the pending one | `a_repeated_price_change_is_idempotent` |
| A8 | a multiplier outside the range, or an invalid bucket, is refused by name and writes nothing | `an_out_of_range_multiplier_or_bucket_is_refused` |
| A9 | the port reproduces `grant_priced_xp`'s golden for fresh and granted keys | `grant_priced_xp_matches_the_parity_golden` |
| A10 | a change prices every source of its bucket and no other | `a_price_applies_to_every_source_of_its_bucket` |
| A11 | only progression names `xp_price_changes` | `only_progression_names_xp_price_changes` |
| A12 | `xp_price_changes` export and erase are symmetric | `xp_price_changes_export_and_erase_are_symmetric` |
| A13 | the price routes answer 401 without the owner's session and 403 across sites | `the_price_routes_are_owner_only` |
| A14 | a price change answers its effective study day, and the list shows it as pending | `a_price_change_answers_its_effective_study_day` |
| A15 | the screen's row shows the multiplier in force, the pending one apart, and the rate | `a price row shows the price in force and the pending one apart` |
| A16 | no bot command names a price | `no_bot_command_changes_a_price` |

```acceptance
A1: cargo test -p deck-streak-progression --test repricing -- --exact apply_multiplier_matches_the_parity_golden
A2: cargo test -p deck-streak-progression --test repricing -- --exact clamp_multiplier_matches_the_parity_golden
A3: cargo test -p deck-streak-progression --test repricing -- --exact a_grant_is_priced_by_the_multiplier_in_force_on_its_study_day
A4: cargo test -p deck-streak-progression --test repricing -- --exact a_change_takes_effect_from_the_next_study_day_across_the_rollover
A5: cargo test -p deck-streak-progression --test repricing -- --exact a_granted_key_is_never_repriced
A6: cargo test -p deck-streak-progression --test repricing -- --exact a_closed_days_settled_xp_never_falls_after_a_reprice
A7: cargo test -p deck-streak-progression --test repricing -- --exact a_repeated_price_change_is_idempotent
A8: cargo test -p deck-streak-progression --test repricing -- --exact an_out_of_range_multiplier_or_bucket_is_refused
A9: cargo test -p deck-streak-progression --test repricing -- --exact grant_priced_xp_matches_the_parity_golden
A10: cargo test -p deck-streak-progression --test repricing -- --exact a_price_applies_to_every_source_of_its_bucket
A11: cargo test -p deck-streak-progression --test ledger_census -- --exact only_progression_names_xp_price_changes
A12: cargo test -p deck-streak-progression --test rights -- --exact xp_price_changes_export_and_erase_are_symmetric
A13: cargo test -p deck-streak-api --test price_routes -- --exact the_price_routes_are_owner_only
A14: cargo test -p deck-streak-api --test price_routes -- --exact a_price_change_answers_its_effective_study_day
A15: pnpm exec vitest run web/app/src/lib/settings/prices.test.ts -t "a price row shows the price in force and the pending one apart"
A16: cargo test -p deck-streak-bot --test commands -- --exact no_bot_command_changes_a_price
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree. They have no
line in the acceptance fence, because no public test can run a pack's row. This delivery changes no
pack's state: game-economy and privacy-gdpr are enforced, and the rows below judge the files this
SPEC adds.

| id | criterion | decided by |
|---|---|---|
| B1 | over `crates/progression/src/pricing.rs` and `migrations/013801_progression_xp_price_changes.sql`: the multiplier's range is bounded on both sides and a price never reaches a granted row | the game-economy pack |
| B2 | over `privacy.json`, `PRIVACY.md` and `crates/progression/src/data_rights.rs`: `xp_price_changes` is declared with purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/src/pricing.rs` | `deck-streak-progression` | added: the clamp, the price, the multiplier in force, the change and its refusals |
| `crates/progression/src/grant.rs` | `deck-streak-progression` | changed: the grant prices its amount (R6, R7) |
| `crates/progression/src/settle.rs` | `deck-streak-progression` | changed (SPEC-072 adds it): the settle prices its amount (R8) |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the module |
| `crates/progression/src/data_rights.rs` | `deck-streak-progression` | changed: `xp_price_changes` exported and erased |
| `crates/progression/tests/repricing.rs` | `deck-streak-progression` | added: A1 to A10 |
| `crates/progression/tests/ledger_census.rs` | `deck-streak-progression` | changed: A11 |
| `crates/progression/tests/rights.rs` | `deck-streak-progression` | changed: A12 |
| `migrations/013801_progression_xp_price_changes.sql` | `deck-streak-progression` | added: the table, its unique key and its range check |
| `crates/api/src/price_routes.rs` | `deck-streak-api` | added: the two routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes joined |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/price_routes.rs` | `deck-streak-api` | added: A13, A14 |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: A16 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role hands progression's price store to the routes |
| `web/app/src/lib/settings/prices.ts` | miniapp | added: the rows' client |
| `web/app/src/lib/settings/PriceRow.svelte` | miniapp | added: one bucket's row |
| `web/app/src/lib/settings/prices.test.ts` | miniapp | added: A15 |
| `web/app/src/routes/settings/+page.svelte` | miniapp | changed: the prices' section |
| `web/app/messages/*.json` | miniapp | changed: the rows' strings, in each locale's catalog |
| `tools/parity-oracle/registry/spec_138.py` | repo | added: the goldens' registrations |
| `tools/parity-oracle/goldens/repricing.constants.json` | repo | added |
| `tools/parity-oracle/goldens/repricing_apply_multiplier.json` | repo | added |
| `tools/parity-oracle/goldens/repricing_clamp_multiplier.json` | repo | added |
| `tools/parity-oracle/goldens/repricing_grant_priced_xp.json` | repo | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables row for `xp_price_changes` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `xp_price_changes` registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row |
| `privacy.json` | repo | changed: the table's purpose, basis and retention |
| `PRIVACY.md` | docs | changed: one line for the category |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-138-future-grants-are-repriced-per-source-from-the-next-study-day.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-138.md` | docs | added |
| `scripts/mutation-rows.d/S13800-S13899.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It re-prices no granted row and no closed day: ADR-072 stands, and the owner's correction of a
  settled amount keeps its own path (#70).
- It adds no bot command for a price (#281).
- It imports no predecessor multiplier; the v9 import maps them (#61).
- It changes no exchange readout, whose rate reads priced XP like any other (#80).
- It prices no coin, chest or token; the economy's prices are their own (#106).

## 6. Risks

- **A price that restates the record.** R7 and R8; detected by A5 and A6.
- **A mid-day change that splits a day's prices.** R4's next study day; detected by A4 and A7.
- **A faucet.** R2's check and R5's refusal cap the multiplier at 5.0; detected by A2 and A8.
- **A rounding drift from the goldens.** R1's ties-to-even; detected by A1.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_138.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `repricing.constants` | `exchange.py:MULTIPLIER_MIN`, `exchange.py:MULTIPLIER_MAX`, `exchange.py:DEFAULT_MULTIPLIER` | constants | none |
| `repricing_clamp_multiplier` | `exchange.py:clamp_multiplier` | function | none; cases 0.09, 0.1, 1.0, 5.0, 5.01, NaN and both infinities |
| `repricing_apply_multiplier` | `exchange.py:apply_multiplier` | function | none; cases base 0, base 1 at 0.1 (the floor), 5 at 0.5 (2.5 gives 2), 3 at 0.5 (1.5 gives 2), 7 at 5.0 and 7 at 9.0 (clamped) |
| `repricing_grant_priced_xp` | `pipeline_layers/economy.py:EconomyLayer.grant_priced_xp` | adapter | a store in memory holding a synthetic multiplier map and, for the refused case, one ledger row |

## 8. Tables and the v9 import

`xp_price_changes` (progression, `migrations/013801_progression_xp_price_changes.sql`). W8's import
maps the predecessor's settings entry `xp_source_multipliers` (`exchange.py:MULTIPLIER_SETTING_KEY`)
into one row per bucket, clamped as `exchange.py:get_multipliers` reads it, effective from the
import's study day.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13801-CLAMP-LOW` | `crates/progression/src/pricing.rs` | below 0.1 gives 0.1; the golden names 0.09 and 0.1 | `repricing::clamp_multiplier_matches_the_parity_golden` |
| `S13802-CLAMP-HIGH` | `crates/progression/src/pricing.rs` | above 5.0 gives 5.0; the golden names 5.0 and 5.01 | `repricing::clamp_multiplier_matches_the_parity_golden` |
| `S13803-NON-FINITE` | `crates/progression/src/pricing.rs` | a non-finite value gives 1.0 | `repricing::clamp_multiplier_matches_the_parity_golden` |
| `S13804-FLOOR-ONE` | `crates/progression/src/pricing.rs` | a positive base prices to at least 1 | `repricing::apply_multiplier_matches_the_parity_golden` |
| `S13805-TIES-EVEN` | `crates/progression/src/pricing.rs` | half to even; the golden names 2.5 and 1.5 | `repricing::apply_multiplier_matches_the_parity_golden` |
| `S13806-NEXT-STUDY-DAY` | `crates/progression/src/pricing.rs` | the effective day is the next study day | `repricing::a_change_takes_effect_from_the_next_study_day_across_the_rollover` |
| `S13807-ON-OR-BEFORE` | `crates/progression/src/pricing.rs` | a change dated on the grant's own study day is in force; the test grants on that day and the day before | `repricing::a_grant_is_priced_by_the_multiplier_in_force_on_its_study_day` |
| `S13808-NO-RESTATE` | `crates/progression/src/grant.rs` | a granted key is answered, never priced again | `repricing::a_granted_key_is_never_repriced` |
| `S13809-SETTLE-PRICED` | `crates/progression/src/settle.rs` | the settle prices by the settled day's multiplier | `repricing::a_closed_days_settled_xp_never_falls_after_a_reprice` |
| `S13810-IDEMPOTENT` | `crates/progression/src/pricing.rs` | an unchanged multiplier writes nothing | `repricing::a_repeated_price_change_is_idempotent` |
| `S13811-RANGE-CHECK` | `migrations/013801_progression_xp_price_changes.sql` | the range check (a script-mutation row) | `repricing::an_out_of_range_multiplier_or_bucket_is_refused` |
| `S13812-OWNER-ONLY` | `crates/api/src/price_routes.rs` | the routes take `OwnerSession` | `price_routes::the_price_routes_are_owner_only` |
