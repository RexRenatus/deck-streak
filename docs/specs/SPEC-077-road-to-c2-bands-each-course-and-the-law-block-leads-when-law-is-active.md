# SPEC-077: Road to C2 bands each course, and the law block leads whenever there is law activity

- **Wave:** W3. **Issue:** #85, #134 (epic #4). **Context(s):** `deck-streak-curriculum` (card
  mastery, course progress and bands, the band-up rule, the law mastery pillar, `language_progress`,
  `band_milestones`, `law_dues`); `deck-streak-ingest` (each card's memory state, parsed from its
  `cards.data`); `deck-streak-coordination` (the recompute's progress step, the band-up's grant,
  badge and celebration, the law block's view); `deck-streak-daemon` (the persona engine's live
  band); `deck-streak-api`, `deck-streak-bot` and the Mini App (the progress screen and the law tab).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-041 (every celebration passes
  through the one router), ADR-044 (the persona engine reads the live band through its port), ADR-071
  (the recompute reaches every study day once, in order) and ADR-087 (the owner's courses, their unit
  bands included, are private configuration).
- **Prerequisites:** SPEC-040 (the grant port and the level curve), SPEC-041 (the router), SPEC-044
  (the agent's `LiveBand` port), SPEC-071 (the fold, the courses configuration and the card
  snapshot), SPEC-072 (the XP settlement and the law tab's tier distribution), SPEC-073 (the badge
  award port), SPEC-076 (the law streak) and SPEC-084 (the band-up's T5). **Mutation band:**
  `S07700-S07799`.
- **Status:** delivered by CU85a in part (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-077.md`, ADR-016): the Road to C2 of #85 and the law block of #134 in two pull
  requests. CU85a delivers the memory state, the progress rules, the store, the progress step, the
  band-up's grant, badge and celebration, and the law block's view; CU85b delivers the composition
  root's `LiveBand`, the routes, the bot's command and the Mini App's screens (section 3c).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` c3d769b `crates/curriculum/src/` holds only `lib.rs`, and ingest's
  reader selects each card's scheduling columns but not its memory state
  (`crates/ingest/src/reader.rs`, the `CARDS` statement). No course progress, band, band-up or law
  block exists, and SPEC-044's persona engine falls back to the roster's band because no live band
  is wired (its §5: "the Road to C2 wires it (#85)").
- **What is ported.** The predecessor's `road-to-c2` (#85): `progress.py:card_mastery`,
  `compute_progress`, `parse_unit` and `band_for_unit`; `fsrs.py:parse_fsrs`, `retrievability` and
  `clamped_decay_exponent`; and `pipeline.py:GamifyPipeline._persist_progress` and
  `_celebrate_band_up`. Its `law-track-summary` (#134):
  `pipeline_layers/digests.py:DigestsLayer.law_track_summary` and `law_total_xp`,
  `scoring.py:law_mastery_pillar`, `telegram.py:render_law_block`, and the law dues of
  `pipeline.py:GamifyPipeline._run_sync_cycle_impl`.
- **The courses are configuration.** The predecessor hard-codes its language decks, their codes and
  flags, and each course's unit-to-band table (`curriculum.LANGUAGE_DECKS`, `CEFR_UNIT_BANDS`), all of
  them the owner's. Here a course's code, name, flag, deck root and unit bands come from the private
  courses configuration (ADR-087), and ingest names each card's course.
- **Traps a hand port falls into.**
  - `progress.py` imports `LANGUAGE_DECKS` and `CEFR_UNIT_BANDS` into its own namespace, so a golden's
    adapter patches `progress`'s names with synthetic courses, never `curriculum`'s.
  - Mastery zeroes a suspended card only (queue −1): a buried card keeps its mastery.
  - The memory-state path runs only when the stability is finite and positive; a missing or
    non-positive decay is the default, and the forgetting curve clamps the decay's magnitude before it
    divides by it.
  - The current band defaults to A1 even when A1 is not achieved, and it is the last band of the run
    of achieved bands from A1, so a later achieved band after a gap does not count.
  - Mastery reads the clock (retrievability decays), so a course's progress is recomputed at each
    recompute and can fall with no review at all.
  - Only the band reached is celebrated: a band skipped on the way, or a band reached again after a
    drop, is not.
  - The law block's text names the owner's law deck root, so its golden returns which fields the block
    shows, never its text.
- **Corrections to the issues.**
  - #85 says "suspended = 0"; the predecessor's test is `queue == -1`, so a buried card is not zeroed.
  - #134 says the block is empty "when there is no law activity". The predecessor's rule is exact: the
    block is empty when the law streak, today's law XP, the active law leeches and lifetime law XP
    are all zero; the dues alone never show it, and inside it the dues line appears only above zero.
  - #134 depends on #133 for the active law leeches, which is W4's. Here the leech count and the law
    mastery pillar read W4's leech port, and render as pending until it exists.
- **What the parity oracle proves.** The memory-state parse over valid, missing, malformed and
  non-finite text; mastery on both paths, at each clamp and threshold; the unit parse; whole-course
  progress over synthetic courses with gaps, drops and skipped bands; the band-up's baselines and
  celebrations; the law mastery pillar; the law block's fields; and the block's shown rule.
- **Prerequisites.** SPEC-040, SPEC-041, SPEC-044, SPEC-071, SPEC-072, SPEC-073, SPEC-076 and SPEC-084,
  as the header names them.

## 2. Requirements

Road to C2 (#85)

R1. Ingest reads each card's `cards.data` and publishes its memory state as numbers (stability,
    difficulty, decay, desired retention and the last review's time in epoch seconds), equal to
    `goldens/memory_state.json` (`fsrs.py:parse_fsrs`): missing, unparsable, non-object text, or a
    stability that is missing, not finite or not positive, gives no state; a decay that is missing or
    not positive is 0.2 (`fsrs.DEFAULT_DECAY`). The text itself is never kept.
R2. A card's mastery, from 0 to 1, is 0 for a suspended card (queue −1); with a memory state it is
    `min(1, ln(1 + S) / ln(1 + 100)) × R`, where R is the predecessor's retrievability at the
    recompute's clock, from the last review's time, with the decay exponent's magnitude clamped to
    0.001..10; with none it is 1 for a review card whose interval is at least 21 days and 0 otherwise.
    It equals `goldens/card_mastery.json` (`progress.py:card_mastery`; the stability target of 100
    days, the mature interval of 21 days, the default decay and both clamps in
    `goldens/curriculum.constants.json`).
R3. A card counts toward a course when ingest names its course (ADR-087), its home deck's full name
    holds a unit number (`goldens/unit_parse.json`, `progress.py:parse_unit`), and the course's
    configured unit bands hold that unit; any other card is not counted.
R4. Per course and per band, in the order A1, A2, B1, B2, C1, C2 (`curriculum.CEFR_BANDS`), progress
    counts the cards, sums their mastery and counts the mature ones (mastery of at least 0.5). A band is
    achieved when it holds cards and their mean mastery is at least 80%; the current band is the last
    band of the run of achieved bands that starts at A1, and A1 when there is none; the course's
    mastery is the mean over its counted cards; its current unit is the highest unit holding a card of
    mastery at least 0.5; the courses are ordered by name. It equals `goldens/course_progress.json`
    (`progress.py:compute_progress`; `curriculum.CEFR_BAND_ACHIEVED_PCT` and
    `MATURE_MASTERY_THRESHOLD`).
R5. The unit bands are the courses configuration's (ADR-087), which the kernel loads and checks once
    at start (SPEC-071 R1: no band overlapping, running backwards or out of order); curriculum reads
    them from `Courses` and never loads the file. A unit that falls in no configured band leaves its
    card uncounted (R3). The private deploy rail generates the table from the owner's syllabi and
    checks it against them (#41).
R6. Progress registers its step in phase 4 of SPEC-071's fold and runs it for the current study day
    only, since it reads the current card state and the recompute's clock, never a closed day's; the
    band badge's award registers in phase 7. The predecessor computes progress at each cycle before
    its day's recompute (`pipeline.py:GamifyPipeline._run_sync_cycle_impl`), so its band-up XP is in
    the day's base before the derived bonuses, as phase 4 places it. Each course is stored in
    `language_progress` with its name, flag, mastery, current band, mature and total counts, current
    unit and its bands as JSON (band, total, mature, mastery percent, achieved).
R7. After a recompute, a course never seen before records its current band in `band_milestones` as a
    silent baseline, with no XP, badge or celebration. A course whose current band comes later in the
    order than its stored one records that band once (the key on course and band lives in the
    migration) and then, through coordination: 500 XP through SPEC-040's `grant` with scope once,
    source `bandup:<code>:<band>`, track language (`constants.XP_BONUS_BAND_UP`); the band badge
    `band_<code>_<band>`, named by the course's name and the band, with the course's flag, through
    SPEC-073's award port; and one budget-exempt T5 celebration, event `band_up`, dedupe key
    `bandup:<code>:<band>`, through the router. It equals `goldens/band_up.json`
    (`pipeline.py:GamifyPipeline._persist_progress` and `_celebrate_band_up`).
R8. The persona engine's live band (SPEC-044 R8, the agent's `LiveBand` port) is implemented in the
    composition root, behind a newtype, over the stored current band of the course a language subject
    names; a subject that names no course answers no band, so the roster's band applies.

The law block (#134)

R9. The law mastery pillar is `clamp(100 − min(30, 3 × active law leeches))`
    (`goldens/law_mastery_pillar.json`, `scoring.py:law_mastery_pillar`; `constants.MASTERY_LEECH_PENALTY`
    and `MASTERY_LEECH_PENALTY_CAP` in `goldens/curriculum.constants.json`).
R10. The law dues are the backlog plus the cards due today over the law track's cards, from SPEC-071's
    card snapshot at the current study day's collection day number (proved by SPEC-071's
    `card_snapshot` golden over the law cards), stored in `law_dues` at each recompute. Before the first
    recompute there is no row, and every surface renders the dues as pending, never 0.
R11. The law block carries the law streak (SPEC-076), today's law-track XP and lifetime law-track XP
    over both XP tables, the level of that lifetime XP on the shared curve (SPEC-040), the law dues,
    the active law leeches and the law mastery pillar, equal to `goldens/law_track_summary.json`
    (`pipeline_layers/digests.py:DigestsLayer.law_track_summary` and `law_total_xp`).
R12. The active law leeches and the law mastery pillar read W4's leech port (#133); until it is wired,
    both are pending, never 0, and the pillar is not computed.
R13. The block is omitted, never rendered with zeros, when the law streak, today's law XP, the active
    law leeches and lifetime law XP are all zero, a pending count reading as zero for this rule; inside
    it, the dues appear only above zero, and the mastery and the leeches only while leeches are active.
    The rule equals `goldens/law_block_shown.json` (`telegram.py:render_law_block`).
R14. This SPEC provides the law block's view and its bot rendering; the owner's today view places it
    first whenever it is shown (SPEC-086), and the law tab shows it with SPEC-072's tier distribution.

The screens and the data

R15. `GET /api/progress` (each course's code, name, flag, mastery, current band, current unit and
    bands) and `GET /api/law` (the law block, with each pending count as null beside a pending flag,
    and whether the block is shown) serve the owner only (SPEC-024); any other caller gets 401 or 403
    and no data.
R16. The bot's `/progress` (SPEC-026's command table) answers each course's band, mastery and current
    unit, with an Open button into the Mini App's progress screen; `/today` uses this SPEC's law block
    rendering (SPEC-086).
R17. The Mini App's `/progress` screen draws each course's ladder as six band cells from the bands
    JSON, each with its mastery, and marks the current unit, in HTML cells with text (the Road to C2
    chart is SPEC-085's); `/law` shows the law block and the tier distribution. Both join `ROUTES`.
R17a. The next milestone's mature-card input (SPEC-073 R15) is the sum of `mature` over the stored
    `language_progress` rows of the configured courses, as the predecessor sums
    `progress.py:compute_progress`'s mature cards into `gamification/rewards.py:next_milestone`
    (`pipeline.py:GamifyPipeline._compute_and_store_coaching`); this delivery wires it into SPEC-073's
    milestone view, which stops answering `pending` once the first recompute has stored a course.
R18. Curriculum owns `language_progress`, `band_milestones` and `law_dues`, each `STRICT` with
    `created_at`, created by `migrations/007701_curriculum_road_to_c2_and_law.sql` (SPEC-020 R15,
    R18). Each is registered in the context map's register of DeckStreak's own tables, declared in
    `privacy.json` (the categories `course-progress` and `law-dues`), given a line per category in
    `PRIVACY.md`, and listed as exported and erased by curriculum's new data-rights port, which joins
    coordination's registry; the symmetry test seeds each.
R19. Every constant this SPEC uses (the stability target, the mature interval and mastery threshold,
    the achieved percent, the default decay and its clamps, the band order, the band-up XP and the
    law leech penalty and its cap) equals `goldens/curriculum.constants.json`, held by a test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | ingest's memory state of every case equals the golden of `fsrs.py:parse_fsrs` (examined count reported, zero refused) | `the_memory_state_parse_matches_the_predecessors_golden` |
| A2 | the mastery of every case equals the golden of `progress.py:card_mastery` | `card_mastery_matches_the_predecessors_golden` |
| A3 | the unit of every synthetic deck name equals the golden of `progress.py:parse_unit` | `the_unit_parse_matches_the_predecessors_golden` |
| A4 | every synthetic course's progress, bands, current band and current unit equal the golden of `progress.py:compute_progress` | `course_progress_matches_the_predecessors_golden` |
| A5 | every curriculum constant equals `goldens/curriculum.constants.json` | `the_curriculum_constants_equal_the_predecessors` |
| A6 | a card whose unit falls in no configured band, and a card with no course, are not counted | `a_unit_outside_every_configured_band_is_not_counted` |
| A7 | the band-ups of every case equal the golden of `_persist_progress`, and each grant, badge and celebration is written once over two recomputes | `band_ups_match_the_predecessors_golden_and_pay_once` |
| A8 | a course seen for the first time records a silent baseline and pays nothing | `the_first_sighting_of_a_course_is_a_silent_baseline` |
| A9 | the persona engine reads the live band of the course a language subject names, and the roster's band for a subject with none | `the_persona_engine_reads_the_live_band` |
| A10 | curriculum's data-rights port lists its three tables as exported and erased, and an erase leaves them empty | `the_curriculum_tables_are_exported_and_erased` |
| A11 | the law mastery pillar of every case equals the golden of `scoring.py:law_mastery_pillar` | `the_law_mastery_pillar_matches_the_predecessors_golden` |
| A12 | the law block's fields for every case equal the golden of `DigestsLayer.law_track_summary` | `the_law_block_matches_the_predecessors_golden` |
| A13 | the block is omitted with no law activity, and its lines follow the golden of `telegram.py:render_law_block`'s shown rule | `the_law_block_is_omitted_without_law_activity` |
| A14 | before the first recompute the law dues are pending, never 0 | `law_dues_are_pending_before_the_first_recompute` |
| A15 | the active law leeches and the law mastery pillar are pending until the leech port is wired | `law_leeches_are_pending_until_the_leech_port_is_wired` |
| A16 | the progress route answers the owner and refuses every other caller with no data | `the_progress_route_answers_only_the_owner` |
| A17 | the law route answers the owner and refuses every other caller with no data | `the_law_route_answers_only_the_owner` |
| A18 | `/progress` states each course's band, mastery and current unit as the route does | `progress_shows_each_course_band_and_mastery` |
| A21 | the next milestone reads the sum of the courses' stored mature cards, and stays pending before the first recompute stores a course | `the_milestone_reads_the_courses_mature_cards` |

```acceptance
A1: cargo test -p deck-streak-ingest --test progress_memory_state -- --exact the_memory_state_parse_matches_the_predecessors_golden
A2: cargo test -p deck-streak-curriculum --test progress_goldens -- --exact card_mastery_matches_the_predecessors_golden
A3: cargo test -p deck-streak-curriculum --test progress_goldens -- --exact the_unit_parse_matches_the_predecessors_golden
A4: cargo test -p deck-streak-curriculum --test progress_goldens -- --exact course_progress_matches_the_predecessors_golden
A5: cargo test -p deck-streak-curriculum --test progress_goldens -- --exact the_curriculum_constants_equal_the_predecessors
A6: cargo test -p deck-streak-curriculum --test progress_unit_bands -- --exact a_unit_outside_every_configured_band_is_not_counted
A7: cargo test -p deck-streak-coordination --test progress_band_up -- --exact band_ups_match_the_predecessors_golden_and_pay_once
A8: cargo test -p deck-streak-coordination --test progress_band_up -- --exact the_first_sighting_of_a_course_is_a_silent_baseline
A9: cargo test -p deck-streak-daemon --test progress_live_band -- --exact the_persona_engine_reads_the_live_band
A10: cargo test -p deck-streak-curriculum --test progress_store -- --exact the_curriculum_tables_are_exported_and_erased
A11: cargo test -p deck-streak-curriculum --test law_goldens -- --exact the_law_mastery_pillar_matches_the_predecessors_golden
A12: cargo test -p deck-streak-coordination --test law_block -- --exact the_law_block_matches_the_predecessors_golden
A13: cargo test -p deck-streak-coordination --test law_block -- --exact the_law_block_is_omitted_without_law_activity
A14: cargo test -p deck-streak-coordination --test law_block -- --exact law_dues_are_pending_before_the_first_recompute
A15: cargo test -p deck-streak-coordination --test law_block -- --exact law_leeches_are_pending_until_the_leech_port_is_wired
A16: cargo test -p deck-streak-api --test progress_routes -- --exact the_progress_route_answers_only_the_owner
A17: cargo test -p deck-streak-api --test law_routes -- --exact the_law_route_answers_only_the_owner
A18: cargo test -p deck-streak-bot --test progress_commands -- --exact progress_shows_each_course_band_and_mastery
A21: cargo test -p deck-streak-coordination --test progress_milestone -- --exact the_milestone_reads_the_courses_mature_cards
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, notifications-policy and accessibility
packs stay enforced; no check is deferred or lifted for this delivery, so the private wiring does not
change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/curriculum/src/data_rights.rs`: the `course-progress` and `law-dues` categories name their three tables with purpose, basis and retention, and export and erase cover all three | the privacy-gdpr pack |
| B2 | over `notifications-policy.json` and every file under `crates/coordination/src/`: `band_up` stays T5 and budget-exempt, and the band-up's celebration goes through the one router | the notifications-policy pack |

## 3c. Delivered by the next pull request

This SPEC lands in two pull requests, in order. This one (CU85a) delivers the criteria of section
3's table that no row below names, and the box rows B1 and B2. CU85b delivers the composition root's
`LiveBand` over the stored band, `GET /api/progress` and `GET /api/law`, the bot's `/progress` and
the Mini App's two screens. The table below holds the criteria that pull request delivers, each row
naming it, and the lines under it are their fence lines, each prefixed with it. CU85b moves each of
its criteria back verbatim: the row into section 3's table (or section 3a's, for B3), without the
`delivered by` column, and the fence line into the acceptance fence, without the prefix. R8's
consumer, the port `LiveBand` over curriculum's stored band, is CU85b's first part.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A19 | the progress screen draws each band cell with its mastery and marks the current unit | `draws each band cell with its mastery and the current unit` | CU85b |
| A20 | the law tab renders a pending count as pending, never as zero | `renders a pending count as pending, never zero` | CU85b |
| B3 | over `web/app/src/routes/progress/+page.svelte`, `web/app/src/routes/law/+page.svelte`, `web/app/src/lib/progress/` and `web/app/src/lib/law/`: both screens pass the accessibility audit in both Telegram colour schemes | the accessibility pack | CU85b |

CU85b: A19: pnpm exec vitest run web/app/src/lib/progress/CourseLadder.test.ts -t "draws each band cell with its mastery and the current unit"
CU85b: A20: pnpm exec vitest run web/app/src/lib/law/LawBlock.test.ts -t "renders a pending count as pending, never zero"

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/memory_state.rs` | `deck-streak-ingest` | added: the predecessor's parse of `cards.data` into numbers |
| `crates/ingest/src/reader.rs` | `deck-streak-ingest` | changed: the card read selects `data` and publishes the parsed memory state, never the text |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the memory-state module |
| `crates/ingest/tests/progress_memory_state.rs` | `deck-streak-ingest` | added: A1 |
| `crates/curriculum/src/progress.rs` | `deck-streak-curriculum` | added: mastery, the unit parse, course progress, the band-up rule |
| `crates/curriculum/src/unit_bands.rs` | `deck-streak-curriculum` | added: the band a unit falls in, read from the kernel's `Courses` |
| `crates/curriculum/src/law.rs` | `deck-streak-curriculum` | added: the law mastery pillar and the law dues |
| `crates/curriculum/src/store.rs` | `deck-streak-curriculum` | added: the repository over the three tables |
| `crates/curriculum/src/data_rights.rs` | `deck-streak-curriculum` | added: curriculum's data-rights port |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the modules above |
| `crates/curriculum/Cargo.toml` | `deck-streak-curriculum` | changed: `sqlx`, `thiserror` and `serde_json` (ADR-003, ADR-029), and the golden reader's `serde` and `serde_json` as dev-dependencies (SPEC-029 R8) |
| `crates/curriculum/tests/progress_goldens.rs` | `deck-streak-curriculum` | added: A2 to A5 |
| `crates/curriculum/tests/progress_unit_bands.rs` | `deck-streak-curriculum` | added: A6 |
| `crates/curriculum/tests/progress_store.rs` | `deck-streak-curriculum` | added: A10 |
| `crates/curriculum/tests/law_goldens.rs` | `deck-streak-curriculum` | added: A11 |
| `migrations/007701_curriculum_road_to_c2_and_law.sql` | `deck-streak-curriculum` | added: the three tables, `STRICT`, with `created_at` and their keys |
| `crates/coordination/src/recompute/progress.rs` | `deck-streak-coordination` | added: the progress step, the law dues and the band-up's grant, badge and celebration |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the progress step in phase 4 and the band badge's award in phase 7 of SPEC-071's fold |
| `crates/coordination/src/law/mod.rs` | `deck-streak-coordination` | added: the law block's view over the streak, the XP tables, the dues and the leech port |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the law module |
| `crates/coordination/tests/progress_band_up.rs` | `deck-streak-coordination` | added: A7, A8 |
| `crates/coordination/tests/law_block.rs` | `deck-streak-coordination` | added: A12 to A15 |
| `crates/coordination/src/progression/milestone_view.rs` | `deck-streak-coordination` | changed: the mature-card input from the stored course progress (R17a) |
| `crates/coordination/tests/progress_milestone.rs` | `deck-streak-coordination` | added: A21 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the agent's `LiveBand` over curriculum's stored band, behind a newtype |
| `crates/daemon/tests/progress_live_band.rs` | `deck-streak-daemon` | added: A9 |
| `crates/api/src/progress_routes.rs` | `deck-streak-api` | added: `GET /api/progress` |
| `crates/api/src/law_routes.rs` | `deck-streak-api` | added: `GET /api/law` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: both routes |
| `crates/api/tests/progress_routes.rs` | `deck-streak-api` | added: A16 |
| `crates/api/tests/law_routes.rs` | `deck-streak-api` | added: A17 |
| `crates/bot/src/progress_commands.rs` | `deck-streak-bot` | added: /progress and the law block's rendering |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu gain /progress |
| `crates/bot/tests/progress_commands.rs` | `deck-streak-bot` | added: A18 |
| `web/app/src/routes/progress/+page.svelte` | miniapp | added: the progress screen |
| `web/app/src/routes/law/+page.svelte` | miniapp | added: the law tab |
| `web/app/src/lib/progress/CourseLadder.svelte` | miniapp | added: one course's band cells |
| `web/app/src/lib/progress/progress.ts` | miniapp | added: the route's client and types |
| `web/app/src/lib/progress/CourseLadder.test.ts` | miniapp | added: A19 |
| `web/app/src/lib/law/LawBlock.svelte` | miniapp | added: the law block |
| `web/app/src/lib/law/law.ts` | miniapp | added: the route's client and types |
| `web/app/src/lib/law/LawBlock.test.ts` | miniapp | added: A20 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /progress and /law join `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `language_progress`, `band_milestones` and `law_dues` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry gains curriculum's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for the three tables |
| `privacy.json` | repo | changed: the `course-progress` and `law-dues` categories |
| `PRIVACY.md` | repo | changed: one line per category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_077.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/memory_state.json` | repo | added: the golden of `fsrs.py:parse_fsrs` (adapter; returns the state's fields) |
| `tools/parity-oracle/goldens/card_mastery.json` | repo | added: the golden of `progress.py:card_mastery` (adapter; builds a card) |
| `tools/parity-oracle/goldens/unit_parse.json` | repo | added: the golden of `progress.py:parse_unit` (function) |
| `tools/parity-oracle/goldens/course_progress.json` | repo | added: the golden of `progress.py:compute_progress` (adapter; synthetic courses) |
| `tools/parity-oracle/goldens/band_up.json` | repo | added: the golden of `pipeline.py:GamifyPipeline._persist_progress` (adapter; a stub store) |
| `tools/parity-oracle/goldens/law_mastery_pillar.json` | repo | added: the golden of `scoring.py:law_mastery_pillar` (function) |
| `tools/parity-oracle/goldens/law_track_summary.json` | repo | added: the golden of `pipeline_layers/digests.py:DigestsLayer.law_track_summary` (adapter; a stub store) |
| `tools/parity-oracle/goldens/law_block_shown.json` | repo | added: the golden of `telegram.py:render_law_block` (adapter; the fields it shows, never its text) |
| `tools/parity-oracle/goldens/curriculum.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S07700-S07799.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-077-road-to-c2-bands-each-course-and-the-law-block-leads-when-law-is-active.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-077.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It builds no leech snapshot or remediation, so the active law leeches and the law mastery pillar stay
  pending until the leech port is wired (#133).
- It serves no law counter to the agent's machine read tool, with or without its bearer scope (#158,
  #157).
- It forecasts no velocity or time to the next band (#86).
- It derives no skill strands, weak spots or memory health (#88, #90).
- It builds no test-preparation section board (#135).
- It draws no Road to C2 chart: the progress screen's band cells are text (#152).
- It writes no Road to C2 or law block into a digest or a weekly report (#129, #130).
- It publishes no course band or law counter on the public page (#156).
- It exports no syllabus band table and runs no syllabus parity gate (#159).
- It imports none of the predecessor's course progress, band milestones or law dues (#61).

## 6. Risks

- **A band is celebrated twice, or a skipped band is celebrated.** Detected by A7, which runs two
  recomputes over the golden's cases, and by the row on the migration's key (§9).
- **Mastery's floating point drifts from the predecessor's** in the logarithm or the power. Detected
  by A2, which compares each mastery within 1e-9 and every band decision exactly.
- **The configured unit bands drift from the owner's syllabi.** Detected by the private deploy rail's
  check (#41); an overlapping or unordered table is refused at start by SPEC-071's loader.
- **The persona engine reads a stale band** between recomputes. Accepted: it reads the band the last
  recompute stored, which is the band every screen shows.
- **A private deck name reaches a golden** through the law block's text or a course's deck. Prevented
  by the adapters, which return fields and synthetic courses only, and detected by the public scrub.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_077.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic; no golden carries a deck name of the
owner's, a unit-band value of the owner's, or a calendar string.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `memory_state` | `fsrs.py:parse_fsrs` | adapter | calls it with the case's text and returns the state's five fields, or none; cases with valid, missing, unparsable, non-object, non-finite and non-positive values |
| `card_mastery` | `progress.py:card_mastery` | adapter | a card from the case's queue, type, interval, stability, decay and last review, and the case's clock; cases on both paths, suspended and buried cards, at the decay clamps and at the mature interval |
| `unit_parse` | `progress.py:parse_unit` | function | none: synthetic deck names with and without a unit, leading zeros and several units |
| `course_progress` | `progress.py:compute_progress` | adapter | patches `progress.LANGUAGE_DECKS` and `progress.CEFR_UNIT_BANDS` with synthetic courses and unit bands, builds the cards and synthetic deck names, and returns each course's fields and bands; cases with gaps, an unachieved A1, drops and skipped bands |
| `band_up` | `pipeline.py:GamifyPipeline._persist_progress` | adapter | a stub store holding the stored bands and milestones, and the progress computation patched to return the case's per-course bands; it returns the baselines recorded silently and the band-ups celebrated, with their XP source, badge key and event key |
| `law_mastery_pillar` | `scoring.py:law_mastery_pillar` | function | none: zero, one, the cap and past it |
| `law_track_summary` | `pipeline_layers/digests.py:DigestsLayer.law_track_summary` | adapter | a stub store holding the law streak row, today's and lifetime law-track XP, the law leech rows and the law dues; it returns the summary's fields |
| `law_block_shown` | `telegram.py:render_law_block` | adapter | calls it with the case's payload and returns whether it rendered and which fields' lines it holds, never its text |
| `curriculum.constants` | `curriculum.CEFR_BANDS`, `CEFR_BAND_ACHIEVED_PCT`, `PROGRESS_MATURE_IVL_DAYS`, `PROGRESS_STABILITY_TARGET_DAYS`, `MATURE_MASTERY_THRESHOLD`; `fsrs.DEFAULT_DECAY`, `_MIN_ABS_DECAY`, `_MAX_ABS_DECAY`; `constants.XP_BONUS_BAND_UP`, `MASTERY_LEECH_PENALTY`, `MASTERY_LEECH_PENALTY_CAP` | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `language_progress` | `deck-streak-curriculum` | `migrations/007701_curriculum_road_to_c2_and_law.sql` | its per-language progress rows, keyed by the language code, which becomes the course code | exported and erased |
| `band_milestones` | `deck-streak-curriculum` | `migrations/007701_curriculum_road_to_c2_and_law.sql` | its band milestones, one row per language and band reached, the silent baselines included | exported and erased |
| `law_dues` | `deck-streak-curriculum` | `migrations/007701_curriculum_road_to_c2_and_law.sql` | the law due count it kept as one coaching value | exported and erased: no row reads as pending |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S07701-STABILITY-TARGET` | `crates/curriculum/src/progress.rs` | the stability target of 100 days in the mastery score | `progress_goldens::card_mastery_matches_the_predecessors_golden` |
| `S07702-SUSPENDED-SCORES-ZERO` | `crates/curriculum/src/progress.rs` | a suspended card's mastery of 0 | `progress_goldens::card_mastery_matches_the_predecessors_golden` |
| `S07703-MATURE-INTERVAL` | `crates/curriculum/src/progress.rs` | the fallback's interval of 21 days | `progress_goldens::card_mastery_matches_the_predecessors_golden` |
| `S07704-DECAY-CLAMP` | `crates/curriculum/src/progress.rs` | the decay exponent's magnitude clamp | `progress_goldens::card_mastery_matches_the_predecessors_golden` |
| `S07705-DEFAULT-DECAY` | `crates/ingest/src/memory_state.rs` | the default decay of 0.2 | `progress_memory_state::the_memory_state_parse_matches_the_predecessors_golden` |
| `S07706-ACHIEVED-AT-EIGHTY` | `crates/curriculum/src/progress.rs` | a band achieved at 80% | `progress_goldens::course_progress_matches_the_predecessors_golden` |
| `S07707-MATURE-MASTERY-HALF` | `crates/curriculum/src/progress.rs` | a mature card at mastery 0.5 | `progress_goldens::course_progress_matches_the_predecessors_golden` |
| `S07708-BAND-UP-XP` | `crates/curriculum/src/progress.rs` | the band-up's 500 XP | `progress_goldens::the_curriculum_constants_equal_the_predecessors` |
| `S07709-ONE-MILESTONE-PER-BAND` | `migrations/007701_curriculum_road_to_c2_and_law.sql` | the key on `band_milestones (course, band)` (a script row; the cargo killer) | `progress_band_up::band_ups_match_the_predecessors_golden_and_pay_once` |
| `S07710-LAW-LEECH-PENALTY` | `crates/curriculum/src/law.rs` | three points a leech, capped at 30 | `law_goldens::the_law_mastery_pillar_matches_the_predecessors_golden` |

## 10. Amendments, 2026-10-02: what the first pull request corrects, and what it adds beside the manifest

Section 3's table now holds the criteria CU85a delivers: the rows of A9 and A16 to A20 moved out of
it, and section 3a's table lost B3 the same way. Each stays verbatim in section 3c's table with its
fence line there, as section 3c says CU85b moves it back. Section 4's table is unchanged. Each
correction below replaces its Old text with its New text.

- T1, R7. Old: "500 XP through SPEC-040's `grant` with scope once, source `bandup:<code>:<band>`" and
  "dedupe key `bandup:<code>:<band>`, through the router." New: "500 XP through SPEC-040's grant inside
  the day's write (`grant_on`) with scope once, source `bandup:<code>:<band in lowercase>`" and
  "dedupe key `bandup:<code>:<band in lowercase>`, offered through the router between the fold's
  writes from the milestone's unset mark, and marked when the router answers (ADR-303)." The bands
  are uppercase in the kernel's `CEFR_BANDS`, while the grant's source grammar and the router's
  dedupe-key grammar refuse uppercase; the badge key keeps the uppercase band.
- T2, section 8, `band_milestones`. Old: "the silent baselines included". New: "the silent baselines
  included; each row records whether it is a silent baseline and, for a band-up, its celebration
  mark, unset until the router answers (ADR-303)".
- T3, the header's "Decided by". It gains ADR-303 (an award carries its celebration mark and is
  offered until the router answers) and ADR-077, written by this delivery's migration part, which
  records the lowercase band, the 500 XP constant's home in curriculum and the milestone's columns.
- T4, section 4, `crates/coordination/src/recompute/mod.rs`. Old: "changed: the progress step in
  phase 4 and the band badge's award in phase 7 of SPEC-071's fold". New: "changed: the progress
  module, and the band-up's offers beside the badges' and records' in `AwardOffers`".
  `crates/daemon/src/wiring.rs` gains: "the progress step registered in phase 4 and the band badge's
  step in phase 7 (`recompute_fold_with_relights`)", which CU85b delivers.
- T5, section 4 gains these files: `crates/progression/src/ledger.rs` (today's law XP on one track);
  `crates/api/src/badges_routes.rs` (the milestone route stops answering `pending`, R17a);
  a coordination progress view for the API and the bot; `crates/coordination/src/recompute/band_badges.rs`;
  `formal/lean/Formal/RoadToC2.lean`, its vectors file and the `Formal.lean` import;
  `formal/tla/BandUpOnce/`; ADR-077; and `crates/curriculum/src/store.rs` only if the migration part
  creates it. The eleven `Card` literal files named below each gain
  `memory: None` and nothing else.
- T7, R8. Old: "the course a language subject names; a subject that names no course answers no band".
  New: "the course whose code is the area of a `language/<area>` subject; a subject whose area is no
  configured course's code answers no band".
- T8, section 4, `crates/curriculum/src/law.rs`. Old: "added: the law mastery pillar and the law
  dues". New: "added: the law mastery pillar and the law dues' stored value (coordination counts them
  with analytics' card snapshot over the law cards)", because curriculum may not depend on analytics.
- T9, R16. Old: "`today` uses this SPEC's law block rendering (SPEC-086)." New: "SPEC-086's `today`
  will use this SPEC's law block rendering; no `today` exists at this delivery's base."
- T10, R17a. Old: "wires it into SPEC-073's milestone view, which stops answering `pending`". New:
  "wires it into SPEC-073's milestone view and `GET /api/milestone`, which stop answering `pending`".

R1 adds a field to each card ingest publishes, so these files, which build a `Card` and are not named
in section 4, each gain one line, `memory: None`, inside their `Card { .. }` literal:

- `crates/analytics/tests/rollup_metrics.rs`
- `crates/coordination/tests/awards_support/mod.rs`
- `crates/coordination/tests/relight_order.rs`
- `crates/coordination/tests/relight_settle.rs`
- `crates/coordination/tests/settle_fold.rs`
- `crates/coordination/tests/streak_fold.rs`
- `crates/coordination/tests/wallet_mint.rs`
- `crates/coordination/tests/xp_steps.rs`
- `crates/daemon/tests/record_offers.rs`
- `crates/daemon/tests/streak_calendar_route.rs`
- `crates/readings/tests/support/mod.rs`

`crates/curriculum/src/lib.rs` also states, in its crate doc, that curriculum owns the band-up's own
500 XP amount and no other: R7's constant lives in `crates/curriculum/src/progress.rs`, as streaks
keeps its relight amount, and `economy.json` gains no key.

Rows this pull request does not touch, delivered by CU85b:

- `crates/daemon/src/wiring.rs`: unchanged in this pull request; delivered by CU85b
- `crates/daemon/tests/progress_live_band.rs`: unchanged in this pull request; delivered by CU85b
- `crates/api/src/progress_routes.rs`: unchanged in this pull request; delivered by CU85b
- `crates/api/src/law_routes.rs`: unchanged in this pull request; delivered by CU85b
- `crates/api/src/router.rs`: unchanged in this pull request; delivered by CU85b
- `crates/api/tests/progress_routes.rs`: unchanged in this pull request; delivered by CU85b
- `crates/api/tests/law_routes.rs`: unchanged in this pull request; delivered by CU85b
- `crates/bot/src/progress_commands.rs`: unchanged in this pull request; delivered by CU85b
- `crates/bot/src/commands.rs`: unchanged in this pull request; delivered by CU85b
- `crates/bot/tests/progress_commands.rs`: unchanged in this pull request; delivered by CU85b
- `web/app/src/routes/progress/+page.svelte`: unchanged in this pull request; delivered by CU85b
- `web/app/src/routes/law/+page.svelte`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/progress/CourseLadder.svelte`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/progress/progress.ts`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/progress/CourseLadder.test.ts`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/law/LawBlock.svelte`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/law/law.ts`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/law/LawBlock.test.ts`: unchanged in this pull request; delivered by CU85b
- `web/app/src/lib/routes.ts`: unchanged in this pull request; delivered by CU85b

- T11, R7, the band badge's tier. R7 names no tier for the band badge `band_<code>_<band>`. It is
  awarded at tier 0, as SPEC-073 R2 places each catalog badge ("each at tier 0") and as progression's
  award test awards the band keys (`crates/progression/tests/badges_award.rs`: "Awards `key` at tier
  0 in one write of its own."); the predecessor's band badge passes no tier, so its default of 0
  holds.
- T12, section 7, `course_progress`. The golden gains the case `unit_beyond_u32`: a deck whose unit
  does not fit in 32 bits counts none of its cards, as the predecessor counts them where no band of
  the course holds that unit. `parse_unit` reads such a unit as no unit, which A3's test asserts
  beside the golden; every other golden of this SPEC changes only its registry digest.
- T13, section 4 gains these files beside T5's: `formal/lean/Formal/RoadToC2Vectors.lean` (the
  entry's vector writer), `formal/lean/Formal/Vectors.lean` (one arm for it),
  `formal/vectors/road-to-c2.jsonl` (the vectors the checker byte-compares) and
  `crates/curriculum/tests/formal_vectors_road_to_c2.rs` (the Rust cross-check of those vectors).
- T14, R7's last sentence, the band-up golden (`tools/parity-oracle/goldens/band_up.json`). Its
  cases `no_notifier` and `milestones_muted` pay the band-up's XP and badge and record no
  celebration, because the predecessor reads its notifier and its milestone setting before it
  celebrates. Here coordination offers each owed band-up to the router whatever the notifier and
  the setting, and the router decides, as it does for every other celebration (SPEC-041, ADR-041):
  with no bot it withholds the occasion with reason `no_notifier`, and with the celebration kind's
  switch off it withholds it by that switch. A7 asserts the golden's celebrations for every other
  case and, for these two, the offer the router receives; it reads the band-up's T5 and its budget
  exemption from `notifications-policy.json`, whose `band_up` rows this delivery leaves unchanged.
- T15, the list above of the files that gain `memory: None`. Old (T5): "each gain `memory: None`
  and nothing else". New: "each gain `memory: None`; `crates/coordination/tests/relight_order.rs`
  also lists the data-rights registry's `static CURRICULUM` in its census of the statics
  coordination links", because the census refuses a static of
  `crates/coordination/src/data_rights_registry.rs` that it does not write out, and this
  delivery's store part added that static.
- T16, T9's two lines in this section. T9 spelt the bot's today command, a route and not a file,
  with a leading slash, which reads as a path outside the repository; at this delivery's base the
  Mini App's routes hold no today directory and no bot source names the command, so T9 now names
  it by the plain word `today`. The Old and New lines, verbatim, are held in the block below, so
  the Old spelling is quoted and not named. No other line of this SPEC moves.

```text
Old:
- T9, R16. Old: "`/today` uses this SPEC's law block rendering (SPEC-086)." New: "SPEC-086's `/today`
  will use this SPEC's law block rendering; no `/today` exists at this delivery's base."
New:
- T9, R16. Old: "`today` uses this SPEC's law block rendering (SPEC-086)." New: "SPEC-086's `today`
  will use this SPEC's law block rendering; no `today` exists at this delivery's base."
```
- T17, R11 and A12, the law track summary golden's `negative_ledger` case. Its stub store holds a
  law row of -30 beside one of 10 on the same day, and the predecessor sums them to -20. Here
  neither XP table takes a negative amount (each column is `CHECK (amount >= 0)`: XP is never
  confiscable, CHARTER 5), so that row cannot be stored and the block reads 10. A12 seeds every
  case's rows as the golden writes them, asserts that this case is the only one with a row the
  tables refuse, and compares the block's lifetime and day XP with the golden's less the refused
  rows; every other field, the level included, is compared as the golden states it. The golden is
  unchanged. The block's day XP is read by `SqliteXpLedger::track_day_total` in
  `crates/progression/src/ledger.rs`, already in the manifest by T5, and its query's cache entry
  joins `.sqlx/`, also in the manifest.
- T18, section 9 and section 4's band file. Section 9 names `S07701`-`S07710`, and section 4 says
  `scripts/mutation-rows.d/S07700-S07799.json` holds the rows of section 9. The band file also holds
  the twelve rows below, which guard the band-up's record, mark and offer, the unit parse, the law
  pillar's cap and the law dues' erase; section 4's line for the band file covers them too. Two
  killers are new tests in test files section 4 already lists, each mutation coverage and not an
  acceptance criterion: `progress_band_up::the_progress_step_runs_for_the_current_day_only` in
  `crates/coordination/tests/progress_band_up.rs`, and `progress_store::a_band_up_is_marked_once`
  in `crates/curriculum/tests/progress_store.rs`. `S07712` is a script row with a cargo killer, as
  `S07709` is: its target is the curriculum store and its killer runs in coordination. Its mutant
  writes a baseline unmarked, which the table's CHECK refuses, so A8 fails at its expectation that
  the step evaluates rather than at an assertion: the CHECK kills it. `S07722`'s mutant runs the
  mark's statement unchecked and without its guard, because a changed checked statement has no
  offline cache entry and would not build. `S07703`'s mutant flips the fallback's inclusive bound
  on the mature interval rather than moving the constant: every card mastery golden case carries
  its own mature interval, so a moved constant is not observed there, and the constant's value is
  held by A5's constants golden.

| row | target | what it guards | killer |
|---|---|---|---|
| `S07711-FIRST-SIGHTING-SILENT` | `crates/coordination/src/recompute/progress.rs` | a first sighting grants no band-up | `progress_band_up::the_first_sighting_of_a_course_is_a_silent_baseline` |
| `S07712-BASELINE-WRITTEN-MARKED` | `crates/curriculum/src/store.rs` | a baseline is written marked (a script row; the cargo killer) | `progress_band_up::the_first_sighting_of_a_course_is_a_silent_baseline` |
| `S07713-ERASE-LAW-DUES` | `crates/curriculum/src/data_rights.rs` | the erase deletes the law dues | `progress_store::the_curriculum_tables_are_exported_and_erased` |
| `S07714-BAND-UP-LATER-ONLY` | `crates/curriculum/src/progress.rs` | only a band later than the stored one is a band-up | `formal_vectors_road_to_c2::the_road_to_c2_rules_answer_every_lean_vector` |
| `S07715-PROGRESS-CURRENT-DAY-ONLY` | `crates/coordination/src/recompute/progress.rs` | the progress step runs for the current study day only | `progress_band_up::the_progress_step_runs_for_the_current_day_only` |
| `S07716-UNIT-OVERFLOW-IS-NO-UNIT` | `crates/curriculum/src/progress.rs` | a unit beyond 32 bits is no unit | `progress_goldens::the_unit_parse_matches_the_predecessors_golden` |
| `S07717-PILLAR-CAP` | `crates/curriculum/src/law.rs` | the leeches take at most 30 points | `formal_vectors_road_to_c2::the_road_to_c2_rules_answer_every_lean_vector` |
| `S07718-CURRENT-BAND-CONTIGUOUS` | `crates/curriculum/src/progress.rs` | the current band ends the achieved run from A1 | `formal_vectors_road_to_c2::the_road_to_c2_rules_answer_every_lean_vector` |
| `S07719-OFFER-MARK-AFTER-ANSWER` | `crates/coordination/src/recompute/progress.rs` | a band-up the router did not answer stays owed | `progress_band_up::band_ups_match_the_predecessors_golden_and_pay_once` |
| `S07720-BAND-UP-KEY-LOWERCASE` | `crates/coordination/src/recompute/progress.rs` | the dedupe key spells the band lowercased | `progress_band_up::band_ups_match_the_predecessors_golden_and_pay_once` |
| `S07721-BAND-UP-THIRD-ARM` | `crates/coordination/src/recompute/mod.rs` | the offers hand every owed band-up to the router | `progress_band_up::band_ups_match_the_predecessors_golden_and_pay_once` |
| `S07722-BAND-UP-MARK-ONCE` | `crates/curriculum/src/store.rs` | a band-up is marked once | `progress_store::a_band_up_is_marked_once` |

- T19, mutation coverage of the CU85a files, none an acceptance criterion. Three new test files
  kill the missed mutants of the diff by file, each through the crate's public API:
  `crates/ingest/tests/memory_state_boundaries.rs` holds the memory state's field-by-field equality
  (bits semantics, so a NaN equals itself and 0.0 differs from -0.0) and the lenient parse's
  escapes, multibyte characters and number-shaped runs inside strings;
  `crates/curriculum/tests/store_boundaries.rs` holds the progress rows' round trip and replace,
  the stored bands, the milestones with their baseline flag and mark, the band-ups per day and
  owed oldest first, and the law dues read back as none, then written, then replaced; and
  `crates/coordination/tests/progress_step_boundaries.rs` holds the two steps' exact registered
  names, the band-up line and key spelled exactly, the law dues counted from the law track's cards
  only, and an owed band-up offered under its own course's flag and name, or its code once erased.
  One mutant in `crates/curriculum/src/progress.rs` is equivalent and is not killed: the band's
  achieved test reads `count > 0 && pct >= CEFR_BAND_ACHIEVED_PCT`, and the mutant reads `count >= 0`.
  The count is unsigned, so the mutant's guard is always true, and the only count it changes is
  zero, whose percentage is 0.0 and fails the threshold test either way, so no band reads
  differently. The three `lib.rs` files each
  carry one row in the band file, `S07723-INGEST-MEMORY-STATE-PUBLIC`,
  `S07724-CURRICULUM-MODULES-PUBLIC` and `S07725-COORDINATION-LAW-PUBLIC`. Each mutant prefixes the
  changed `pub mod` lines with `#[doc(hidden)]`, which builds; its killer is a source-text guard
  that reads the crate root with `include_str!` and asserts the declaration, as ruling 44 accepted
  for S08137. The guards sit in the three test files above, so section 4's lines for the band file
  and its tests cover them.
- T20, the two findings of the mutation verdict on the pull request's earlier head (ruling 53). The
  band threshold mutant `replace > with >= in course_progress` in `crates/curriculum/src/progress.rs`
  is declared equivalent by one record in `scripts/mutation-equivalent.d/deck-streak-curriculum.json`,
  reached by `progress_goldens::course_progress_matches_the_predecessors_golden`; the count is
  unsigned and a zero count reads 0.0, so the two comparisons agree on every input and the code of
  that line is unchanged. The `+=` mutant in the lenient reader of `crates/ingest/src/memory_state.rs`
  stopped the memory cap's tests, so `leniently` now computes the end of each in-string piece once,
  `let next = at + first + escaped;`, and uses it for both the slice and the advance. Behaviour is
  unchanged and the existing tests pass as they were. A mutant that stalls `next` spins without
  allocating and ends as a timeout, and one that moves it below the position panics on the slice.
- T21, T5 and T10. `crates/api/src/badges_routes.rs` and the coordination progress view for the API and the bot are delivered by CU85b, whose first part wires the stored mature cards into `GET /api/milestone` (R17a); until then the route answers `pending`.
- `crates/api/src/badges_routes.rs`: unchanged in this pull request; delivered by CU85b
- T22, the merge of live `dev`. `crates/curriculum/tests/horizon_goldens.rs`, which the horizon slice (SPEC-091) added, builds a `Card` and so gains one line, `memory: None`, inside its one `Card { .. }` literal, as the files listed under T15 do; `crates/curriculum/tests/store_boundaries.rs` and S07724 name the crate root's six modules, `horizon` among them, since the merge added that module.

## 11. Amendments, 2026-10-03: what the second pull request corrects, and what it adds beside the manifest

The second pull request (CU85b) lands in two parts on one branch. Its first part delivers R6's
registration in the production fold (T4), R8 as T7 amends it, R14's view half, R15 and R16's
progress command; its second part delivers the Mini App's two screens. It moves A9, A16, A17 and
A18 back into section 3, verbatim, as section 3c directs: each row into section 3's table without
its `delivered by` column, and each fence line into the acceptance fence without its prefix. A19,
A20 and B3 stay in section 3c for the second part. Section 10 is unchanged; the lines below
correct earlier lines as T1 to T22 do, and section 12 holds the one criterion this amendment adds.

- T23, R17a (T10) and T21. T10's New, "wires it into SPEC-073's milestone view and
  `GET /api/milestone`, which stop answering `pending`", becomes "wires it into SPEC-073's
  milestone view (`stored_mature`, A21). The milestone route keeps answering `pending`: SPEC-073
  R15 also reads the lifetime study reviews, which no table stores (#560), so an answer now would
  be computed from a stand-in. It answers from stored inputs once they are stored (#579)." T21's
  "whose first part wires the stored mature cards into `GET /api/milestone` (R17a); until then the
  route answers `pending`" becomes "which corrects the milestone route's documentation only; the
  route answers `pending` until its lifetime-review input is stored (#579, #560)". The route's
  code, and rows `S07321` and `S07322`, are unchanged; its two doc comments in
  `crates/api/src/badges_routes.rs` say why it stays `pending`.
- T24, R16, T9 and T16. T16's New for T9 is replaced; the Old and New lines are held in the block
  below, as T16's are.

```text
Old:
SPEC-086's `today` will use this SPEC's law block rendering; no `today` exists at this delivery's
base.
New:
SPEC-086's `today` (the bot's command and the owner's today view) will use this SPEC's law block
rendering. At this delivery's base the bot has no `today` command, and the Mini App's Today screen
(SPEC-028; the route table's `TODAY`, its root route) shows the study day and no law block;
SPEC-086 places the block first there (#69).
```

- T25, R14 and section 4's `crates/bot/src/progress_commands.rs`. R14's Old, "This SPEC provides
  the law block's view and its bot rendering;", becomes "This SPEC provides the law block's view;
  its bot rendering arrives with SPEC-086's `today`, its only bot caller (#69);". The manifest
  row's Old, "added: /progress and the law block's rendering", becomes "added: the progress
  command". A rendering with test callers only would be code no production path reaches.
- T26, R8 (T7) and A9. The adapter is `CurriculumLiveBand` in `crates/daemon/src/wiring.rs`, behind
  the agent's `LiveBand` port: a subject of kind language whose area is a configured course's code
  answers the band curriculum stores for that course, and any other subject, or a course with no
  stored row, answers no band, so the roster's band applies. Its production caller is the persona
  engine's output path, which #566 owns: at this delivery's base no production code calls
  `Roster::parse`, `Frontmatter::new` or `resolve_band`. Until it does, A9 is the adapter's only
  caller.
- T27, R6 and T4. `recompute_fold_with_relights` in `crates/daemon/src/wiring.rs` registers the
  progress step in phase 4 and the band badge step in phase 7 after the records step, so the
  production fold stores each course's progress, the law dues, the band-ups and their badges.
  A22 decides it. The in-file test of the fold's steps,
  `the_recompute_fold_registers_the_analytics_xp_and_streak_steps_in_their_phases`, gains both, so
  it lists the nine steps.
- T28, T5's coordination progress view. `crates/coordination/src/progress_view.rs` (added: the
  stored progress of the configured courses, ordered by name, which the progress route and the
  progress command both read; it re-exports curriculum's `StoredProgress` and `StoredBand`, since
  neither the API nor the bot depends on curriculum), `crates/coordination/tests/progress_view.rs`
  (added: the filter and the order, mutation coverage and not a criterion) and
  `crates/coordination/src/lib.rs` (changed: the module).
- T29, section 4 gains these files: `crates/api/src/lib.rs` and `crates/bot/src/lib.rs` (changed:
  the new modules); `crates/bot/tests/commands.rs` (changed: the twelve menu commands);
  `crates/bot/tests/messages/progress.msg.json`, `crates/bot/tests/messages/progress-none.msg.json`
  and `crates/bot/tests/messages/progress-failed.msg.json` (added: the progress command's golden
  messages); `crates/bot/tests/messages/start.msg.json` and `crates/bot/tests/messages/help.msg.json`
  (changed: the progress command's line); `crates/daemon/src/role_bot.rs` (changed: the bot reads
  the configured courses); `crates/api/src/badges_routes.rs` (changed: documentation only, T23);
  and `crates/coordination/tests/progress_band_up.rs` (changed: the band-up is paid once ever,
  mutation coverage and not a criterion, as T18's two tests are).
- T30, section 9 and section 4's band file. The band file also holds the seven rows below, and
  section 4's line for it covers them. `S07726`'s killer is the new coordination test T29 names;
  the others are criteria's tests.

| row | target | what it guards | killer |
|---|---|---|---|
| `S07726-BAND-UP-GRANT-ONCE` | `crates/coordination/src/recompute/progress.rs` | a band-up's XP is granted with scope once, so a band whose milestone was erased is not paid again on a later study day | `progress_band_up::a_band_up_is_paid_once_ever_even_after_its_milestone_is_erased` |
| `S07727-THE-PROGRESS-ROUTE-ASKS-FOR-THE-OWNER` | `crates/api/src/progress_routes.rs` | the progress route takes the owner's session before it reads anything | `progress_routes::the_progress_route_answers_only_the_owner` |
| `S07728-THE-LAW-ROUTE-ASKS-FOR-THE-OWNER` | `crates/api/src/law_routes.rs` | the law route takes the owner's session before it reads anything | `law_routes::the_law_route_answers_only_the_owner` |
| `S07729-LAW-DUES-PENDING-IS-NULL` | `crates/api/src/law_routes.rs` | pending law dues answer null, never 0 | `law_routes::the_law_route_answers_only_the_owner` |
| `S07730-THE-PROGRESS-STEP-IS-REGISTERED` | `crates/daemon/src/wiring.rs` | the production fold registers the progress step in phase 4 | `lib::wiring::tests::the_recompute_fold_registers_road_to_c2s_steps` |
| `S07731-THE-BAND-BADGE-STEP-IS-REGISTERED` | `crates/daemon/src/wiring.rs` | the production fold registers the band badge step in phase 7 | `lib::wiring::tests::the_recompute_fold_registers_road_to_c2s_steps` |
| `S07732-THE-LIVE-BAND-READS-THE-SUBJECTS-COURSE` | `crates/daemon/src/wiring.rs` | the live band answers the stored band of the course the subject's area names, and of no other | `progress_live_band::the_persona_engine_reads_the_live_band` |

## 12. Acceptance criteria of the 2026-10-03 amendment

| id | criterion | decided by |
|---|---|---|
| A22 | the production recompute fold registers the progress step in phase 4 and the band badge step in phase 7, after the records step | `the_recompute_fold_registers_road_to_c2s_steps` |

```acceptance
A22: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::the_recompute_fold_registers_road_to_c2s_steps
```
