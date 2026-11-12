# Red-first record: SPEC-345

Part 1 of SPEC-345 (R1 to R6, A1 to A11). The SPEC, ADR-356 and the schematic were committed
first (4e88991b), and the core's shape next (81e92126): empty tables, a `decide` that refuses every
pair, and a `run` and `read` that refuse everything. The tests of A1 to A6 were then committed
alone (24e5ed54) and run over that shape, and the graph census of A7 and A8 alone (beae6cdf),
before any manifest or map changed. Each red below is quoted from the run at its commit.

```red-first
A1: red at 24e5ed54: the pairs Native admits left: {} right: {(3, 0), (3, 8), (7, 13), (13, 3), (13, 4)}
A1: green at 750fc82a
A2: red at 24e5ed54: left: [] right: [(Forget, 13, 17, "SchedulerService.ScheduleCardsAsNew", Card), (SetDueDate, 13, 19, "SchedulerService.SetDueDate", Card), …(4)]
A2: green at 750fc82a
A3: red at 24e5ed54: the native dispatcher opens the collection left: Err(NotAllowed { service: 3, method: 0 }) right: Ok(())
A3: green at 750fc82a
A4: red at 24e5ed54: left: (Err(NotAllowed { service: 3, method: 0 }), Err(Engine { error: [] }), Err(Engine { error: [] })) right: (Ok(()), Ok(Some(Number(<the second card's id>))), Ok(Some(Number(2))))
A4: green at 750fc82a
A5: red at 24e5ed54: crates/ffi/src/allow_list.rs: in the adapter's table only: [(3, 0, "BackendCollectionService.OpenCollection"), …(4)]; in the core's column only: [] left: {} right: {…(5)}
A5: green at 750fc82a
A6: red at 24e5ed54: crates/ffi starts its dispatcher on Transport::Native in src/engine.rs alone, and never names Transport::Web left: ([], []) right: (["src/engine.rs"], [])
A6: green at e0ece388
A7: red at beae6cdf: the members that name the core, and the table each names it in left: [] right: [("ffi", Normal, None), ("web-engine", Normal, Some("cfg(target_arch = \"wasm32\")"))]
A7: green at e0ece388
A8: red at beae6cdf: docs/CONTEXT-MAP.md's fence left: [("deck-streak-engine-core", None), ("deck-streak-ffi", Some("nothing")), ("deck-streak-web-engine", Some("nothing"))] right: [("deck-streak-engine-core", Some("nothing")), ("deck-streak-ffi", Some("engine-core")), ("deck-streak-web-engine", Some("engine-core"))]
A8: green at e0ece388
A9: not red: it pins the native adapter's refusal text, which the base already has; it guards the rewire (row S33600)
A10: not red: it pins the native adapter's round trip, which the base already has; it guards the rewire through the core
A11: not red: it pins the web engine's study-call table, which the base already has; it guards the rewire
```

## What each red disclosed

- **A1 to A6 failed on their own assertions, not on a build.** The shape commit compiles, so each
  test ran and failed on the behaviour its criterion names: an empty admitted set (A1), an empty
  exempt table (A2), an open refused before the engine saw it (A3), a read refused (A4), an empty
  column against each adapter's table (A5), and no adapter naming a transport (A6). A6 and A1 print
  their examined counts (4 source files of the native adapter; 4225 pairs per transport).
- **A7 and A8 were committed before any manifest or map changed.** A7's census examined 28 member
  manifests and found no member naming the core; A8 read 30 lines of the map's fence and found no
  line for the core and `nothing` for both adapters. Each test also judges a planted tree or map in
  the same test; those controls run after the real tree's assertions, so they were run on their
  own before the commit, with the real tree's assertions set aside in a working copy that was then
  restored byte for byte: both planted controls passed.

## Green, and what stayed green

- **A1 to A5 went green with the core (750fc82a).** The tables were filled and `decide`, `run`
  and `read` written over them; the adapters were not yet touched, so A5's parity already held
  against the tables both adapters keep.
- **A6 to A8 went green with the rewire (e0ece388).** The native adapter starts the core on the
  native transport and the web engine on the web one; ffi and the web engine name the core in place
  of the engine, and the map declares the core and both edges. A7's census examined 28 member
  manifests, A8 read 31 lines of the fence, and A6 examined 4 source files in each adapter.
- **A9 to A11 stayed green at every commit**, the rewire's included (e0ece388): the native
  refusal text, the native round trip and the web engine's study table pass unchanged.
- **The commit between them (228d303c) changed no assertion.** It holds the core's test targets to
  clippy: an allowance for `expect` and the examined counts' printing in each target, as the
  workspace's census tests carry, and clippy's idioms in the graph census's header reader.

