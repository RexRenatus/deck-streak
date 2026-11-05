# Card renderer

Kind: data flow and component. Read at the `dev` commit this delivery is cut from; it cites no line.

The render crate (SPEC-603, ADR-624) turns a note type, a note's content and a card's context into the card's two
faces, or into one `RenderIssue` class with DeckStreak's own text.

## Data flow

```mermaid
flowchart LR
  tpl["CardTemplate front and back"] --> tok["tokenize: Text, Comment, Var, Open, Close"]
  tok --> parse["parse: a tree of Node, explicit stack, at most 32 deep"]
  parse --> render_q["render the question"]
  note["NoteContent fields and tags"] --> fmap["field map: fields, then Tags, Type, Deck, Subdeck, CardFlag, Card, CardID, c n"]
  card["CardContext ordinal, deck, flag, id"] --> fmap
  fmap --> render_q
  render_q --> empty{"front shows a filled field?"}
  empty -- no --> issue["RenderIssue::EmptyFront"]
  empty -- yes --> render_a["render the answer, FrontSide is the question"]
  render_a --> out["Rendered question and answer"]
  parse -- refused --> issue2["RenderIssue: UnclosedSection, StrayClose, TooDeep, TooLarge"]
  render_q -- unknown name --> issue3["RenderIssue::UnknownField"]
```

Filters run right to left over a field's text: `text`, `hint` (a `details` element), the `type` marker,
`furigana`, `kana`, `kanji` and `tts`; an unknown filter passes the text through.

## Types

| type | holds |
|---|---|
| `NoteType` | `name`, `kind` (`Standard` or `Cloze`), `fields` (names in order), `templates` |
| `CardTemplate` | `name`, `front`, `back`, as written |
| `NoteContent` | the note's field values and tags |
| `CardContext` | the card's ordinal, deck path, flag and id |
| `Rendered` | `question` and `answer` HTML |
| `RenderIssue` | a class; `message` and `face` give DeckStreak's text |

## Component view

```mermaid
flowchart TB
  render["render (deck-streak-render): no dependency"]
  engine_core["engine-core"]
  ffi["ffi"]
  web_engine["web-engine"]
  engine_core --- ffi
  engine_core --- web_engine
```

`render` stands beside `engine-core`, `ffi` and `web-engine` with no edge yet (ADR-624 D2): the first delivery
that calls it draws its edge.
