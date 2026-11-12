### Changed

- The formal settings file states the most one Kani run may take, `budgets.kani_seconds`, beside
  the TLA+, Lean and per-entry budgets, and the settings test holds the file to it, so a Kani
  entry of this tree is checked under a stated bound (SPEC-605).
