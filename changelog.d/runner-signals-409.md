### Fixed

- The mutation-row runner now ends a killer's process group when it receives SIGTERM, to its pid or
  its whole group, as it already did on an interrupt (SPEC-025 sections 10 and 11, issue 409). A
  timed-out killer returns at its bound even when a descendant outside the group holds its output
  pipes, and every path closes both pipes. Rows `S02506` to `S02508` pin the three edges.
