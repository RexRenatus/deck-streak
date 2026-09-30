### Fixed

- The agent's tests that pin its run-record prune and its verdict's must-use attribute now read the
  source as Rust's tokens, so a copy of what they look for inside a comment or any literal is not
  code, a second delete statement is refused, and a source that does not lex is refused; each is
  shown red before and green after.
