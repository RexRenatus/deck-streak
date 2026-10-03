### Added

- The obligation horizon (SPEC-091, #91): a fixed 365-day forward histogram of the reviews every
  card owes, with the owed-now, beyond-horizon, new and excluded counts, and a readout that names
  the true 30-day obligation and the two levers that create more. Each number is proved against the
  predecessor's own function, and the horizon's constants against a golden of their own (ADR-317).
