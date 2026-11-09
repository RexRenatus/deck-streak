### Fixed

- `ruff.toml` targets Python 3.12, the Python the tests need and CI runs, so ruff reads the tree
  as the code that runs it; a test holds the target, and the new target raises no finding and
  changes no formatting (SPEC-626, ADR-648).
