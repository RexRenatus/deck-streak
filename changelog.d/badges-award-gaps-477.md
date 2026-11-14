### Added

- The award-once model now covers the badge award port, so a change to how a badge is awarded is
  reported by the formal check until the model is read against it again. A new test holds the
  edges of the night and early badge windows under three study-day rules, the default, rollover 4
  at UTC+1 and rollover 5 at UTC-5:30, each with 50 answers placed on either side of midnight, of
  the rollover and of seven o'clock, and six mutation rows keep the windows' bounds and the rule
  each window reads checked (SPEC-477, ADR-494, #544).
