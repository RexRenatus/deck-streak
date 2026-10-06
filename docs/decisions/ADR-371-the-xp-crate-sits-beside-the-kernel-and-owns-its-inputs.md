---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# The XP crate sits beside the kernel, owns its inputs, reads economy.json itself, and progression translates at its edge

## Context and Problem Statement

SPEC-334 R13 puts per-review XP on both clients "from the XP crate", computing the predecessor's
math "held by the parity oracle"; R3 places that crate beside the engine, never linked into it;
SPEC-334's header says the XP crate joins the context map "through its own delivery's ADR"; and
ADR-357 D1 has it join the umbrella FFI crate by an unconditional edge in the change that first
exposes it. ADR-012 settles that the rule is proved against the predecessor's goldens, and ADR-040
that an XP amount is unsigned. None of them settles what #635 must decide:

1. where the crate sits in the crate graph and in the map's layers;
2. which types it takes, given that the rule today takes ingest's `Review` and `Tier`;
3. who calls it on the server, and whether the old body stays;
4. where its constants come from, given SPEC-072 R2 forbids a typed constant;
5. where the study-event guard the rule starts with lives;
6. how CI refuses an I/O dependency, a kernel edge, or a dependent the map does not draw;
7. whether it is compiled for `wasm32` and the Apple targets now or with its first client;
8. how the parity oracle proves the server's XP unchanged.

Read at `dev` `b32a40b4`: the rule is `crates/progression/src/review_xp.rs:20`; it imports
ingest's `Review`, `is_study_event` and `Tier` (`:7-8`) and progression's parsed economy (`:10`);
progression depends on the kernel, ingest and `sqlx` (`crates/progression/Cargo.toml:14-22`), and
the kernel on `sqlx` and `tokio` (`crates/kernel/Cargo.toml:19,21`).

## Decision Drivers

- The crate graph is the context map: an edge is drawn in the change that first uses it, an unused
  edge is refused, and a change that needs a new edge is answered by an ADR that amends the map.
- The map's layers point down: the kernel at the bottom, domain contexts above it that depend on
  no sibling domain context, adapters and the root on top (`docs/CONTEXT-MAP.md:73-90`).
- An adapter translates at the edge, so a client crate never learns a server context's types, and
  the anti-corruption layer for Anki (ingest) never learns a game rule.
- Every constant is read from `economy.json`, never typed (SPEC-072 R2; the game-economy pack).
- The server's XP must not move by one point: the parity oracle's 265 cases are the oracle, and
  public CI never regenerates a golden (ADR-012).
- A census is held by a positive artifact and a planted control, and prints what it examined.
- A client graph lacks ingest, whose dependency turns on serde_json's `float_roundtrip` for every
  server build; a client build must parse the multipliers to the same bits.

## Considered Options (the alternatives it was chosen against)

### D1, where the crate sits

- Chosen: a new crate, `deck-streak-xp` at `crates/xp`, in the map's bottom layer beside the
  kernel, `depends on: nothing`, because it is the one place both the server and a client can link
  without the kernel, and an edge from progression to it points down, as every edge must.
- A module of progression, gated by a cargo feature: lost, because a client would still link
  progression's manifest, whose kernel, ingest and `sqlx` edges are unconditional, and features
  unify across a workspace build.
- A domain context in the middle layer: lost, because progression would then depend on a sibling
  domain context, which the map's second layer forbids.
- The kernel: lost, because it pulls `sqlx` and `tokio`, and a game rule placed in the kernel is a
  context hiding in shared code.
- The engine core: lost, because it holds Anki's engine, the server never links it, and SPEC-334 R3
  keeps the XP crate beside the engine, never in it.
- The umbrella FFI crate or the web engine: lost, because both are adapters, the server cannot
  link an adapter, and the engine core's graph test refuses a member naming an adapter.

### D2, the inputs it owns

- Chosen: the crate owns `ReviewFacts { ease, interval, kind }`, each an `i64` as the collection
  stores it, and its own `Tier { T1, T2, T3, T4 }`; progression's `review_xp` builds them at its
  edge and maps ingest's tier by an exhaustive `match`, because the crate then names no other
  crate's type and the golden's odd inputs (a negative interval, a type of 4 to 6, an ease of 0)
  pass through unchanged.
- Ingest's `Review` and `Tier`: lost, because ingest depends on the kernel, `sqlx` and the engine.
- A shared types crate below both ingest and the XP crate: lost, because it adds a crate and an
  ingest edge for three fields and four tiers, and the anti-corruption layer's own `Review` would
  start to follow the XP rule's needs.
- Positional arguments, `review_xp(ease, interval, kind, tier)`: lost, because three `i64`
  arguments can be swapped and still compile.
- Narrower integers (`u8` types, `u32` intervals): lost, because the conversion becomes fallible,
  and the golden's negative intervals and out-of-range types would need a refusal the
  predecessor's function never makes.
- A cast through the tier's index (`tier as usize`) across the two enums: lost, because reordering
  either enum would remap a tier silently, where an exhaustive `match` fails to compile or fails a
  test.

### D3, the callers and the old body

- Chosen: progression's `review_xp(review: &Review, tier: Option<Tier>) -> u32` keeps its path and
  signature and becomes the translation over the crate; the rule's body leaves progression in the
  same build; coordination's two callers and every test caller are unchanged, because progression
  owns per-review XP and coordination gains no edge.
