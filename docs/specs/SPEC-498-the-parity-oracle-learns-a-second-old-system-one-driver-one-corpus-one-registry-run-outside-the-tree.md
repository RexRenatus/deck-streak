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
  `65e053b829825f534803b2b772c507a73ba936a3`), and no file under `tools/parity-oracle` drives it: `ls
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
  case's class is declared; the driver refuses a registry that breaks either before anything runs.
- **R10. The clean-room record.** The record of the clean-room line, covering every crate an engine-replacing delivery
  writes, with each delivery's read list and similarity-scan score, is kept in the owner's private evidence, never in
  this tree.
- **R11. The run's guards.** The driver refuses (exit 2) a source it could write to, an unpinned harness or one whose
  source digest differs from the pin; it fails the run (exit 3) and writes nothing on a harness of another engine, a
  protocol fault, a refusal in words rather than a label, or a census hit; and it writes only in its output folder.

## 3. Acceptance criteria of SPEC-498

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1, R7, R11: every golden the committed registry gives through a stand-in harness is strict, closed-key `phx.parity-golden.v1` with the pinned commit, an integer seed and synthetic inputs; each golden's digests equal the committed files' and a one-byte edit of a temporary copy of the driver turns every golden red; a writable source is refused, the run writes only its goldens and census, and a protocol fault or another engine fails the run | the stub driver writes no golden and checks nothing | ADR-516 D1 |
| A2 | R2, R3, R9: the corpus has one integer seed and every class is declared by a registration; the 33 functions are registered by id, each but F12 with classes; G-L9-1 covers review types 0 to 5 and a review at the rollover; every declared class has a case, and a declared class with no case is refused | the stub registry registers nothing | ADR-516 D2 |
| A3 | R4: the tree holds no member, lock package or workflow that reaches the harness, and planted copies of each are caught | the stub census refuses nothing | ADR-516 D3 |
| A4 | R5, R10: the runbook names the planted network control, the read-only mounts and the run record, and the clean-room record exists in the owner's private evidence (read by the reviewer) | none: text | ADR-516 D4 |
| A5 | R6: a golden holding a stock line fails the run and writes nothing; short lines are not counted | the stub census refuses nothing | ADR-516 D5 |
| A6 | R8: the consumer reports the fixture's one wrong case, and its assertion panics naming it | the stub comparison finds nothing | ADR-516 D6 |

```acceptance
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_every_golden_is_strict_synthetic_and_seeded
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_golden_provenance
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_a_writable_source_is_refused
A1: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_the_run_writes_only_in_its_output_directory
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_the_corpus_is_seeded_and_every_class_is_declared
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_every_function_is_registered
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_revlog_to_events_is_registered_with_every_review_type
A2: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_registry.py' -k test_edge_classes_covered
A3: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_workspace.py' -k test_generator_outside_workspace
A5: python3 -m unittest discover -s tools/parity-oracle -p 'test_anki_driver.py' -k test_a_golden_holding_a_stock_line_is_refused
A6: cargo test -p deck-streak-ingest --test oracle_consumer -- --exact golden_consumer_can_fail
A6: cargo test -p deck-streak-ingest --test oracle_consumer -- --exact the_consumer_panics_naming_the_wrong_case
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

## 5. What this does NOT cover

- The harness's source and its build, and the one run that writes the goldens, the census digests and the run record;
  that run pins `HARNESS_SOURCE_SHA256` in the same commit as the goldens.
- The tests that read the committed goldens of the second old system: `test_goldens.py`'s coverage of them, and one
  Rust consumer per golden, each in the delivery that ports the function.
- The schema snapshot's pin line, which the run's delivery adds beside F12's golden.

## 6. Risks

- **The corpus is below the design's hand-case target.** It draws 241 hand cases, one or more per declared class of
  each function, 9,600 grammar-random render cases and 409 other random cases, 10,250 in all; the design aims at
  about 1,500 hand cases beside 10,000 random ones. The run time is unmeasured until the run.
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
