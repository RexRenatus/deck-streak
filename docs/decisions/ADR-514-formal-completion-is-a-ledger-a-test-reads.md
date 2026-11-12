# ADR-514: Formal completion is a ledger a test reads

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-496 |
| builds on | ADR-295 (the formal settings file; none amended) |
| measured at | `5c44700b` |

## Context

DeckStreak's engine is being replaced by its own scheduler and its own sync, store, import, media, search and
rendering code, and every property that replacement relies on is to get a checked artifact in TLA+, Lean or Kani, a
named tie to the shipped code, and a published list of what stays unproved. At `5c44700b` the tree holds 43 formal
entries (27 TLA+, 16 Lean) and no file that says which engine property each one carries, which test binds it to the
code, or what it leaves unproved; eleven Rust tests read Lean vectors, every TLA+ entry is bound by `@phx covers`
stamps alone, and CI runs no formal tool (SPEC-496 section 1). This ADR decides what "formal completion" is in this
repository and how a test holds it.

## Considered Options (the alternatives it was chosen against)

- **A ledger a test reads** (chosen): one JSON file, `formal/obligations.json`, with a closed schema, and a checker in
  the python stage that refuses each malformed or overclaimed row by name.
- **A prose list in the documentation**: lost, because nothing fails when it drifts from the tree, and a claim of a
  tie or a status reads the same whether or not the test exists.
- **A README per formal entry**: lost, because an obligation spans several entries and harnesses (one property may
  be carried by a model, a theorem and a harness at once), and a planned entry has no folder to hold its README.
- **No ledger** (today): lost, because completion then has no definition a commit can be judged by, and a property
  bound by covers stamps alone reads as proved.

## D1. The ledger and its schema

`formal/obligations.json` holds exactly `schema` (`formal-obligations.v1`), `phases`, `obligations` and `entries`.
Every object level is closed: an unknown key, a missing key or a value of the wrong type is refused (arm `shape`), so
a ledger cannot carry a field no reader judges. An obligation's `status` is one of `planned`, `artifact`, `tied` and
`complete`, in that order of progress, and moves back on any regression. Completion is tracked, never gating: every
formal property enters at `ramp=report` and can block only after `config/formal.json`'s `k` clean landings, and this
ledger blocks only its own malformed rows.

## D2. An obligation names its artifacts and its gaps

An obligation holds at least one artifact (arm `no-artifact`) and at least one non-blank gap line (arm `no-gap-line`).
A gap line is what no tool closes: the bounds a model holds at, the code a Lean port does not extract, the compiler,
database or platform that is trusted. An obligation with no gap line claims a completeness no formal tool gives.

The phase sets are `"1"`: `O1a`, `O4`, `O5`, `O6`, and `"2"`: `O1` to `O9` and `O11`, and a phase set naming an id the
ledger lacks is refused (arm `phase`). `O10` joins a set only if a learner companion ships. `O12` to `O17` are tracked
in no phase set until a ruling widens the sets.

## D3. An artifact is planned or present, and a present one is declared

An artifact names its kind (`tla`, `lean`, `kani`) and its state. A `present` TLA+ or Lean artifact names an entry and a
property that the entry's own `@phx` header declares in the checked tree (arms `undeclared-entry`,
`undeclared-property`); the checker parses the headers itself, because CI runs no formal tool. A `present` Kani
artifact names `at`, a function of a Rust file marked `#[kani::proof]` (arm `undeclared-harness`). A `planned`
artifact names the entry or harness its delivery will build, and is not looked up. An obligation past `planned` holds
no planned artifact (arm `planned-past-planned`), so a status cannot be raised over an artifact nobody built.

An artifact of the scheduling companion's own tree holds `"tree": "companion"` and an `about` line and names no entry,
property or harness. Its names are private, and its own ledger judges it. No tie of this tree can name it, so an
obligation that holds one reads at most `artifact` here.

