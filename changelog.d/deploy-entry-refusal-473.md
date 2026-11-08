### Changed

- Each Caddy step's check of the deploy settings it receives is now one function, and a Lean proof
  shows that a step proceeds exactly when it received a non-empty environment in which every
  prefixed entry names a listed setting, once and with a value, and no unlisted prefixed variable
  exists. A test runs the step on the proof's vectors whose block holds no empty entry, 60 of 63,
  and compares each exit status and message with the proof's answer; it names the other 3, which it
  does not run (SPEC-473, #484).
