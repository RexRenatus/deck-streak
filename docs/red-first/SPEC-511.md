# Red-first record: SPEC-511

The SPEC, ADR-529, the loader's schematic and the context map's new row were committed first (15f1453c). The tests
came next, whole, over stubs that keep every shape but the behaviour (6a5b3eaf): the route's three paths are missing and
the directory setting reads every value as off; the Worker's loader fetches no manifest, checks no digest and
instantiates the module with an import object of its own holding `env.x`, `Absent` answers no bytes, the checkpoint
store keeps nothing and the load sequence keeps whatever it reads; the web engine's `decide` answers one constant
refusal, its load check keeps everything and its `restore` and `purge` build no frame, and `wasm.rs` exports none of the
four; the size gate takes no `--owner-file`, and the site census passes every site having read nothing. Two later test
commits fixed the size test's own helper (63e6eef8, c02de7dc). 75cce0ca turns them green.

Each Rust red is quoted from one run at 6a5b3eaf of `cargo test --locked --offline --no-fail-fast -p deck-streak-api
--test companion_route` (6 failed, 1 passed) and of `-p deck-streak-web-engine --test companion_decide --test
companion_load --test boundary` (7 failed, 11 passed); each web red from one run at 6a5b3eaf of `pnpm --dir web/app
exec vitest run src/lib/engine/companion.test.ts` (7 failed); C17's red from `python3 -B -m unittest discover -s
scripts/tests -p test_owner_bundle_size.py` at c02de7dc (3 failed), and C18's from the census's module at 6a5b3eaf (4
failed). Each green is the same command at 75cce0ca.

```red-first
C1: red at 6a5b3eaf: crates/api/tests/companion_route.rs:279: assertion `left == right` failed: no cookie on /api/companion/v1/manifest.json (left: 404, right: 401)
C1: green at 75cce0ca
C2: red at 6a5b3eaf: crates/api/tests/companion_route.rs:318: assertion `left == right` failed: /api/companion/v1/manifest.json (left: 404, right: 200)
C2: green at 75cce0ca
C3: red at 6a5b3eaf: crates/api/tests/companion_route.rs:350: the route serves exactly the three names (left: Err("served beyond the three: []; not served: [\"/api/companion/v1/manifest.json\", \"/api/companion/v1/module.wasm\", \"/api/companion/v1/weights.bin\"]"), right: Ok(()))
C3: green at 75cce0ca
C4: not red: the census's control reads a planted router and its own judgement only, so it held over the stub as over the route
C5: red at 6a5b3eaf: crates/api/tests/companion_route.rs:390: a file of the cap is served (left: 404, right: 200)
C5: green at 75cce0ca
C6: red at 6a5b3eaf: crates/api/tests/companion_route.rs:436: no session (left: 404, right: 401)
C6: green at 75cce0ca
C7: red at 6a5b3eaf: crates/api/tests/companion_route.rs:464: companion refuses start: Ok(None)
C7: green at 75cce0ca
C8: red at 6a5b3eaf: companion.test.ts: AssertionError: a manifest that answers 404: expected true to be false
C8: green at 75cce0ca
C9: red at 6a5b3eaf: companion.test.ts: AssertionError: expected [ …(2) ] to deeply equal [] (the stub admits both one-byte changes)
C9: green at 75cce0ca
C10: red at 6a5b3eaf: companion.test.ts: AssertionError: expected [ …(2) ] to deeply equal [] (the stub links both modules that ask for an import)
C10: green at 75cce0ca
C11: red at 6a5b3eaf: companion.test.ts: AssertionError: expected [ [ …(2) ], [ …(2) ] ] to deeply equal [ [ …(2) ], [ …(2) ], [ …(2) ] ] (the stub fetches two names, not the manifest first); and AssertionError: no memory export: expected true to be false
C11: green at 75cce0ca
C12: red at 6a5b3eaf: companion.test.ts: AssertionError: expected false to be true (the stub store stores nothing)
C12: green at 75cce0ca
C13: red at 6a5b3eaf: companion.test.ts: AssertionError: verdict 0: expected [] to deeply equal [ 'restore 5 3700018400feff7f', …(1) ]
C13: green at 75cce0ca
C14: red at 6a5b3eaf: crates/web-engine/tests/companion_load.rs:218: a fixture's hello does not negotiate: NotNegotiated's code (left: 0, right: 1); companion_load.rs:238: the table refuses the planted check by the row it breaks (left: ["a fixture's hello does not negotiate: Keep, not NotNegotiated", ...], right: ["the head names no review: Keep, not HeadNamesNoReview"])
C14: green at 75cce0ca
C15: red at 6a5b3eaf: crates/web-engine/tests/companion_load.rs:249: a restore frame: Truncated
C15: green at 75cce0ca
C16: red at 6a5b3eaf: crates/web-engine/tests/companion_decide.rs:286: assertion `left == right` failed (left: ["{\"card\":42,\"due\":[42,3,103],...}: web [1, 2, 255], native ...", ...], right: []); companion_decide.rs:352: the absent reply (left: [1, 2, 255], right: [1, 3, 255])
C16: green at 75cce0ca
C17: red at c02de7dc: test_owner_bundle_size.py: AssertionError: 2 != 1 (usage: web-engine-size [-h] --module MODULE --bindings BINDINGS; web-engine-size: error: unrecognized arguments: --owner-file ...)
C17: green at 75cce0ca
C18: red at 6a5b3eaf: test_companion_site_census.py: AssertionError: 0 != 1 : a module.wasm: companion-site-census: examined 0 file(s)
C18: green at 75cce0ca
C19: red at 15f1453c: test_context_map_schematic.py: test_every_arrow_is_a_manifest_edge_the_fence_names: AssertionError: findings: 2 (web-engine --> companion: an arrow no manifest names; web-engine: the fence names companion, the manifest does not)
C19: green at 6a5b3eaf
```

## What each pair disclosed

- **The boundary census was red for the four exports too.** At 6a5b3eaf `each_boundary_function_reaches_the_engine_through_the_dispatcher`
  read `companion_decide: \`fn companion_decide(\` occurs 0 times, not once` and the same for `companion_restore`,
  `companion_load_check` and `companion_purge`, and `a_boundary_function_that_answers_a_constant_is_refused_by_name`
  stopped on the first of them; both pass at 75cce0ca, with the census's 39 boundary functions.
- **C17's first two runs were red for the helper, not only for the gate.** At 6a5b3eaf and 63e6eef8 the helper that sizes
  four files to an exact `gzip -9` total never reached it (`no four files sum to 8000001 bytes gzip -9`): its random pools
  were too short, then a near miss moved two files against each other. c02de7dc's run is red for the gate's refusal alone.
- **C19 was red at the docs commit, the other way round from the SPEC's first draft.** The fence and the drawing named the
  web engine's edge before any manifest did (15f1453c). The same commit's map also turned the engine core's
  `the_context_map_declares_the_core_and_its_two_edges` red, which reads the web engine's row as `engine-core`; 6a5b3eaf
  reads `companion, engine-core` there.
- **The planted controls read only what they plant.** C4's fourth route, C9's digest that answers the manifest's own,
  C10's instantiation handed `env.x`, C11's planted `import(` and `importScripts(` lines, and C14's check that skips the
  head clause each judge a planted subject with the census of its test, so they prove the census can fail.
