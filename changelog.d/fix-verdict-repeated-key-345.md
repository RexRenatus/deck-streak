### Fixed

- The mutation verdict's `plan` refuses a band file that repeats a key with the one-line refusal
  and exit 1 instead of a traceback, and the equivalence records under
  `scripts/mutation-equivalent.d/` are read by the same parser, so a key repeated at any depth is
  refused by name instead of keeping the last value (SPEC-122, #345).
