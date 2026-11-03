### Added

- DeckStreak's product palette, Tide (a deep blue-black with a teal accent), has one home in the
  design token file: a `modes` tier of six role values, a background, an ink and an accent for each
  of a dark and a light mode. The token compile emits each as its own custom property,
  `--tide-<mode>-<role>`, and the iPhone and iPad app's asset catalog gains three colour sets,
  `TideBackground`, `TideInk` and `TideAccent`, that copy them. A census holds every copy equal
  to the token file and refuses a raw colour literal outside the token route. No screen reads the
  palette yet, so no page or screen changes colour (#1043; SPEC-578).
