# SPEC-046: each topic's reading is written by its persona, passes every gate, gets one repair, and is never a placeholder

- **Wave:** W1. **Issue:** #32 (epic #2). **Context(s):** `deck-streak-readings` (the seed, the form, the coverage gates, the repair rule, the reading and its telemetry); `deck-streak-coordination` (the generation use case).
- **Decided by:** ADR-012 (the parity oracle), ADR-015 (the agent, fail closed), ADR-019 (the
  readings' surface, the law primer and the language form), ADR-054 (the AI route is optional; with
  it absent, no reading is generated and nothing pages), and ADR-046 (the word target, the coverage
  gates on a persona output, the repair and the text crates).
- **Status:** promoted from `docs/specs/planned/` by the delivery that builds it, with its tests and
  `docs/red-first/SPEC-046.md` (ADR-016).

## 1. The problem, measured

- **What is ported.** The predecessor's lane (`preread-lane`) asked for a narrative primer of 800 to
  1500 words that covers every new card (`preread.py:law_narrative_primer`), refused a seed whose
  anchors were all unusable before paying for a call (`preread.py:seed_determined_failure`),
  judged the reply with six coverage gates (`preread.py:check_coverage`: complete, roster, anchors,
  words, no list markers, a well-formed body), and wrote one note per topic
  (`reading_notes.py:write_reading`). It failed loud and never wrote a placeholder, and it had no
  spend cap but full telemetry.
- **What it never had.** Its specified single repair naming the failed gate was never built, so one
  gate failure meant no reading that day. Its language form was deferred and never built; the owner
  since chose the language mentors' daily reading instead (ADR-019). Its length was fixed however few
  cards a topic had, so a topic with one new note still had to fill 800 words.
- **The owner's decisions.** A law primer scales with the topic's new-card count inside 800 to 1500
  words; there is no daily cap on readings; the Mini App is the primary surface and the vault keeps
  an archive copy; fail loud, never a placeholder.
- **What the parity oracle proves.** `preread.py:anchor_for_note` and `preread.py:is_anchor_usable`
  over synthetic note texts with markup, entities, full-width characters and short fields.
- **Prerequisites.** SPEC-045 (the day set and the topic states), SPEC-044 (the persona engine and the
  golden readings), SPEC-043 (the AI route, the runner, the gate and the verdict), SPEC-042 (the date
  tree), SPEC-020 and SPEC-021. SPEC-047 to SPEC-053 build on it.

## 2. Requirements

R1. A topic's seed is its day set's card ids, its distinct notes (each note id and its fields'
    text joined in field order) and its new-card count n; for a language topic also its new words,
    each note's configured term field with its markup removed. A new word is card text: it reaches
    the model fenced with the cards, on a `new word <i>:` line, and never in a trusted slot.
R2. The persona is the roster's template for the topic (SPEC-044). A topic whose template lacks the
    daily-reading duty ends `failed` with `form_unregistered`, with no call.
R3. The law form is an IRAC primer with the sections `reading`, `issue`, `rule`, `application` and
    `conclusion`, then `retrieval` last. Its prose (every section but `retrieval`) is asked to run
    T(n) = min(1500, 800 + 50 × (n − 1)) words (ADR-046) inside the owner's band of 800 to 1500. The
    engine writes `x-new-cards: n` and `sources` equal to the seed's note ids as the citation keys
    `n<note id>`, and a `corpus.json` holding each note's text as the passage the model was given.
R4. The language form is the mentor's daily reading, with the sections `reading`, `glosses`,
    `grammar`, `pronunciation` and `culture`, then `retrieval` last. The engine writes `x-new-words`
    equal to the seed's new words, each of which must be glossed and used in the reading; it has no
    word band.
R5. Before any call, a law seed whose every note's anchor is unusable ends `failed` with
    `anchor_unusable_all`, and a seed with no note ends `failed` with `seed_empty`.
