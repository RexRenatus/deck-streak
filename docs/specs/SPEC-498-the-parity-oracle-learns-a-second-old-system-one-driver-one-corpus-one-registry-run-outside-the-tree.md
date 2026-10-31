# SPEC-498: The parity oracle learns a second old system: one driver, one corpus, one registry, run outside the tree

| field | value |
|---|---|
| status | built locally; lands with the run that writes its goldens |
| issue | none yet |
| decided by | ADR-516, under ADR-012 and ADR-029 |
| measured at | `5c44700b` |
| schematic | `docs/schematics/the-second-oracle-from-a-sandboxed-engine-run-to-a-rust-assertion.md` (new) |
| mutation band | S49800-S49899 (no rows in this delivery) |

## 1. The problem, measured

DeckStreak proves its game math against its predecessor's own functions (ADR-012): `tools/parity-oracle/generate.py`
calls them over seeded synthetic inputs, and 176 committed goldens under `tools/parity-oracle/goldens/` are read by the
Rust tests through `tools/parity-oracle/golden.rs` (ADR-029). What was read at `5c44700b`:

- **The oracle knows one old system.** `generate.py` imports a Python package by name (`resolve`, `:155-172`). The
  engine DeckStreak ingests through is Rust, pinned in the root manifest (`Cargo.toml:173-175`, commit
  `a078820fd4fec3068ed7863344cd50effddabc6b`), and no file under `tools/parity-oracle` drives it: `ls
  tools/parity-oracle/goldens | grep -c anki` prints 0.
- **A replacement of the engine's import, export, rendering, media, search and history needs the engine's own
  outputs** over inputs nobody owns, so each port is compared with the engine rather than re-derived.
- **The consumer has no shared comparison.** `golden.rs` reads a golden and runs a closure per case (`each_case`), and
  each consumer compares on its own; no test shows that a consumer reports a wrong recorded output.
- **The workspace is `crates/*` only** (`Cargo.toml:4`), and no workflow under `.github/workflows` names
  `tools/parity-oracle` (`grep -rn parity-oracle .github/workflows` prints nothing).

## 2. Requirements

- **R1. One driver and its registry.** `tools/parity-oracle/anki/generate.py` loads `anki/registry/spec_NNN.py`
  modules, runs one harness session, and writes one golden per registration in `phx.parity-golden.v1`, with the
  engine's commit, its own sha256, the registry module's sha256, `inputs: synthetic` and the corpus's integer seed.
- **R2. One corpus, seeded, with classes.** `tools/parity-oracle/anki/corpus.py` holds the seed, every edge class with
  what its cases hold, and the builders of DeckStreak-authored note types, field texts and collections. Nothing in it
  is anyone's data or the engine's own text. The registry module pins the corpus's sha256, so a corpus edit stales
  the goldens it registered.
- **R3. Every function registered.** F1 to F12 (import and export), R-1 to R-12 (rendering), G-L5-1, G-L5-2 (media),
  G-L6-1, G-L6-2 (search), G-L9-1 (review history to events) and GL-1 to GL-4 (storage), 33 in all, each with its
  edge classes; only F12, the schema snapshot, declares none.
- **R4. The harness stays outside.** The harness, a small program on the engine's API, is kept outside this
  repository, with its source's sha256 pinned in the driver. No workspace member covers a harness path, the root
  lock names no package the harness adds, and no workflow names the harness, its paths or the driver.
- **R5. A runbook.** `tools/parity-oracle/anki/RUNBOOK.md` builds the sandbox and runs the driver once, with a planted
  outbound connection refused inside the sandbox and accepted outside it (the positive control), and lists what the
  run record holds.
- **R6. The stock-text census.** The harness reports the texts of the note types the engine adds by itself; the
  driver keeps only the sha256 of each stripped line of 12 characters or more, refuses to write any golden that holds
  one of those lines, and writes the digests, never the text.
- **R7. Provenance.** Each golden's `generator_sha256` equals the committed driver's digest, which covers the harness
  pin, and its `registry_sha256` the committed registry module's.
- **R8. A consumer can fail.** `golden.rs` gains `mismatches` and `assert_parity`; a fixture golden with one wrong
  output makes the consumer report exactly that case.
- **R9. Edge classes covered.** Every declared class has a case for every function it is declared for, and every
  case's class is declared; the driver refuses a registry that breaks either before anything runs, and a class that
  holds one hand case twice.
