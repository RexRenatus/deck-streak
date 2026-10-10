### Added

- A link strip in the card view on iPhone and iPad (SPEC-392, ADR-406). Every `<link` opener in a
  card, in any ASCII letter case and wherever it stands, is renamed to a `<wbr` opener before the
  card is loaded, so no link element from a card's markup reaches the view, and every other byte
  of the card is kept. The five characters shown as text are renamed too; a card that shows them
  writes `&lt;link`, which is left as it is.
- The planted card suite on iPhone and iPad counts the link elements in each card view, and reads
  the link strip alone holding every linked card it can observe. Its reading of no connection from
  a followed link, on both simulators, is now the recorded closure of that channel.