Correction (merge round): the reds this round measured are its own. At the merge of dev (export of
6936640d) `deck-streak-engine-core::parity each_adapter_table_equals_its_transport_column` and
`deck-streak-ffi::render a1_renders_the_queued_cards_question` failed, and nothing else in the
two packages did, because the swift harness added the pair (27, 6) to the native allow-list after
this pull request's CI ran; both are green at the cure. The red lines above are an earlier
round's: A1 to A8 are the build round's own measurements at 24e5ed54 and beae6cdf, as the record's
opening says, and A9 to A11 were never red. The later build and fix rounds added no line to this
record, and the mutation rows' proofs and the wasm boundary census they added are theirs.

## Part 2: the owner-gesture token and the containment census

Part 2 of SPEC-345 (R7 to R10, A12 to A18; sections 7 to 9). The amendments came first, then the
shape: an `OwnerGesture` whose `from_tap` refuses every target, a `run_exempt` that refuses every
gesture, and a native entry that refuses every tap and names no gesture. The tests of A12 to A18
were then committed alone and run over that shape, the containment census with them. Each red below
is quoted from the run at its commit.

```red-first
A12: red at 341f85af: gesture.rs:52 left: ([], [WrongKind { write: Forget, target: Card(11) }, …(17)]) right: ([(Forget, Card(11)), (SetDueDate, Card(11)), (DeletePreset, Preset(13)), (ChangeNoteType, Note(12)), (DeleteCard, Card(11)), (DeleteNote, Note(12))], [WrongKind …(12)])
A12: green at 695773a6
A13: red at 341f85af: exempt.rs:323 left: (Err(WrongKind { write: Forget, target: Card(..) }), Some(1), Some(1), ..) right: (Ok(()), Some(0), Some(0), ..)
A13: green at 695773a6
A14: red at 341f85af: exempt.rs:359 left: [(Forget, "refused before the engine: the Forget write does not take Card(..)"), …(5) refused before the engine] right: [(Forget, "ran"), (SetDueDate, "ran"), (DeletePreset, "the engine refused it"), (ChangeNoteType, "ran"), (DeleteCard, "ran"), (DeleteNote, "ran")]
A14: green at 695773a6
A15: red at 48f55c8f: containment.rs:570 each UI adapter's entry file builds the gesture from the tap and runs it left: [] right: ["crates/ffi/src/engine.rs", "crates/web-engine/src/wasm.rs"]
A15: green at ebda7f5f
A16: not red: the shape's `OwnerGesture` derives nothing, so the probe that it is neither `Clone` nor `Copy` passes over the stub; it guards the type against a later derive
A17: red at 48f55c8f: exempt.rs:157 the Forget tap returned its one card to the new queue left: (Err(WrongKind), Head { card_id: …, queue: 1, new: 0, learning: 1 }) right: (Ok(()), Head { card_id: …, queue: 0, new: 1, learning: 0 })
A17: green at 9222c052
A18: not red: mutation coverage of the native refusal text written with the shape, as A9 is for part 1's refusals
```

### What each red disclosed

- **A12 to A14 failed on their own assertions over the shape (341f85af).** The shape compiles, so
  each test ran: the stub refused the write's own kind (A12), refused the Forget gesture, so its
  one card was not reset (A13), and refused the target-only request that A14 asserts first,
  so A14's red is its positive arm and not one of its refusals.
- **A15 failed on its positive artifact (48f55c8f):** neither entry file named a caller yet, while
  its absence assertions held over the real tree. Its planted half was checked before that commit
  by a temporary test in a working copy, removed again with the file restored from a copy, and its
  14 planted refusals equalled the expected list.
- **A17 failed on its positive artifact (48f55c8f):** the native entry refused the Forget tap, and
  the card stayed in learning (queue 1, new 0, learning 1), the precondition measured equal.
- **A16 and A18 were never red, by reason:** the shape's `OwnerGesture` derives nothing, so A16
  guards it against a later derive; A18 pins the native refusal text written with the shape, and
  row S34527 proves it can fail.
- **One commit edited tests between A15's red and its green: 71c43716, alone.** It grew the wasm
  boundary census's `OWED` by the `run_exempt` entry (red at 71c43716: boundary.rs:321, the export
  `fn run_exempt(` occurs 0 times, not once) and the containment census's held set by the three
  boundary literal rows (17 to 20), each held by its exact text with its count and reason (section
  8). It removed no assertion and relaxed none; A15 then read only the web entry missing, and the
  export at ebda7f5f made both green.

### Green, and what stayed green

- **A12, A13, A14 and A16 went green with the core (695773a6):** `from_tap`'s kind check, and the
  per-write decode that runs a request only when it names the one target.
- **A17 and A18 went green with the native entry (9222c052)**, which builds the gesture from the
  tap and maps its refusal to the adapter's own.
- **A15 went green with the web export (ebda7f5f):** the census examined 679 files, 25 caller
  lines outside the core and 14 planted refusals, and printed no drill-named line.
- **The rows came after the last green (fae58d85)** and changed no test: S34520 to S34527, each
  proved against its criterion's test.
