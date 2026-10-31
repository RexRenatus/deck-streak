### Fixed

- A wrongly shaped retired list is refused by name instead of crashing the retirement check, and
  the row runner ends its killer's process group on a hang-up, on a signal while the killer starts
  and on a second signal during its cleanup, and leaves the signal handlers as it found them
  (SPEC-465, #421, #432).
