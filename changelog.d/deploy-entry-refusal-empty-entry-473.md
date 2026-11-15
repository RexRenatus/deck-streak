### Changed

- The deploy test that compares each Caddy step with the Lean proof of its settings check now runs
  all 63 of the proof's vectors, the 3 whose environment block holds an empty entry among them,
  and compares each exit status and message with the proof's answer: the route that hands a step
  its environment exactly now carries an empty entry as itself (SPEC-473, #484).
