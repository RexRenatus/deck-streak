### Added

- A card renderer of the app's own: it renders a card's two faces from its note type, its templates and its
  fields, with sections, inverted sections, the special fields, the front side on the answer and the text filters,
  held equal to the committed render goldens on every non-cloze case. A hint opens as a `details` element with no
  script, and each template problem is reported in the app's own words. It has no dependencies and no I/O, and no
  screen uses it yet (SPEC-603, ADR-624).
