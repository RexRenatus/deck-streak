### Added

- The host scrub (SPEC-060, ADR-060). `deploy/host-scrub/inventory.py` reads a host with a read-only
  allow list of commands, each under `nice` and `ionice -c3`, and counts a hard-linked file once;
  `plan.py` lists each obsolete item with the rule and reason that selected it, the bytes it frees
  and its digest; `apply.py` runs dry by default and deletes only what the owner approved, with every
  check passing before the first deletion (and each deletion measuring its item again and stopping
  the run there, earlier deletions kept), after checking the list's digest, the rules the inventory read, a boot-disk snapshot
  taken after the inventory, a host clock that reads synchronised, an item that lies on another
  device or is or holds a mount point or lies inside a bind mount, the protected paths, links out of an item and every item's digest
  again, and it stops the scrub when a health check turns red. It runs only read commands as health
  checks, refuses a path it does not read canonically, and deletes nothing it did not check first: each
  item is read again, through directories opened without following a link, just before it goes.
  `docs/runbooks/host-scrub.md` is the order of work with its rollback, and
  `deploy/host-scrub/rules.example.json` gives the rules' shape with neutral values.
