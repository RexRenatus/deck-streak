### Fixed

- The log-capture helper names a missing floor apart from a nested capture (SPEC-024, issue
  #522). It counts the captures it holds on each thread: a capture made while that count is above
  0 is refused as nested, with the same message as before, and a capture made while it is 0 where
  the floor is not the thread's default is refused with a message naming the missing floor. The
  refusal's doc now says its reading of the default holds outside a dispatcher's own call, and
  states when the default reads as none inside one (issue #511, wording 3).
