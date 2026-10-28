# SPEC-496: The formal ledger names every engine property, its artifact, its tie and its gaps

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | named by the pull request that delivers it |
| decided by | ADR-514 |
| measured at | `5c44700b` |
| schematic | `docs/schematics/the-formal-obligations-ledger-and-its-checker.md` (new) |
| mutation band | S49600-S49699 (no row: section 9) |

## 1. The problem, measured

DeckStreak proves its protocols in TLA+ and its pure rules in Lean, and its engine is being replaced by its own
scheduler and its own sync, store, import, media, search and rendering code. Each of those brings properties that are
to be proved before they are believed. Nothing in the tree says which engine property is proved, by what, how the
proof is bound to the shipped code, or what it leaves unproved. What was read, at `5c44700b`:

- **The formal tree.** `git ls-tree -d --name-only 5c44700b formal/tla/ | wc -l` prints 27 TLA+ entries, and
  `git grep -l '@phx theorem' 5c44700b -- formal/lean | wc -l` prints 16 Lean entries: 43 entries. Their headers
  declare 91 properties and 45 theorems (`git grep -h -c -E '@phx property' 5c44700b -- formal | paste -sd+ | bc`,
  and the same with `@phx theorem`), every one `ramp=report`; the same count with `ramp=block` names 0 files.
- **No ledger.** `git ls-tree --name-only 5c44700b formal/` prints `formal/lean`, `formal/tla` and `formal/vectors`:
  no file names an obligation, its artifact, its tie or its gaps.
- **Ties today.** `git ls-tree -r --name-only 5c44700b -- crates | grep -c tests/formal_vectors` prints 11: eleven
  Rust tests read Lean vectors. Every TLA+ entry is bound to the code by `@phx covers` stamps alone, and a stamp
  fails on an edit of the covered span, never on a divergence of the code from the model.
- **No formal tool in CI.** `git grep -n -i -w -E 'formal|lean|tla|tlc|kani|phxd' 5c44700b -- .github/workflows |
  wc -l` prints 0. A check that CI runs reads the `@phx` headers itself.

## 2. Requirements

R1. **The ledger.** `formal/obligations.json` is one JSON object with exactly the keys `schema`, `phases`,
`obligations` and `entries`, and `schema` is `formal-obligations.v1`.

R2. **An obligation.** Each obligation holds exactly `id` (`O<n>`, or `O<n>a` for an arm of `O<n>`, unique),
`property` (non-blank text), `layers` (a non-empty list of distinct non-blank names), `artifacts` (at least one),
`ties`, `gaps` (at least one non-blank line) and `status`, one of `planned`, `artifact`, `tied` and `complete`, in
that order of progress.

R3. **An artifact.** Each artifact holds `kind` (`tla`, `lean` or `kani`) and `state` (`planned` or `present`).
- A `tla` or `lean` artifact names its `entry` (`tla/<Name>` or `lean/<Name>`, its prefix its kind) and may name a
  `property`; a `present` one names its `property`, which the entry's `@phx property` (TLA+) or `@phx theorem`
  (Lean) header declares at the checked tree. A TLA+ entry is a folder `formal/tla/<Name>` holding `<Name>.tla`; a
  Lean entry is a module `formal/lean/Formal/<Name>.lean` whose header declares at least one theorem.
- A `kani` artifact names its `harness`; a `present` one also names `at`, `<path>.rs::<fn>`, a function of that
  file marked `#[kani::proof]`.
- An artifact of the scheduling companion's own tree holds `"tree": "companion"` and an `about` line, and names no
  entry, property or harness: the companion's own ledger judges it.

R4. **A tie.** Each tie holds `method`, one of `vectors`, `trace`, `kani-identity`, `mbt`, `census` and
`cross-target-hash`; `for`, the entry or harness of an artifact of its own obligation; `test`,
`<path>.rs::<fn>` (a function marked `#[test]`, `#[tokio::test]` or `#[kani::proof]`, comments not counted) or
`<path>.py::<Class>.<method>` (a `test` method of a top-level class), defined in the checked tree; and, for `mbt`,
may hold `mutant`, the planted code mutant that turns it red. A tie counts only when its method is `vectors`, `trace`,
`kani-identity` or `mbt`, its test is defined, and an `mbt` tie names its mutant. `census` and `cross-target-hash` ties
are recorded and never count. A `@phx covers` stamp is never a tie.

