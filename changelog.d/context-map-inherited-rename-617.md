### Fixed

- The context-map census reads a package rename that a crate inherits from the workspace
  dependencies, so it compares the crate the rename names (SPEC-617).
