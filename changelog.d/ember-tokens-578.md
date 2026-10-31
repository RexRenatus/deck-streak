### Added

- DeckStreak's product palette, Ember (warm charcoal with an amber accent), has one home in the
  design token file: a `modes` tier of six role values, a background, an ink and an accent for each
  of a dark and a light mode. The token compile emits each as its own custom property,
  `--ember-<mode>-<role>`, and the iPhone and iPad app's asset catalog gains three colour sets,
  `EmberBackground`, `EmberInk` and `EmberAccent`, that copy them. A census holds every copy equal
  to the token file and refuses a raw colour literal outside the token route. No screen reads the
  palette yet, so no page or screen changes colour (#1043; SPEC-578).
