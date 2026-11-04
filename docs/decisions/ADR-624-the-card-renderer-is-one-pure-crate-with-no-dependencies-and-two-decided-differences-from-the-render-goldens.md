# ADR-624: The card renderer is one pure crate with no dependencies, and two decided differences from the render goldens

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-603 |
| builds on | ADR-029 (not amended) |
| measured at | the delivery's base on `dev` |

## Context

A card's faces are drawn today by the engine's template code. DeckStreak needs its own renderer that both clients
and the server can link, held to the committed render goldens (`tools/parity-oracle/goldens/anki.render.card.json`,
`anki.render.note_type_check.json`, read through `tools/parity-oracle/golden.rs`). The template language is the
public one: field replacements, sections and inverted sections, special fields, `{{FrontSide}}` and filters chained
by colons. Where the language's manual is silent, the goldens decide.

## D1. One crate, with no dependency and no I/O

`deck-streak-render` at `crates/render` is a library with `#![forbid(unsafe_code)]`, the workspace's lints and no
`[dependencies]` entry: it reads no clock, file, network or environment, so the server and both clients link one
rule. Its dev-dependencies are the golden reader's `serde` and `serde_json` only, and it has no features. A
template is tokenized (`tokenize`), parsed into a tree with an explicit stack (`parse`), and rendered against a
field map that holds the note's fields and the special fields `Tags`, `Type`, `Deck`, `Subdeck`, `CardFlag`, `Card`,
`CardID` and the card's own `c<n>`. `check_note_type` refuses a note type whose template does not parse, whose front
names no field, which names a field the note type lacks, which repeats an earlier front, or, for a cloze note type,
whose front or back has no cloze filter. The filters of this delivery are `text`, `hint`, the `type` marker,
`furigana`, `kana`, `kanji` and `tts`; an unknown filter, a cloze filter and the empty filter pass the text through.
The cloze filters, sound and speech tags, media references, math and the field allowlist are later deliveries, and
the legacy alternative tag syntax is not read.

## D2. The context map: a crate that depends on nothing, and nothing on it yet

The fence line is `deck-streak-render  depends on: nothing`, beside the media core. No edge from `engine-core`,
`ffi` or `web-engine` is drawn: the first delivery that calls the renderer adds its edge, with its use. The
lexicon names the render context's terms: `note_type` (replacing model) and `card_template` (replacing cardtype).

## D3. Two decided differences from `anki.render.card`

The golden test compares every case whose note type is standard, whose class is not a cloze class and whose
templates use no cloze filter. Two differences are decided, each counted and printed by the test, which refuses a
zero count of either:

- **`hint` is a `details` element.** The engine's hint is a link that reveals its text by script; DeckStreak's card
  frame runs none, so `{{hint:F}}` renders `<details class="hint"><summary>F</summary>` the text `</details>`, which
  opens with no script. Rule: a case whose templates use the `hint` filter is set aside from equality. Count: 241
  cases of `anki.render.card`.
- **A blank front is DeckStreak's `EmptyFront`, compared by class.** Where a card's front shows none of its filled
  fields, the goldens record the engine's notice as both faces. DeckStreak reports its own `EmptyFront` issue, in its
  own words. Rule: a case DeckStreak reports as `EmptyFront` passes when the golden's answers over all such cases are
  one string and each golden question ends with it; the test reads that string from the goldens at run time and never
  prints it, and no committed file holds it. A blank-front case DeckStreak does not report fails equality. Count:
  45 cases of `anki.render.card`.

Every other mismatch is a red. Each `RenderIssue` class has a DeckStreak string, and its face is that string in a
`div`; no engine message text is copied.

## D4. A fixed-seed generator, not proptest

The totality test draws 10,000 templates and field sets from a fixed-seed generator written in the test file. No
property-testing crate is admitted by this delivery, so the crate's dependency set stays empty in every table.

## D5. Kani harnesses behind `cfg(kani)`, VOID until the verifier is installed

`tokenize_never_panics` and `parse_never_panics` live in `crates/render/tests/kani_harnesses.rs` behind `cfg(kani)`,
each over an 8-byte input, calling the shipped functions of `crates/render/src/template.rs`. The verifier is not
installed, so they read VOID and stay unratcheted, and `formal/obligations.json` keeps its render entries `planned`.
Their witness features come with the delivery that installs the verifier, because each feature is counted against
the workspace's witness list.

## What it was chosen AGAINST

- An engine dependency for rendering: it keeps the engine in every client, and the clients must render alone.
- An HTML tokenizer crate now: no filter of this delivery needs one, and the dependency set stays empty until the
  field allowlist arrives.
- Copying the engine's blank-front notice into a test: it is engine message text, so the test reads it from the
  goldens at run time instead.
- A script-driven hint: the card frame runs no script, so a `details` element is the only hint that opens.
- proptest for the totality test: a new dev-dependency for one test, where a fixed-seed generator holds the same
  bound with no package.

## Consequences

- The renderer can be called by any crate that adds the edge; none does yet.
- Two goldens now have a reader, and every non-cloze case is held equal or set aside by a counted rule.
- Template problems are classes with DeckStreak's own text, ready for the clients' own translations.

## What would make this wrong

- A golden case that differs for a reason neither rule covers: it reads red, and the rule or the code changes.
- A client that needs a filter or field this delivery does not render before the later deliveries land.
