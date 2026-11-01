### Fixed

- The mutation verdict's Rust tokenizer refuses a reading that does not advance, naming its
  position, so each mutant that once kept a release run's Python leg reading the same token until
  the leg was stopped with no report now fails a test and reads killed (#822). The ingest crate's
  lenient memory-state reader runs at most one pass per byte of its text, so a mutant that stops
  its reading from advancing ends with a wrong text that an existing test catches. Both changes
  are the identity on every admitted input: one census reads every Rust source of the tree with
  and without the guard, and one reads 4096 seeded texts with and without the bound. A Lean proof
  shows the guarded loop ends within its text's length.
