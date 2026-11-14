### Changed

- SPEC-430 and ADR-444 record that the CI cache's restore and save protocol takes no formal model: each of its
  seven properties is held step by step, by a workflow test or by the cache service's own rules, in every order
  of runs (#871).
