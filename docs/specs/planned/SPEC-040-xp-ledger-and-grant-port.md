# SPEC-040: every XP grant is written once through one port, and the level is read from the ledger

- **Wave:** W1. **Issue:** #26 (epic #2). **Context(s):** `deck-streak-progression`.
- **Decided by:** ADR-002 (one crate per context), ADR-008 (one SQLite database, owned table by
  table), ADR-012 (the parity oracle proves the math), and ADR-040 (the ledger's key, the once
  scope and the unsigned amount).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-040.md` (ADR-016).

## 1. The problem, measured

- **Nothing can grant XP yet.** `crates/progression/src/lib.rs` holds only its module
  documentation (`ls crates/progression/src` lists one file). The readings grant XP on the read
  tap and on the studied measure (SPEC-047), so the port they grant through must exist first.
- **What is ported.** The predecessor's feature `xp-ledger-and-levels` keeps every grant in one
  ledger keyed by study day and source (`database.py:GamifyStore.upsert_xp_grant`) and maps the
  total to a level (`gamification/xp.py:level_for_xp`). Its once-ever grants needed a second,
  per-feature guard table (`drill_xp_grants`), because a grant replayed on a later study day has a
  different ledger key and would be written again.
- **Two traps a hand port falls into.** The level is an integer square root with floor division;
  a floating-point square root misplaces thresholds once totals grow. And a signed amount lets a
  penalty path debit XP, which the charter forbids (constraint 5: XP is never confiscable).
- **What the parity oracle proves.** `gamification/xp.py:level_for_xp` at every level threshold
  and one XP below it, for levels 1 to 100.
- **Prerequisites.** SPEC-020 (the kernel's study day, track, SQLite base and data-rights port),
  SPEC-021 (the export and erase framework the ledger registers with) and SPEC-029 (the golden
  reader). No other W1 SPEC precedes this one; SPEC-047 builds on it.

## 2. Requirements

R1. The progression context owns the table `xp_ledger` with the columns `id`, `study_day`,
    `source`, `track`, `amount`, `scope` and `created_at`, created by
    `migrations/004001_progression_xp_ledger.sql` (`STRICT`), in the one migration directory the
    kernel embeds and under its naming rule (SPEC-020 R15, R18).
R2. A grant request carries a study day, a source, a track (`language` or `law`), an amount and a
    scope (`per-day` or `once`). The source is an opaque token matching
    `^[a-z0-9][a-z0-9:._-]{0,127}$`; any other source is refused before a write, with an error that
    names the rule and never echoes the value.
R3. With scope `per-day`, the ledger holds at most one row per (study day, source, track): a
    second request for that key writes nothing and answers `AlreadyGranted` carrying the amount of
    the row that exists.
R4. With scope `once`, the ledger holds at most one row per (source, track) across every study
    day: a second request on any study day writes nothing and answers `AlreadyGranted`. Both rules
    are unique indexes in the migration, not checks in code alone.
R5. The existence check and the insert run in one `BEGIN IMMEDIATE` transaction through the
    kernel's repository base, so two concurrent requests for one key write one row.
R6. The amount is an unsigned newtype with no constructor from a signed integer, and the port
    offers no operation that subtracts, debits, updates or deletes a grant. The column carries
    `CHECK (amount >= 0)`.
R7. The total is the sum of `amount`, overall and per track. The level for a total is
    `max(1, (50 + isqrt(2500 + 200 × total)) // 100)` in integer arithmetic, and the XP to reach
    level L is `50L² − 50L` (economy.json `xp.level_curve`: quadratic 50, linear −50), proved by the
    golden of `gamification/xp.py:level_for_xp`.
R8. The level is derived from the total when it is read; no stored level can drift from the
    ledger.
R9. `xp_ledger` is declared in `privacy.json` as user data, exported and erased, and progression's
    data-rights port lists it as exported and erased. It is already registered to progression in the
    context map's ownership register, so the register does not change.
R10. The grant port is the only code that writes `xp_ledger`: no crate but progression names the
    table in a query.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a per-day grant requested twice for one (study day, source, track) writes one row, and the second answer is `AlreadyGranted` with the first amount | `a_grant_requested_twice_writes_one_row` |
| A2 | a once grant requested on two different study days writes one row | `a_once_grant_requested_on_two_study_days_writes_one_row` |
| A3 | two concurrent requests for one key, from two tasks, write one row | `two_concurrent_grants_for_one_key_write_one_row` |
| A4 | the level of every threshold total, and of one XP below it, equals the golden of `gamification/xp.py:level_for_xp` for levels 1 to 100 (examined count reported, zero refused) | `the_level_of_a_total_matches_the_parity_golden` |
| A5 | the level rises when a grant crosses a threshold, read from the ledger's total with no stored level | `the_level_is_derived_from_the_ledger_total` |
| A6 | an amount built from a signed integer does not compile (a compile-fail fixture) | `a_signed_amount_does_not_compile` |
| A7 | a raw insert of a negative amount is refused by the table's check | `the_ledger_refuses_a_negative_amount` |
| A8 | a source outside the token grammar is refused before a write, and the error names the rule | `a_source_outside_the_token_grammar_is_refused` |
| A9 | no crate but progression names `xp_ledger` in a query, and no migration but progression's (the context its file name carries) names it; a planted fixture that does is refused (examined count reported) | `only_the_grant_port_writes_the_xp_ledger` |
| A10 | progression's data-rights port lists `xp_ledger` as exported and erased, and an erase leaves it empty | `the_xp_ledger_is_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-progression --test grant -- --exact a_grant_requested_twice_writes_one_row
A2: cargo test -p deck-streak-progression --test grant -- --exact a_once_grant_requested_on_two_study_days_writes_one_row
A3: cargo test -p deck-streak-progression --test grant -- --exact two_concurrent_grants_for_one_key_write_one_row
A4: cargo test -p deck-streak-progression --test level -- --exact the_level_of_a_total_matches_the_parity_golden
A5: cargo test -p deck-streak-progression --test level -- --exact the_level_is_derived_from_the_ledger_total
A6: cargo test -p deck-streak-progression --test amount_type -- --exact a_signed_amount_does_not_compile
A7: cargo test -p deck-streak-progression --test grant -- --exact the_ledger_refuses_a_negative_amount
A8: cargo test -p deck-streak-progression --test grant -- --exact a_source_outside_the_token_grammar_is_refused
A9: cargo test -p deck-streak-progression --test ledger_census -- --exact only_the_grant_port_writes_the_xp_ledger
A10: cargo test -p deck-streak-progression --test rights -- --exact the_xp_ledger_is_exported_and_erased
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/Cargo.toml` | `deck-streak-progression` | changed: workspace dependencies it uses; `trybuild` as a dev-dependency |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the modules below |
| `crates/progression/src/xp.rs` | `deck-streak-progression` | added: the unsigned amount, the level curve |
| `crates/progression/src/grant.rs` | `deck-streak-progression` | added: the grant port, request, scope and answer |
| `crates/progression/src/ledger.rs` | `deck-streak-progression` | added: the repository over `xp_ledger` |
| `crates/progression/src/rights.rs` | `deck-streak-progression` | added: the data-rights port |
| `migrations/004001_progression_xp_ledger.sql` | `deck-streak-progression` | added |
| `crates/progression/tests/grant.rs` | `deck-streak-progression` | added |
| `crates/progression/tests/level.rs` | `deck-streak-progression` | added |
| `crates/progression/tests/amount_type.rs` | `deck-streak-progression` | added |
| `crates/progression/tests/ui/signed_amount.rs` | `deck-streak-progression` | added: the compile-fail fixture |
| `crates/progression/tests/ui/signed_amount.stderr` | `deck-streak-progression` | added |
| `crates/progression/tests/ledger_census.rs` | `deck-streak-progression` | added |
| `crates/progression/tests/rights.rs` | `deck-streak-progression` | added |
| `tools/parity-oracle/registry/spec_040.py` | repo | added: registers `gamification/xp.py:level_for_xp` with its threshold cases (SPEC-029's registry) |
| `tools/parity-oracle/goldens/level_for_xp.json` | repo | added: generated on the owner's checkout |
| `Cargo.toml` | workspace | changed: `[workspace.dependencies]` gains `trybuild` (ADR-040) |
| `Cargo.lock` | workspace | changed |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `privacy.json` | repo | changed: the XP ledger category |
| `docs/specs/SPEC-040-xp-ledger-and-grant-port.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-040-xp-ledger-key-once-scope-and-unsigned-amount.md` | docs | added |
| `docs/red-first/SPEC-040.md` | docs | added |

The migration lives in the one `migrations/` directory the kernel's `MIGRATOR` embeds, named by
SPEC-020's rule (R15, R18): this SPEC's number in four digits, its sequence in two, the owning
context and a slug.

## 5. What this does NOT do

- It grants no per-review XP, daily bonus or Bloom-tier multiplier, and does not clear and
  re-derive study-owned sources on each recompute (#70, #71).
- It creates no level titles, no level bar and no level-up celebration (#128).
- It keeps no cached total or `xp_state` singleton; a cache, if a measurement asks for one, is
  added with the game core (#70).
- It mints no coins from XP (#106).
- It imports none of the predecessor's ledger rows (#61).

## 6. Risks

- **A caller picks `per-day` where `once` was meant**, so a grant replayed on a later study day
  pays twice. Detected by SPEC-047's grant tests, which assert the reading grants' scope, and by
  A2 here.
- **The compile-fail snapshot moves with the pinned toolchain.** Detected by A6 failing loudly on a
  toolchain bump; the stderr file is regenerated in the same change as the bump.
- **Summing the ledger on every level read grows with the row count.** Detected by the API's
  latency SLO (SPEC-031); a cache is then a measured, separate delivery.
- **The golden is regenerated from a different predecessor commit.** Each golden records the
  predecessor commit it was generated at (SPEC-029), and review compares it with `27ee2bc`. Nothing
  refuses it automatically; `test_goldens.py` checks the digests, not the commit.
