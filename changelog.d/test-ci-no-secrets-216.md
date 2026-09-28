### Added

- A test holds every workflow to ADR-017's rule: none reads a secret other than `GITHUB_TOKEN`,
  passes its secrets on with `secrets: inherit`, or checks out, clones or fetches another
  repository. It judges any directory of workflows, names each refusal by its file and place,
  refuses a directory with no workflow as VOID, and proves itself on planted workflows under
  `scripts/tests/fixtures/secrets-and-checkouts/` (SPEC-034 R7).
- The test's own workflow reader fails closed: a value or key in a form it does not read as YAML
  does, a quoted escape, an anchor, alias or tag among them, is refused by its line. A checkout of
  another repository named in any case, and a fetch in git's scp-like form, are refused too, and
  every workflow test reads `.yml` and `.yaml` files (SPEC-034 A13).
