### Changed

- The internal TestFlight lane also starts on a push to `dev` that changes one of the app's inputs,
  beside its dispatch. A dispatch and a push wait in groups of their own, no build in progress is
  cancelled, and a newer push replaces the push build still waiting (SPEC-352 R22, ADR-363 D2).
