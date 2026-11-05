### Changed

- The `dev` ruleset file now records the live ruleset: the merge queue app as its one bypass actor,
  no up-to-date head, and the two pull request fields the live ruleset carries, so applying the
  rulesets again keeps the merge queue able to merge (SPEC-620).
- `.mergify.yml` gains one comment line saying it is read from `dev`, the default branch.
