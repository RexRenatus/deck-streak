### Fixed

- The Rust coverage lane no longer reads red on a run whose tests all pass: the sync server helpers
  of the ingest, engine-core and ffi tests stop a server by closing its standard input and waiting a
  bounded grace for it to exit on its own, instead of killing it at once, so a server's raw profile
  is never cut short in its exit. The lane still refuses a profile it cannot read.
