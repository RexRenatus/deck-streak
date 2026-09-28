# SPEC-040: every XP grant is written once through one port, and the level is read from the ledger

- **Wave:** W1. **Issue:** #26 (epic #2). **Context(s):** `deck-streak-progression`.
- **Decided by:** ADR-002 (one crate per context), ADR-008 (one SQLite database, owned table by
  table), ADR-012 (the parity oracle proves the math), and ADR-040 (the ledger's key, the once
  scope and the unsigned amount).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-040.md`, and one golden
  (`level_for_xp`) generated at the predecessor's `27ee2bc`. The delivery made R5, R7, R9, R10, A4
  and the manifest exact where the code decided them (§7).

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
| `crates/progression/src/data_rights.rs` | `deck-streak-progression` | added: the data-rights port, named as every context's port is (§7) |
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
| `docs/decisions/ADR-040-xp-ledger-key-once-scope-and-unsigned-amount.md` | docs | changed: accepted |
| `docs/red-first/SPEC-040.md` | docs | added |
| `docs/schematics/xp-grant-port.md` | docs | added: the grant port's components, its write and its read (§7) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `xp_ledger` (§7) |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry gains progression's port (§7) |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: 101 seeded `xp_ledger` rows (§7) |
| `PRIVACY.md` | repo | changed: the XP ledger category's line (§7) |
| `scripts/mutation-rows.d/S04000-S04099.json` | repo | added: the hand-proved rows of the economy maths and the key (§7) |
| `changelog.d/` fragment | repo | added |

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

## 7. Amendments at delivery

- **R9 and the manifest: the register's own section, the registry and the policy.** `xp_ledger` is
  registered to progression in the register of the predecessor's tables, but the two tests that
  hold every migrated table to its owner read only the register of DeckStreak's own tables
  (`crates/kernel/tests/schema.rs`, and SPEC-021's A2), so that section gains its row and
  `docs/CONTEXT-MAP.md` joins the manifest. SPEC-021 R1 keeps every stateful context's port in
  coordination's registry, and its A1 seeds 101 rows into every table of the schema, so the
  registry and the seeds join it too; and SPEC-021's A9 holds `PRIVACY.md` to one line per category
  of `privacy.json`, so the policy names the new category with its basis and its retention.
- **The manifest: the port is `data_rights.rs`.** `privacy.json`'s export and erase code is
  `crates/*/src/data_rights.rs` (SPEC-021 §7, R5), and the privacy-gdpr pack's `export-complete` and
  `erase-complete` rows read only those files, so a port in `rights.rs` would be read by neither.
  A10's test target keeps its name, `rights`.
- **The manifest: a schematic and the rows.** The grant port is a new component with a data flow,
  so `docs/schematics/xp-grant-port.md` draws it before the code. The level's floor, the two keys
  and the refusal of a source are invariants the diff's generated mutants cannot all reach: the keys
  live in the migration and the refusal in a method named `new`. Each has a hand-proved row in this
  SPEC's band (SPEC-039).
- **R5: the existence check is the insert's own conflict.** A grant is one
  `INSERT ... ON CONFLICT DO NOTHING` inside the port's `BEGIN IMMEDIATE` write: its conflict with
  the two unique indexes is the check, so the key lives in the migration alone and no second
  statement of it in code can drift from the index. When the insert writes nothing, the row that
  holds the key is read in the same transaction for the `AlreadyGranted` answer (ADR-040, at
  acceptance).
- **R7 and A4: the golden's cases, and the widths.** Level 1 begins at 0 XP and no unsigned total
  lies one XP below it, so the golden holds the thresholds of levels 1 to 100 and the total one XP
  below those of levels 2 to 100. It also holds eight seeded levels between 2 × 10^7 and 6 × 10^8,
  where a floating-point square root places the total one XP below the threshold a level too high
  (it does for all eight), and the widest total an unsigned 64-bit integer holds, 2^64 − 1, which is
  level 607,400,100. A4 also holds the XP to reach each level to the total at which the
  predecessor's level rises. The amount is an unsigned 32-bit integer, the total an unsigned 64-bit
  one, and the level is computed in 128-bit integers. The predecessor's `max(1, …)` never binds on a
  total that cannot be negative, since the least total, 0 XP, is level 1 already, so the port leaves
  it out.
- **R10: the owner's erase.** The one other statement that changes `xp_ledger` is the data-rights
  port's erase, in progression beside the grant port: it deletes every grant at once when the owner
  erases their data (CHARTER 13), and the grant port itself offers no debit, update or delete (R6).
  A9's census reads each line of every crate's sources and every migration, where a comment is
  prose, and refuses a planted crate and a planted migration, in a temporary tree, by name.
