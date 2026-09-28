### Added

- A test holds every workflow to ADR-017's rule: none reads a secret other than `GITHUB_TOKEN`,
  passes its secrets on with `secrets: inherit`, or checks out, clones or fetches another
  repository. It judges any directory of workflows, names each refusal by its file and place,
  refuses a directory with no workflow as VOID, and proves itself on planted workflows under
  `scripts/tests/fixtures/secrets-and-checkouts/` (SPEC-034 R7).
