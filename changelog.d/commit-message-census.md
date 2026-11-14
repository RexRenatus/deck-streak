### Added

- A census in CI reads every commit of a pull request and of a push: a subject that is not a
  Conventional Commit of the house's types, a subject over 72 characters, and a `Co-authored-by`
  or `Signed-off-by` trailer each print a warning naming the commit, while the merges and reverts
  git or GitHub wrote are counted and not judged. It starts at warn, prints how many commits it
  examined, and fails on an empty population (SPEC-675).
