---
status: proposed
decision-makers: "the DeckStreak architect"
---

# The engine core holds the engine for both clients behind a per-transport table, and its owner-gesture token is contained by the crate graph

## Context and Problem Statement

ADR-337 decided that the engine core reaches Anki's backend only through an allow-listed
dispatcher, that each exempt function takes an `OwnerGesture` only the UI adapters construct and
that names one target, and that a containment test in the core enumerates every caller in the
workspace, reports the count, refuses any non-UI caller and refuses a planted caller by name.
ADR-335 makes one umbrella FFI crate link the engine core (and later the XP and FSRS-7 crates), and
ADR-336 and ADR-348 run the browser's engine "behind the same allow-list". ADR-345 built the
native adapter as one crate depending on the engine alone.

At `dev` `1eec0870e67e801fc3aa279318219d6986116ec7` (SPEC-345 section 1), no engine core exists.
The native adapter allow-lists five pairs (`crates/ffi/src/allow_list.rs:22-49`) and checks them
before the engine (`crates/ffi/src/engine.rs:87-96`). The web engine lists eight study calls
(`crates/web-engine/src/study.rs:74-83`) but checks them only in `run_method`
(`crates/web-engine/src/wasm.rs:277-280`): its named exports reach `run_service_method` directly
(line 63), and its reads pass SQL to the engine's database door (line 82). Nothing holds the two
tables equal, and no exempt write exists behind a token.

Those ADRs leave open where the core lives, how one table serves two transports whose lists
differ, whether the adapters keep their own tables, what holds the token's constructor to the
adapters, how a gesture's one target is checked, which never-list taps the first exempt table
admits, and how the containment test enumerates callers. This ADR decides them.

## Decision Drivers

- ADR-337's containment must be proved by a test, and the brief for #623 asks that the
  constructor's reach be held by the compiler, not by a check at run time.
- The web engine's native build must stay engine-free (ADR-348), and its engine dependency is
  `wasm32`-only.