- **R10. The clean-room record.** The record of the clean-room line, covering every crate an engine-replacing delivery
  writes, with each delivery's read list and similarity-scan score, is kept in the owner's private evidence, never in
  this tree.
- **R11. The run's guards.** The driver refuses (exit 2) a source it could write to, an unpinned harness or one whose
  source digest differs from the pin; it fails the run (exit 3) and writes nothing on a harness of another engine, a
  protocol fault, a refusal in words rather than a label, or a census hit; and it writes only in its output folder.

The second delivery of this SPEC, "the engine writes every parity golden once, sandboxed outside the tree", adds:

- **R12. A golden for every registered function.** One committed golden per registration of the engine registry, named
  as registered, and no other engine golden.
- **R13. Provenance, synthetic and seeded** (`dm.oracle-provenance`, `dm.oracle-synthetic`). Each engine golden is strict
  `phx.parity-golden.v1` as the driver writes it; its `source_commit` is the commit the root manifest's engine patch
  pins (`a078820f…`, the upstream engine plus the fork's patches, `Cargo.toml:165-176`); its `generator_sha256` and
  `registry_sha256` are the committed driver's and registry module's; its inputs are `synthetic` and its seed is the
  corpus's integer seed.
- **R14. Each engine golden read by a Rust test** (`dm.oracle-consumed`). `crates/ingest/tests/engine_goldens.rs` reads
  every engine golden through `golden.rs`, holds it to the pin and the driver, and holds every case's answer to the
  harness's answer keys for its function (section 2a) or to a refusal label. The port of each function compares its
  own outputs through `golden::assert_parity` in the delivery that ports it.
- **R15. No date, id, clock or stock line.** No engine golden holds a calendar date or clock time, an integer (or a 10-
  or 13-digit run in a text) in the run's clock range unless the case's input gave it (a field checksum, a digest of
  the input's own text, is not a clock), or a stock line the committed census `tools/parity-oracle/anki/stock-lines.sha256`
  names. A refusal is a label, never the engine's words.
- **R16. The schema snapshot and its pin line.** `tools/parity-oracle/anki/snapshot.py` derives
  `tools/parity-oracle/anki/schema-snapshot.sql` from the fresh-collection golden: two comment lines, one pin line
  `PRAGMA user_version = N;` with the golden's version, then each statement of the fresh collection's schema.
- **R17. The one run.** The goldens, the census and the snapshot come from one run of the runbook's sandbox: an
  egress-closed guest that mounts only a clean `git archive` export of the pinned DeckStreak commit and cargo's registry
  and git checkouts, read-only, with one writable output folder. Its private record shows the planted outbound
  connection refused inside and accepted outside, a planted canary outside the mounts unreadable inside and read
  outside, the mounts and their modes, the driver's `examined` line, the digest of every file written, and the dirty
  side's read list. Runs after this one follow R19.

The third delivery of this SPEC, "each engine golden is a function of its input", adds:

- **R18. Each golden is a function of its input.** An engine golden's answers depend on the case's input, the pinned
  harness, the driver, the registry and the engine pin, and on nothing of the host that runs them: (a) the harness
  holds the process's real-time clock at the case's own recorded instant (`collection.now`, else `timing.now`, else
  `now`), moving 1 microsecond a read; (b) it runs under `TZ=UTC`; (c) a GUID the package shares with the target
  carries an explicit `mod` on both sides, so an import's updated or duplicate count is read from the input; (d) the
  per-collection phase: the k-th engine collection a call opens, counted from 0, starts k held seconds after the
  call's instant, so two collections of one call never share the ids the engine takes from its clock.
- **R19. A re-run reads before it writes.** Goldens are written only after non-writing passes read equal at the pin:
  two passes of the pinned harness, one of them under a planted host instant, byte-equal file for file; and a semantic
  gate, fixed before the run, reads the previous pin's pass against the committed goldens and the new pin's pass
  against the previous pin's. Any reading off the gate's rule writes nothing.

## 2a. The harness's answers

Each case's `output` is either `{"refused": LABEL}` or an object of exactly these keys:

| golden | keys |
|---|---|
| `anki.read.collection_view` | `cards`, `decks`, `notes`, `notetypes`, `revlog` |
| `anki.export.package` | `media`, `members`, `rows` |
| `anki.import.package` | `counts`, `notes`, `revlog` |
| `anki.timing.day`, `anki.sched.timing_today` | `day`, `next_rollover` |
| `anki.card.due_view` | `due` |
| `anki.note.checksum` | `csum`, `sfld` |
| `anki.media.name`, `anki.render.media_refs`, `anki.media.field_refs` | `names` |
| `anki.collation.order` | `order` |
| `anki.blob.encode` | `hex` |
| `anki.history.local_answer` | `revlog` |
| `anki.guid.accepts` | `kept` |
| `anki.schema.snapshot` | `col_ver`, `schema` (each statement), `user_version` |
| `anki.render.card`, `anki.render.occlusion_render` | `answer`, `question` |
| `anki.render.card_ordinals`, `anki.render.generated_ordinals` | `ordinals` |
| `anki.render.cloze_only_text`, `anki.render.strip_av_tags`, `anki.render.html_to_text_line` | `text` |
| `anki.render.extract_av_tags` | `tags`, `text` |
| `anki.render.latex_images` | `images`, `text` |
| `anki.render.compare_answer` | `html` |
| `anki.render.note_type_check`, `anki.render.note_check` | `result` |
| `anki.media.check` | `missing`, `unused` |
| `anki.search.operator`, `anki.search.browser` | `cards` (corpus indices, in the engine's order) |
| `anki.history.revlog_events` | `rows` |
| `anki.deck.name_equal` | `decks` |
| `anki.read.reader_fixture` | `tables` |

Every row is named by its corpus index, never by an id the engine assigns.

## 3. Acceptance criteria of SPEC-498

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1, R7, R11: every golden the committed registry gives through a stand-in harness is strict, closed-key `phx.parity-golden.v1` with the pinned commit, an integer seed and synthetic inputs; each golden's digests equal the committed files' and a one-byte edit of a temporary copy of the driver turns every golden red; a writable source is refused, the run writes only its goldens and census, and a protocol fault or another engine fails the run | the stub driver writes no golden and checks nothing | ADR-516 D1 |
| A2 | R2, R3, R9: the corpus has one integer seed and every class is declared by a registration; the 33 functions are registered by id, each but F12 with classes; G-L9-1 covers review types 0 to 5 and a review at the rollover; every declared class has a case, and a declared class with no case is refused | the stub registry registers nothing | ADR-516 D2 |
| A3 | R4: the tree holds no member, lock package or workflow that reaches the harness, and planted copies of each are caught | the stub census refuses nothing | ADR-516 D3 |
| A4 | R5, R10: the runbook names the planted network control, the read-only mounts and the run record, and the clean-room record exists in the owner's private evidence (read by the reviewer) | none: text | ADR-516 D4 |
| A5 | R6: a golden holding a stock line fails the run and writes nothing; short lines are not counted | the stub census refuses nothing | ADR-516 D5 |
| A6 | R8: the consumer reports the fixture's one wrong case, and its assertion panics naming it | the stub comparison finds nothing | ADR-516 D6 |
| A8 | R12: every registered engine function has its committed golden, and the two oracles split the goldens whole | no engine golden is committed | ADR-516 D4 |
| A9 | R13: each engine golden records the manifest's engine pin, the committed driver and registry, synthetic inputs and the corpus seed; planted faults of each are caught | no engine golden is committed | ADR-516 D1 |
| A10 | R14: each engine golden is read by a Rust test holding its answers to section 2a's keys; a planted answer off its keys or a worded refusal is a fault, and a golden named only in a comment is caught | the goldens the tests read are absent | ADR-516 D6 |
| A11 | R15: no engine golden holds a date, an id or clock of the run, or a stock line; planted ones are caught | no engine golden or census is committed | ADR-516 D5 |
| A12 | R16: the committed snapshot is the fresh-collection golden's schema under one pin line equal to its `user_version` | the writer, the golden and the snapshot are absent | ADR-516 D1 |
| A13 | R17: the run record shows the planted connection refused inside and accepted outside, the canary, the read-only mounts, the writes and the read list (read by the reviewer) | none: the run's private record | ADR-516 D4 |
| A14 | R18: every drawn import case whose package shares a GUID with the target gives that note a `mod` on both sides (examined 72); the driver starts the harness with `TZ=UTC` beside `PATH` and `LC_ALL`; no engine golden holds a number of the run's clock its input did not give | 40 of 72 import cases share a GUID with no mod on both sides; the harness's environment holds `LC_ALL` and `PATH` only | ADR-516 D7 |
| A15 | R19: the run record shows the two non-writing passes byte-equal, one under a planted host instant, and the semantic gate's reading before the one writing pass (read by the reviewer) | none: the run's private record | ADR-516 D7 |

```acceptance
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_every_golden_is_strict_synthetic_and_seeded
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_golden_provenance
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_a_writable_source_is_refused
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_the_run_writes_only_in_its_output_directory
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_the_corpus_is_seeded_and_every_class_is_declared
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_every_function_is_registered
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_revlog_to_events_is_registered_with_every_review_type
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_edge_classes_covered
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_hand_cases.py' -k test_corpus_holds_the_design_hand_case_target
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_hand_cases.py' -k test_a_hand_case_repeated_in_its_class_is_refused
A3: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_workspace.py' -k test_generator_outside_workspace
A5: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_a_golden_holding_a_stock_line_is_refused
A6: cargo test -p deck-streak-ingest --test oracle_consumer -- --exact golden_consumer_can_fail
A6: cargo test -p deck-streak-ingest --test oracle_consumer -- --exact the_consumer_panics_naming_the_wrong_case
A8: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_every_registered_engine_function_has_its_golden
A8: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_the_two_oracles_split_the_goldens_whole
A9: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_engine_golden_provenance
A9: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_engine_goldens_are_synthetic_and_seeded
A10: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_every_engine_golden_is_read_by_a_rust_test
A10: cargo test -p deck-streak-ingest --test engine_goldens
A11: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_no_engine_golden_holds_a_date_an_id_or_a_clock
A11: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_no_engine_golden_holds_a_stock_line
A12: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_the_schema_snapshot_carries_its_pin_line
A12: cargo test -p deck-streak-ingest --test engine_goldens -- --exact the_schema_snapshot_is_its_golden_under_one_pin_line
A14: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_a_note_the_package_shares_with_the_target_carries_a_mod_on_both_sides
A14: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_the_driver_starts_the_harness_in_utc
A14: python3 -m unittest discover -s tools/parity-oracle -p 'test_goldens.py' -k test_no_engine_golden_holds_a_date_an_id_or_a_clock
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `tools/parity-oracle/anki/generate.py` | oracle | new: the driver, its guards, the census, the strict, provenance and workspace checks |
| `tools/parity-oracle/anki/corpus.py` | oracle | new: the seed, the classes and the builders |
| `tools/parity-oracle/anki/registry/spec_498.py` | oracle | new: the 33 registrations and the corpus pin |
| `tools/parity-oracle/anki/fixtures/stand_in_harness.py` | oracle tests | new: a stand-in that speaks the protocol and computes nothing of the engine's |
| `tools/parity-oracle/anki/fixtures/consumer_can_fail.json` | oracle tests | new: the fixture golden with one wrong output |
| `tools/parity-oracle/anki/RUNBOOK.md` | oracle | new: the sandbox and the one run |
| `tools/parity-oracle/test_anki_driver.py` | oracle tests | new: A1 and A5 |
| `tools/parity-oracle/test_anki_registry.py` | oracle tests | new: A2 |
| `tools/parity-oracle/test_anki_hand_cases.py` | oracle tests | new: A2 (the hand-case target and the repeated-case refusal) |
| `tools/parity-oracle/test_anki_workspace.py` | oracle tests | new: A3 |
| `tools/parity-oracle/golden.rs` | oracle | changed: `Mismatch`, `mismatches` and `assert_parity` |
| `tools/parity-oracle/README.md` | oracle | changed: the second old system's files |
| `crates/ingest/tests/oracle_consumer.rs` | ingest tests | new: A6 |
| `ruff.toml` | style | changed: the driver, corpus and registry are excluded, because their bytes are pinned |
| `docs/specs/SPEC-498-the-parity-oracle-learns-a-second-old-system-one-driver-one-corpus-one-registry-run-outside-the-tree.md` | docs | new: this file |
| `docs/decisions/ADR-516-the-second-oracle-is-one-driver-over-a-pinned-harness-kept-outside-the-tree.md` | docs | new: the decisions |
| `docs/schematics/the-second-oracle-from-a-sandboxed-engine-run-to-a-rust-assertion.md` | docs | new: the data flow and the two roles |
| `docs/red-first/SPEC-498.md` | docs | new: the red-first record |
| `changelog.d/parity-oracle-second-system-498.md` | changelog | new: the fragment |
| `tools/parity-oracle/test_goldens.py` | oracle tests | changed (second delivery): A8 to A12; the predecessor's checks read only its own goldens |
| `tools/parity-oracle/test_generate.py` | oracle tests | changed (second delivery): A9 of SPEC-029 counts only the predecessor's goldens |
| `crates/ingest/tests/engine_goldens.rs` | ingest tests | new (second delivery): A10 and A12 |
| `tools/parity-oracle/anki/snapshot.py` | oracle | new (second delivery): the snapshot writer |
| `tools/parity-oracle/anki/schema-snapshot.sql` | oracle | new (second delivery, with the run): the snapshot |
| `tools/parity-oracle/anki/stock-lines.sha256` | oracle | new (second delivery, with the run): the census digests |
| `tools/parity-oracle/goldens/anki.*.json` | oracle | new (second delivery, with the run): the 33 engine goldens |
| `tools/parity-oracle/anki/generate.py` | oracle | changed (second delivery): `HARNESS_SOURCE_SHA256` pinned |
| `tools/parity-oracle/test_anki_driver.py` | oracle tests | changed (second delivery): the unpinned refusal is held with the pin unset |
| `changelog.d/engine-writes-parity-goldens-498.md` | changelog | new (second delivery): the fragment |
| `tools/parity-oracle/anki/generate.py` | oracle | changed (third delivery): the harness starts under `TZ=UTC`; `ENGINE_COMMIT` moves to `a078820f…`; `HARNESS_SOURCE_SHA256` pins the harness with the held clock |
| `tools/parity-oracle/anki/registry/spec_498.py` | oracle | changed (third delivery): `explicit_mods`, a GUID the package shares with the target carries a `mod` on both sides |
| `tools/parity-oracle/anki/fixtures/stand_in_harness.py` | oracle tests | changed (third delivery): the stand-in's commit follows the engine pin |
| `tools/parity-oracle/anki/RUNBOOK.md` | oracle | changed (third delivery): the engine pin |
| `tools/parity-oracle/test_anki_registry.py` | oracle tests | changed (third delivery): A14, the shared GUID's `mod` on both sides |
| `tools/parity-oracle/test_anki_driver.py` | oracle tests | changed (third delivery): A14, the harness starts under `TZ=UTC` |
| `tools/parity-oracle/test_goldens.py` | oracle tests | changed (third delivery): the planted engine golden's commit follows the engine pin |
| `docs/specs/SPEC-498-the-parity-oracle-learns-a-second-old-system-one-driver-one-corpus-one-registry-run-outside-the-tree.md` | docs | changed (third delivery): R18, R19, A14 and A15 |
| `docs/decisions/ADR-516-the-second-oracle-is-one-driver-over-a-pinned-harness-kept-outside-the-tree.md` | docs | changed (third delivery): D7 |
| `docs/red-first/SPEC-498.md` | docs | changed (third delivery): A14's red and green |

## 5. What this does NOT cover

- The harness's source and its build, and the one run that writes the goldens, the census digests and the run record;
  that run pins `HARNESS_SOURCE_SHA256` in the same commit as the goldens.
- The tests that read the committed goldens of the second old system: `test_goldens.py`'s coverage of them, and one
  Rust consumer per golden, each in the delivery that ports the function.
- The schema snapshot's pin line, which the run's delivery adds beside F12's golden.

## 6. Risks

- **The corpus holds the design's hand-case target; its run time is unmeasured.** It draws 1,808 hand cases over
  the 227 declared (function, class) pairs, at least 6 per pair: 241 drawn from the seed, one or more per declared
  class of each function, and 1,567 written with no draw, five to eight for each pair but the blob default class,
  whose drawn cases already cover every column. Beside them are 9,600 grammar-random render cases and 409 other
  random cases, 11,817 in all; the design aims at about 1,500 hand cases beside 10,000 random ones. The run time is
  unmeasured until the run.
- **The harness answers in the corpus's terms.** A harness that answered with ids the engine assigns or with the run's
  clock would put them in a golden; the runbook forbids both and the run's review reads the goldens for them.

## 7. What only CI proves

The workspace's clippy and tests (`rust`), the formatting check, and the Python stage of `scripts/check.sh`, which
discovers the three new test files beside `test_generate.py` and `test_goldens.py`.

## 8. Formal model

- **Lean: NOT APPLICABLE.** The driver writes test data; it decides no gate verdict, ledger, money or release.
- **TLA+: NOT APPLICABLE.** One driver talks to one harness in strict turns; nothing interleaves.

## 9. Mutation rows

None in this delivery. The band S49800-S49899 is SPEC-498's; the driver's refusals are held by the planted controls
of A1, A2, A3 and A5.