- Coordination calls the crate directly: lost, because coordination would gain an edge and repeat
  the translation in two places, and progression would no longer own the XP it grants.
- Both bodies kept, with a test comparing them: lost, because two copies of one rule are the drift
  the crate exists to end.
- A deprecated forwarding function, removed by a later delivery: lost, because it is a second
  delivery for no behaviour.

### D4, the constants

- Chosen: the crate embeds `economy.json` at build time and parses its per-review keys once into
  `ReviewXpTable` behind one `static`, `table()`; progression drops the nine per-review fields
  from `XpEconomy` and its constants test reads them from the crate, because each constant then
  has one reader and SPEC-072 R2 holds for the crate as it did for progression.
- Typed constants in the crate: lost, because SPEC-072 R2 and the game-economy pack refuse a
  typed constant.
- A table each caller passes in: lost, because each client and the server would embed and parse
  `economy.json` themselves, three parsers of one section.
- A build script that writes the constants: lost, because it reads a file at build time and adds
  the build script the census refuses.
- Both crates parsing the same keys: lost, because each constant would be declared twice.

### D5, the study-event guard

- Chosen: the crate holds its own `is_study_event(kind, ease)`, and ingest keeps its copy as the
  reader's filter; both are held to the same 64 cases of `study_event.json`, because the
  predecessor's `review_xp` starts with the guard and the golden's non-study class proves it.
- Ingest depends on the crate for the guard: lost, because the anti-corruption layer's definition
  of a review would then live in a crate about XP, behind a new ingest edge for one function.
- The crate drops the guard and callers filter: lost, because the crate would then differ from the
  predecessor's function on five golden cases, and a client could price an answer that is not a
  study event.

### D6, the census

- Chosen: one Python census, `scripts/tests/test_xp_crate_graph.py`, in the shape of
  `test_fsrs7_pin.py`: `tomllib` over the crate's manifest, the lockfile's closure from the crate
  against an allow-list, the fence lines that name the crate against the manifests that do, and a
  scan of the crate's source; each refusal paired with a planted control and an examined count,
  because it runs in CI's python stage, needs no new dependency, and reads the lockfile.
- A Rust census in the crate's own tests: lost, because it needs a TOML parser as a
  dev-dependency, which the golden reader's "and nothing else" refuses, or a hand parser.
- The ddd probe alone: lost, because CI does not run it, and it reads neither the lockfile's
  closure nor the source.
- A dependency-ban tool's configuration: lost, because it adds a tool and a CI step for one crate,
  and its rules would sit outside the manifests and map a reviewer reads.

### D7, the targets

- Chosen: no `wasm32` or Apple compile of the crate at this delivery; the web engine's job and the
  Apple job compile it when #639 draws their edges, because its closure is `std` and serde_json,
  which both client graphs already compile.
- A `wasm32` `cargo check -p deck-streak-xp` step now: lost, because it edits the workflow and its
  census to prove a graph nothing links yet.
- The umbrella FFI and web engine edges now: lost, because an edge nothing uses is refused, and
  ADR-357 D1 draws each in the change that first exposes the crate.

### D8, the parity proof

- Chosen: two golden tests in the crate (`review_xp.json`, 265 cases; `study_event.json`, 64),
  and progression's existing golden test, unchanged, through the translation; no golden or
  registry module changes, because the crate is then proved where cargo-mutants mutates it, and
  the server's path is proved by the test that held it before the move.
- A differential test between the old and the new body: lost, because it needs the old body kept.
- New goldens: lost, because the predecessor is private, public CI never regenerates a golden
  (ADR-012), and the 240 combinations already cover every ease, maturity class, type and tier.
- The crate proved only through progression's test: lost, because a row's killer must be a test of
  its own crate, and cargo-mutants runs the mutated crate's own tests.

## Decision Outcome

Chosen options: D1 to D8 as each "Chosen:" bullet states, because together they leave a crate
that a client can link with no kernel, no I/O and no server type, while the server's XP is held to
the same 265 cases by the test that held it before.

### Consequences

- Good, because a client links the rule with `std` and serde_json alone, and the census refuses
  anything more in CI.
- Good, because coordination, the API and every test caller are unchanged.
- Good, because each constant still has one reader, and the 24-name constants test still names
  all 24.
- Bad, because the study-event rule now has two copies, held equal only by one golden.
- Bad, because the translation is one more hop, and two hand rows are needed to guard it, since
  cargo-mutants generates nothing for an exhaustive `match` that returns an enum.
- Bad, because `economy.json` is embedded whole in a crate a client will link; its weight is
  measured under the web engine's budget when #639 links it.
- Bad, because the census's allow-list must be reviewed whenever serde_json's own dependencies
  change.

### Confirmation

SPEC-360 A1 and A2 (the crate's goldens), A3 to A7 (the census, with planted controls), A8 to
A10 (the server's path, unchanged), A11 (the statics census); rows `S07201` and `S07202`
re-anchored, and the new rows of SPEC-360 section 7.

## More Information

#635; SPEC-334 R3, R13 and section 9; SPEC-072 R1, R2 and R4; SPEC-346 section 1.5; ADR-012,
ADR-029, ADR-040, ADR-357 D1. Instant XP on both clients, and the Lean entry for reconciliation,
are #639's.
