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
