# Clean room: the card renderer

The record of how `crates/render` was written beside the engine it replaces (SPEC-603 A10,
ADR-624). It states what was read, by category, how similarity to the engine's rendering sources
is measured, and the scores.

## What was read

- DeckStreak's own tree: the committed render goldens and their reader
  (`tools/parity-oracle/goldens/anki.render.card.json`,
  `tools/parity-oracle/goldens/anki.render.note_type_check.json`,
  `tools/parity-oracle/golden.rs`), read through summaries and by the tests; the media crate as
  the model of a crate's manifest and its tests; SPEC-513 and ADR-531 as the model of the
  documents.
- The engine's source at the fork pin the workspace names, read only: its template tokenizer and
  parser, its template filters, its text helpers (tag stripping and entity decoding), its note
  type check, and its card render entry point. Each file was read in line ranges, not whole.
- The entity decoder the engine uses for its text filter, at the version the lock pins: its
  decoding routine, and its table of named references, compared by count with the table here.
- The delivery's design notes, which are not part of this repository.

The behaviour was taken from what those sources do and from the goldens. No engine code was
copied: every module here is written in its own structure and its own names, and every message a
user can see is DeckStreak's own text (`RenderIssue::message`). No engine message text is in any
file of this delivery.

The blank-front cases of `anki.render.card` compare by class, never by text: DeckStreak's render
reports its own `RenderIssue::EmptyFront` for each of them, and the golden test reads the engine's
notice from the goldens at run time only to check that every such case carries the same one. The
notice is in no committed file, and the test's failure message names case indexes and counts only.

## Method

A standard-library scan compares each file under `crates/render/src` with each of the engine's
rendering files at the fork pin: `rslib/src/template.rs`, `rslib/src/template_filters.rs`,
`rslib/src/cloze.rs`, `rslib/src/text.rs`, `rslib/src/latex.rs`, `rslib/src/typeanswer.rs`, and
every Rust file under `rslib/src/card_rendering/` and `rslib/src/image_occlusion/`.

1. Each file becomes a stream of lower-cased identifier and keyword tokens. Comments, string
   literals, character literals and numbers are dropped.
2. The stream is cut into k-grams of 5 tokens, and each k-gram is hashed.
3. Winnowing with a window of 4 hashes keeps each window's least hash (the rightmost on a tie) as
   a fingerprint.
4. A file pair's score is the Jaccard index of their two fingerprint sets: the shared
   fingerprints over all the fingerprints of either file.

Scores are recorded as numbers with this method only; they carry no threshold.

## Scores

The scan read 4 files of `crates/render/src` against 18 of the engine's rendering files, 72 pairs;
no listed engine file was absent. Each file's highest score:

| DeckStreak file | fingerprints | highest score | against |
|---|---|---|---|
| `crates/render/src/filters.rs` | 224 | 0.0025 | `rslib/src/template_filters.rs` |
| `crates/render/src/lib.rs` | 87 | 0.0020 | `rslib/src/template.rs` |
| `crates/render/src/render.rs` | 290 | 0.0020 | `rslib/src/card_rendering/writer.rs` |
| `crates/render/src/template.rs` | 170 | 0.0038 | `rslib/src/template.rs` |

The pairs that share any fingerprint:

| DeckStreak file | engine file | score | shared fingerprints |
|---|---|---|---|
| `crates/render/src/filters.rs` | `rslib/src/template_filters.rs` | 0.0025 | 1 |
| `crates/render/src/lib.rs` | `rslib/src/template.rs` | 0.0020 | 2 |
| `crates/render/src/render.rs` | `rslib/src/template.rs` | 0.0017 | 2 |
| `crates/render/src/render.rs` | `rslib/src/card_rendering/writer.rs` | 0.0020 | 1 |
| `crates/render/src/template.rs` | `rslib/src/template.rs` | 0.0038 | 4 |

Every other pair, 67 of the 72, scores 0.0000 with no shared fingerprint.
