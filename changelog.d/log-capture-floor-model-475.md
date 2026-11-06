### Added

- A model of the test log capture's floor against the callsite interest cache checks, over every
  interleaving of a thread with no capture and a thread that captures twice, that no cached
  interest is "never" while a capture is live, that no line on a capturing thread is lost, and
  that no interest is asked before the floor is the global default; its three witnesses, the floor
  removed among them, are reported as violations (SPEC-475, issue #499).
