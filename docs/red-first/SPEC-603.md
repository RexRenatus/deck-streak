# Red-first record: SPEC-603

The SPEC, ADR-624 and the schematic were installed first. The tests came next, whole, over an inert renderer that
keeps every public type and function the tests import but none of the behaviour: `render` gives two empty faces,
`check_note_type` accepts every note type, `RenderIssue::message` and `RenderIssue::face` give empty text, and
`tokenize` and `parse` give nothing. Each red below is an assertion on that inert seam, quoted from a run of the
render crate's test targets and of `scripts/tests/test_context_map_schematic.py` on the test commit's tree. The
fence of sha lines is appended after the green commit, because a commit cannot name its own sha.

## The criteria

- A1: `crate_has_no_dependencies` (census): the manifest has no dependency entry, and a planted one is refused.
- A2: `goldens_render_card_equal` (golden): the non-cloze cases of `anki.render.card`, with the two decided rules.
- A3: `goldens_note_type_check_equal` (golden): every case of `anki.render.note_type_check`.
- A4: `hint_is_details` (renderer): a hint is a `details` element with no script.
- A5: `error_face_is_ds_text` (renderer): each issue class has DeckStreak's own text and face.
- A6: `render_total_proptest` (renderer): 10,000 fixed-seed cases, none panics.
- A7: `tokenize_never_panics` and `parse_never_panics` (Kani harnesses, behind `cfg(kani)`).
- A8: the `web-engine` job's render step builds for wasm32 and iOS.
- A9: `test_every_crate_directory_is_one_node_named_by_it` and `test_every_arrow_is_a_manifest_edge_the_fence_names`
  (`scripts/tests/test_context_map_schematic.py`), with `test_context_map_register.py` green beside them.
- A10: the similarity record, `docs/clean-room/card-renderer.md`.

## The reds at the test commit

A2, `goldens_render_card_equal`:

```text
thread 'goldens_render_card_equal' (1785078) panicked at crates/render/tests/golden.rs:201:5:
447 of 447 compared case(s) differ; the first, by index: [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
```

A3, `goldens_note_type_check_equal`:

```text
thread 'goldens_note_type_check_equal' (1785077) panicked at crates/render/tests/golden.rs:236:5:
30 of 880 case(s) differ; the first, by index: [0, 1, 2, 3, 4, 5, 6, 7, 14, 16, 17, 18, 19, 20, 21, 22, 23, 34, 40, 41]
```

A4, `hint_is_details`:

```text
thread 'hint_is_details' (1795925) panicked at crates/render/tests/renderer.rs:55:5:
assertion `left == right` failed
  left: ""
```

A5, `error_face_is_ds_text`:

```text
thread 'error_face_is_ds_text' (1795924) panicked at crates/render/tests/renderer.rs:155:9:
assertion `left == right` failed: the class of UnknownField { name: "Nowhere" }
  left: Ok(())
```

A9, `test_context_map_schematic`:

```text
FAIL: test_every_crate_directory_is_one_node_named_by_it: AssertionError: findings: 1 / render: no node
FAIL: test_every_arrow_is_a_manifest_edge_the_fence_names: AssertionError: findings: 1 / render: no fence row
```

Not red at the test commit, each by design: A1 (the crate is born without dependencies; its planted manifest is
refused inside the test), A6 (the inert seam never panics, so the totality test holds; it guards the shipped
render), A7 (VOID: the verifier is not installed), A8 (a build step, read green by name in the green run) and A10
(a record).

## The greens at the implementation commit

The implementation commit adds the renderer and the context-map, lexicon and schematic lines, and changes no test.
Each green below is quoted from a run of the render crate's test targets on one thread, and of
`scripts/tests/test_context_map_schematic.py`, on that commit's tree.

A2, `goldens_render_card_equal`:

```text
test goldens_render_card_equal ... examined 447 case(s) of anki.render.card: equal by text 378, refused as the golden refuses 24, blank-front by EmptyFront 45, differ 0; set aside: hint 241, cloze note type 326, cloze class 0, cloze filter 0
ok
```

A3, `goldens_note_type_check_equal`:

```text
test goldens_note_type_check_equal ... examined 880 case(s) of anki.render.note_type_check: refused by the golden 30, differ 0
ok
```

A4, `hint_is_details`:

```text
test hint_is_details ... ok
```

A5, `error_face_is_ds_text`:

```text
test error_face_is_ds_text ... examined 10 issue class(es)
ok
```

A9, `test_context_map_schematic` (4 tests, among them the two named above), with `test_context_map_register` OK
beside it:

```text
OK
examined 43 fence rows
examined 91 arrows
examined 39 crate directories
```

Green at both commits, as designed: A1 (`test crate_has_no_dependencies ... examined 20 manifest line(s): dependency
entries 0`, then `ok`) and A6 (`test render_total_proptest ... examined 10000`, then `ok`).
