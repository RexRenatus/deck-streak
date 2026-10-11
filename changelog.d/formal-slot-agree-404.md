### Fixed

- The formal check's slot setting is stated once, in `config/formal.json`, and a test now holds
  that file to the reading the formal checker takes: a document that leaves out a slot key, or
  holds a value past the checker's range, fails the settings test instead of being read as a
  default (SPEC-404, ADR-418, #703).