- Rows `S33804` and `S33805` anchor on `web-engine/src/study.rs`, row `S33600` on the native
  adapter's refusal text, and the harness's open delivery (#616) anchors `S33900` on
  `ffi/src/allow_list.rs` and edits it. A row may not leave while its file stays.
- The settle census refuses a member whose feature another member turns on
  (`crates/progression/tests/xp_census.rs:805-836`).
- The owner-taps ruling admits only "one gesture's own write, on the card, note, preset or
  collection the gesture names", and keeps SYNC-01's guards on the one-way sync.

## Considered Options (the alternatives it was chosen against)

### Where the engine core lives

- A new crate, `crates/engine-core`, that both adapters depend on: chosen because it is the one
  shape in which both transports share one table and the umbrella FFI crate can link it (ADR-335).
- Grow `crates/ffi` into the core: rejected because the FFI crate carries UniFFI and becomes the
  umbrella that links the core (ADR-335), so the web engine would have to depend on the FFI
  adapter or keep its own dispatcher.
- Put the dispatcher in `crates/web-engine`: rejected because that crate is `wasm32`-only for the
  engine (ADR-348), so the native adapter could not link it.
- Keep a dispatcher in each adapter: rejected because it is today's state, two tables nothing holds
  equal, and the web engine's named exports bypass its own.

### One table for two transports

- One ordinary table whose rows carry a native and a web mark, read through the transport a dispatcher is started on: chosen because
  each pair is named once and each transport admits exactly what its client calls today.
- The union of both lists for both transports: rejected because it gives the native client
  `CloseCollection` and `AddNotes`, which it does not call, and reddens the native refusal test and
  round-trip test that pin (3,1) as refused.
- Two separate tables in the core: rejected because a pair both clients call would be named twice,
  and nothing would hold its name equal.

### The adapters' own tables

- Keep `ffi/src/allow_list.rs` and `web-engine/src/study.rs` unchanged, each checked first as today, and hold each equal to its transport's column with a parity test in the core: chosen because
  it moves no mutation row, leaves the harness's concurrent edit to the allow-list untouched, and
  keeps each adapter's boundary readable at the adapter.
- Move both tables into the core and re-export them: rejected because `S33804` and `S33805` could
  not leave `study.rs` while it stays, the harness edits `allow_list.rs` concurrently, and the web
  engine's native tests cannot name a core that is its `wasm32` dependency only.
- Delete the adapters' tables: rejected because the native adapter's `allowed()` would then have no
  production caller, and the adapter's boundary would no longer read at the adapter.

### What holds the token's constructor to the UI adapters

- The crate graph, held by a test: only the two adapters may depend on the core, so only they can
  name `OwnerGesture::from_tap`, and a manifest census plus a source census refuse any other path:
  chosen because the compiler refuses a crate without the edge, and the census refuses the edge
  and the two ways around it (`#[path]` and `include!`).
- A constructor behind a cargo feature only the adapters turn on: rejected because features unify
  across one workspace build, so every member would see the constructor, and the settle census
  refuses a member whose feature another member turns on.
- A check at run time (a caller tag, or the call stack): rejected because any caller can supply a
  tag, and nothing compiles it away.
- A sealed trait the adapters implement: rejected because the core cannot name the adapters' types
  without an edge pointing the wrong way.
- The adapters as modules of the core, with a `pub(crate)` constructor: rejected because the
  umbrella FFI crate links the core beside the XP and FSRS-7 crates (ADR-335), and the web engine is
  its own `cdylib` (ADR-348), so neither adapter can be a module of the core.
- The settle census's compiler-found callers (a deprecation under a census cfg, every target checked): rejected because
  the web adapter's entry compiles only for `wasm32`, so a native pass never reports it, and a
  `wasm32` pass needs the C toolchain only the `web-engine` job carries; the graph already bounds
  the callers to two crates.

### How a gesture's one target is checked

- The gesture names one target; `run_exempt` decodes the request as its write's own message, refuses it unless it names exactly that target, and passes the engine the message it checked, re-encoded: chosen because
  the ruling admits one gesture's own write on the one thing it names, and the engine then runs
  exactly what was checked.
- Typed constructors in the core that build each request: rejected because every tap's request
  shape would be written twice, once in the client and once in the core.
- No target check: rejected because a Forget gesture on one card could carry every card of a
  deck, and `RemoveNotes` removes the notes of its `card_ids` when `note_ids` is empty.
- Check the decoded request but pass the caller's bytes: rejected because the engine would decode
  bytes the check did not produce, so a parser difference would decide what runs.

### Which taps the first exempt table admits

- The six methods of entries 2, 3, 6, 7 and 8, each with one target at the pin: chosen because
  each maps to one method and one target the request names.
- Admit the one-way sync (1,6) now: rejected because the full-sync path is still to be measured
  (#620) and SYNC-01's guards are not built (#631, #633); a token alone would admit a full sync
  without them.
- Admit `UpdateDeckConfigs` (11,7) for the scheduler switch or a preset's deletion: rejected
  because one call of it saves presets, removes presets, turns FSRS on or off for the whole
  collection and reschedules, so no single target bounds it; the one-preset switch is the FSRS-7
  crate's (#641).

### What the core does with the engine's database door

- Closed reads (`Read::NoteCount`, `Read::CardSnapshot(id)`) whose statements the core holds: chosen because
  the door takes any SQL, writes included, and a table of service pairs cannot see it.
- Leave the door to the web adapter: rejected for that reason.

### How the containment test reads the workspace

- Every member's sources, examples, tests and build script, with the mirror's seven lines that name an engine write or its own port method held by text, each with its reason: chosen because
  a new line fails by name and no part of the tree is unread.
- Exclude test code: rejected because it leaves a blind spot, and the one engine upload in the
  tree today is a test fixture's.
- Move the mirror's engine port behind the core: rejected because the mirror is a server context
  whose port SPEC-022 decided, and its full download writes only DeckStreak's read copy, which no
  sync carries to the server.

## Decision Outcome

Chosen options: the first under each heading above.

- **D1. The core.** `crates/engine-core`, package `deck-streak-engine-core`, depends on `anki`,
  `anki_proto`, `prost` and `serde_json` and on no DeckStreak crate. It holds the engine's
  `Backend` privately behind a `Dispatcher` (`start`, `run`, `read`, and `run_exempt`), declares no
  feature and no `staticlib`, and is `Send` and `Sync` natively. The context map adds it as
  `depends on: nothing`, and both adapters as `depends on: engine-core`.
- **D2. The table.** One ordinary table with a native and a web mark per row, and one exempt table;
  `Dispatcher::start(Transport, init)` fixes the transport. `run` admits the transport's ordinary
  pairs, refuses an exempt pair as `NeedsGesture` and any other as `NotAllowed`.
- **D3. The adapters.** Each keeps its table, checked first, unchanged; a parity test holds each
  equal to its column, and a source test holds each to its own transport. The native adapter's
  `EngineRefusal` and its text are unchanged; its exempt entry has its own refusal type.
- **D4. Containment.** The graph (only the two adapters name the core; only the core and `ingest`
  name `anki` outside dev-dependencies; nothing names an adapter) plus a source census in the core
  (the constructor and `run_exempt` named only in the adapters' entry files; the engine's write
  names only at the held lines; no `#[path]` or `include!` of a core file), each with its examined
  count and a planted caller refused by name.
- **D5. The target.** `OwnerGesture` is one write and one target, neither `Clone` nor `Copy`, built
  only by `from_tap`, which refuses a target of another kind. `run_exempt` consumes it, checks the
  decoded request against the target, and passes the re-encoded message.
- **D6. The first exempt table.** (13,17) Forget, (13,19) set due date, (11,5) delete a preset,
  (23,15) change note type, (5,2) delete a card, (25,7) delete a note. (1,6), (11,7) and (13,26)
  stay refused.
- **D7. The door.** `Dispatcher::read` takes a closed `Read`; no adapter passes SQL.
- **D8. The census's population.** Every member's sources, examples, tests and build script; the
  mirror's seven lines are held by text with their reasons.

### Consequences

- Good: both clients share one table; the web engine's named exports no longer bypass it; the
  engine's database door takes no adapter SQL; a non-UI crate cannot name the token without a
  manifest change the census refuses.
- Good: no mutation row moves, and the harness's delivery can land either side of this one; the
  parity test names the render pair the second to land must add.
- Bad: each adapter's table is the core's column written a second time, held only by the parity
  test.
- Bad: the mirror's engine port is held by text, not by the compiler; a reviewer admits each new
  line by name.
- Neutral: a new exempt tap adds an exempt row, its target rule and its rows here, and its screen
  and its gesture-handler census in the client that taps it.

### Confirmation

SPEC-345's A1 to A11 (part 1) and section 7's A12 to A18 (part 2), and the mutation rows in
`S345`.

## What would make this wrong

- The engine adds a single-target method for the scheduler switch or for undo after a sync: the
  exempt table grows by that row, with its target rule.
- The full-sync measurement (#620) shows that a one-way sync is more than one call: the gesture
  then names a sequence, and this ADR's one-call shape is amended.
- A third client transport appears: the table's marks become a set of transports.
- The adapters' tables drift so often that the parity test is the usual red: the tables move into
  the core in a delivery that re-anchors their rows.

## More Information

ADR-301 (a), ADR-335, ADR-336, ADR-337, ADR-345, ADR-348, ADR-022, the owner-taps ruling in
`docs/rulings/`; SPEC-022, SPEC-334, SPEC-336, SPEC-338; #616, #620, #623, #624, #631, #633, #641.

## Amendment: part 2's refusal and the census's measured population (SPEC-345)

Part 2 builds D5 and D8 at `dev` `79f321902f729a6d703660bc9d3810d3d7728a4e`. D8 is read with
SPEC-345 section 8's held lines: 25 lines, 20 pairs of a file and an exact trimmed line in 12
files. 21 of them are `crates/ingest`'s lines that name the engine's write and door names (16
pairs in 10 files, the mirror's skip-take write of #600 and its census's string literals
included), where the design read seven. The other four are two test files' lines that name the
gesture. The rest of the Decision Outcome stands. Part 2 decides five things it left open.

The gesture's refusal:

- One core refusal, `GestureRefusal` (`WrongKind`, `NotTheTarget`, `Undecodable`, `Engine`), returned by `from_tap` and `run_exempt`, with `Refusal` keeping its three variants: chosen because no existing match on `Refusal` changes, and each adapter maps the new type to its own refusal as D3 says.
- Two new `Refusal` variants, `TargetMismatch` and `Malformed`, as the design draft had them: rejected because three exhaustive matches on `Refusal` would stop compiling (the native adapter's map into `EngineRefusal`, whose variants and text row S33600 holds; the web engine's `call`; SPEC-348's pair test), and a gesture's refusal is not the ordinary table's.

How the census holds a line (D8):

- Each held line by its file, its exact trimmed text, its count and its reason: chosen because a new line, a second copy of a held line, or more text on a held line fails by name, and an edit that moves lines moves no hold.
- A held pattern per file (a name, or a whole file): rejected because a new call in a held file would then pass unread.
- A held line number: rejected because any edit above a held line would move it, and the census would refuse a line it should hold.

The census's scope:

- Outside the core only, the core's own seven engine-name lines unheld: chosen because R10 says "outside the core", the graph census already bounds who reaches the core, and a sibling's new core line then costs no census edit.
- The core's lines held by text too: rejected because every new core read or write path would edit the census, while the core is the engine's one holder by D1.

The web export's killer:

- The export's statements owed in one new `OWED` entry of `crates/web-engine/tests/boundary.rs`, insert-only, its tap-to-write arms included: chosen because the export compiles only for `wasm32`, so its mutants run natively and only that source census can read them, as it does for part 1's boundary.
- A native build of the web boundary so that a test runs the export: rejected because it rewrites the whole boundary module, which this part does not touch beyond the export.

The adapters' test lines that name the gesture:

- chosen: hold the native adapter's test lines by text; rejected: rename R9's entry, which moves a requirement's name to satisfy a test.
- The boundary census's `OWED` literals that name the export and the gesture's constructor, held by text as #600's census literals are: chosen because they are data a test compares, not calls, and a held literal copied or extended still fails by name.
- String literals stripped before the gesture's names are read: rejected because the census reads the engine's names in literals too, and a literal can carry a call into a macro, so one stripping rule serves both.

## Amendment: the answer is held by its own token and its own table set (SPEC-365)

ADR-376 amends D2 and D4. D2's two tables, ordinary and exempt, gain a third set, `ANSWERED`, whose
one row is AnswerCard (service 13, method 4): `decide` answers `Decision::NeedsAnswer` for it on both
transports, `Dispatcher::run` refuses it with `Refusal::NeedsAnswer`, and `Dispatcher::run_answer`
is the one door that records a grade, consuming an `OwnerAnswer` that holds one card and one grade.
D4's census holds the answer's names, entry calls and held lines as it holds the gesture's, with no
gesture row, name or assertion removed or narrowed. The rest of D2 and D4 stands.

- A third set, `ANSWERED`: chosen because the table test's arms and `run`'s refusal then tell "only by a press" apart from "never".
- AnswerCard kept in the ordinary table with both transport marks false: rejected because `run` would refuse it as not allowed, and a reader could not tell a held answer from a forbidden call.
- AnswerCard in `EXEMPT`: rejected because `EXEMPT` is the never-list's exemption under ADR-301, and joining it would put every grade under the owner-taps ruling's conditions.
- The same census file, extended: chosen because the population and the walker are the same.
- A second census file for the answer: rejected because it copies the walker, and two walkers must then be kept equal.

## Amendment: the one-way sync is an exempt write (SPEC-364)

ADR-375 D5 amends D6. (1,6) joins `EXEMPT` as `OneWaySync`, its target the collection, and
`run_exempt` refuses it: it runs only through the core's full-sync driver, with the owner's
gesture and the choice's `Write`. The gesture still names one call, (1,6) on the open collection,
so the one-call shape stands: the server copies are fetched on private engines into empty files,
which replace nothing the device holds. (11,7) and (13,26) stay refused.

- (1,6) as an exempt write that needs both the gesture and the choice's `Write`: chosen because the restore after eviction and every full-sync conflict need a write, and the choice's counts, backup and re-check then come before it.
- (1,6) kept refused, as D6 had it: rejected because neither client would then have a one-way write at all; ADR-375 D5 names the other options and why each lost.

## Amendment: Undo reverts only the review's own last answer (SPEC-371)

ADR-382 amends D2, D4 and D6. The rest of each stands.

- **D2 (the table, `:150-152`).** Undo (3,8) leaves the ordinary rows and joins the exempt table
  as `ExemptWrite::Undo`, `TargetKind::Card`, deciding `NeedsGesture` on both transports.
  HtmlToTextLine (27,14) joins the ordinary rows, web only, so the review can show the card it
  would undo as one line of text. ADR-382 D1 and D6 decide both, and name what each was chosen
  against.
- **D4 (containment, `:156-160`).** The census's engine names gain `undo`, and its held lines gain
  four entries outside the core, each with its reason, and the boundary census's owed literals of
  `undo` that name the gesture (ADR-382 D9).
- **D6 (the first exempt table, `:164-166`).** The table holds eight rows. The eighth, Undo,
  reverts only the review's own last answer while it has not synced, and is checked at the write
  (ADR-382 D2, D3). The undo after a sync this record anticipated (`:192`) is still unbuilt.

## Amendment (SPEC-386, ADR-400): the core's one workspace edge

D4's sentence that the core depends on no crate of this workspace now has one exception, `deck-streak-fsrs7`, reached
through `crates/engine-core/src/replay.rs` alone and held by the graph census
(`only_the_replay_module_names_the_fsrs7_crate`). The two client adapters remain the core's only dependents.

Chosen against:
- Each client adapter depending on the FSRS-7 crate: two joins of one rule, and the web join cannot be tested natively against the engine.
