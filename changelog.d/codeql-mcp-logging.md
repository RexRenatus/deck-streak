### Fixed

- The MCP guard census test names its helper `refused_clauses` for the code clauses it
  refuses, so code scanning no longer reads the assertion message that prints them as sensitive
  data written to a log.
