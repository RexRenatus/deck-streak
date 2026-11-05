# SPEC-603: The card renderer: note types, templates, fields, sections and filters, equal to the render goldens

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-624, under ADR-029 |
| measured at | the delivery's base on `dev` (every file this SPEC cites is byte-equal there) |
| schematic | `docs/schematics/card-renderer.md` (new: data flow and component); `docs/schematics/context-map.md` (changed: the render crate's node) |
| mutation band | S60300-S60399 (no hand row) |

## 1. The problem, measured

- **No renderer of DeckStreak's own.** A card's two faces are drawn by the engine's template code; no crate of the
  workspace reads a note type's templates itself. `ls crates` at the base lists no `render` directory.
- **The goldens have no reader.** `tools/parity-oracle/goldens/anki.render.card.json` holds 1014 cases
  (`jq '.cases | length'`): 688 of a standard note type and 326 of a cloze note type
  (`jq '[.cases[].input.notetype.kind] | group_by(.) | map(length)'`). Of the 688, 241 use the `hint` filter and none
  uses a cloze filter, so 447 compare (counted by the golden test, which prints each count).
  `tools/parity-oracle/goldens/anki.render.note_type_check.json` holds 880 cases, 850 `ok` and 30
  `card_type_error` (`jq '[.cases[].output.result] | group_by(.) | map(length)'`).
- **A blank front.** Where a card's front shows none of its filled fields, the goldens record the engine's own
  notice text as both faces. That text is the engine's, and DeckStreak never copies an engine message.
- **A hint needs script.** The engine's `hint` filter writes a link that reveals its text by script; DeckStreak's
  card frame runs none.

## 2. Requirements

R1. One crate, `deck-streak-render` at `crates/render`, forbids unsafe code and has no dependency at all: no clock,
file, network or other I/O; its dev-dependencies are the golden reader's `serde` and `serde_json` only.

R2. `render` gives the goldens' question and answer for every standard, non-cloze case of
`anki.render.card.json`, or refuses where the golden refuses, but for the two differences ADR-624 D3 decides.

R3. `check_note_type` accepts or refuses every note type of `anki.render.note_type_check.json` as the golden does.

R4. `{{hint:F}}` renders a `details` element whose `summary` names the field, with no script.

R5. Every `RenderIssue` class has DeckStreak's own English text, and its face shows that text; no engine message
text is copied.

R6. `render` never panics: over 10,000 generated templates and field sets within the bounds it returns a card or a
`RenderIssue`.

R7. The tokenizer and the parser carry Kani harnesses that call them over every 8-byte input, behind `cfg(kani)`.

R8. The crate builds for the web client's `wasm32-unknown-unknown` target and the iOS `aarch64-apple-ios` target in
the existing `web-engine` job.

R9. The context map, the lexicon and the context-map schematic name the crate, with no edge to or from it.

R10. A similarity score of the crate's sources against the engine's rendering sources is recorded with its method.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1: the crate's manifest has no `[dependencies]` entry, and a planted manifest with one is refused by the same reader | ADR-624 D1 |
| A2 | R2: every compared case is equal by text, or is refused as the golden refuses, or is one of the blank-front cases DeckStreak reports as `EmptyFront`; the `hint` cases are counted and set aside; each count is printed and none is zero | ADR-624 D3 |
| A3 | R3: the check's answer equals the golden's for every case | ADR-624 D1 |
| A4 | R4: a hint renders `<details class="hint"><summary>` the field name `</summary>` the text `</details>` | ADR-624 D3 |
| A5 | R5: each class's text and face are DeckStreak's own | ADR-624 D3 |
| A6 | R6: 10,000 fixed-seed cases, each call caught and asserted `Ok` | ADR-624 D4 |
| A7 | R7: the harnesses `tokenize_never_panics` and `parse_never_panics` are present; VOID until the verifier is installed | ADR-624 D5 |
| A8 | R8: the `web-engine` job's render step is green by name | ADR-624 D1 |
| A9 | R9: `test_context_map_schematic` holds every crate directory as one node, and `test_context_map_register` stays green | ADR-624 D2 |
| A10 | R10: `docs/clean-room/card-renderer.md` records the method and the scores | ADR-624 D3 |

```acceptance
A1: cargo test -p deck-streak-render --test census -- --exact crate_has_no_dependencies
A2: cargo test -p deck-streak-render --test golden -- --exact goldens_render_card_equal
A3: cargo test -p deck-streak-render --test golden -- --exact goldens_note_type_check_equal
A4: cargo test -p deck-streak-render --test renderer -- --exact hint_is_details
A5: cargo test -p deck-streak-render --test renderer -- --exact error_face_is_ds_text
A6: cargo test -p deck-streak-render --test renderer -- --exact render_total_proptest
A7: cargo kani -p deck-streak-render --tests --harness tokenize_never_panics
A7: cargo kani -p deck-streak-render --tests --harness parse_never_panics
A8: cargo build --locked -p deck-streak-render --lib --target wasm32-unknown-unknown --target aarch64-apple-ios
A9: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
A9: python3 -m unittest discover -s scripts/tests -p test_context_map_register.py
A10: test -s docs/clean-room/card-renderer.md
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/render/Cargo.toml` | render | new: no dependency; the golden reader's two dev-dependencies; no features |
| `crates/render/src/lib.rs` | render | new: the crate's doc, its types, `render`, `check_note_type`, `RenderIssue` and its DeckStreak text |
| `crates/render/src/template.rs` | render | new: `tokenize`, `parse`, `Token`, `Node` and the bounds |
| `crates/render/src/render.rs` | render | new: the field map, sections, the empty-front rule and the note-type check |
| `crates/render/src/filters.rs` | render | new: `text`, `hint`, the `type` marker, `furigana`, `kana`, `kanji` and `tts` |
| `crates/render/tests/golden.rs` | render | new: `goldens_render_card_equal`, `goldens_note_type_check_equal` |
| `crates/render/tests/renderer.rs` | render | new: `hint_is_details`, `error_face_is_ds_text`, `render_total_proptest` |
| `crates/render/tests/census.rs` | render | new: `crate_has_no_dependencies` |
| `crates/render/tests/kani_harnesses.rs` | render | new: `tokenize_never_panics`, `parse_never_panics`, behind `cfg(kani)` |
| `Cargo.lock` | workspace | changed: the one `deck-streak-render` package entry |
| `docs/CONTEXT-MAP.md` | docs | changed: the render fence line and its paragraph |
| `docs/LEXICON.md` | docs | changed: `note_type` and `card_template` in the render context |
| `docs/schematics/context-map.md` | docs | changed: the `render` node and its line |
| `docs/schematics/card-renderer.md` | docs | new: the data flow and the component view |
| `.github/workflows/ci.yml` | CI | changed: one `web-engine` step that builds the render crate for both targets |
| `docs/specs/SPEC-603-the-card-renderer-note-types-templates-fields-sections-and-filters-equal-to-the-render-goldens.md` | docs | new: this SPEC |
| `docs/decisions/ADR-624-the-card-renderer-is-one-pure-crate-with-no-dependencies-and-two-decided-differences-from-the-render-goldens.md` | docs | new: the decision |
| `docs/red-first/SPEC-603.md` | docs | new: the red-first record |
| `docs/clean-room/card-renderer.md` | docs | new: what was read, the similarity method and its scores |
| `changelog.d/card-renderer-603.md` | docs | new: the changelog fragment |
| `scripts/mutation-rows.d/S39400-S39499.json` | rows | changed: row S39406 re-anchored to the header this delivery sets; every other row unchanged |

## 5. What this does NOT cover

- No crate depends on the renderer yet: the first delivery that calls it adds its edge (ADR-624 D2).
- The cloze filters, `card_ordinals`, sound and speech tag extraction, media references, math and the field
  allowlist are later deliveries of the renderer; a cloze filter is passed through unchanged here (ADR-624 D1).
- The legacy alternative tag syntax is not read; a template that uses it renders its tags as text (ADR-624 D1).
- `formal/obligations.json` keeps its two render entries `planned` until the verifier is installed (ADR-624 D5).

## 6. Risks

- **A renderer that differs from the engine's on a template the goldens do not hold.** The goldens hold every edge
  class of the corpus; a new difference enters only as a red golden case and an ADR.
- **An engine message copied into the tree.** Every issue's text is written here; the golden test finds the
  blank-front cases by DeckStreak's own `EmptyFront`, and its failure names case indexes and counts only.
- **A template that costs too much.** A template over 1,048,576 bytes is refused as `TooLarge` and sections nest at most 32
  deep (`TooDeep`); the parser keeps an explicit stack, so its depth costs no call stack.

## 7. Formal model

- **Kani: the tokenizer and the parser, VOID.** `crates/render/tests/kani_harnesses.rs` holds both harnesses behind
  `cfg(kani)`, each over an 8-byte input. The verifier is not installed, so they are unratcheted and the ledger stays
  `planned`; their witnesses come with the delivery that installs it (ADR-624 D5).
- **Totality by test.** `render_total_proptest` holds R6 over 10,000 fixed-seed cases.
- **TLA+ and Lean: NOT APPLICABLE.** Each function here is pure; nothing interleaves.
