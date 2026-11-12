### Fixed

- The review no longer reads the device's installed voices on the main thread before a card
  shows. The list is read once, off the main thread, from the first time a review asks for it, and
  each card step hands the engine the voices read so far, so a line spoken before the list is in
  speaks in its language's own voice. Before it, the read could hold the main thread past the UI
  tests' waits on the iPhone simulator (SPEC-687, ADR-707).
