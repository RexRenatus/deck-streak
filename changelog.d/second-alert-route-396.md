### Added

- A second route tells the owner when the alert sender fails or goes silent (SPEC-396, ADR-410;
  #285). A timer of its own runs `deploy/scripts/second-route.sh` as a oneshot unit with its own two
  credentials, `second-route-check-in` and `second-route-report`: it reads the alert path from the
  service manager, reports each failed alert instance once by its invocation id, an absent or
  masked alert template and an unreadable answer to the receiver off the host, records the keys
  only after the report is delivered, and checks in, withholding the check-in after an unreadable
  read. The census, the host budget, the rail contract and the deploy README carry the unit, and
  the TLA+ entry `formal/tla/SecondRoute` checks its three properties with a witness for each.
  The formal checker's per-run cap in `config/formal.json` rises for every entry, and the entry
  gains a budget of its own.
