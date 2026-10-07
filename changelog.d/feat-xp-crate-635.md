### Added

- The per-review XP rule as its own crate, `deck-streak-xp` (SPEC-360, #635): the rule, its
  study-event guard and its table from `economy.json` move out of progression unchanged, with no
  file, network or clock access, so both clients can link the same rule later. Progression
  translates an ingest review into it at its edge, the server's XP is held to the predecessor's
  recorded cases as before, and a census refuses any other dependency, an I/O call or a dependent
  the context map does not draw.
