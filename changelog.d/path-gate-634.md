### Added

- The path gate's decisions, which gated Rust job a pull request into dev may skip and which
  results the ci aggregate admits, are pure functions in `scripts/path_gate.py`, with a reader for
  the map of the paths each job reads. The Lean entry `PathGate` proves that the gate skips exactly
  the changes no read reaches, and that a skipped job passes only on the gate's own skip; its
  vectors are read against the script (SPEC-634). Nothing calls the module yet.
