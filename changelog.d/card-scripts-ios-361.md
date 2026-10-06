### Added

- A containment layer for card scripts on iPhone and iPad (SPEC-361, ADR-372). A link's activation
  inside a card is refused before the engine follows it, a card cannot activate an element it has
  detached or rewrite its own document, the card's document carries a policy that admits only data:
  media and inline style and script, and a long press offers no link preview and no menu item that
  opens a link. The switch defaults off on iOS pending a measured containment layer.

### Changed

- A tap on a link inside a card on iPhone or iPad no longer opens it, with card scripts on or off,
  and a tap on an inline element inside a summary or a label no longer toggles it.