## D4. A tie counts only when it can fail

A tie names its `method`, one of six (arm `tie-method`): `vectors`, `trace`, `kani-identity`, `mbt`, `census`,
`cross-target-hash`. It names, in `for`, the artifact of its own obligation that it binds (arms `tie-without-for`,
`tie-for-unknown`), and in `test` a test the checked tree defines (arm `undefined-test`): a Rust function marked
`#[test]`, `#[tokio::test]` or `#[kani::proof]`, comments not counted, or a `test` method of a top-level Python class.

A tie counts when its method is `vectors`, `trace`, `kani-identity` or `mbt`, its test is defined, and an `mbt` tie
names the code mutant that turns it red. `census` and `cross-target-hash` ties are recorded and never count: they test
an absence or an equality and prove no property. A `@phx covers` stamp is never a tie: it fails when the covered span
is edited, never when the code diverges from the model. An obligation at `tied` or `complete` with an artifact no
counting tie binds is refused (arm `untied-artifact`); the planted control is an obligation whose artifact is bound by
covers stamps only. Such an obligation stays at `artifact` with the gap line "tied by covers only".

`complete` also needs every property clean, every witness caught, every theorem's axioms admitted and every counting
tie passing with an examined count above 0. Those are run results that the formal checker reads outside this
repository, so the ledger's checker holds only the structure that makes them meaningful.

## D5. Every formal entry of the tree is a row

`entries` holds one row per formal entry of the tree (arms `entry-without-row`, `row-without-entry`, `duplicate-row`).
A row's `disposition` is `keep`, `re-stamp`, `re-point` or `retire`, its `act` says what happens and when, and its
`obligations` are exactly the obligations whose artifacts name the entry (arm `row-obligations`). At `5c44700b` the
rows are the 43 existing entries:
- two models of the engine's whole-collection sync retire with that sync;
- four models are re-pointed onto the replacement, keeping their properties;
- seven models and one Lean entry are re-read and re-stamped when the spans they cover move;
- the other 29 are kept, the replay history's entry while the FSRS-7 crate stays.

## D6. The census holds names by digest

The ledger is public, and the scheduling companion's names are not. A census over every line refuses a token derived
from the scheduler's brand, at a word start or a camel hump (arm `private-brand`), and a token, or a part of it
between `-`, `.` and `:`, whose salted SHA-256 digest is on the census list (arm `private-name`). A hit is reported by
its line, never by its text, so a red run publishes nothing.

- **Literal names in the test**: lost, because the test is as public as the ledger.
- **A private list outside the tree** (the public scrub's form): lost, because CI never reads it, so the census
  would judge only the brand there.
- **Digests** (chosen): CI reads them, they name nothing, and the maintainer adds a name by adding its digest.

## D7. The checker sits beside its test

The checker is `scripts/tests/_formal_ledger.py`, and `scripts/tests/test_formal_obligations.py` runs it. The tests
are committed first over a stub of the checker that keeps its names and refuses nothing, so each planted control is
seen red for its own reason; the checker then replaces the stub, and the test file does not change. Keeping the
checker inside the test file was rejected: its red stub would change the test between the red and the green commit. Both
files sit under `scripts/tests`, which the Python mutation population excludes, as the formal settings test does.

### Consequences

- Good, because a commit's formal completion is a question a test answers, and an overclaimed status, an invented tie
  or a missing gap line fails the python stage by name.
- Good, because a delivery that builds an entry, adds a tie or retires an entry edits one file, judged on the spot.
- Bad, because the planned rows name entries before they exist; a delivery that renames one edits the ledger.
- Bad, because the companion's obligations read at most `artifact` in the public ledger, and their completion is read
  in the companion's own ledger.

### Confirmation

SPEC-496 A1 to A6, in `scripts/tests/test_formal_obligations.py`: each arm refuses a planted fault over a tree the test
builds, and the committed ledger is accepted over the repository's own tree.