R5. **Status.** An obligation past `planned` holds no planned artifact. An obligation at `tied` or `complete` holds
no artifact without a counting tie; one whose artifact is bound by covers stamps only stays at `artifact`, with the
gap line "tied by covers only".

R6. **Phase sets.** `phases` holds exactly `"1"`, the list `O1a`, `O4`, `O5`, `O6`, and `"2"`, the list `O1` to
`O9` and `O11`. Every id a phase set names is an obligation of the ledger, once per set.

R7. **The rows.** `entries` holds one row per formal entry of the tree, and none for an entry the tree lacks. A row
holds exactly `entry`, `disposition` (`keep`, `re-stamp`, `re-point` or `retire`), `act` (non-blank text) and
`obligations`, the ids of the obligations whose artifacts name the entry. At `5c44700b` the rows are the 43 existing
entries, with these dispositions:
- **retire:** `tla/FullSyncChoice`, `tla/SyncSnapshotWindow`;
- **re-point:** `tla/MinimumClientHandshake`, `tla/SyncCredential`, `tla/UndoOwnAnswer`, `tla/SkipDayWrite`;
- **re-stamp:** `tla/AwardOnce`, `tla/FoldSettlesOnce`, `tla/HeldFlush`, `tla/LandmarkOnce`, `tla/RelightOrder`,
  `tla/SensitiveDeckGate`, `tla/SkipDayOnce`, `lean/SensitiveDeck`;
- **keep:** the other 14 TLA+ entries and the other 15 Lean entries, `lean/ReplayHistory` among them while the
  FSRS-7 crate stays.

R8. **The content.** The obligations are `O1a` and `O1` to `O17`, every one `planned`:
- `O1a` the first phase's arm of `O1`, the existing sync models and fail closed;
- `O1` no lost review; `O2` an append-only, monotone review log; `O3` the op log converges;
- `O4` interval determinism; `O5` the fold and the merge; `O6` fail closed where the scheduler is absent;
- `O7` the relay holds ciphertext only; `O8` the import's idempotency and the cut-over; `O9` the round trip;
- `O10` the learner boundary (conditional: no learner companion ships); `O11` the engine-tied entries retired or
  re-pointed, the coverage of no property dropping;
- `O12` the store; `O13` media; `O14` search; `O15` rendering and authoring; `O16` the sync core's pure functions;
  `O17` the release gates.

  The planned entries they name are `tla/ScheduledAnswer`, `tla/OpLog`, `tla/CollectionStore`, `tla/MediaSync`,
  `tla/RelayEnvelope`, `tla/AnkiCutover`, and `lean/CompanionDecide`, `lean/ScheduledState`, `lean/ReleaseGates`,
  `lean/SyncClock`, `lean/ReviewKey`, `lean/FieldMerge`, `lean/PushJudge`, `lean/IdSlot`, `lean/EffectiveLog`,
  `lean/QueueRule`, `lean/AnkiDay`, `lean/MediaBind`, `lean/MediaGc`, `lean/SearchCompile`, `lean/ClozeFront`,
  `lean/Allowlist`, `lean/EditUndo`, `lean/ImportMerge`, `lean/ImportPlan`, `lean/AnkiRoundTrip` and
  `lean/EnvelopeLayout`. The planned Kani harnesses are the 29 of this repository's own code, each in the
  obligation of its layer. `O4`, `O5` and `O10` hold companion artifacts. The present artifacts are every property
  of the three sync models (`O1a`) and every property and theorem of the engine-tied entries (`O11`), and the one
  tie is the replay history's vectors test.

R9. **The census.** No line of the ledger holds a token derived from the scheduler's brand, at a word start or at
a camel hump, and no token, nor any part of it between `-`, `.` and `:`, is a name on the census list. The list
holds salted SHA-256 digests (`formal-ledger-census:` and the name in lower case), never a name, and a hit is reported
by its line, never by its text.

R10. **The checker.** `scripts/tests/_formal_ledger.py` reads files only and starts no process; it reports every
refusal by its arm. `scripts/tests/test_formal_obligations.py` runs it in the python stage.

