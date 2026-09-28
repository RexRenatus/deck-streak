### Added

- A test holds every workflow to ADR-017's rule: none reads a secret other than `GITHUB_TOKEN`,
  passes its secrets on with `secrets: inherit`, or checks out, clones or fetches another
  repository. It judges any directory of workflows, names each refusal by its file and place,
  refuses a directory with no workflow as VOID, and proves itself on planted workflows under
  `scripts/tests/fixtures/secrets-and-checkouts/` (SPEC-034 R7).
- The test's own workflow reader fails closed: a character outside printable ASCII, and a value or
  key in a form it does not read as YAML does, a quoted escape, an anchor, alias or tag among them,
  are refused by its line. A checkout of another repository named in any case, its action and its
  inputs read as the runner reads them, a checkout from another server, and a fetch in git's
  scp-like form are refused too. Every workflow test reads `.yml` and `.yaml` files, and an action
  is pinned only in its plain form (SPEC-034 A13).
