### Added

- The formal ledger, `formal/obligations.json`, names every engine property the formal tools are to
  carry: its artifacts in TLA+, Lean or Kani, the tie that binds each to the shipped code, and the
  gaps no tool closes, with the 43 existing entries' dispositions as rows. A checker in the python
  stage refuses an obligation with no artifact or no gap line, an artifact its entry's header does
  not declare, a tie the tree does not define or that names no artifact, an obligation read as tied
  with an artifact no counting tie binds, and a private core name, each by its own planted control
  (SPEC-496, ADR-514).
