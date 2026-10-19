### Added

- The photo caption's length bound is proved for every caption (#480): a Lean entry ports the
  check that the photo and the prepared share both ask, proves that each takes a caption exactly
  when its length is within the platform's limit of 1,024 UTF-16 units, a character outside the
  basic plane counting two, and writes vectors that a new test of the notifications crate reads
  through the crate's own code (SPEC-472).