R6. Before any call, each untrusted input (the memory and the cards, the new words included) is run
    through the gate's input class (SPEC-043 R11); a refused input ends the topic `failed` with
    `gate_failed:contract`, with no call and no attempt recorded. A reading is gated by the task's
    pack classes (SPEC-043) and by the readings' coverage gates; the first failure decides:
    1. complete: the run succeeded with a non-empty result;
    2. roster: every seed note is cited and every citation is a seed note (law: law-professors'
       `citations-resolve` over `sources`; language: language-mentors' `i1-glosses` over `x-new-words`);
    3. anchors (law): every usable anchor appears in the normalised prose, where an anchor is the
       note's text with entities decoded, markup stripped, NFKC-normalised, rail characters removed,
       whitespace collapsed and cut to 48 characters, and is usable at 8 characters or more; unusable
       anchors are excluded and reported as the advisory `anchors_partially_unverifiable:<unusable>/<total>`;
       the anchor and its usability equal the goldens of `preread.py:anchor_for_note` and
       `preread.py:is_anchor_usable`;
    4. band (law): study-duties' `reading-length`;
    5. no list markers: no line of the `reading`, `issue`, `rule`, `application` or `conclusion`
       sections (law), or of the `reading` section (language), begins a bullet or a numbered item;
       the `retrieval` and `glosses` sections are lists by their packs' contracts;
    6. contract: persona-core's `output-contract`.
R7. A topic whose first attempt fails a gate is regenerated once, with a repair instruction that
    names the failed gate and never quotes the rejected text. An own gate's finding lines are named:
    a finding line that carries a fence marker, repeats a rejected line of 16 characters or more, or
    quotes a span (`'...'` or `"..."`) found in the rejected text or holding a backslash or a quote
    character, is left out. A pack gate's finding lines are never named: the repair names the check
    that refused, in the engine's own words. The rule that lost leaves out any pack line that holds a
    colon: it does not fail closed on probe output that is not in the protocol's shape. A second failure
    stores nothing, writes nothing and ends the topic `failed` with `gate_failed:<gate>`. An
    unavailable verdict from a configured route ends the topic `failed` with
    `agent_unavailable:<cause>`, with no retry; an absent route is R16's, never a failure.
R8. Nothing is stored or written for a topic unless its output passed every gate, and no code path
    produces stand-in text; study-duties' `no-placeholder` class is in the gate.
R9. A ready reading is stored in `readings`: its identity (topic, first generated study day,
    digest), its id (the first 32 hexadecimal digits of the SHA-256 of that identity, so a
    regeneration of the same identity keeps its id), persona, gated text, word count, reading
    minutes (words divided by 200 a minute, rounded up; Chinese and Japanese counted in characters
    at language-mentors' 1.5 and 2.0 characters to a word), n, note count, covered card ids,
    generation instant and version 1. Its
    vault copy is then written by the date tree (SPEC-042) and its path recorded. A failed vault
    write keeps the reading (the Mini App is primary), records `vault_write_failed` on it, and is
    reported by the health check.
R10. A topic whose digest equals its previous ready reading's (the same new cards still queued)
    generates nothing and stays `ready` with that reading; it and every topic with no new cards,
    could not tell or paused is carried by the nightly roll-forward (SPEC-042), and a carried
    reading's carried nights rise by one.
R11. With an AI route configured, every topic with new cards gets a reading: there is no daily cap
    and no job-wide deadline that skips a topic. Topics run one after another, each run under its own
    caps (SPEC-043).
R12. Every attempt is recorded in `reading_attempts` (run, topic, study day, attempt number, repair
    gate, verdict, cause or class, turns, input and output tokens, the CLI's cost estimate, duration,
    `created_at`), kept 90 days (the predecessor's retention), and exported and erased.
R13. `privacy.json` declares the readings' processing by route (ADR-054). Only when an AI route is
    configured is a topic's card text sent to the model provider to write its reading, through that
    route (the owner's subscription proxy, for `Proxy`), with the provider named from configuration.
    Under `Absent`, the default, it declares that no card text leaves the host for a reading and
    that no reading is generated.
R14. No reading carries an exam date, a target date, a countdown or any forward-looking timeline
    (the owner's rule, with no exception): persona-core's `no-dates` is in every reading's gate, and a
    reading it refuses is repaired once and otherwise fails like any other gate.
R15. `readings` and `reading_attempts` are created `STRICT` by
    `migrations/004601_readings_and_attempts.sql`, and registered in the context map's ownership
    register (the predecessor's `preread_notes` and `preread_run_events` map onto them) and in
    readings' data-rights port.
R16. The generation reads the agent's AI route (SPEC-043) before it resolves anything. With the route
    `Absent` (ADR-054), every topic of the taxonomy ends the study day `ai_route_absent` (SPEC-045):
    no day set is resolved, no seed is built, no attempt is made or recorded in `reading_attempts`,
    no reading or vault byte is stored, nothing is alerted, and the run records `ai_route_absent` as
    its outcome. R7 and R8 stand: no stand-in text is produced, ever.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the word target is 800 at one new card, rises with n, and stops at 1500 | `the_word_target_grows_with_new_cards_inside_the_band` |
| A2 | study-duties' `reading-length` and `reading-scale` are green on the golden law readings of different n | study-duties `reading-length`, `reading-scale`; `test_the_length_and_scale_rows_are_green_on_the_goldens` |
| A3 | a law reading missing a usable anchor is refused by the anchors gate (fake runner) | `a_reading_missing_a_note_anchor_is_refused` |
| A4 | a reading whose citations or glosses differ from the seed is refused by the roster gate (fake runner) | `a_reading_whose_roster_differs_from_the_seed_is_refused` |
| A5 | a list marker in the primer's prose is refused, and the retrieval list is accepted (fake runner) | `a_list_marker_in_the_primer_prose_is_refused` |
| A6 | anchors and their usability equal the goldens of `preread.py:anchor_for_note` and `preread.py:is_anchor_usable` | `anchors_match_the_parity_golden` |
| A7 | a first gate failure is regenerated once with the failed gate and its findings named and no rejected text quoted | `a_gate_failure_is_repaired_once_naming_the_gate` |
| A8 | a second failure stores nothing, writes nothing and ends the topic `failed` with `gate_failed:<gate>` | `a_second_failure_writes_nothing_and_records_its_reason` |
| A9 | every blocking row of study-duties, learning-science, law-professors and language-mentors that applies to the daily-reading duty is green on the golden readings, and examines at least one reading | those packs' blocking rows; `test_every_blocking_reading_row_is_green_on_the_goldens` |
| A10 | every attempt is recorded with its turns, tokens, duration and verdict | `every_attempt_is_recorded_with_tokens_latency_and_verdict` |
| A11 | twelve synthetic topics with new cards each get a reading in one run: no daily cap | `every_topic_with_new_cards_gets_a_reading_with_no_daily_cap` |
| A12 | an unusable seed fails before any model call (the fake runner records none) | `an_unusable_seed_fails_before_any_model_call` |
| A13 | a failing output leaves no row in `readings` and no byte in the vault | `no_reading_is_stored_or_written_unless_every_gate_passed` |
| A14 | an unchanged day set carries its reading with no model call, and its carried nights rise by one | `an_unchanged_day_set_carries_its_reading_without_a_model_call` |
| A15 | a failed vault write keeps the reading and records `vault_write_failed` | `a_vault_write_failure_keeps_the_reading_and_records_it` |
| A16 | reading minutes are words at 200 a minute, with Chinese and Japanese counted in characters | `reading_minutes_count_words_at_200_per_minute` |
| A17 | readings' data-rights port exports and erases `readings` and `reading_attempts` | `the_readings_and_attempts_are_exported_and_erased` |
| A18 | a reading that states a date or a countdown is withheld by persona-core's `no-dates`, repaired once, and never stored (fake runner) | persona-core `no-dates`; `a_reading_with_a_date_or_countdown_is_never_delivered` |
| A19 | with the route absent, every topic ends `ai_route_absent`: no day set is resolved, the fake runner records no call, no attempt is recorded, nothing is stored or written, and no alert is raised | `an_absent_route_ends_every_topic_ai_route_absent_with_no_attempt` |

```acceptance
A1: cargo test -p deck-streak-readings --test form -- --exact the_word_target_grows_with_new_cards_inside_the_band
A2: python3 -m unittest discover -s scripts/tests -p test_reading_rows.py -k test_the_length_and_scale_rows_are_green_on_the_goldens
A3: cargo test -p deck-streak-readings --test coverage -- --exact a_reading_missing_a_note_anchor_is_refused
A4: cargo test -p deck-streak-readings --test coverage -- --exact a_reading_whose_roster_differs_from_the_seed_is_refused
A5: cargo test -p deck-streak-readings --test coverage -- --exact a_list_marker_in_the_primer_prose_is_refused
A6: cargo test -p deck-streak-readings --test coverage -- --exact anchors_match_the_parity_golden
A7: cargo test -p deck-streak-coordination --test readings_generate -- --exact a_gate_failure_is_repaired_once_naming_the_gate
A8: cargo test -p deck-streak-coordination --test readings_generate -- --exact a_second_failure_writes_nothing_and_records_its_reason
A9: python3 -m unittest discover -s scripts/tests -p test_reading_rows.py -k test_every_blocking_reading_row_is_green_on_the_goldens
A10: cargo test -p deck-streak-coordination --test readings_generate -- --exact every_attempt_is_recorded_with_tokens_latency_and_verdict
A11: cargo test -p deck-streak-coordination --test readings_generate -- --exact every_topic_with_new_cards_gets_a_reading_with_no_daily_cap
A12: cargo test -p deck-streak-coordination --test readings_generate -- --exact an_unusable_seed_fails_before_any_model_call
A13: cargo test -p deck-streak-coordination --test readings_generate -- --exact no_reading_is_stored_or_written_unless_every_gate_passed
A14: cargo test -p deck-streak-coordination --test readings_generate -- --exact an_unchanged_day_set_carries_its_reading_without_a_model_call
A15: cargo test -p deck-streak-coordination --test readings_generate -- --exact a_vault_write_failure_keeps_the_reading_and_records_it
A16: cargo test -p deck-streak-readings --test minutes -- --exact reading_minutes_count_words_at_200_per_minute
A17: cargo test -p deck-streak-readings --test rights -- --exact the_readings_and_attempts_are_exported_and_erased
A18: cargo test -p deck-streak-coordination --test readings_generate -- --exact a_reading_with_a_date_or_countdown_is_never_delivered
A19: cargo test -p deck-streak-coordination --test readings_generate -- --exact an_absent_route_ends_every_topic_ai_route_absent_with_no_attempt
```

## 3a. Acceptance criteria of the amendment

| id | criterion | decided by |
|---|---|---|
| A20 | a new word reaches the model only inside the fence, on a `new word <i>:` line, and the trusted instruction counts the words without naming one | `a_new_word_reaches_the_model_only_inside_the_fence` |
| A21 | a fence marker inside a new word does not make compose refuse the topic | `a_fence_marker_in_a_new_word_does_not_refuse_the_prompt` |
| A22 | a finding that quotes the rejected text is left out of the repair instruction | `a_finding_quoting_the_rejected_text_is_dropped_from_the_repair` |
| A23 | a finding that quotes a new word is left out of the repair instruction | `a_finding_quoting_a_new_word_is_dropped_from_the_repair` |
| A24 | each untrusted input is checked before any call, and a refused input ends the topic with no call and no attempt | `an_untrusted_input_is_checked_before_any_call` |
| A25 | an unclosed tag ends the walk of `strip_tags` and leaves the rest of the text as it is | `an_unclosed_tag_leaves_the_rest_of_the_text_as_it_is` |
| A26 | the repair slot drops an escaped span, and a finding that quotes nothing of the rejected text is kept | `no_escape_form_of_a_new_word_reaches_the_trusted_repair_slot` |
| A27 | a new word is checked with the cards before any call, and a refused word ends the topic on the contract gate with no call | `a_new_word_is_checked_before_any_call` |
| A28 | a pack gate's failure reaches the repair only as the name of the check that refused | `a_pack_finding_never_reaches_the_trusted_repair_slot` |

```acceptance
A20: cargo test -p deck-streak-coordination --test readings_trust -- --exact a_new_word_reaches_the_model_only_inside_the_fence
A21: cargo test -p deck-streak-coordination --test readings_trust -- --exact a_fence_marker_in_a_new_word_does_not_refuse_the_prompt
A22: cargo test -p deck-streak-coordination --test readings_trust -- --exact a_finding_quoting_the_rejected_text_is_dropped_from_the_repair
A23: cargo test -p deck-streak-coordination --test readings_trust -- --exact a_finding_quoting_a_new_word_is_dropped_from_the_repair
A24: cargo test -p deck-streak-coordination --test readings_generate -- --exact an_untrusted_input_is_checked_before_any_call
A25: cargo test -p deck-streak-readings --test coverage -- --exact an_unclosed_tag_leaves_the_rest_of_the_text_as_it_is
A26: cargo test -p deck-streak-coordination --test readings_trust -- --exact no_escape_form_of_a_new_word_reaches_the_trusted_repair_slot
A27: cargo test -p deck-streak-coordination --test readings_generate -- --exact a_new_word_is_checked_before_any_call
A28: cargo test -p deck-streak-readings --test repair -- --exact a_pack_finding_never_reaches_the_trusted_repair_slot
```

The class behind A28 is one rule, provenance: a pack gate's failure reaches the repair only as the
engine's words naming the check. `every_pack_class_reaches_the_repair_only_as_the_name_of_the_check`
in `crates/readings/tests/repair.rs` generates its members when it runs. They are every pack class
the engine configures, read from `ai-safety.json` and from the classes `first_failure` ranks, each
against hostile lines under the class's own name and under a forged one.
`a_gate_outcome_class_reaches_the_repair_only_as_the_name_of_the_check` in
`crates/coordination/tests/readings_trust.rs` does the same for every class the gate can report. It
reads them from source when it runs: the configured classes, and each constant the gate passes to
`failed(..)`, which are `CLASS_VOID` and `CLASS_EMPTY`. A constant the gate adds becomes a member
with no test edit. Each class also meets a failure with no finding line, and the test checks the
whole repair text, its header included.
`a_gate_outcome_class_reaches_the_prompt_only_as_the_name_of_the_check` runs the same members
through the attempt loop in `generate.rs`. There the second prompt is the first plus exactly that
repair text.

Two more tests pin the rule that the repair slot drops an escaped span:
`a_finding_quoting_an_escaped_span_is_dropped` in `crates/readings/tests/repair.rs` and
`a_finding_quoting_an_escaped_new_word_is_dropped_from_the_repair` in `readings_trust.rs`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/readings/src/seed.rs` | `deck-streak-readings` | added: the seed and the seed-determined screen |
| `crates/readings/src/form.rs` | `deck-streak-readings` | added: the word target and the law and language forms |
| `crates/readings/src/coverage.rs` | `deck-streak-readings` | added: the roster, anchor and list-marker gates |
| `crates/readings/src/repair.rs` | `deck-streak-readings` | added: the one-repair rule as a pure step |
| `crates/readings/src/reading.rs` | `deck-streak-readings` | added: the reading, its identity and its minutes |
| `crates/readings/src/attempts.rs` | `deck-streak-readings` | added: `reading_attempts` |
| `crates/readings/src/store.rs` | `deck-streak-readings` | changed: `readings` |
| `crates/readings/src/data_rights.rs` | `deck-streak-readings` | changed: the two new tables |
| `crates/readings/src/state.rs` | `deck-streak-readings` | changed: the topic states the generation ends in |
| `crates/agent/src/compose.rs` | `deck-streak-agent` | changed: the form, word-target and repair slots are trusted text, fence-checked (amendment; orchestrator ruling) |
| `crates/agent/tests/compose.rs`, `duty.rs`, `redteam.rs`, `persona.rs` | `deck-streak-agent` | changed: the new slots' tests, and the golden roster names the second law golden |
| `crates/coordination/src/maintenance.rs` | `deck-streak-coordination` | changed: the nightly upkeep prunes `reading_attempts` past their retention (amendment) |
| `crates/coordination/tests/maintenance.rs` | `deck-streak-coordination` | changed: the retention test |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: the two new tables' seeds |
| `crates/coordination/src/readings/mod.rs` | `deck-streak-coordination` | changed |
| `PRIVACY.md` | repo | changed: the two new tables and the card text sent to the model provider |
| `scripts/mutation-rows.d/S04600-S04699.json` | repo | added: the constants, the word-target bounds, the repair cap and the gate order |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed |
| `crates/readings/Cargo.toml` | `deck-streak-readings` | changed: `unicode-normalization`, `html-escape`, `unicode-segmentation` |
| `migrations/004601_readings_and_attempts.sql` | `deck-streak-readings` | added |
| `crates/readings/tests/form.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/coverage.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/minutes.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/rights.rs` | `deck-streak-readings` | changed |
| `crates/readings/tests/repair.rs` | `deck-streak-readings` | added: pins the repair's cap, its quote floor and the quote-span drop |
| `crates/readings/tests/attempts.rs` | `deck-streak-readings` | added: pins the retention's whole value |
| `crates/readings/tests/stored.rs` | `deck-streak-readings` | added: pins the reading id's shape |
| `crates/coordination/tests/readings_trust.rs` | `deck-streak-coordination` | added: the fence and the repair's trust (amendment) |
| `crates/coordination/src/readings/generate.rs` | `deck-streak-coordination` | added: the generation use case and the nightly roll-forward |
| `crates/coordination/tests/readings_generate.rs` | `deck-streak-coordination` | added |
| `agent/prompts/daily-reading.prompt.md` | agent (public) | changed: the form, the word target and the repair slots |
| `ai-safety.json` | repo | changed: the new slots and their sources |
| `agent/golden/daily-reading/law/` | agent (public) | changed: a second law golden with a larger n |
| `Cargo.toml` | workspace | changed: `[workspace.dependencies]` gains `unicode-normalization`, `html-escape`, `unicode-segmentation` (ADR-046) |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `scripts/tests/test_reading_rows.py` | repo | added |
| the box-run packs' private wiring (ADR-069) | the maintainer's | unchanged: study-duties and learning-science stay pending until drill and practice goldens exist (#46, #52) |
| `tools/parity-oracle/registry/spec_046.py` | repo | added: registers the two anchor functions (SPEC-029's registry) |
| `tools/parity-oracle/goldens/anchor_for_note.json` | repo | added |
| `tools/parity-oracle/goldens/is_anchor_usable.json` | repo | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the ownership register gains `readings` and `reading_attempts` |
| `privacy.json` | repo | changed: readings, attempts, and card text to the model provider |
| `docs/schematics/readings-generation-flow.md` | docs | existing on dev, unchanged here |
| `docs/specs/SPEC-046-readings-generation-gates-and-repair.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-046-word-target-coverage-gates-and-one-repair.md` | docs | existing on dev, changed here |
| `docs/red-first/SPEC-046.md` | docs | added |
| `changelog.d/feat-readings-046.md` | repo | added |

**Rows.** Band S04600-S04699 holds 40 rows, each proved killed. S04601 to S04603 pin the word band's
floor, ceiling and step (killed by `form::the_word_target_grows_with_new_cards_inside_the_band`).
S04604 and S04605 pin the anchor bounds (killed by `coverage::the_anchor_bounds_are_pinned`). S04606
pins the reading id's length (killed by
`minutes::the_reading_id_is_32_hex_digits_of_the_topic_the_first_day_and_the_digest`). S04607 pins
the reading rate (killed by `minutes::reading_minutes_count_words_at_200_per_minute`). S04608 and
S04609 pin the Chinese and Japanese rates (killed by
`minutes::chinese_and_japanese_count_characters_at_a_word_rate`). S04610 and S04611 pin the
attempts' retention (killed by `attempts::an_attempt_is_kept_for_exactly_ninety_whole_days`).
S04612 and S04613 pin the repair cap (killed by
`repair::a_topic_gets_the_first_attempt_and_exactly_one_repair`). S04614 pins the quoted line's
floor (killed by `repair::a_finding_that_repeats_a_long_rejected_line_is_dropped`). S04615 to S04619
pin the gate order (killed by `coverage::the_first_failure_decides_in_the_gate_order`). S04620 pins
the unavailable verdict's end (killed by
`readings_generate::an_unavailable_route_ends_the_topic_with_no_retry`). S04621 and S04622 pin the
input check (killed by `readings_generate::an_untrusted_input_is_checked_before_any_call`). S04623
pins the rejected-text drop (killed by
`repair::a_finding_quoting_a_span_of_the_rejected_text_is_dropped`). S04624 pins the count-only
instruction (killed by `form::a_language_instruction_counts_the_new_words_and_never_names_one`).
S04625 pins the unclosed tag (killed by
`coverage::an_unclosed_tag_leaves_the_rest_of_the_text_as_it_is`). S04626 pins the stored id's
shape (killed by
`stored::a_reading_id_parses_only_as_thirty_two_lowercase_hex_digits`). S04627 pins the escaped-span
drop (killed by `repair::a_finding_quoting_an_escaped_span_is_dropped`). S04628 pins the engine's
words in `named()` (killed by `repair::a_pack_finding_never_reaches_the_trusted_repair_slot`).
S04629 pins the quote-in-span drop (killed by
`repair::a_finding_quoting_a_span_that_holds_a_quote_is_dropped`). S04630 pins the contract gate's
words for the configured classes (killed by
`repair::every_pack_class_reaches_the_repair_only_as_the_name_of_the_check`). S04631 to S04633 and
S04636 to S04638 pin how a class the gate reports is named in the failure (killed by
`readings_trust::a_gate_outcome_class_reaches_the_repair_only_as_the_name_of_the_check`). S04639
and S04640 pin how the gate reports a class (killed by the same test). S04634 and S04635 pin the
repair text the attempt loop sends (killed by
`readings_trust::a_gate_outcome_class_reaches_the_prompt_only_as_the_name_of_the_check`).

## 5. What this does NOT do

- It records no read tap, measures no studied cards and grants no XP (#33).
- It takes no lock and offers no on-demand regeneration (#34).
- It schedules no nightly run (#39).
- It pages nobody for a failed topic; the health check does (#36).
- It feeds no leech or drill grade into the prompt (#133, #136).
- It shows no reading on any surface (#37, #38).
- It does not change the input class that checks a new word (#437).
- It enforces no rule that judges the drill or the practice duty: those rules examine nothing on reading goldens, and the deliveries that add those goldens enforce them (#46 for the drill coach, #52 for practice questions).

## 6. Risks

- **The model pads or truncates.** The band and the word target are gated (A1, A2), and a gate
  failure gets one repair before it fails loud.
- **Anchors fail on notes heavy with markup.** Unusable anchors are excluded and reported, and an
  all-unusable seed is refused before any call (A12).
- **A heavy day takes long with no daily cap.** Each run is capped, every attempt's duration is
  recorded (A10), and the health check pages a run that did not finish in its window (SPEC-050).
- **Card text leaves the host.** It is fenced as untrusted data, sent only through a configured
  route (the owner's proxy), never while the route is absent, and declared in the privacy policy
  (R13).
- **Two writers of the readings folder** while both run side by side. Prevented by the first live
  night's one-writer prerequisite (SPEC-053).
