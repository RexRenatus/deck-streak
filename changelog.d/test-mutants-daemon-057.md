### Changed

- Every mutant of the daemon crate is now killed by a test or recorded as equivalent with the
  argument that no test could observe it: the notifier's enabled state and its once-per-episode
  warning, the abstract-socket send, the watchdog warning's millisecond figures and the role
  name that alone runs the `data` role are pinned by tests.