## 3. Acceptance criteria of SPEC-496

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1, R6, R8: the ledger exists in schema v1 and the checker accepts it; it holds `O1a` and `O1` to `O17` with the planned entries and harnesses; its phase sets are R6's; planted schema faults are refused by arm | the ledger is absent, and the stub checker refuses no planted fault | ADR-514 D1, D2 |
| A2 | R2: an obligation with no artifact, and one with no gap line, are refused (planted) | the stub checker refuses nothing | ADR-514 D2 |
| A3 | R3, R5: an artifact naming an entry, a property or a harness the tree does not declare is refused, and so is a planned artifact under an obligation past `planned` (planted) | the stub checker refuses nothing | ADR-514 D3 |
| A4 | R4, R5: a tie naming a test the tree does not define, a method outside the six, a tie with no `for` or a `for` naming no artifact of its obligation, and an obligation at `tied` or `complete` with an artifact no counting tie binds are refused, among them an obligation tied only by a covers stamp (planted) | the stub checker refuses nothing | ADR-514 D4 |
| A5 | R7: the 43 existing entries' dispositions are rows, each as R7 gives it, and a missing, extra, doubled or mislinked row is refused (planted) | the stub checker refuses nothing, and the ledger is absent | ADR-514 D5 |
| A6 | R9: a census finds no private core name in the ledger; a planted brand token and a planted listed name are refused, by line and never by text | the stub census finds nothing | ADR-514 D6 |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_the_ledger_exists_in_schema_v1
A1: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_the_rows_hold_o1_to_o17_and_the_planned_entries
A1: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_the_phase_sets_list_o1a_and_o1_to_o11
A2: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_an_obligation_with_no_artifact_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_an_obligation_with_no_gap_line_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_an_artifact_naming_an_undeclared_entry_or_property_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_a_tie_the_tree_does_not_define_or_that_cannot_count_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_every_existing_entry_has_its_disposition_row
A6: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_a_census_finds_no_private_core_name_in_the_ledger
```

Each planted fault is a copy of an accepted ledger with one change, judged over a small tree the test builds in a
temporary folder, so each arm is seen to refuse alone. The committed ledger is judged over the repository's own tree.

## 4. File manifest

| path | part | change |
|---|---|---|
| `formal/obligations.json` | formal | new: the ledger |
| `scripts/tests/_formal_ledger.py` | guard tests | new: the ledger's checker and its census |
| `scripts/tests/test_formal_obligations.py` | guard tests | new: A1 to A6 |
| `docs/specs/SPEC-496-the-formal-ledger-names-every-engine-property-its-artifact-its-tie-and-its-gaps.md` | docs | new: this file |
| `docs/decisions/ADR-514-formal-completion-is-a-ledger-a-test-reads.md` | docs | new: the decisions |
| `docs/schematics/the-formal-obligations-ledger-and-its-checker.md` | docs | new: the components |
| `docs/red-first/SPEC-496.md` | docs | new: the red-first record |
| `changelog.d/formal-ledger-names-engine-496.md` | changelog | new: the fragment |

Neither new module under `scripts/tests` loads code through `importlib`, `runpy` or `exec`.

## 5. What this does NOT do

- Build any planned entry, harness or tie, or move any obligation past `planned`: each is the delivery that builds
  its entry, and that delivery edits its rows.
- Put `O12` to `O17` into a phase set: the phase sets stay R6's until a ruling widens them.
- Run TLC, Lean or Kani, or read their results: whether a property is clean, a theorem's axioms are admitted or a
  witness is caught is the formal checker's, outside this repository.
- Pin Kani or give the formal checker a Kani kind: the hub's delivery.
- The scheduling companion's own ledger, which judges its artifacts in its own tree.

## 6. Risks

- **The ledger drifts from the tree.** Every run judges the rows against the tree's entries and every present
  artifact against its header, so a new, renamed or removed entry or property fails the python stage by arm.
- **A checker gone blind.** Each arm is planted alone over a tree the test builds, and must refuse.
- **The census list misses a name.** The list grows by digest only; a hit is reported by line, so a red run
  publishes no name.
- **A planned entry is renamed by its delivery.** That delivery edits the ledger in the same pull request.

## 7. What only CI proves

The python stage over the whole suite at the pull request's head (`scripts/check.sh` stage python); the build ran
this file alone on python3 3.12.3.

## 8. Formal model

- **Lean: NOT APPLICABLE.** The checker judges a data file in a test; it decides no money, release, review ledger
  or gate verdict, and each of its arms is proved by a planted fault.
- **TLA+: NOT APPLICABLE.** One reader reads committed files; nothing interleaves.
- **No cover moves.** No file under `formal/tla`, `formal/lean` or `formal/vectors` changes, and no entry covers a
  new file.

## 9. Mutation rows

None. The checker and its tests sit under `scripts/tests`, outside the Python mutation population
(`scripts/mutation_python.py:9`); the delivery adds no Rust and no `scripts/<name>.py`. The band S49600-S49699 holds
no row.
