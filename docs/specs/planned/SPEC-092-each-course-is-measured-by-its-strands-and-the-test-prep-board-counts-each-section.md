# SPEC-092: each course is measured by its strands, and the test-prep board counts each section's cards

- **Wave:** W4. **Issue:** #88, #135 (epic #5). **Context(s):** `deck-streak-ingest` (a card's law
  subject and test-prep section, from the law taxonomy settings); `deck-streak-curriculum` (the strand
  parse, the strand statistics and weak spots, the test-prep board, `strand_readouts`);
  `deck-streak-coordination` (the strands step in phase 4 of the fold, the read models, the `law_taxonomy` module);
  `deck-streak-api`, `deck-streak-bot` and the Mini App (the routes, `/weakspots` and its alias
  `/weak`, `/lsat`, the strand table and the law tab's coverage grid).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-087 (the courses are private
  configuration), ADR-091 (curriculum's readouts are stored as the latest readout) and ADR-092 (the
  law taxonomy is private configuration beside the law root, and ingest translates a card's law
  subject).
- **Prerequisites:** SPEC-023 (the scope settings and the law root), SPEC-029, SPEC-045 (the readings
  taxonomy and its law-subject golden), SPEC-071 (the fold and the courses), SPEC-077 (card mastery and the memory state) and SPEC-090 (the kernel's numeric port).
  **Mutation band:** `S09200-S09299`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-092.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 `crates/curriculum/src/` holds only `lib.rs`, and ingest
  knows a card's track (`reader.rs:track_of`) but not its law subject. SPEC-077's §5 leaves strands
  and weak spots to #88 and the test-prep board to #135.
- **What is ported.** The strands (#88): `strands.py:parse_strand`, `compute_strand_stats` and
  `weak_strands`, and the digest's choice of the top two weak spots
  (`pipeline_layers/digests.py:DigestsLayer._coaching_digest_block`). The law subject:
  `leeches.py:_law_subject`, which the leech board, the runway and the sabbatical clock also read. The
  test-prep board (#135): `lsat.py:compute_lsat_board` and `render_lsat_board`.
- **The taxonomy is configuration.** The predecessor names the law year bands and the test-prep
  subtree as literals beside its private law root (`leeches._LAW_BANDS`, `lsat._LSAT_TRACK`). They
  describe the owner's deck layout, so here they are settings beside the law root (ADR-092), with
  neutral example values.
- **Traps a hand port falls into.**
  - The strand is the LEAF segment of the card's home deck (a filtered deck resolves to the home
    deck), with one leading index token stripped by `^\d+[a-zA-Z]?(?:\s+|$)`; a name of fewer than
    three segments, or a leaf that is only an index, yields no strand.
  - A strand's retention counts each card's first review-type answer (type 1) of a study day, not
    every answer and never a learning step.
  - A strand is mature at mastery 0.5 or more, SPEC-077's mastery, which reads the clock.
  - The weak-spot baseline is the mean retention over the course's strands with at least 30 answered
    first reviews, summed as CPython sums floats (SPEC-090's `pynum`), and a strand is weak only when
    it trails that mean by more than 8 points; the sort is the largest shortfall first, then course
    and strand.
  - When a law path's second segment is a year band, the subject is its fourth segment (the third
    is a grouping), or its last when the path is shorter; otherwise the subject is the second
    segment. The bare root is its own subject, and a deck outside the root has none.
  - On the test-prep board a card was studied when its reps are above 0, suspended cards included; a
    deck path too short to name a section is unclassified, never dropped.
  - The board's tag axes are dormant in the predecessor (`tag_axis_computed` is false) and it says
    so rather than showing zeros.
- **Corrections to the issues.** #135's "section names come from the configured deck taxonomy":
  a section is the segment after the test-prep subtree in a deck's path, read at run time; what is
  configured is the law root and the subtree's name, never a section. #135 calls the board
  on-demand: here `/lsat` and the grid read the latest readout, which the recompute after each sync
  writes (ADR-091), so asking never reads the collection.
- **What the parity oracle proves.** The strand parse over synthetic deck names, the statistics and
  weak spots over synthetic cards and reviews, the law subject over synthetic law paths with and
  without year bands, and the board with its unclassified and dormant fields.
- **Prerequisites.** SPEC-023, SPEC-029, SPEC-045, SPEC-071, SPEC-077 and SPEC-090, as the header names them.

## 2. Requirements

The law taxonomy (ADR-092)

R1. Ingest's scope settings read `DECKSTREAK_LAW_YEAR_BANDS` (a comma-separated list) and
    `DECKSTREAK_LAW_TEST_PREP_DECK` (one deck name) beside `DECKSTREAK_LAW_DECK_ROOT`. Either set
    without the root refuses the cycle's step, which reads the scope on each cycle (`role_job.rs`
    `run_scheduled`, the owner sync's read in `wiring.rs`), naming the setting and never a value and
    ending in the refusal reason `scope_settings_refused`; `.env.example` shows
    neutral examples.
R2. `crates/ingest/src/law_subject.rs` gives a card's law subject and, under the test-prep subtree,
    its section. The law subject equals `goldens/law_subject.json` (`leeches.py:_law_subject`, with the
    module's year bands and law root patched to synthetic ones), and the test-prep section is the
    segment after the subtree's name, which `goldens/test_prep_board.json` holds (A7). The readings context already ports the same
    function (`crates/readings/src/topic.rs`, SPEC-045); ingest ports `_law_subject` again because
    ingest depends only on the kernel (ADR-002) and may not read the readings context's code or schema. The two ports
    stay equal because both tests read the ONE golden, which SPEC-045 added and this SPEC does not
    overwrite. When both the readings taxonomy and ingest's settings are configured, a law root or a
    year band that they name differently refuses start, naming both settings and neither value (A17);
    "the same law roots" means the readings taxonomy's `law.roots` list equals the one-element list
    of ingest's `DECKSTREAK_LAW_DECK_ROOT`.

The strands (#88)

R3. The strand of a deck name equals `goldens/strand_parse.json` (`strands.py:parse_strand`).
R4. Each course's strand statistics (cards, mature cards, mastery, answered and passed first reviews,
    retention and mean difficulty) equal `goldens/strand_stats.json`
    (`strands.py:compute_strand_stats`), and its weak spots equal `goldens/weak_strands.json`
    (`strands.py:weak_strands` with 30 reviews and 8.0 points).
R5. The digest's weak spots are the first two of the sorted list; `curriculum::strands` exposes them
    for the digest (#129), and a test holds the count and the order.

The test-prep board (#135)

R6. The board counts, per section of the test-prep subtree, the cards and the cards ever studied,
    with the unclassified count, and records its tag axes as not computed, equal to
    `goldens/test_prep_board.json` (`lsat.py:compute_lsat_board`). With no subtree configured or
    found, the board says it found no test-prep deck, never zeros.

The readouts (ADR-091)

R7. The strands step registers in phase 4 of SPEC-071's fold and runs in the current study day's
    step only. It writes each course's strand statistics and weak spots, and the board, to
    `strand_readouts`, one row per kind and scope, replacing the previous row in one write.
R8. `strand_readouts` is a `STRICT` table with `created_at`, created by
    `migrations/009201_curriculum_strand_readouts.sql` (SPEC-020 R15, R18). It is registered in the
    context map's register of DeckStreak's own tables, declared in `privacy.json` (the category
    `course-strands`), given a line in `PRIVACY.md`, and exported and erased by curriculum's
    data-rights port; the symmetry test seeds it.

The surfaces

R9. `GET /api/curriculum/strands` and `GET /api/curriculum/test-prep` serve the readouts to the
    owner's session only; any other caller is answered 401 or 403 with no data, and a missing readout
    reads as pending.
R10. `/weakspots` (alias `/weak`) and `/lsat` keep their names and state what the routes state. The
     Mini App shows each course's strands as a table, coloured by retention with its values as
     text, that opens a strand's detail, and the board as a coverage grid in the law tab (SPEC-077).
R11. Every constant this SPEC uses (the index pattern, the mature mastery, the weak-spot minimum and
     gap, the digest's two) equals `goldens/strands.constants.json`, held by a test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the law subject of every synthetic path equals the golden of `leeches.py:_law_subject` | `the_law_subject_matches_the_predecessors_golden` |
| A2 | a year-band or test-prep setting without the law root refuses the cycle's step with `scope_settings_refused`, naming the setting only | `a_law_taxonomy_without_a_root_refuses_the_cycles_step` |
| A3 | the strand of every synthetic deck name equals the golden of `strands.py:parse_strand` | `the_strand_parse_matches_the_predecessors_golden` |
| A4 | every course's strand statistics equal the golden of `strands.py:compute_strand_stats` | `strand_stats_match_the_predecessors_golden` |
| A5 | the weak spots equal the golden of `strands.py:weak_strands` | `weak_spots_match_the_predecessors_golden` |
| A6 | the digest's weak spots are the first two, in the golden's order | `the_digest_takes_the_first_two_weak_spots` |
| A7 | the board equals the golden of `lsat.py:compute_lsat_board`, a suspended studied card counted | `the_test_prep_board_matches_the_predecessors_golden` |
| A8 | with no test-prep subtree the board reports none found, never zeros, and its tag axes read not computed | `the_board_discloses_what_it_did_not_compute` |
| A9 | every strands constant equals `goldens/strands.constants.json` | `the_strands_constants_equal_the_predecessors` |
| A10 | two recomputes store one row per kind and scope, and the second replaces the first | `the_strand_readouts_are_replaced_once_per_recompute` |
| A11 | `strand_readouts` is exported and erased by curriculum's port | `the_strand_readouts_are_exported_and_erased` |
| A12 | both routes answer the owner and refuse every other caller with no data | `the_strand_routes_answer_only_the_owner` |
| A13 | `/weak` answers as `/weakspots` does, and both name each weak strand and its shortfall | `weak_is_an_alias_of_weakspots` |
| A14 | `/lsat` names each section's studied share and discloses the dormant axes | `lsat_states_each_section_and_the_dormant_axes` |
| A15 | the strand table states each strand's retention as text beside its colour | `states each strand retention as text beside its colour` |
| A16 | the coverage grid renders a missing board as none found, never as zeros | `renders a missing board as none found` |
| A17 | a law root or year band that ingest's settings and the readings taxonomy name differently refuses start (the readings `law.roots` list must equal the one-element list of ingest's root) | `a_law_root_or_band_named_differently_refuses_start` |
| A18 | the strands step passes the strand statistics the scoped cards with their memory state and home deck, the study reviews of the read window, the kernel's study-day rule and the run's instant, so each card's first review answer of a study day counts once and mastery reads the same clock | `the_strands_step_is_passed_its_reads_and_study_day_rule` |

```acceptance
A1: cargo test -p deck-streak-ingest --test law_subject -- --exact the_law_subject_matches_the_predecessors_golden
A2: cargo test -p deck-streak-daemon --test roles -- --exact a_law_taxonomy_without_a_root_refuses_the_cycles_step
A3: cargo test -p deck-streak-curriculum --test strands_goldens -- --exact the_strand_parse_matches_the_predecessors_golden
A4: cargo test -p deck-streak-curriculum --test strands_goldens -- --exact strand_stats_match_the_predecessors_golden
A5: cargo test -p deck-streak-curriculum --test strands_goldens -- --exact weak_spots_match_the_predecessors_golden
A6: cargo test -p deck-streak-curriculum --test strands_digest -- --exact the_digest_takes_the_first_two_weak_spots
A7: cargo test -p deck-streak-curriculum --test strands_goldens -- --exact the_test_prep_board_matches_the_predecessors_golden
A8: cargo test -p deck-streak-curriculum --test test_prep_board -- --exact the_board_discloses_what_it_did_not_compute
A9: cargo test -p deck-streak-curriculum --test strands_goldens -- --exact the_strands_constants_equal_the_predecessors
A10: cargo test -p deck-streak-coordination --test strands_step -- --exact the_strand_readouts_are_replaced_once_per_recompute
A11: cargo test -p deck-streak-curriculum --test strands_store -- --exact the_strand_readouts_are_exported_and_erased
A12: cargo test -p deck-streak-api --test strands_routes -- --exact the_strand_routes_answer_only_the_owner
A13: cargo test -p deck-streak-bot --test strands_commands -- --exact weak_is_an_alias_of_weakspots
A14: cargo test -p deck-streak-bot --test strands_commands -- --exact lsat_states_each_section_and_the_dormant_axes
A15: pnpm exec vitest run web/app/src/lib/strands/StrandTable.test.ts -t "states each strand retention as text beside its colour"
A16: pnpm exec vitest run web/app/src/lib/law/CoverageGrid.test.ts -t "renders a missing board as none found"
A17: cargo test -p deck-streak-coordination --test law_taxonomy_agrees -- --exact a_law_root_or_band_named_differently_refuses_start
A18: cargo test -p deck-streak-coordination --test strands_step -- --exact the_strands_step_is_passed_its_reads_and_study_day_rule
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and accessibility packs stay
enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/curriculum/src/data_rights.rs`: the `course-strands` category names `strand_readouts` with its purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/strands/+page.svelte`, `web/app/src/routes/law/+page.svelte`, every file under `web/app/src/lib/strands/` and `web/app/src/lib/law/CoverageGrid.svelte`: the strand table and the coverage grid pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/law_subject.rs` | `deck-streak-ingest` | added: a card's law subject and test-prep section |
| `crates/ingest/src/settings.rs` | `deck-streak-ingest` | changed: the year bands and the test-prep deck beside the law root |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the law subject module |
| `crates/ingest/tests/law_subject.rs` | `deck-streak-ingest` | added: A1 |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A2, beside `the_sync_job_pages_on_a_malformed_scope_before_it_syncs` |
| `.env.example` | repo | changed: neutral examples of the two settings |
| `deploy/deck-streak.env.example` | deploy | changed: the same two settings |
| `crates/coordination/src/law_taxonomy.rs` | `deck-streak-coordination` | added: the start-up agreement of ingest's law settings with the readings taxonomy (R2) |
| `crates/coordination/tests/law_taxonomy_agrees.rs` | `deck-streak-coordination` | added: A17 |
| `crates/curriculum/src/strands.rs` | `deck-streak-curriculum` | added: the strand parse, the statistics, the weak spots and the digest's two |
| `crates/curriculum/src/test_prep.rs` | `deck-streak-curriculum` | added: the test-prep board |
| `crates/curriculum/src/strands_store.rs` | `deck-streak-curriculum` | added: `strand_readouts`, read and replaced |
| `crates/curriculum/src/data_rights.rs` | `deck-streak-curriculum` | changed: the port exports and erases `strand_readouts` |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the strands modules |
| `crates/curriculum/tests/strands_goldens.rs` | `deck-streak-curriculum` | added: A3 to A5, A7, A9 |
| `crates/curriculum/tests/strands_digest.rs` | `deck-streak-curriculum` | added: A6 |
| `crates/curriculum/tests/test_prep_board.rs` | `deck-streak-curriculum` | added: A8 |
| `crates/curriculum/tests/strands_store.rs` | `deck-streak-curriculum` | added: A11 |
| `migrations/009201_curriculum_strand_readouts.sql` | `deck-streak-curriculum` | added: `strand_readouts` |
| `crates/coordination/src/recompute/strands.rs` | `deck-streak-coordination` | added: the strands step, and the cards, the read window's study reviews, the kernel's study-day rule and the run's instant it passes the strand statistics (A18) |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: declares the strands step's module |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: `RecomputeSetup::load` checks ingest's law root and year bands against the readings taxonomy and refuses start when they disagree (R2, A17); registers the strands step in phase 4 of `recompute_fold` (SPEC-071 R19) |
| `crates/coordination/src/strands.rs` | `deck-streak-coordination` | added: the strands and board read models |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the read models and the `law_taxonomy` module |
| `crates/coordination/tests/strands_step.rs` | `deck-streak-coordination` | added: A10, A18 |
| `crates/api/src/strands_routes.rs` | `deck-streak-api` | added: the two routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes behind the owner's session |
| `crates/api/tests/strands_routes.rs` | `deck-streak-api` | added: A12 |
| `crates/bot/src/strands_commands.rs` | `deck-streak-bot` | added: the weak-spots command, its alias and the test-prep command |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the commands join the table |
| `crates/bot/tests/strands_commands.rs` | `deck-streak-bot` | added: A13, A14 |
| `web/app/src/routes/strands/+page.svelte` | miniapp | added: each course's strands |
| `web/app/src/routes/law/+page.svelte` | miniapp | changed: the coverage grid in the law tab |
| `web/app/src/lib/strands/StrandTable.svelte` | miniapp | added |
| `web/app/src/lib/strands/StrandTable.test.ts` | miniapp | added: A15 |
| `web/app/src/lib/strands/strands.ts` | miniapp | added: the routes' client and types |
| `web/app/src/lib/law/CoverageGrid.svelte` | miniapp | added |
| `web/app/src/lib/law/CoverageGrid.test.ts` | miniapp | added: A16 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /strands joins `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `strand_readouts` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry lists `strand_readouts` under curriculum's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `strand_readouts` |
| `privacy.json` | repo | changed: the `course-strands` category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_092.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/law_subject.json` | repo | exists (SPEC-045); ingest's test reads the same golden |
| `tools/parity-oracle/goldens/strand_parse.json` | repo | added: the golden of `strands.py:parse_strand` (function) |
| `tools/parity-oracle/goldens/strand_stats.json` | repo | added: the golden of `strands.py:compute_strand_stats` (adapter; synthetic courses, cards and reviews) |
| `tools/parity-oracle/goldens/weak_strands.json` | repo | added: the golden of `strands.py:weak_strands` (adapter; synthetic statistics) |
| `tools/parity-oracle/goldens/test_prep_board.json` | repo | added: the golden of `lsat.py:compute_lsat_board` (adapter; synthetic subtree) |
| `tools/parity-oracle/goldens/strands.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S09200-S09299.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-092-each-course-is-measured-by-its-strands-and-the-test-prep-board-counts-each-section.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-092.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It writes no weak spot into the daily digest; it exposes the digest's two (#129, #53).
- It reads no note tags, so the test-prep board's section-tag and band-tag axes stay dormant, as
  the predecessor's do (#135).
- It builds no leech board, though it provides the law subject the leech board reads (#133).
- It serves no strand or board to the agent's machine read tool (#157).
- It lets no setting be edited in the Mini App (#57).
- It imports none of the predecessor's coaching payloads; the first recompute computes them (#61).

## 6. Risks

- **A deck layout the year bands do not describe** puts a subject at the wrong segment. Detected by
  A1's cases, which cover a path with a year band, a banded path too short for a fourth segment,
  one without a band, the bare root and a deck outside it.
- **A strand's retention counts every answer** instead of each card's first of the day. Detected by
  A4, whose cases answer a card twice on one study day.
- **The board hides that its tag axes were not computed.** Detected by A8 and A14.
- **A private deck name reaches a golden** through a law path. Prevented by the adapters, which patch
  the module's root and bands with synthetic ones, and detected by the public scrub.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_092.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic; no golden carries a deck name of the
owner's.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `strand_parse` | `strands.py:parse_strand` | function | none: leaves with and without an index token, a letter suffix, and an index alone |
| `strand_stats` | `strands.py:compute_strand_stats` | adapter | synthetic courses patched into `progress`, cards with memory states (one at mastery exactly 0.5) and reviews across two study days |
| `weak_strands` | `strands.py:weak_strands` | adapter | statistics at, above and below 30 answered, and a shortfall of exactly 8 points and one either side |
| `test_prep_board` | `lsat.py:compute_lsat_board` | adapter | patches the module's law root and `_LSAT_TRACK` with synthetic names; sections, a short path, cards of 0 and of 1 reps and suspended studied cards |
| `strands.constants` | `strands._STRAND_IDX_RE`, `_MATURE_MASTERY_THRESHOLD`; `weak_strands`' defaults | constants | none |

`law_subject` is SPEC-045's golden (`registry/spec_045.py`, adapter `under_the_synthetic_taxonomy`); ingest's A1
reads it unchanged, and this SPEC registers no case of it.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `strand_readouts` | `deck-streak-curriculum` | `migrations/009201_curriculum_strand_readouts.sql` | the strand, weak-spot and board payloads it kept as coaching values, which the import does not carry: the first recompute recomputes them | exported and erased: no row reads as pending |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09201-STRAND-INDEX-PATTERN` | `crates/curriculum/src/strands.rs` | the index token's pattern | `strands_goldens::the_strand_parse_matches_the_predecessors_golden` |
| `S09202-MATURE-AT-HALF` | `crates/curriculum/src/strands.rs` | a strand card is mature at mastery 0.5 | `strands_goldens::strand_stats_match_the_predecessors_golden` |
| `S09203-WEAK-MIN-REVIEWS` | `crates/curriculum/src/strands.rs` | the baseline's minimum of 30 answered | `strands_goldens::weak_spots_match_the_predecessors_golden` |
| `S09204-WEAK-GAP-EIGHT` | `crates/curriculum/src/strands.rs` | the shortfall of 8 points | `strands_goldens::weak_spots_match_the_predecessors_golden` |
| `S09205-DIGEST-TWO` | `crates/curriculum/src/strands.rs` | the digest's two weak spots | `strands_digest::the_digest_takes_the_first_two_weak_spots` |
| `S09206-STUDIED-IS-REPS` | `crates/curriculum/src/test_prep.rs` | studied means reps above 0, suspended included | `strands_goldens::the_test_prep_board_matches_the_predecessors_golden` |
| `S09207-SUBJECT-BEHIND-BAND` | `crates/ingest/src/law_subject.rs` | the fourth segment behind a year band | `law_subject::the_law_subject_matches_the_predecessors_golden` |
| `S09208-ONE-ROW-PER-KIND` | `migrations/009201_curriculum_strand_readouts.sql` | the key on `strand_readouts (kind, scope)` (a script row; the cargo killer) | `strands_step::the_strand_readouts_are_replaced_once_per_recompute` |
