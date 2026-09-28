### Added

- The host scrub (SPEC-060, ADR-060). `deploy/host-scrub/inventory.py` reads a host with a read-only
  allow list of commands, each under `nice` and `ionice -c3`, and counts a hard-linked file once;
  `plan.py` lists each obsolete item with the rule and reason that selected it, the bytes it frees
  and its digest; `apply.py` runs dry by default and deletes only what the owner approved, all or
  nothing, after checking the list's digest, a boot-disk snapshot taken after the inventory, the
  protected paths, links out of an item and every item's digest again, and it stops the scrub when a
  health check turns red. `docs/runbooks/host-scrub.md` is the order of work with its rollback, and
  `deploy/host-scrub/rules.example.json` gives the rules' shape with neutral values.
