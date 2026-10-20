# Red-first record: SPEC-387

The reds and the stubs ADR-401 D4 names are committed at `439da380`, before the code they judge.
Each red line below was read at that tree, from the test binary built there, and fails on its
criterion's assertion: the stubs return no preset, record the stored vector as the proposed one,
record a row on every call, settle nothing, write no text and parse no `preset` command; ingest's
data-rights port declares no `preset_proposals` table; and the privacy page and its record still
hold the old paragraph and no category. A6's first assertion, that the proposal is still open,
passed at the red; its red is the settle. The Rust lines are cut at the population dump.

`d1a0d17a` moves `pub mod preset;` into a group of its own in `crates/ingest/src/lib.rs`, so the
crate root's source-text guard in `crates/ingest/tests/memory_state_boundaries.rs` reads its three
neighbouring lines as before. It changes no test.

`32e84a6f` is the green, R1 to R12. No criterion's test changed between the red and the green.
Three test files changed beside the code, each for a reason a ruling names, and no assertion was
removed: `crates/daemon/tests/roles.rs` names the sixth role in both lists it pins, and
`crates/coordination/tests/data_rights_symmetry.rs` seeds the new table (SPEC-387 section 11);
`crates/daemon/tests/logging.rs` runs the new role as `preset list`, as it runs `data export`.
`crates/daemon/src/role_preset.rs` gained one unit test: the role refuses on its settings before
it reads anything.

A7, A8 and A14 are guards. Each was green at the red, and each carries a plant that proves it can
fail.

```red-first
A1: red at 439da380: assertion `left == right` failed  left: []  right: [Preset { id: 1, name: "Default", vector: [], field: Empty, desired_retention: 0.9, deck_ids: [1], non_new_cards: 0 }, Preset { id: 1001, name: "Main", ...
A1: green at 32e84a6f
A2: red at 439da380: assertion `left == right` failed  left: []  right: [(1, Empty), (1001, Fsrs6), (1002, Fsrs5), (1003, Fsrs4), (1004, Fsrs6)]
A2: green at 32e84a6f
A3: red at 439da380: assertion `left == right` failed: the proposed vector is the engine's defaults  left: [0.40255, 1.18385, 3.17305, 15.69105, ...]  right: [0.212, 1.2931, 2.3065, 8.2956, 6.4133, ...]
A3: green at 32e84a6f
A4: red at 439da380: assertion `left == right` failed: nothing is recorded  left: (1, 1)  right: (0, 0)
A4: green at 32e84a6f
A5: red at 439da380: assertion `left == right` failed  left: Recorded(Proposal { id: 2, preset_id: 1001, preset_name: "Main", ...  right: AlreadyOpen(Proposal { id: 1, preset_id: 1001, preset_name: "Main", ...
A5: green at 32e84a6f
A6: red at 439da380: assertion `left == right` failed  left: Unchanged(Proposal { id: 1, preset_id: 1001, ...  right: Settled(Proposal { id: 1, preset_id: 1001, ...
A6: green at 32e84a6f
A7: not red: the stubs write nothing to the collection; its plant, a write through the write port followed by the owner's sync, changes the copy's sha256 and records 2 local changes, and is seen before any preset path is judged
A8: not red: the stubs name no engine write method; its plant, a line naming CollectionWrite, is refused by that name first (planted.rs:1: CollectionWrite)
A9: red at 439da380: the text names the preset: ""
A9: green at 32e84a6f
A10: red at 439da380: assertion `left == right` failed  left: []  right: ["1001 Main: field fsrs6, not on the defaults, desired retention 0.85, 2 deck(s), 5 non-new card(s)", "1002 Five: field fsrs5, ...
A10: green at 32e84a6f
A11: red at 439da380: preset list parsed to None
A11: green at 32e84a6f
A12: red at 439da380: assertion `left == right` failed: preset_proposals is the owner's data: exported and erased (SPEC-387 R9)  left: None  right: Some(ExportAndErase)
A12: green at 32e84a6f
A13: red at 439da380: AssertionError: "deckstreak does not train or fine-tune a neural network or a language model on your data, and does not build a dataset from it. ..." not found in the models section
A13: green at 32e84a6f
A14: not red: the shipped test passes at the base and does not change; its plant, a lock that lists deck-streak-ingest as a third user of the registry's scheduler, is refused first: "the registry's scheduler is used by ['anki', 'deck-streak-fsrs7', 'deck-streak-ingest'], not ['anki', 'deck-streak-fsrs7']"
A15: red at 439da380: AssertionError: 'preset-proposals' not found in {'sync-history': {'id': 'sync-history', ...
A15: green at 32e84a6f
```
