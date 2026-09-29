# SPEC-095: the Echo Test, the Price of a Day Off, Bench II and the Docket run weekly on the frame

- **Wave:** W4. **Issues:** #138, #139, #141, #145 (epic #5). **Context(s):** `deck-streak-kernel`
  (`pynum` gains CPython's Mersenne Twister and `lgamma`); `deck-streak-ingest` (the instruments'
  review and preset reads); `deck-streak-insights` (the four instruments and their registry rows);
  `deck-streak-coordination` (the reads' wiring, and this week's stored reports passed to the
  Docket); the Mini App (four sections of the insights screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-090 (CPython's numeric
  semantics live once, in the kernel's `pynum`), ADR-094 (the frame: weekly after the sync's
  recompute, one latest report per instrument) and ADR-095 (the reads walk the wire format by hand
  and keep SPEC-023's scope and window).
- **Prerequisites:** SPEC-020 (the study-day rule and the offload), SPEC-023 (the read, its scope
  and its window), SPEC-029, SPEC-090 (`pynum`) and SPEC-094 (the frame, the wire walk, the
  structure reads and `instrument_reports`). **Mutation band:** `S09500-S09599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-095.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 the insights crate holds no code, and SPEC-094 (planned)
  builds the frame with Dark Fields as its only instrument. Ingest reads no review's note, note type
  or ordinal, no preset and no deck kind.
- **What is ported.** The Echo Test (`echo.py:build_echo_report` and its reader), the Price of a
  Day Off (`restday.py:build_restday_report`, its reader and its calendar sweep), Instrument Bench II
  (`bench2.py:build_preset_report`, `build_lateness_report`, `build_bakeoff_report`, their readers
  and `pipeline_layers/digests.py:_bench2_blocking`) and the Contradiction Docket
  (`docket.py:build_docket`).
- **The weekly report is W5.** All four issues depend on the weekly report (#130); as SPEC-094 set
  out, W4 computes and stores each report and shows it on the insights screen (ADR-094).
- **Traps a hand port falls into.**
  - The Echo's population is the first review-type answer (type 1, ease at least 1) of each card on
    each study day. It is contaminated when an earlier answer that day fell on another card of the
    same note; its comparator is a later-in-day answer with no earlier answer on its note. Cells are
    the (note type name, template name) pair, falling back to ordinal 0 for an unresolved cloze
    ordinal, and never the ordinal itself.
  - The Mantel-Haenszel sums are plain additions in the order each cell was first seen, so a port
    that pools in sorted order differs in the last bits. A cell under 20 in either arm is dropped,
    and a gap under 4.0 points or an interval spanning 0 is null with the headline withheld.
  - The rest-day study days come from every study event (types 0 to 3), but only type-1 answers
    fill a cell; days off are the gap minus one, capped at "≥5". The four calendars are the
    kernel's study-day rule and three at rollover 0 with the rule's offset, its negation and 0; the
    sweep always runs all four, even when two coincide, and the generator is consumed for each.
  - One generator is seeded `20260803` per report and consumed in the predecessor's order: cells by
    count descending (ties in first-seen order), then the four calendars, then the dose's samples
    before the baseline's. Each sample draws `choices(episodes, k=m)`, which CPython computes as
    `floor(random() * m)`.
  - Cells past the 15 the budget allows are "budget exceeded", never a number.
  - The Fisher test sums `exp` of `lgamma` differences, with a relative tolerance of `1e-7`, and
    CPython's `lgamma` is its own, not the platform's. The pair's p is Bonferroni-adjusted over
    `q(q-1)/2` pairs.
  - A deck with reviews in the band but no name is skipped and counted; a preset whose config
    fails to decode is "unknown", never "empty".
  - The lateness band compares the gap to the previous review of the same card with 1.5, 3 and 6
    times the last interval; a sub-day or first review is excluded, not banded.
  - The Docket reads no collection. It fires on the Echo's positive, non-null gap beside a rest-day
    band whose last usable cell passes less than its first, and on a lateness band at 80% or more
    beside a Bake-Off whose worst deck is at 50% or less.
- **Corrections to the issues.**
  - #139 asks for a test of both calendar sources. DeckStreak's study-day calendar has one source,
    the settings (SPEC-020 R2); a test asserts the sweep's four calendars instead.
  - The predecessor's rest-day reader reads the whole log of every deck; ADR-095 keeps SPEC-023's
    scope and window, so the rest-day read joins the cards to know each answer's deck.
- **What the parity oracle proves.** The generator and `lgamma` over edge seeds and arguments; each
  report over synthetic rows, cold, null, resolved and failed; the lateness banding over a
  synthetic collection; the Fisher test; and the constants.
- **Prerequisites.** SPEC-020, SPEC-023, SPEC-029, SPEC-090 and SPEC-094, as the header names them.

## 2. Requirements

The numbers (ADR-090)

R1. `pynum` gains CPython's Mersenne Twister, seeded from an integer as `random.Random(n)` seeds it,
    its 53-bit `random()` and its unweighted `choices`, equal to `goldens/pynum_random.json`; and
    `lgamma`, equal to `goldens/pynum_lgamma.json` (CPython's `math.lgamma`).

The reads (ADR-095)

R2. `crates/ingest/src/review_reads.rs` reads, from the private copy and read-only, keeping
    SPEC-023's scope and counting only answers after the window's floor: each review-type answer with
    its card, note, note type and ordinal; each study event's id, ease, last interval and type; the
    lateness bands' counts and passes; the in-band review-type answers and passes of each home deck;
    and the presets, the deck kinds and each home deck's card count. Each read returns its rows and
    the name of every read that failed, and the preset and deck reads follow SPEC-094 R4's rule for
    `unicase` name columns.
R3. The lateness read bands each answer against its card's previous review over the whole scoped log,
    equal to `goldens/lateness_rows.json` (`bench2.py:read_lateness_rows` over a synthetic
    collection), so the first answer inside the window is never read as having no prior.

The Echo Test (#138)

R4. The Echo's report equals `goldens/echo_report.json` (`echo.py:build_echo_report`), with the note
    type and template names from SPEC-094's structure read and the study day from the kernel's rule.
R5. Its constants (20 per arm, 4.0 points, 5 first-of-day answers per note type, 5 offenders and
    z 1.96) equal `goldens/echo.constants.json`, and the offender list never holds more than 5.

The Price of a Day Off (#139)

R6. The rest-day report equals `goldens/restday_report.json` (`restday.py:build_restday_report`),
    and the four calendars equal `goldens/restday_calendars.json`
    (`restday.py:_candidate_calendars`) built from the kernel's study-day rule.
R7. Its constants (200 answers, 20 episodes, the six days-off labels, the three bands, 2.0 points,
    2000 resamples, the seed, 2.5 and 97.5, 4 calendars and 15 cells) equal
    `goldens/restday.constants.json`.

Instrument Bench II (#141)

R8. The Preset Audit equals `goldens/preset_report.json` (`bench2.py:build_preset_report`), each
    preset's parameter count and each deck's preset read through SPEC-094's wire walk
    (`bench2.py:_decode_fsrs_param_count`, `_decode_deck_config_id`).
R9. The Lateness Ceiling equals `goldens/lateness_report.json` (`bench2.py:build_lateness_report`).
R10. The Bake-Off equals `goldens/bakeoff_report.json` (`bench2.py:build_bakeoff_report`), and its
     exact test equals `goldens/fisher_p.json` (`bench2.py:_fisher_two_sided_p`).
R11. Bench II is one weekly instrument whose three parts build in one run; a part that fails carries
     its failure and the other two still build (`bench2.py:instrument_failure_reason`). Its constants
     (5.0%, 1.5, 3 and 6, the 1-6 day band, 20 per deck, 0.05 and 4.0 points) equal
     `goldens/bench2.constants.json`.

The Contradiction Docket (#145)

R12. The Docket is a weekly instrument that reads no collection. Its registry row follows the three
     it folds, and coordination passes it the stored Echo, rest-day and Bench II reports of the same
     run; its report equals `goldens/docket.json` (`docket.py:build_docket`). With no contradiction it
     holds no entry, a missing or failed source is named as unavailable, and a failed fold is a
     failure, never an empty docket.

The surfaces

R13. The insights screen shows each report as a section. The Echo gives its pooled gap and interval,
     or its null reason with no headline, its offenders, and the advice to turn on Anki's option to
     bury review siblings, which the owner changes himself. The rest-day section carries the
     "observational, not causal" label, gives a dose as a number only when resolved and as
     UNRESOLVED with its range otherwise, and names each cell under its floor with its count. Bench
     II's parts each render their own failure line. The Docket quotes both sides without a verdict,
     shows nothing when empty, and renders its failure line when failed.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the generator and `choices` equal CPython's for every golden seed | `the_generator_matches_cpythons_golden` |
| A2 | `lgamma` equals CPython's for every golden argument | `lgamma_matches_cpythons_golden` |
| A3 | the lateness banding over a synthetic collection equals the predecessor's reader | `the_lateness_banding_matches_the_predecessors_golden` |
| A4 | the first answer in the window keeps its previous review from before the floor | `the_first_answer_in_the_window_keeps_its_prior` |
| A5 | every review read keeps the scope and the window | `review_reads_keep_the_scope_and_the_window` |
| A6 | the Echo's report equals the golden | `the_echo_report_matches_the_predecessors_golden` |
| A7 | the Echo's constants equal the predecessor's | `the_echo_constants_equal_the_predecessors` |
| A8 | the offender list never holds more than 5 note types | `the_offender_list_holds_at_most_five` |
| A9 | the rest-day report equals the golden | `the_restday_report_matches_the_predecessors_golden` |
| A10 | the sweep's four calendars equal the golden, built from the kernel's rule | `the_four_calendars_come_from_the_study_day_rule` |
| A11 | the rest-day constants equal the predecessor's | `the_restday_constants_equal_the_predecessors` |
| A12 | the Preset Audit equals the golden | `the_preset_audit_matches_the_predecessors_golden` |
| A13 | the Lateness Ceiling equals the golden | `the_lateness_ceiling_matches_the_predecessors_golden` |
| A14 | the Bake-Off equals the golden | `the_bake_off_matches_the_predecessors_golden` |
| A15 | the exact test equals the golden | `the_fisher_test_matches_the_predecessors_golden` |
| A16 | one failing part leaves the other two built | `one_failing_part_leaves_the_other_two_built` |
| A17 | Bench II's constants equal the predecessor's | `the_bench_constants_equal_the_predecessors` |
| A18 | the Docket equals the golden | `the_docket_matches_the_predecessors_golden` |
| A19 | the Docket is empty without a contradiction and a failed fold is a failure | `the_docket_is_empty_without_a_contradiction_and_fails_loudly` |
| A20 | the Docket folds the reports stored by the same run | `the_docket_folds_the_reports_of_the_same_run` |
| A21 | an unresolved dose renders UNRESOLVED with its range, under the label | `renders an unresolved dose with its range under the label` |
| A22 | a null Echo withholds its headline | `withholds the headline of a null echo` |
| A23 | a failed Bench II part renders its line beside the other two | `renders a failed part beside the other two` |
| A24 | a failed Docket renders a failure, never an empty docket | `renders a failed docket as a failure` |
| A25 | each instrument is passed its reads: the Echo the review-type answers with the note type and template names of SPEC-094's structure read and the kernel's study-day rule, the rest day the study events and the same study-day rule, and Bench II the presets, the deck kinds, each home deck's card count, the lateness bands and the in-band answers | `each_instrument_is_passed_its_reads` |

```acceptance
A1: cargo test -p deck-streak-kernel --test pynum_random -- --exact the_generator_matches_cpythons_golden
A2: cargo test -p deck-streak-kernel --test pynum_random -- --exact lgamma_matches_cpythons_golden
A3: cargo test -p deck-streak-ingest --test review_reads -- --exact the_lateness_banding_matches_the_predecessors_golden
A4: cargo test -p deck-streak-ingest --test review_reads -- --exact the_first_answer_in_the_window_keeps_its_prior
A5: cargo test -p deck-streak-ingest --test review_reads -- --exact review_reads_keep_the_scope_and_the_window
A6: cargo test -p deck-streak-insights --test echo -- --exact the_echo_report_matches_the_predecessors_golden
A7: cargo test -p deck-streak-insights --test echo -- --exact the_echo_constants_equal_the_predecessors
A8: cargo test -p deck-streak-insights --test echo -- --exact the_offender_list_holds_at_most_five
A9: cargo test -p deck-streak-insights --test restday -- --exact the_restday_report_matches_the_predecessors_golden
A10: cargo test -p deck-streak-insights --test restday -- --exact the_four_calendars_come_from_the_study_day_rule
A11: cargo test -p deck-streak-insights --test restday -- --exact the_restday_constants_equal_the_predecessors
A12: cargo test -p deck-streak-insights --test bench_ii -- --exact the_preset_audit_matches_the_predecessors_golden
A13: cargo test -p deck-streak-insights --test bench_ii -- --exact the_lateness_ceiling_matches_the_predecessors_golden
A14: cargo test -p deck-streak-insights --test bench_ii -- --exact the_bake_off_matches_the_predecessors_golden
A15: cargo test -p deck-streak-insights --test bench_ii -- --exact the_fisher_test_matches_the_predecessors_golden
A16: cargo test -p deck-streak-insights --test bench_ii -- --exact one_failing_part_leaves_the_other_two_built
A17: cargo test -p deck-streak-insights --test bench_ii -- --exact the_bench_constants_equal_the_predecessors
A18: cargo test -p deck-streak-insights --test docket -- --exact the_docket_matches_the_predecessors_golden
A19: cargo test -p deck-streak-insights --test docket -- --exact the_docket_is_empty_without_a_contradiction_and_fails_loudly
A20: cargo test -p deck-streak-coordination --test docket_step -- --exact the_docket_folds_the_reports_of_the_same_run
A21: pnpm exec vitest run web/app/src/lib/insights/RestDaySection.test.ts -t "renders an unresolved dose with its range under the label"
A22: pnpm exec vitest run web/app/src/lib/insights/EchoSection.test.ts -t "withholds the headline of a null echo"
A23: pnpm exec vitest run web/app/src/lib/insights/BenchSection.test.ts -t "renders a failed part beside the other two"
A24: pnpm exec vitest run web/app/src/lib/insights/DocketSection.test.ts -t "renders a failed docket as a failure"
A25: cargo test -p deck-streak-coordination --test docket_step -- --exact each_instrument_is_passed_its_reads
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges this over the committed tree and posts its
verdict on the pull request as the `box/packs` status. It has no line in the acceptance fence,
because no public test can run a pack's row. This SPEC adds no table: the reports stay in SPEC-094's
`instrument_reports`. The accessibility pack stays enforced; no check is deferred or lifted for this
delivery, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over every file under `web/app/src/lib/insights/`: the four new sections pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/pynum.rs` | `deck-streak-kernel` | changed: CPython's Mersenne Twister, `choices` and `lgamma` (ADR-090) |
| `crates/kernel/tests/pynum_random.rs` | `deck-streak-kernel` | added: A1, A2 |
| `crates/ingest/src/review_reads.rs` | `deck-streak-ingest` | added: the instruments' review and preset reads |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/review_reads.rs` | `deck-streak-ingest` | added: A3 to A5 |
| `crates/insights/src/echo.rs` | `deck-streak-insights` | added: the Echo Test |
| `crates/insights/src/restday.rs` | `deck-streak-insights` | added: the Price of a Day Off |
| `crates/insights/src/bench.rs` | `deck-streak-insights` | added: Bench II's three parts |
| `crates/insights/src/docket.rs` | `deck-streak-insights` | added: the Docket |
| `crates/insights/src/registry.rs` | `deck-streak-insights` | changed: four weekly rows, the Docket's last |
| `crates/insights/src/lib.rs` | `deck-streak-insights` | changed: the modules |
| `crates/insights/tests/echo.rs` | `deck-streak-insights` | added: A6 to A8 |
| `crates/insights/tests/restday.rs` | `deck-streak-insights` | added: A9 to A11 |
| `crates/insights/tests/bench_ii.rs` | `deck-streak-insights` | added: A12 to A17 |
| `crates/insights/tests/docket.rs` | `deck-streak-insights` | added: A18, A19 |
| `crates/coordination/src/instruments.rs` | `deck-streak-coordination` | changed: the new reads, and the stored reports passed to the Docket |
| `crates/coordination/tests/docket_step.rs` | `deck-streak-coordination` | added: A20, A25 |
| `web/app/src/lib/insights/EchoSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/EchoSection.test.ts` | miniapp | added: A22 |
| `web/app/src/lib/insights/RestDaySection.svelte` | miniapp | added |
| `web/app/src/lib/insights/RestDaySection.test.ts` | miniapp | added: A21 |
| `web/app/src/lib/insights/BenchSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/BenchSection.test.ts` | miniapp | added: A23 |
| `web/app/src/lib/insights/DocketSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/DocketSection.test.ts` | miniapp | added: A24 |
| `web/app/src/lib/insights/insights.ts` | miniapp | changed: the four reports' types |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `tools/parity-oracle/registry/spec_095.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/pynum_random.json` | repo | added: CPython's `random.Random`, `random` and `choices` (adapter; seeds 0, 1, `20260803` and one past 32 bits) |
| `tools/parity-oracle/goldens/pynum_lgamma.json` | repo | added: CPython's `math.lgamma` (adapter; small, integral and large arguments) |
| `tools/parity-oracle/goldens/lateness_rows.json` | repo | added: the golden of `bench2.py:read_lateness_rows` (adapter; a temporary synthetic collection) |
| `tools/parity-oracle/goldens/echo_report.json` | repo | added: the golden of `echo.py:build_echo_report` (adapter; synthetic answers, names and calendars) |
| `tools/parity-oracle/goldens/echo.constants.json` | repo | added: the Echo's constants (constants) |
| `tools/parity-oracle/goldens/restday_report.json` | repo | added: the golden of `restday.py:build_restday_report` (adapter; synthetic study events and calendars) |
| `tools/parity-oracle/goldens/restday_calendars.json` | repo | added: the golden of `restday.py:_candidate_calendars` (adapter) |
| `tools/parity-oracle/goldens/restday.constants.json` | repo | added: the rest-day constants (constants) |
| `tools/parity-oracle/goldens/preset_report.json` | repo | added: the golden of `bench2.py:build_preset_report` (adapter; synthetic blobs encoded in the adapter) |
| `tools/parity-oracle/goldens/lateness_report.json` | repo | added: the golden of `bench2.py:build_lateness_report` (adapter) |
| `tools/parity-oracle/goldens/bakeoff_report.json` | repo | added: the golden of `bench2.py:build_bakeoff_report` (adapter; synthetic decks, one unnamed) |
| `tools/parity-oracle/goldens/fisher_p.json` | repo | added: the golden of `bench2.py:_fisher_two_sided_p` (function) |
| `tools/parity-oracle/goldens/bench2.constants.json` | repo | added: Bench II's constants (constants) |
| `tools/parity-oracle/goldens/docket.json` | repo | added: the golden of `docket.py:build_docket` (adapter; reports built by their own builders, each entry recorded as its pair and numbers) |
| `scripts/mutation-rows.d/S09500-S09599.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-095-the-echo-test-the-price-of-a-day-off-bench-ii-and-the-docket-run-weekly-on-the-frame.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-095.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no weekly report and splices no block into one; the weekly report reads the stored
  reports (#130).
- It revives neither the Optimiser Ledger nor the Preset Census, each inert in v9 and waiting for
  the owner's decision (#181).
- It writes no deck option: turning on sibling burying is the owner's change in Anki (#138).
- It serves no instrument report to the agent's machine read tool (#157).
- It imports none of the predecessor's instrument reports; the first weekly run computes them (#61).

## 6. Risks

- **A report differs from the predecessor's in side by side,** because the reads keep the scope
  and the window. Accepted by ADR-095; the side-by-side verification compares within the scope and
  the window (#62).
- **The pooled gap drifts in the last bits** from a sorted pooling order. Detected by A6, whose
  golden holds cells in an order that changes the sum.
- **The bootstrap's intervals drift** from a generator consumed out of order. Detected by A9, and
  by A1 for the generator itself.
- **A platform `lgamma` moves a p across the alpha.** Detected by A2 and A15.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_095.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `pynum_random` | CPython's `random.Random`, `random` and `choices` | adapter | sequences of draws and `choices` over short lists from four seeds |
| `pynum_lgamma` | CPython's `math.lgamma` | adapter | arguments from 1 to `10**6`, integral and not |
| `lateness_rows` | `bench2.py:read_lateness_rows` | adapter | a temporary collection file with synthetic review logs at each ratio's edge, sub-day and first reviews |
| `echo_report` | `echo.py:build_echo_report` | adapter | synthetic answers, a template-name map and a `CollectionConfig` from the case, and an arm with 19, 20 and 21 answers in a cell |
| `restday_report` | `restday.py:build_restday_report` | adapter | synthetic study events and a `CollectionConfig`, resolved, unresolved, suppressed and cold |
| `restday_calendars` | `restday.py:_candidate_calendars` | adapter | configurations with positive, negative and zero offsets |
| `preset_report` | `bench2.py:build_preset_report` | adapter | preset and deck blobs encoded in the adapter, one malformed |
| `lateness_report` | `bench2.py:build_lateness_report` | adapter | band rows with a failed read and none |
| `bakeoff_report` | `bench2.py:build_bakeoff_report` | adapter | per-deck rows and names, one deck unnamed, a surviving pair and a null |
| `fisher_p` | `bench2.py:_fisher_two_sided_p` | function | none: tables at the 20 floor, identical and extreme, and a table whose mirror image ties the observed probability to the last bits |
| `docket` | `docket.py:build_docket` | adapter | the three reports from their builders over synthetic rows, a source missing, a worst deck at exactly 50% and one just above it, and each entry recorded as its pair and the numbers it quotes |
| `echo.constants`, `restday.constants`, `bench2.constants` | the modules' constants | constants | none |

## 8. Tables and the v9 import

This SPEC adds no table. Every report is stored in SPEC-094's `instrument_reports` under the
`research-instruments` category, and none is imported (#61).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09501-CHOICES-FLOOR` | `crates/kernel/src/pynum.rs` | `choices` takes the floor of `random() * n` | `pynum_random::the_generator_matches_cpythons_golden` |
| `S09502-FIRST-IN-WINDOW` | `crates/ingest/src/review_reads.rs` | the previous review is read before the window's floor is applied | `review_reads::the_first_answer_in_the_window_keeps_its_prior` |
| `S09503-MH-FLOOR` | `crates/insights/src/echo.rs` | a cell under 20 in either arm is dropped | `echo::the_echo_report_matches_the_predecessors_golden` |
| `S09504-OFFENDER-CAP` | `crates/insights/src/echo.rs` | the offender list's 5 | `echo::the_offender_list_holds_at_most_five` |
| `S09505-DAYS-OFF` | `crates/insights/src/restday.rs` | days off are the gap minus one | `restday::the_restday_report_matches_the_predecessors_golden` |
| `S09506-SWEEP-NEGATES` | `crates/insights/src/restday.rs` | the third calendar negates the offset | `restday::the_four_calendars_come_from_the_study_day_rule` |
| `S09507-FISHER-TOLERANCE` | `crates/insights/src/bench.rs` | the `1e-7` tolerance on the observed table | `bench_ii::the_fisher_test_matches_the_predecessors_golden` |
| `S09508-PART-ISOLATION` | `crates/insights/src/bench.rs` | a failed part never stops the others | `bench_ii::one_failing_part_leaves_the_other_two_built` |
| `S09509-DOCKET-WORST-DECK` | `crates/insights/src/docket.rs` | the worst deck at 50% or less | `docket::the_docket_matches_the_predecessors_golden` |
