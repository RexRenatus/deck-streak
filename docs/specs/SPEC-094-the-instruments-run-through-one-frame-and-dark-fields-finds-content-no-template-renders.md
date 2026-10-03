# SPEC-094: the instruments run through one frame, and Dark Fields finds content no template renders

- **Wave:** W4. **Issue:** #142 (epic #5), and the frame every W4 instrument (#137 to #151) runs
  through. **Context(s):** `deck-streak-kernel` (the owner's note conventions, ADR-096);
  `deck-streak-ingest` (the structure reads and the wire walk, ADR-095); `deck-streak-insights`
  (the instrument port, the registry, Dark Fields); `deck-streak-coordination` (the instruments
  step, the on-demand run, `instrument_reports`); `deck-streak-api` and the Mini App (the insights
  routes and screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-085 (charts render on the
  client), ADR-094 (the instruments run weekly after the sync or on demand, one at a time on the host, and coordination stores each one's latest report), ADR-095 (ingest walks the protobuf wire format by
  hand) and ADR-096 (the owner's note conventions are private configuration the kernel loads).
- **Prerequisites:** SPEC-020 (the offload), SPEC-023 (the read and its scope), SPEC-027 (the
  sync's study day), SPEC-029 and SPEC-071 (the recompute the step follows). **Mutation band:**
  `S09400-S09499`.
- **Status:** delivered by #403, which moved it from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-094.md` (ADR-016). ~~planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-094.md` (ADR-016).~~

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 `crates/insights/src/lib.rs` holds the crate's doc and
  no code; its line on the context map reads "research instruments, chart series" and depends on
  the kernel and ingest only. Ingest reads cards, study reviews and deck names
  (`reader.rs:CollectionData`) and nothing of a collection's structure: no template, field, preset,
  deck kind or note content.
- **Fifteen instruments share one shape.** The predecessor's weekly instruments (the Illusion
  Ledger, the Echo Test, the Price of a Day Off, the Hanzi Dividend, Instrument Bench II, Dark
  Fields, Dead Air, the Runway and the Docket) each run as a blocking call behind one offload
  submission (`pipeline_layers/digests.py:_darkfields_blocking` and its siblings) and splice a block
  into the weekly report; the on-demand ones (`/fluency`, `/divest`, `/tilt`, `/hand`, `/park` and
  `/otherhand`) run the same way from a command. Each reader returns its rows beside the reads that
  failed, and a report with a failed read says so rather than claiming a clean result
  (`darkfields.py:_failure_line`).
- **The weekly report is W5.** Every weekly instrument's issue depends on the weekly report (#130),
  which is wave 5. W4 therefore computes and stores each report, and shows it in an insights screen;
  #130 later reads the stored report (ADR-094).
- **Structure lives in protobuf blobs.** A template's front and back formats are fields 1 and 2 of
  `templates.config`; the predecessor walks the wire format with a pure-stdlib reader rather than
  Anki's schema (`darkfields.py:_iter_fields`, `_read_varint`), and Bench II walks
  `deck_config.config` and `decks.kind` the same way (ADR-095).
- **The owner's note layout is private.** The direction table (`direction.py:_TEMPLATE_RULES`,
  `_NOTETYPE_SUBSTRING_RULES`), the transfer instrument's field names
  (`transfer.py:_EXCLUDED_FIELD_NAMES`, `_CHOICE_FIELD_PREFIX`, `_RANK_FIELD_NAME`,
  `_MEANING_FIELD_NAMES`) and the Can-Do field (`cando.py:_CAN_DO_FIELD`) name the owner's note
  types and fields. This SPEC loads them from one private file (ADR-096); SPEC-096 and SPEC-099
  read them.
- **What is ported here.** Dark Fields (#142): `darkfields.py:build_dark_fields_report`, its
  normalisation (`_normalize_token`, `_extract_config_tokens_ex`) and its reads
  (`read_template_configs`, `read_declared_fields`, `read_reviewed_nids`,
  `read_field_presence_counts`).
- **Traps a hand port falls into.**
  - A token's leading `#`, `/` or `^` is stripped with nothing further; otherwise the RIGHTMOST
    `:`-separated segment is the field (`{{text:cloze:Text}}` is `Text`); `FrontSide`, `Tags`,
    `Type`, `Deck`, `Subdeck` and `Card` are never fields.
  - A dark field is populated on at least 3 REVIEWED notes; the list shows 40 at most and the
    unparseable note types 20 at most.
  - A template whose config is non-empty and fails to decode makes its note type unparseable,
    never "no fields referenced", which would call every field dark.
  - Zero dark fields is a checked result stated as such; a report with a failed read states the
    failure and never "all clear"; a collection with no reviewed note is cold, not clean.
  - The reviewed-note read is a set of note ids over the whole review log, never the reviews
    themselves, so it holds no review in memory and needs no window.
  - The field-presence read goes 400 notes at a time (`transfer.py:NOTES_BATCH_SIZE`), reducing
    each batch to counts before the next, so no note's content is held past its batch.
  - Names shown come from the collection, so control characters are replaced before display
    (`darkfields.py:_safe_name`).
- **What the parity oracle proves.** The wire walk over synthetic blobs (a truncated varint, an
  unknown wire type, a length past the end), the token normalisation, and Dark Fields' report over
  synthetic note types, templates, fields and presence counts, cold, clean, dark and unparseable.
- **Prerequisites.** SPEC-020, SPEC-023, SPEC-027, SPEC-029 and SPEC-071, as the header names them.

## 2. Requirements

The note conventions (ADR-096)

R1. The kernel reads `DECKSTREAK_CONVENTIONS_FILE`, a private JSON file of schema
    `deckstreak.conventions.v1`, once at start into `Conventions`: the direction rules (the
    pair, template and note-type rules and the exempt tokens), the transfer fields (the excluded
    names, the choice prefix, the rank field and the meaning fields) and the Can-Do field. An
    unreadable or malformed file refuses start, naming the setting and never a value; unset, there
    are no conventions and the instruments that need them report that they are not configured.
    `deploy/config/conventions.example.json` shows the shape with neutral values.
R2. A direction rule whose matched name contains a bare forbidden token (recall, recognition,
    recognize) and is not on the file's exempt list refuses start, as the predecessor's audit of
    `direction.py:EXEMPT_TOKEN_SUBSTRINGS` refuses it; the forbidden tokens are code, not
    configuration.

The structure reads (ADR-095)

R3. `crates/ingest/src/wire.rs` walks protobuf wire format (varints, fixed and length-delimited
    fields) with no schema, equal to `goldens/wire_walk.json` (`darkfields.py:_iter_fields`), and a
    truncation or an unknown wire type is an error, never a partial result.
R4. `crates/ingest/src/structure.rs` reads, from the private copy and read-only: each template's
    note type, ordinal, names and front and back formats; each note type's declared fields; the set
    of reviewed note ids; and each field's count of reviewed notes with content, 400 notes at a
    time. Each read returns its rows and the name of every read that failed, and keeps SPEC-023's
    scope: only note types of cards in scope are read. Anki collates the name columns of templates,
    fields and note types `unicase`; as SPEC-023's read does, no read registers a collation or
    orders, groups or seeks on such a column, so a full scan is complete without it.
R5. Every name a read returns has its control characters replaced before it leaves ingest, equal to
    `goldens/safe_name.json` (`darkfields.py:_safe_name`).

The frame (ADR-094)

R6. `crates/insights/src/instrument.rs` defines the instrument port: an instrument has an id, a
    cadence (weekly or on demand), the reads it needs, and a pure build from those reads to a
    serialisable report that carries its failed reads. `crates/insights/src/registry.rs` holds one
    row per instrument; a row marked inert never runs, and reviving it is that one row (ADR-094).
R7. After each sync's recompute, coordination's instruments step runs each weekly instrument whose
    stored report is absent or at least 7 study days older than the sync's study day. Instruments
    run one at a time on the host through the kernel's offload (R8), named by their id, and a failure of one is
    recorded as its report's failed read and never stops the next.
R8. An on-demand run (the route below, or an instrument's command from a later SPEC) runs one instrument through the same offload. Every instrument run, weekly or on demand and in any role process, first takes an exclusive `flock` on `<state directory>/instruments.lock` without waiting (the collection lock's pattern, SPEC-022 R7, released by an explicit unlock), so no two instruments read at once on the host: an on-demand request that finds it held is answered that a run is in progress and starts nothing, and a weekly turn that finds it held leaves the instrument due for the next sync's step.
R9. `instrument_reports` holds the latest report per instrument (the instrument id, the study day,
    the report's schema version and its JSON), replaced in one write. It is a `STRICT` table with
    `created_at`, created by `migrations/009401_coordination_instrument_reports.sql`; it is
    registered in the context map's register of DeckStreak's own tables, declared in `privacy.json`
    (the category `research-instruments`), given a line in `PRIVACY.md`, and exported and erased by
    coordination's data-rights port; the symmetry test seeds it.

Dark Fields (#142)

R10. Dark Fields' report equals `goldens/dark_fields.json`
     (`darkfields.py:build_dark_fields_report`), and a template's referenced fields equal
     `goldens/dark_fields_tokens.json` (`darkfields.py:_extract_config_tokens_ex`).
R11. Dark Fields is a weekly instrument; its constants (3 notes, 40 shown, 20 unparseable shown,
     the six special names, the section prefixes and the two template field numbers) equal
     `goldens/dark_fields.constants.json`.

The surfaces

R12. `GET /api/insights` lists each live instrument with its cadence and its stored report's study
     day; `GET /api/insights/{id}` serves its stored report; `POST /api/insights/{id}/run` runs an
     on-demand instrument. All three answer the owner's session only; any other caller is answered
     401 or 403 with no data; a missing report reads as pending.
R13. The Mini App's `/insights` screen shows each stored report as a section. A report with a
     failed read renders its failure line and never an all-clear; Dark Fields' section states zero
     dark fields as a checked result, and lists each dark field with its note type and its count as
     text.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a well-formed conventions file loads into one `Conventions` value | `the_conventions_file_loads_into_one_value` |
| A2 | a malformed conventions file refuses start, naming the setting and no value | `a_malformed_conventions_file_refuses_start` |
| A3 | a direction rule naming a forbidden token and not on the exempt list refuses start | `a_forbidden_direction_label_refuses_start` |
| A4 | the wire walk equals the golden of `darkfields.py:_iter_fields`, errors included | `the_wire_walk_matches_the_predecessors_golden` |
| A5 | a failed structure read is returned by name beside the rows that did read | `a_failed_structure_read_is_named` |
| A6 | the presence read holds no more than 400 notes' content at once | `the_presence_read_reduces_each_batch_of_400` |
| A7 | a note type used only outside the scope is never read | `structure_reads_keep_the_scope` |
| A8 | a returned name's control characters are replaced as `darkfields.py:_safe_name` does | `names_are_made_safe_as_the_predecessor_does` |
| A9 | Dark Fields' report equals the golden of `darkfields.py:build_dark_fields_report` | `dark_fields_match_the_predecessors_golden` |
| A10 | a template's referenced fields equal the golden of `_extract_config_tokens_ex` | `template_tokens_match_the_predecessors_golden` |
| A11 | every Dark Fields constant equals `goldens/dark_fields.constants.json` | `the_dark_fields_constants_equal_the_predecessors` |
| A12 | every instrument in the registry has one row, and an inert row never runs | `an_inert_instrument_never_runs` |
| A13 | a weekly instrument runs after the sync once in seven study days, and not again sooner | `a_weekly_instrument_runs_once_in_seven_study_days` |
| A14 | one instrument's failure is stored as its failed read and the next still runs | `one_failure_never_stops_the_next_instrument` |
| A15 | a run requested while one runs, in the same or another role process, returns without waiting for the holder, starts nothing and says a run is in progress | `a_run_while_one_runs_starts_nothing` |
| A16 | a second report replaces its instrument's first in one write | `a_report_replaces_its_instruments_previous_one` |
| A17 | `instrument_reports` is exported and erased by coordination's port | `the_instrument_reports_are_exported_and_erased` |
| A18 | the insights routes answer the owner and refuse every other caller with no data | `the_insights_routes_answer_only_the_owner` |
| A19 | a section with a failed read renders the failure line, never an all-clear | `renders a failed read as a failure line` |
| A20 | Dark Fields' section states zero dark fields as a checked result | `states zero dark fields as a checked result` |
| A21 | Dark Fields is passed R4's reads: each template's note type, ordinal, names and front and back formats, each note type's declared fields, the set of reviewed note ids and each field's count of reviewed notes with content | `dark_fields_is_passed_its_reads` |

```acceptance
A1: cargo test -p deck-streak-kernel --test conventions -- --exact the_conventions_file_loads_into_one_value
A2: cargo test -p deck-streak-kernel --test conventions -- --exact a_malformed_conventions_file_refuses_start
A3: cargo test -p deck-streak-kernel --test conventions -- --exact a_forbidden_direction_label_refuses_start
A4: cargo test -p deck-streak-ingest --test structure -- --exact the_wire_walk_matches_the_predecessors_golden
A5: cargo test -p deck-streak-ingest --test structure -- --exact a_failed_structure_read_is_named
A6: cargo test -p deck-streak-ingest --test structure -- --exact the_presence_read_reduces_each_batch_of_400
A7: cargo test -p deck-streak-ingest --test structure -- --exact structure_reads_keep_the_scope
A8: cargo test -p deck-streak-ingest --test structure -- --exact names_are_made_safe_as_the_predecessor_does
A9: cargo test -p deck-streak-insights --test dark_fields -- --exact dark_fields_match_the_predecessors_golden
A10: cargo test -p deck-streak-insights --test dark_fields -- --exact template_tokens_match_the_predecessors_golden
A11: cargo test -p deck-streak-insights --test dark_fields -- --exact the_dark_fields_constants_equal_the_predecessors
A12: cargo test -p deck-streak-insights --test registry -- --exact an_inert_instrument_never_runs
A13: cargo test -p deck-streak-coordination --test instruments_step -- --exact a_weekly_instrument_runs_once_in_seven_study_days
A14: cargo test -p deck-streak-coordination --test instruments_step -- --exact one_failure_never_stops_the_next_instrument
A15: cargo test -p deck-streak-coordination --test instruments_step -- --exact a_run_while_one_runs_starts_nothing
A16: cargo test -p deck-streak-coordination --test instrument_reports -- --exact a_report_replaces_its_instruments_previous_one
A17: cargo test -p deck-streak-coordination --test instrument_reports -- --exact the_instrument_reports_are_exported_and_erased
A18: cargo test -p deck-streak-api --test insights_routes -- --exact the_insights_routes_answer_only_the_owner
A19: pnpm exec vitest run web/app/src/lib/insights/InstrumentSection.test.ts -t "renders a failed read as a failure line"
A20: pnpm exec vitest run web/app/src/lib/insights/DarkFields.test.ts -t "states zero dark fields as a checked result"
A21: cargo test -p deck-streak-coordination --test instruments_step -- --exact dark_fields_is_passed_its_reads
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and accessibility packs stay
enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/coordination/src/data_rights.rs`: the `research-instruments` category names `instrument_reports` with its purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/insights/+page.svelte` and every file under `web/app/src/lib/insights/`: the insights screen and its sections pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/conventions.rs` | `deck-streak-kernel` | added: `Conventions`, loaded and validated |
| `crates/kernel/src/settings.rs` | `deck-streak-kernel` | changed: `DECKSTREAK_CONVENTIONS_FILE` |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the conventions module |
| `crates/kernel/tests/conventions.rs` | `deck-streak-kernel` | added: A1 to A3 |
| `deploy/config/conventions.example.json` | deploy | added: neutral values |
| `.env.example` | repo | changed: `DECKSTREAK_CONVENTIONS_FILE` |
| `deploy/deck-streak.env.example` | deploy | changed: `DECKSTREAK_CONVENTIONS_FILE` |
| `crates/ingest/src/wire.rs` | `deck-streak-ingest` | added: the protobuf wire walk |
| `crates/ingest/src/structure.rs` | `deck-streak-ingest` | added: the structure reads and their failed reads |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the two modules |
| `crates/ingest/tests/structure.rs` | `deck-streak-ingest` | added: A4 to A8 |
| `crates/insights/src/instrument.rs` | `deck-streak-insights` | added: the instrument port and the report envelope |
| `crates/insights/src/registry.rs` | `deck-streak-insights` | added: one row per instrument, live or inert |
| `crates/insights/src/dark_fields.rs` | `deck-streak-insights` | added: Dark Fields |
| `crates/insights/src/lib.rs` | `deck-streak-insights` | changed: the modules |
| `crates/insights/Cargo.toml` | `deck-streak-insights` | changed: `serde` and `serde_json` for the report (workspace dependencies already) |
| `crates/insights/tests/dark_fields.rs` | `deck-streak-insights` | added: A9 to A11 |
| `crates/insights/tests/registry.rs` | `deck-streak-insights` | added: A12 |
| `migrations/009401_coordination_instrument_reports.sql` | `deck-streak-coordination` | added: `instrument_reports` |
| `crates/ingest/src/lock.rs` | `deck-streak-ingest` | changed: a non-waiting exclusive take, `try_exclusive`, beside the waiting ones, so an instrument run that finds the host's instrument lock held starts nothing (SPEC-094 R8) |
| `crates/coordination/src/instruments.rs` | `deck-streak-coordination` | added: the step, the on-demand run and the store; each run takes `<state directory>/instruments.lock` through `try_exclusive` before it reads (R8) |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the instruments step after the recompute |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: loads `DECKSTREAK_CONVENTIONS_FILE` once at start and refuses start on a malformed file or a forbidden direction label (R1, R2, A3); joins the instruments step to the sync cycle after the recompute (R7); builds the on-demand run over the private copy's reader in scope, the offload and the conventions, for the api and the bot, which cannot name ingest, as OwnerSyncCycle answers /sync (R8) |
| `crates/coordination/src/data_rights.rs` | `deck-streak-coordination` | changed: the port exports and erases `instrument_reports` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the instruments module |
| `crates/coordination/tests/instruments_step.rs` | `deck-streak-coordination` | added: A13 to A15, A21 |
| `crates/coordination/tests/instrument_reports.rs` | `deck-streak-coordination` | added: A16, A17 |
| `crates/api/src/insights_routes.rs` | `deck-streak-api` | added: the three routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes behind the owner's session; ApiState carries the on-demand run (R8, R12) |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role builds the on-demand run at start and hands it to ApiState (R8, R12) |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role hands the on-demand run to its commands at start, for the instruments' commands of SPEC-097 and SPEC-098 (R8) |
| `crates/api/tests/insights_routes.rs` | `deck-streak-api` | added: A18 |
| `web/app/src/routes/insights/+page.svelte` | miniapp | added: the insights screen |
| `web/app/src/lib/insights/insights.ts` | miniapp | added: the routes' client and the report types |
| `web/app/src/lib/insights/InstrumentSection.svelte` | miniapp | added: one report's section and its failure line |
| `web/app/src/lib/insights/InstrumentSection.test.ts` | miniapp | added: A19 |
| `web/app/src/lib/insights/DarkFields.svelte` | miniapp | added |
| `web/app/src/lib/insights/DarkFields.test.ts` | miniapp | added: A20 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /insights joins `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `instrument_reports` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry lists `instrument_reports` under coordination's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `instrument_reports` |
| `privacy.json` | repo | changed: the `research-instruments` category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_094.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/wire_walk.json` | repo | added: the golden of `darkfields.py:_iter_fields` (function; synthetic blobs) |
| `tools/parity-oracle/goldens/safe_name.json` | repo | added: the golden of `darkfields.py:_safe_name` (function) |
| `tools/parity-oracle/goldens/dark_fields.json` | repo | added: the golden of `darkfields.py:build_dark_fields_report` (adapter; synthetic note types) |
| `tools/parity-oracle/goldens/dark_fields_tokens.json` | repo | added: the golden of `darkfields.py:_extract_config_tokens_ex` (function; synthetic configs) |
| `tools/parity-oracle/goldens/dark_fields.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S09400-S09499.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-094-the-instruments-run-through-one-frame-and-dark-fields-finds-content-no-template-renders.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/insights-instrument-frame.md` | docs | added by the W4 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-094.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It splices no report into the weekly report and sends no message for one; the weekly report
  reads the stored reports (#130).
- It revives no inert instrument: the Unclaimed Effort Ledger stays excluded, inert in v9 (#178),
  and so does the cue ledger, inert in v9 (#180); each waits for the owner's decision.
- It serves no report to the agent's machine read tool (#157).
- It builds no other instrument; each has its own SPEC (#137, #138, #141, #143, #146).
- It lets no convention be edited in the Mini App (#57).
- It imports none of the predecessor's instrument output; the first weekly run computes it (#61).

## 6. Risks

- **A report that says "clean" after a failed read.** Detected by A5, A14 and A19.
- **The presence read holds the collection's notes.** Detected by A6, whose fixture counts the notes
  held per batch, and by the memory watch.
- **A decode failure calls every field dark.** Detected by A9's unparseable case.
- **Two runs at once on the host double the memory.** Prevented by the instrument lock (R8) and detected by A15, in one process or across two, and by the memory watch.
- **An owner's note-type name reaches a golden or an example.** Prevented by the synthetic adapters
  and the neutral example file, and detected by the public scrub.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_094.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `wire_walk` | `darkfields.py:_iter_fields` | function | none: synthetic blobs, well formed and each error case |
| `safe_name` | `darkfields.py:_safe_name` | function | none: names with each C0 control and DEL |
| `dark_fields` | `darkfields.py:build_dark_fields_report` | adapter | synthetic template names, encoded configs, declared fields and presence counts; cold, clean, dark, unparseable and failed-read cases, and a field seen in 2, 3 and 4 reviewed notes |
| `dark_fields_tokens` | `darkfields.py:_extract_config_tokens_ex` | function | none: configs with sections, filter chains, special names and a truncated blob |
| `dark_fields.constants` | `darkfields.MIN_DARK_NOTES`, `MAX_DARK_FIELDS_SHOWN`, `MAX_UNPARSEABLE_SHOWN`, `SPECIAL_FIELD_NAMES`, `_SECTION_PREFIXES`, `_Q_FORMAT_FIELD`, `_A_FORMAT_FIELD`; `transfer.NOTES_BATCH_SIZE` | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `instrument_reports` | `deck-streak-coordination` | `migrations/009401_coordination_instrument_reports.sql` | nothing: the predecessor kept no report, it spliced each into the weekly message | exported and erased: no report reads as pending, and the next weekly run writes it |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09401-MIN-DARK-NOTES` | `crates/insights/src/dark_fields.rs` | a field is dark only when three reviewed notes have content in it, as the predecessor's constant says | `dark_fields::the_dark_fields_constants_equal_the_predecessors` |
| `S09402-DARK-FIELDS-SHOWN-CAP` | `crates/insights/src/dark_fields.rs` | a report shows at most forty dark fields and counts the rest | `dark_fields::a_report_shows_at_most_the_cap_and_counts_the_rest` |
| `S09403-UNPARSEABLE-SHOWN-CAP` | `crates/insights/src/dark_fields.rs` | the unparseable list is capped at the predecessor's twenty | `dark_fields::the_dark_fields_constants_equal_the_predecessors` |
| `S09404-SPECIAL-FIELD-NAMES` | `crates/insights/src/dark_fields.rs` | Anki's own template names are never read as field references, exactly the predecessor's six | `dark_fields::the_dark_fields_constants_equal_the_predecessors` |
| `S09405-PRESENCE-BATCH` | `crates/ingest/src/structure.rs` | the presence read holds no more than 400 notes' content at once | `structure::the_presence_read_reduces_each_batch_of_400` |
| `S09406-WIRE-VARINT-LIMIT` | `crates/ingest/src/wire.rs` | a varint at or past seventy bits of shift is refused, as the predecessor refuses it | `structure::the_wire_walk_matches_the_predecessors_golden` |
| `S09407-WEEKLY-SEVEN-DAYS` | `crates/coordination/src/instruments.rs` | a weekly instrument is due at seven study days | `instruments_step::a_weekly_instrument_runs_once_in_seven_study_days` |
| `S09408-WEEKLY-DUE-AT-SEVEN` | `crates/coordination/src/instruments.rs` | a report exactly seven study days old is due, not one day later | `instruments_step::a_weekly_instrument_runs_once_in_seven_study_days` |
| `S09412-CONVENTIONS-PATH-STATES-ITS-SHAPE` | `crates/kernel/src/conventions.rs` | a refused conventions path names the shape the setting accepts, word for word | `conventions::a_malformed_conventions_file_refuses_start` |
| `S09409-FORBIDDEN-TOKENS` | `crates/kernel/src/conventions.rs` | a direction label naming recall or recognition is refused at start in each spelling | `conventions::a_forbidden_direction_label_refuses_start` |
| `S09410-INSTRUMENT-LOCK-NO-WAIT` | `crates/coordination/src/instruments.rs` | a run requested while one runs returns without waiting for the holder | `instruments_step::a_run_while_one_runs_starts_nothing` |
| `S09413-INSTRUMENTS-STEP-RUNS-AFTER-SYNC` | `crates/coordination/src/sync_cycle.rs` | a sync cycle that holds instruments runs the instruments step after the recompute and stores its reports | `instruments_cycle::a_cycle_with_instruments_runs_the_step_after_its_sync` |
| `S09414-LATE-INSTRUMENTS-FILL` | `crates/daemon/src/wiring.rs` | the api role's holder keeps the instruments it is filled with and answers ready afterwards | `instruments_wiring::the_late_holder_answers_not_ready_until_it_is_filled` |
| `S09415-ROLE-WIRES-VALID-INSTRUMENTS` | `crates/daemon/src/wiring.rs` | a role whose settings are valid is handed the built instruments and not none | `instruments_wiring::a_role_with_valid_settings_gets_the_instruments` |
| `S09416-ONLY-THE-SYNC-JOB-LOADS-CONVENTIONS` | `crates/daemon/src/role_job.rs` | only the sync job builds the instruments and reads the owner's conventions, every other job never does | `roles::only_the_sync_job_loads_the_owners_conventions` |
| `S09417-BOT-HOLDS-THE-INSTRUMENTS` | `crates/bot/src/commands.rs` | the command handlers answer the instruments they were handed and none before | `commands::the_handlers_hold_the_instruments_only_once_they_are_handed_them` |
| `S09418-WIRE-STALLED-STEP-IS-REFUSED` | `crates/ingest/src/wire.rs` | a wire walk step that stays where it began is refused by name, so a reader that stalls ends in a named failure and not in memory (SPEC-094 R3) | `wire_progress::a_reader_that_stays_is_refused_with_nothing_kept` |
| `S09419-TOKEN-STALLED-STEP-IS-REFUSED` | `crates/insights/src/dark_fields.rs` | a token scan step that stays where it began is refused by name, so a reader that stalls ends in a named failure and not in memory (SPEC-094 R10) | `token_progress::a_reader_that_stays_is_refused_with_nothing_kept` |
| `S09420-TOKEN-REFUSAL-IS-NOT-A-BREAK` | `crates/insights/src/dark_fields.rs` | a stalled token scan answers its refusal and never the tokens kept so far (SPEC-094 R10) | `token_progress::a_reader_that_stays_is_refused_with_nothing_kept` |
| `S09421-WIRE-REFUSAL-IS-NOT-A-BREAK` | `crates/ingest/src/wire.rs` | a stalled wire walk answers its refusal and never the fields kept so far (SPEC-094 R3) | `wire_progress::a_reader_that_stays_is_refused_with_nothing_kept` |
| `S09422-REFUSED-SCAN-FAILS-ITS-TEMPLATE` | `crates/insights/src/dark_fields.rs` | a template whose token scan refuses is named unparseable and never judged on part of its tokens (SPEC-094 R10) | `token_progress::a_scan_that_refuses_marks_its_template_failed_with_no_tokens` |
| `S09423-TOKEN-STEP-ADVANCES-BY-ONE` | `crates/insights/src/dark_fields.rs` | the real token step moves one byte past a position that starts no token, so the real scan never refuses a template (SPEC-094 R10) | `token_progress::the_real_scan_finds_two_tokens_and_skips_a_broken_pair` |
| `S09412-ONE-REPORT-PER-INSTRUMENT` | `migrations/009401_coordination_instrument_reports.sql` | the instrument's id is the table's key, so a second report replaces the first and the table never holds two (a script row; the cargo killer) | `instrument_reports::a_report_replaces_its_instruments_previous_one` |

## 10. Amendments

The delivery touched these files beyond the rows above:
- `crates/api/src/lib.rs`
- `crates/bot/src/commands.rs`
- `crates/coordination/Cargo.toml`
- `crates/coordination/tests/data_rights.rs`
- `crates/daemon/src/role_job.rs`
- `crates/ingest/tests/support/synthetic.rs`
- `tools/parity-oracle/goldens/dark_fields.special_names.json`
- `web/app/messages/en.json`
- `web/app/src/lib/api.ts`
- `web/app/src/lib/api.test.ts`
- `web/app/src/lib/insights-page.test.ts`
- `web/app/src/lib/insights/insights.test.ts`
- `scripts/mutation-equivalent.d/miniapp.json`
- `web/app/src/lib/startapp.test.ts`
- `web/app/src/lib/startapp.ts`
- `docs/decisions/ADR-094-the-instruments-run-weekly-after-the-sync-or-on-demand-and-coordination-stores-the-latest-report.md`
- `docs/decisions/ADR-095-ingest-walks-the-wire-format-by-hand-and-the-instrument-reads-keep-the-scope.md`
- `docs/decisions/ADR-096-the-owners-note-conventions-are-private-configuration-the-kernel-loads.md`
- `crates/bot/tests/commands.rs`
- `crates/coordination/tests/instruments_cycle.rs`
- `crates/daemon/tests/instruments_wiring.rs`
- `crates/daemon/tests/roles.rs`
- `crates/ingest/tests/lock.rs`
- `crates/ingest/tests/wire_progress.rs`
- `crates/insights/tests/token_progress.rs`
- `scripts/tests/test_callee_offset_loops.py`
- `docs/red-first/SPEC-094.md`

Rows the delivery leaves unchanged:
- `crates/kernel/src/settings.rs` is unchanged; the conventions file is read by the existing loader.
- `docs/schematics/insights-instrument-frame.md` is unchanged; it landed on dev already.

### Amendment, 2026-10-02: the Status names the delivery

Insert-only under ruling (i) of SPEC-038 section 8: every earlier byte is kept in order. The one insertion is in the Status line of the header: the delivery that moved this SPEC out of `docs/specs/planned/` (#403, ADR-016) and the marks that strike the old "planned" wording. This entry is the other insertion.
