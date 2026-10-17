# SPEC-360: the per-review XP rule is an I/O-free crate both clients can link, held to the server's XP by the parity oracle

- **Wave:** the app campaign, the first part of stretch row 2.2 (SPEC-334 section 7, R13).
  **Issue:** #635. **Context(s):** `deck-streak-xp` (new), `deck-streak-progression`.
- **Decided by:** ADR-371, ADR-012 (the parity oracle, goldens from the predecessor), ADR-040
  (the unsigned amount), ADR-029 (the golden reader), ADR-357 D1 (a client crate joins the umbrella
  FFI crate by an edge drawn in the change that first exposes it).
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-360.md`.

## 1. The problem, measured

Every figure below was read at `dev` `8122dbf180b6837961eb0289cf6baca398e15f40` (DEV) with
`git show` of the path at DEV, or `git grep -n` of the pattern at DEV over the path.

| # | measured | figure | command |
|---|---|---|---|
| M1 | where a review's XP is computed | one function, `review_xp(review: &Review, tier: Option<Tier>) -> u32` at `crates/progression/src/review_xp.rs:20`, a 42-line file | `git grep -n 'fn review_xp' DEV -- crates` |
| M2 | what that function imports | ingest's `Review` and `is_study_event` (`:7`), ingest's `Tier` (`:8`), progression's `economy_config::xp` (`:10`); the function itself reads no file, socket or clock | `git show DEV:crates/progression/src/review_xp.rs \| grep -n '^use'` |
| M3 | what a client links to call it today | progression's `[dependencies]` are the kernel, ingest, `serde_json`, `sqlx` and `thiserror` (`crates/progression/Cargo.toml:14-22`); the kernel's include `sqlx` (`crates/kernel/Cargo.toml:19`) and `tokio` (`:21`); ingest's add the engine, `sqlx` and `tokio` | `git show DEV:crates/progression/Cargo.toml \| sed -n '14,22p'`, and the same for `crates/kernel/Cargo.toml` and `crates/ingest/Cargo.toml` |
| M4 | which members have no kernel edge | 5 of the 30 member manifests: `engine-core`, `ffi`, `fsrs7`, `web-engine` and the kernel itself; every server context depends on the kernel | for each `crates/*/Cargo.toml`, `tomllib` over `[dependencies]` and every `[target.*.dependencies]`, testing for `deck-streak-kernel` |
| M5 | the server's callers | `crates/coordination/src/recompute/xp.rs:110` (the day's review XP, its tier chosen at `:80` for a law card only) and `crates/coordination/src/progression/law_tiers.rs:83` (the law-tier readout); the API reaches XP only through coordination (`crates/api/src/xp_routes.rs:27-28`) | `git grep -n 'review_xp(' DEV -- 'crates/*/src/*'` |
| M6 | the test callers | `crates/coordination/tests/xp_steps.rs:181,188,217,218`, `crates/coordination/tests/law_tiers.rs:184`, `crates/daemon/tests/law_tiers.rs:270` and `crates/progression/tests/xp_review.rs:38` | `git grep -n 'review_xp(' DEV -- 'crates/*/tests/*'` |
| M7 | where the constants live | `economy.json`'s `xp` section, lines 4 to 47: the base (`:5`), the ease (`:7-12`), maturity (`:13-18`) and type (`:19-24`) multipliers, and the tier table (`:38-47`); progression parses them once into `XpEconomy` behind one `static` (`crates/progression/src/economy_config.rs:73-76`) | `git show DEV:economy.json \| sed -n '4,47p'`; `git show DEV:crates/progression/src/economy_config.rs \| sed -n '73,76p'` |
| M8 | who reads the nine per-review fields | `base`, `ease`, `mature_interval_days`, `mature`, `young`, `fresh`, `types`, `tier` and `untagged` are read in source only by `review_xp.rs:26-41`, and in tests only by `crates/progression/tests/xp_constants.rs:44-51` | `git grep -n -E 'economy\.(base\|ease\|mature_interval_days\|mature\|young\|fresh\|types\|tier\|untagged)\b' DEV -- crates` |
| M9 | what the parity oracle holds for this rule | `review_xp.json`: 265 cases (240 combinations, 3 ties, 5 boundaries, 5 not study events, 12 drawn); `study_event.json`: 64 cases; `progression.constants.json`: 24 names. Rust tests read them through `tools/parity-oracle/golden.rs`, included by `#[path]` | `git show DEV:tools/parity-oracle/goldens/review_xp.json \| python3 -c 'import json,sys,collections; d=json.load(sys.stdin); print(len(d["cases"]), collections.Counter(c.get("class") for c in d["cases"]))'`, and the same for the other two |
| M10 | how the oracle runs in CI | the Rust golden tests run in the gate's test stage, `cargo nextest run --workspace --locked --no-fail-fast` (`scripts/check.sh:90-93`); `test_goldens.py` runs in its python stage (`:179-185`); public CI never regenerates a golden (`tools/parity-oracle/README.md:166-167`) | `git show DEV:scripts/check.sh \| sed -n '90,93p;179,185p'` |
| M11 | the rows on code that moves | `S07201-REVIEW-XP-ROUNDING` and `S07202-MATURE-BOUNDARY` on progression's `src/review_xp.rs`, killed by `xp_review::review_xp_matches_the_parity_golden_for_every_combination` (`scripts/mutation-rows.d/S07200-S07299.json:4-21`). `S07203`, `S07218` and `S07226` to `S07228` sit on coordination's callers and `S07215` and `S04001` on levels; none of them moves | `git grep -n -e review_xp -e 'src/level.rs' -e 'src/xp.rs' DEV -- scripts/mutation-rows.d` |
| M12 | the statics census | `STATICS` in `crates/coordination/tests/relight_order.rs:1295` writes out 20 statics, progression's parsed economy among them (`:1372-1375`), and holds them equal to every `static` of every crate coordination links (`:1459-1482`) | `git show DEV:crates/coordination/tests/relight_order.rs \| sed -n '1295p;1372,1375p'` |
| M13 | who links XP on a client today | nobody: the umbrella FFI crate depends on the engine core alone and the web engine on the engine core for `wasm32` alone (`docs/CONTEXT-MAP.md:38,42`) | `git show DEV:docs/CONTEXT-MAP.md \| sed -n '37,43p'` |

**What the parity oracle will prove.** The new crate's `review_xp` equals `review_xp.json` for all
265 cases and its study-event rule equals `study_event.json` for all 64; progression's
`review_xp`, now a translation over the crate, still equals `review_xp.json` for all 265 through
the test that holds it today; and the 24 constants still equal the predecessor's.

**SPEC-334 R13, split by its owning issue.** This delivery (#635) takes "the crate computes the
predecessor's math, held by the parity oracle". Per-review XP shown on the device at once,
day-level bonuses pending until sync, and the reconciliation that confirmed XP is never below the
XP shown are #639's. Economy v10's flat multipliers are #62's and #64's. "One ledger on the server
confirms" is SPEC-072's ledger, which this delivery does not touch.

## 2. Requirements

R1. A new workspace crate, `deck-streak-xp` at `crates/xp`, holds the per-review XP rule:
    `review_xp(review: &ReviewFacts, tier: Option<Tier>) -> u32` is the product of the base and the
    ease, maturity, type and tier multipliers, taken left to right in 64-bit floating point and
    rounded half to even, and 0 for a row that is not a study event (SPEC-072 R1, R4). It equals
    `tools/parity-oracle/goldens/review_xp.json` for every case. No XP figure, constant, golden or
    table changes.
R2. The crate owns its inputs and names no other crate's type: `ReviewFacts` holds a review's
    `ease`, its new `interval` in days and its review type `kind`, each an `i64` as the collection
    stores it, and `Tier` is `T1`, `T2`, `T3` or `T4`.
R3. The crate holds its own study-event rule, `is_study_event(kind, ease)`: a review type from 0 to
    3 with an ease of 1 or more. It equals `tools/parity-oracle/goldens/study_event.json` for every
    case, as ingest's own copy does.
R4. The crate reads every constant of R1 from `economy.json`'s `xp` section, embedded at build
    time and parsed once into one table behind one `static`; no such number is typed in its source
    (SPEC-072 R2). Progression no longer declares or parses the nine per-review constants; the
    constants of SPEC-072 R11, R15, R16 and R21 stay in progression.
R5. The crate does no I/O. Its `[dependencies]` are `serde_json` alone, with the
    `float_roundtrip` feature, so `economy.json`'s floats parse to the same bits in every graph that
    links it; its `[dev-dependencies]` are at most `serde` and `serde_json`; it has no build
    script, `[features]`, `[target]` table or `[build-dependencies]`. Its locked dependency closure
    lies within `serde_json`, `serde`, `serde_core`, `serde_derive`, `proc-macro2`, `quote`, `syn`,
    `unicode-ident`, `itoa`, `memchr` and `zmij`. Its source names no filesystem, network, clock,
    environment, process or thread API, and embeds `economy.json` and nothing else.
R6. The crate sits beside the kernel in the map's bottom layer: its fence line says
    `depends on: nothing`, progression's line gains `xp`, and at this delivery progression is the
    only member that depends on it. The context map's fence and its layer prose, and
    `docs/schematics/context-map.md`, say so in the same change.
R7. Progression's `review_xp(review: &Review, tier: Option<Tier>) -> u32` keeps its path and
    signature and becomes the translation at progression's edge: it builds `ReviewFacts` from the
    review's ease, new interval and type, maps ingest's tier by an exhaustive `match`, and calls
    the crate. No copy of the rule stays in progression. Coordination's two callers do not change
    and gain no edge.
R8. The lexicon's `xp` lock covers `deck-streak-xp`, and the glossary names `review facts` in the
    xp context.
R9. The rows that guarded the moved rule, `S07201-REVIEW-XP-ROUNDING` and
    `S07202-MATURE-BOUNDARY`, are re-anchored, never retired: crate `xp`, file `src/review_xp.rs`,
    killer the crate's own golden test, with their ids, `find` and `replace` unchanged. New hand
    rows guard the translation's tier map and interval, the study-event type range that sits inside
    `matches!`, and the census's refusals on the real manifest, source and map (section 7).
R10. The statics census writes out the crate's one `static`, so it holds 21.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the crate's `review_xp` equals the predecessor's for all 265 cases of `review_xp.json` | `cargo test -p deck-streak-xp --test review_xp -- --exact the_xp_crates_review_xp_matches_the_parity_golden_for_every_case` |
| A2 | the crate's study-event rule equals the predecessor's for all 64 cases of `study_event.json` | `cargo test -p deck-streak-xp --test study_event -- --exact the_xp_crates_study_event_rule_matches_the_predecessors_golden` |
| A3 | the crate's manifest takes `serde_json` alone, with `float_roundtrip`, and declares no build script, feature, target table or build dependency; each planted refusal is refused by name | `python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_xp_crate_takes_serde_json_alone_with_exact_floats` |
| A4 | the crate's locked dependency closure lies within the allow-list, and a planted kernel, `sqlx` or `tokio` entry is refused by name | `python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_xp_crates_locked_closure_is_the_allow_list` |
| A5 | the crate's source embeds `economy.json` once and names no I/O, clock or environment API; each planted token is refused by name | `python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_xp_crates_source_embeds_economy_json_once_and_names_no_io` |
| A6 | the map's fence and the manifests agree that the crate depends on nothing and that progression alone depends on it; a planted extra dependent or a missing map edge is refused | `python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_map_and_the_manifests_agree_that_progression_alone_depends_on_the_xp_crate` |
| A7 | the rule's rounding lives in the crate's `src/review_xp.rs` once, and progression's `src/review_xp.rs` and `src/economy_config.rs` hold no copy of the rule or of its nine constants | `python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_rule_lives_in_the_xp_crate_alone` |
| A8 | the server's XP is unchanged: progression's `review_xp`, through the translation, equals `review_xp.json` for all 265 cases | `cargo test -p deck-streak-progression --test xp_review -- --exact review_xp_matches_the_parity_golden_for_every_combination` |
| A9 | every one of the 24 constants still equals the predecessor's and `economy.json`, the eight per-review ones read from the crate's table | `cargo test -p deck-streak-progression --test xp_constants -- --exact the_progression_constants_equal_the_predecessors_and_economy_json` |
| A10 | the server's two callers price reviews as before | `cargo test -p deck-streak-coordination --test xp_steps -- --exact an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate`; `cargo test -p deck-streak-coordination --test law_tiers -- --exact the_law_cards_are_counted_by_tier_and_today_s_reviews_are_priced_by_their_card` |
| A11 | the statics census writes out the crate's `static` | `cargo test -p deck-streak-coordination --test relight_order -- --exact the_route_keeps_no_per_day_failure_state_a_give_up_could_read` |
| A12 | an ease past four multiplies by one: for each study type 0 to 3, an answer of ease 5 with a new interval and no tier earns the base times the new, type and untagged multipliers of the crate's table, rounded half to even, as the predecessor's `_ => 1.0` arm does | `cargo test -p deck-streak-xp --test review_xp -- --exact an_ease_past_four_multiplies_by_one_for_every_study_type` |

```acceptance
A1: cargo test -p deck-streak-xp --test review_xp -- --exact the_xp_crates_review_xp_matches_the_parity_golden_for_every_case
A2: cargo test -p deck-streak-xp --test study_event -- --exact the_xp_crates_study_event_rule_matches_the_predecessors_golden
A3: python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_xp_crate_takes_serde_json_alone_with_exact_floats
A4: python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_xp_crates_locked_closure_is_the_allow_list
A5: python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_xp_crates_source_embeds_economy_json_once_and_names_no_io
A6: python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_map_and_the_manifests_agree_that_progression_alone_depends_on_the_xp_crate
A7: python3 -m unittest discover -s scripts/tests -p test_xp_crate_graph.py -k test_the_rule_lives_in_the_xp_crate_alone
A8: cargo test -p deck-streak-progression --test xp_review -- --exact review_xp_matches_the_parity_golden_for_every_combination
A9: cargo test -p deck-streak-progression --test xp_constants -- --exact the_progression_constants_equal_the_predecessors_and_economy_json
A10: cargo test -p deck-streak-coordination --test xp_steps -- --exact an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate
A10: cargo test -p deck-streak-coordination --test law_tiers -- --exact the_law_cards_are_counted_by_tier_and_today_s_reviews_are_priced_by_their_card
A11: cargo test -p deck-streak-coordination --test relight_order -- --exact the_route_keeps_no_per_day_failure_state_a_give_up_could_read
A12: cargo test -p deck-streak-xp --test review_xp -- --exact an_ease_past_four_multiplies_by_one_for_every_study_type
```

**The red each criterion shows first.** The census (A3 to A7) is committed alone, before the
crate exists; the crate's shape next (it compiles, `review_xp` returns 0 and `is_study_event`
returns false); then the crate's two golden tests alone.

| id | red first, at which commit, for which reason |
|---|---|
| A1 | at the tests' commit, over the shape: the first combination's assertion, `the XP of {"ease":1,"ivl":0,"rtype":0,"tier":null}`, left 0, right 4 |
| A2 | at the tests' commit, over the shape: the first study event's assertion, left `false`, right `true` |
| A3 | at the census's commit: `deck-streak-xp has no manifest` |
| A4 | at the census's commit: `Cargo.lock holds no deck-streak-xp package` |
| A5 | at the census's commit: `crates/xp/src embeds economy.json 0 times, not once`; the positive artifact is asserted before the examined count, so the red is the criterion's own |
| A6 | at the census's commit: `the map's fence holds no line for deck-streak-xp` |
| A7 | at the census's commit: `crates/xp/src/review_xp.rs rounds half to even 0 times, not once` |
| A8 | not red: it pins the server's XP, which the base already has; it guards the move |
| A9 | not red: it holds the 24 constants the base already holds, eight of them re-read from the crate's table; it guards the move |
| A10 | not red: it pins the two callers' pricing, which the base already has; it guards the move |
| A11 | red at the switch's commit, before its `STATICS` line: the census finds `crates/xp/src/table.rs`'s `static` and does not have it written |
| A12 | at the tests' commit, over the shape: the first study type's assertion, ease 5, interval 0, type 0 and no tier, left 0, right 8 |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/xp/Cargo.toml` | `deck-streak-xp` | added |
| `crates/xp/src/lib.rs` | `deck-streak-xp` | added: the crate's documentation and its public names |
| `crates/xp/src/review_xp.rs` | `deck-streak-xp` | added: `ReviewFacts`, `Tier`, `is_study_event` and `review_xp`, the body moved from progression |
| `crates/xp/src/table.rs` | `deck-streak-xp` | added: `ReviewXpTable`, `table()`, its one `static` and the parse of `economy.json`'s per-review keys |
| `crates/xp/tests/review_xp.rs` | `deck-streak-xp` | added (A1, A12) |
| `crates/xp/tests/study_event.rs` | `deck-streak-xp` | added (A2) |
| `Cargo.toml` | workspace | changed: `deck-streak-xp = { path = "crates/xp" }` in `[workspace.dependencies]` |
| `Cargo.lock` | workspace | changed: the new member, and progression's edge to it |
| `crates/progression/Cargo.toml` | `deck-streak-progression` | changed: `deck-streak-xp.workspace = true` |
| `crates/progression/src/review_xp.rs` | `deck-streak-progression` | changed: the translation over the crate |
| `crates/progression/src/economy_config.rs` | `deck-streak-progression` | changed: the nine per-review fields and their parse removed |
| `crates/progression/tests/xp_constants.rs` | `deck-streak-progression` | changed: the eight per-review names read from the crate's table (A9) |
| `crates/coordination/tests/relight_order.rs` | `deck-streak-coordination` | changed: `STATICS` holds 21, and its comment names the table (A11) |
| `scripts/tests/test_xp_crate_graph.py` | scripts | added (A3 to A7) |
| `scripts/mutation-rows.d/S07200-S07299.json` | scripts | changed: `S07201` and `S07202` re-anchored |
| `scripts/mutation-rows.d/S36000-S36099.json` | scripts | added: this delivery's rows (section 7) |
| `docs/CONTEXT-MAP.md` | docs | changed: the crate's fence line, progression's line, the layer prose |
| `docs/LEXICON.md` | docs | changed: the `xp` lock's scope and the `review facts` row |
| `docs/schematics/context-map.md` | docs | changed: the crate and its edge |
| `docs/schematics/the-io-free-xp-crate.md` | docs | added |
| `docs/specs/SPEC-360-the-per-review-xp-rule-is-an-io-free-crate.md` | docs | added |
| `docs/specs/SPEC-072-xp-is-earned-per-review-and-per-day-settled-once-per-source-with-levels-and-titles.md` | docs | changed: ONE insert-only section 17 naming the moved rule and the new killer of `S07201` and `S07202` (ruling 448 (1)); no line above it changes |
| `docs/decisions/ADR-371-the-xp-crate-sits-beside-the-kernel-and-owns-its-inputs.md` | docs | added |
| `docs/red-first/SPEC-360.md` | docs | added |
| `changelog.d/feat-xp-crate-635.md` | docs | added |

## 5. What this does NOT do

- It links the crate into no client: the umbrella FFI crate's and the web engine's edges to it,
  their wrapper types, and per-review XP shown on the device at once are #639's.
- It builds no reconciliation of shown and confirmed XP, and no Lean entry stating that the XP the
  server confirms is never below the XP a client showed; both belong to the delivery of stretch
  row 2.2's second part (#639).
- It moves no day-level rule: the daily bonuses, consistency, Ascendant, the level curve, levels
  and titles stay in progression, and whether a client computes any of them is decided with
  instant XP (#639).
- It moves no tier parsing: a note's Bloom tier is still read by ingest, and how a client learns a
  card's tier is decided with instant XP (#639).
- It compiles the crate for no `wasm32` or Apple target in CI; the first client edge brings that
  target's existing job over it (#639).
- It changes no XP figure, constant or table, and adopts none of economy v10's flat multipliers,
  which wait on side-by-side verification and the v1.0.0 release (#62, #64).
- It draws no other crate in `docs/schematics/context-map.md`, which lacks several workspace
  members today; that refresh is #689.

## 6. Risks

- **A client's graph parses a multiplier one unit in the last place away.** serde_json's default
  float parser is best-effort, and on the server ingest's dependency turns `float_roundtrip` on for
  the whole graph; a client graph has no ingest. The crate turns the feature on itself (R5), A3
  refuses a manifest without it, and a row plants its removal.
- **A serde_json upgrade adds a package to the closure.** A4 refuses it by name, and the upgrade's
  own change reviews the allow-list.
- **The moved body drifts from the predecessor's.** A1 holds it over 265 cases in its own crate,
  A8 holds the server's path over the same cases, and `S07201` and `S07202` keep guarding it.
- **The translation maps a tier or the interval wrongly.** A8 fails, and two hand rows plant each
  mistake.
- **A dependent appears without the map's edge.** A6 refuses any member naming the crate that the
  fence does not list, in CI rather than only in the ddd probe.
- **The embedded `economy.json` weighs on a client's binary.** The file is embedded whole; the
  web engine's size budget measures it when the crate is first linked (#639).
- **The two copies of the study-event rule drift.** Each is held to the same 64 cases (A2, and
  ingest's `scope::the_study_event_rule_matches_the_predecessors_golden`).

## 7. Mutation rows

`S360` is this delivery's row stem, `S` and the SPEC's number.

| row | crate | file | what it mutates | killer |
|---|---|---|---|---|
| `S07201-REVIEW-XP-ROUNDING` (re-anchored) | `xp` | `src/review_xp.rs` | `.round_ties_even() as u32` to `.round() as u32` | `review_xp::the_xp_crates_review_xp_matches_the_parity_golden_for_every_case` |
| `S07202-MATURE-BOUNDARY` (re-anchored) | `xp` | `src/review_xp.rs` | `review.interval >= economy.mature_interval_days` to `>` | `review_xp::the_xp_crates_review_xp_matches_the_parity_golden_for_every_case` |
| `S36001-STUDY-EVENT-TYPES` | `xp` | `src/review_xp.rs` | the type range inside `matches!`, `0..=3` to `0..=4`, which cargo-mutants does not mutate | `study_event::the_xp_crates_study_event_rule_matches_the_predecessors_golden` |
| `S36002-THE-T4-TIER-MAPS-TO-T4` | `progression` | `src/review_xp.rs` | the translation's `T4` arm to the crate's `T3` | `xp_review::review_xp_matches_the_parity_golden_for_every_combination` |
| `S36003-THE-NEW-INTERVAL-IS-READ` | `progression` | `src/review_xp.rs` | `interval: review.interval,` to `interval: review.last_interval,` | `xp_review::review_xp_matches_the_parity_golden_for_every_combination` |
| `S36004-NO-KERNEL-EDGE` (script) | | `crates/xp/Cargo.toml` | plants `deck-streak-kernel.workspace = true` under `[dependencies]` | `test_xp_crate_graph.TheXpCrateDoesNoIo.test_the_xp_crate_takes_serde_json_alone_with_exact_floats` |
| `S36005-EXACT-FLOATS` (script) | | `crates/xp/Cargo.toml` | drops `float_roundtrip` from the `serde_json` line | `test_xp_crate_graph.TheXpCrateDoesNoIo.test_the_xp_crate_takes_serde_json_alone_with_exact_floats` |
| `S36006-NO-FILESYSTEM` (script) | | `crates/xp/src/table.rs` | plants `use std::fs;` above the `LazyLock` import | `test_xp_crate_graph.TheXpCrateDoesNoIo.test_the_xp_crates_source_embeds_economy_json_once_and_names_no_io` |
| `S36007-THE-MAP-DRAWS-THE-EDGE` (script) | | `docs/CONTEXT-MAP.md` | drops `, xp` from progression's fence line | `test_xp_crate_graph.TheXpCrateDoesNoIo.test_the_map_and_the_manifests_agree_that_progression_alone_depends_on_the_xp_crate` |

The rows on coordination's callers (`S07203`, `S07218`, `S07226` to `S07228`) and on levels
(`S07215`, `S04001`) do not move.
